// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Schema initialization, migrations, seed data.
//! Extracted from db.rs for maintainability.

use super::Database;
use rusqlite::params;

impl Database {
    /// Initialize database schema
    pub(crate) fn init_schema(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tools (
                id              INTEGER PRIMARY KEY,
                name            TEXT    NOT NULL UNIQUE,
                global_path     TEXT    NOT NULL,
                project_rel_path TEXT   NOT NULL,
                created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
                updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS projects (
                id              INTEGER PRIMARY KEY,
                name            TEXT    NOT NULL,
                path            TEXT    NOT NULL UNIQUE,
                created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_projects_path ON projects(path);

            CREATE TABLE IF NOT EXISTS skills (
                id              INTEGER PRIMARY KEY,
                name            TEXT    NOT NULL,
                description     TEXT,
                source_path     TEXT    NOT NULL,
                content_hash    TEXT    NOT NULL,
                core_hash       TEXT    NOT NULL DEFAULT '',
                project_id      INTEGER NOT NULL DEFAULT 0 REFERENCES projects(id) ON DELETE CASCADE,
                ssot_updated_at TEXT,
                created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
                updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),
                UNIQUE(name, project_id)
            );

            CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);

            CREATE TABLE IF NOT EXISTS skill_installations (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                skill_id        INTEGER NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
                tool_id         INTEGER NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
                project_id      INTEGER NOT NULL DEFAULT 0 REFERENCES projects(id) ON DELETE CASCADE,
                status          TEXT    NOT NULL DEFAULT 'disabled'
                    CHECK (status IN ('active', 'disabled')),
                synced_at       TEXT,
                installation_synced_at TEXT,
                created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
                updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),
                UNIQUE (skill_id, tool_id, project_id)
            );

            CREATE INDEX IF NOT EXISTS idx_inst_skill ON skill_installations(skill_id);
            CREATE INDEX IF NOT EXISTS idx_inst_tool ON skill_installations(tool_id);
            CREATE INDEX IF NOT EXISTS idx_inst_project ON skill_installations(project_id);
            CREATE INDEX IF NOT EXISTS idx_inst_status ON skill_installations(status);

            CREATE TABLE IF NOT EXISTS sync_logs (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                skill_id        INTEGER REFERENCES skills(id) ON DELETE SET NULL,
                tool_id         INTEGER REFERENCES tools(id) ON DELETE SET NULL,
                project_id      INTEGER NOT NULL DEFAULT 0 REFERENCES projects(id) ON DELETE CASCADE,
                action          TEXT    NOT NULL DEFAULT 'sync',
                direction       TEXT,
                status          TEXT    NOT NULL CHECK (status IN ('success', 'failed')),
                detail          TEXT,
                created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_log_skill ON sync_logs(skill_id);
            CREATE INDEX IF NOT EXISTS idx_log_project ON sync_logs(project_id);
            CREATE INDEX IF NOT EXISTS idx_log_action ON sync_logs(action);
            CREATE INDEX IF NOT EXISTS idx_log_status ON sync_logs(status);
            CREATE INDEX IF NOT EXISTS idx_log_created ON sync_logs(created_at);

            CREATE TABLE IF NOT EXISTS dismissed_updates (
                skill_id        INTEGER NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
                tool_id         INTEGER NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
                hash_value      TEXT    NOT NULL,
                dismissed_at    TEXT    NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (skill_id, tool_id, hash_value)
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key             TEXT    PRIMARY KEY,
                value           TEXT    NOT NULL
            );
            ",
        )
        .map_err(|e| format!("Failed to create schema: {}", e))?;

        self.run_schema_migrations(&conn)?;
        self.run_tool_id_migration(&conn)?;
        self.run_core_hash_migration(&conn)?;
        self.run_m4_name_identity_migration(&conn)?;
        self.run_remote_skill_migrations(&conn)?;
        Ok(())
    }

    fn run_schema_migrations(&self, conn: &rusqlite::Connection) -> Result<(), String> {
        // Migrate sync_logs: check if old schema exists
        let needs_migration: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('sync_logs') WHERE name = 'action'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count == 0)
            .unwrap_or(true);

        if needs_migration {
            let old_exists: bool = conn
                .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sync_logs'")
                .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
                .map(|c| c > 0)
                .unwrap_or(false);

            if old_exists {
                conn.execute_batch("ALTER TABLE sync_logs RENAME TO sync_logs_old;")
                    .map_err(|e| format!("Migration rename failed: {}", e))?;
            }

            // created fresh above
            if old_exists {
                conn.execute_batch(
                    "INSERT INTO sync_logs (skill_id, tool_id, project_id, action, direction, status, detail, created_at)
                     SELECT skill_id, tool_id, project_id, 'sync', direction, status, error_message, created_at
                     FROM sync_logs_old; DROP TABLE sync_logs_old;",
                )
                .map_err(|e| format!("Migration copy failed: {}", e))?;
            }
        }

        // Normalize source_path in skills (strip \\?\ prefix on Windows)
        #[cfg(target_os = "windows")]
        {
            conn.execute_batch("UPDATE skills SET source_path = SUBSTR(source_path, 5) WHERE source_path LIKE '\\\\?\\%';")
                .map_err(|e| format!("Source path migration failed: {}", e))?;
        }
        Ok(())
    }

    fn run_tool_id_migration(&self, conn: &rusqlite::Connection) -> Result<(), String> {
        // Old tool IDs exceeded JS Number.MAX_SAFE_INTEGER — wipe if detected
        let old_seed_ids: [i64; 5] = [
            -768412307910267356,
            -5387663353590988835,
            8996106060148633658,
            -1843142830140024973,
            6180056807951602058,
        ];
        let placeholders = old_seed_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT COUNT(*) FROM tools WHERE id IN ({})", placeholders);
        let count: i64 = conn
            .query_row(&query, rusqlite::params_from_iter(old_seed_ids.iter()), |row| row.get(0))
            .unwrap_or(0);
        if count > 0 {
            conn.execute_batch(
                "DELETE FROM skill_installations; DELETE FROM sync_logs;
                 DELETE FROM skills; DELETE FROM tools; DELETE FROM projects;",
            )
            .map_err(|e| format!("Stale data migration failed: {}", e))?;
        }
        Ok(())
    }

    fn run_core_hash_migration(&self, conn: &rusqlite::Connection) -> Result<(), String> {
        let has_core_hash: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('skills') WHERE name = 'core_hash'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count > 0)
            .unwrap_or(false);

        if !has_core_hash {
            conn.execute_batch("ALTER TABLE skills ADD COLUMN core_hash TEXT NOT NULL DEFAULT '';")
                .map_err(|e| format!("Failed to add core_hash: {}", e))?;

            let skills_to_backfill: Vec<(i64, String)> = {
                let mut stmt = conn
                    .prepare("SELECT id, source_path FROM skills WHERE core_hash = ''")
                    .map_err(|e| format!("Backfill query failed: {}", e))?;
                let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(|e| format!("Backfill query_map failed: {}", e))?;
                rows.filter_map(|r| r.ok()).collect()
            };

            for (id, source_path) in skills_to_backfill {
                let skill_md = std::path::Path::new(&source_path).join("SKILL.md");
                if let Ok(hash) = crate::hash::compute_core_hash(&skill_md) {
                    let _ = conn.execute("UPDATE skills SET core_hash = ?1 WHERE id = ?2", params![hash, id]);
                }
            }
            let _ = conn.execute_batch(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_skills_core_hash ON skills(core_hash) WHERE core_hash != '';"
            );
        }
        Ok(())
    }

    fn run_m4_name_identity_migration(&self, conn: &rusqlite::Connection) -> Result<(), String> {
        let needs_m4: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('skills') WHERE name = 'project_id'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count == 0)
            .unwrap_or(true);

        if !needs_m4 {
            return Ok(());
        }

        conn.execute_batch(
            "ALTER TABLE skills ADD COLUMN project_id INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE skills ADD COLUMN ssot_updated_at TEXT;",
        ).map_err(|e| format!("M4 add columns failed: {}", e))?;

        // Merge duplicate skills with same name+project_id
        let dup_ids: Vec<i64> = conn
            .prepare("SELECT id FROM skills WHERE id NOT IN (SELECT MAX(id) FROM skills GROUP BY name, project_id)")
            .map_err(|e| format!("M4 dup query failed: {}", e))?
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("M4 dup query_map failed: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        for dup_id in &dup_ids {
            let keeper_id: Option<i64> = conn
                .query_row(
                    "SELECT MAX(id) FROM skills WHERE name = (SELECT name FROM skills WHERE id = ?1)
                     AND project_id = (SELECT project_id FROM skills WHERE id = ?1)",
                    params![dup_id],
                    |row| row.get(0),
                )
                .ok();
            if let Some(keeper) = keeper_id {
                let _ = conn.execute(
                    "UPDATE skill_installations SET skill_id = ?1 WHERE skill_id = ?2",
                    params![keeper, dup_id],
                );
                let _ = conn.execute(
                    "UPDATE sync_logs SET skill_id = ?1 WHERE skill_id = ?2",
                    params![keeper, dup_id],
                );
                let _ = conn.execute("DELETE FROM skills WHERE id = ?1", params![dup_id]);
            }
        }
        Ok(())
    }

    fn run_remote_skill_migrations(&self, conn: &rusqlite::Connection) -> Result<(), String> {
        let has_remote_skills: bool = conn
            .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='remote_skills'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|c| c > 0)
            .unwrap_or(false);

        if has_remote_skills {
            let has_core_hash_col: bool = conn
                .prepare("SELECT COUNT(*) FROM pragma_table_info('remote_skills') WHERE name = 'core_hash'")
                .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
                .map(|count| count > 0)
                .unwrap_or(false);

            if !has_core_hash_col {
                conn.execute_batch("ALTER TABLE remote_skills ADD COLUMN core_hash TEXT NOT NULL DEFAULT '';")
                    .map_err(|e| format!("Failed to add remote_skills.core_hash: {}", e))?;
            }
        }
        Ok(())
    }

    /// Seed initial tools data
    pub(crate) fn seed_data(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let known_tools = crate::models::TOOL_TEMPLATES;
        for tpl in known_tools {
            let id = compute_id_hash(tpl.name);
            conn.execute(
                "INSERT OR IGNORE INTO tools (id, name, global_path, project_rel_path) VALUES (?1, ?2, ?3, ?4)",
                params![id, tpl.name, tpl.global_path, tpl.project_rel_path],
            )
            .map_err(|e| format!("Failed to seed tool '{}': {}", tpl.name, e))?;
        }
        Ok(())
    }
}