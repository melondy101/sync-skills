// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hash::compute_id_hash;
use crate::models::{ConflictVersion, ConflictView, InstallationInfo, Project, Skill, SkillView, SyncLog, Tool};
use crate::scanner;
use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

/// Database wrapper with mutex for thread safety
pub struct Database {
    pub(crate) conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the database at the default location
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create DB directory: {}", e))?;
        }

        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        Self::from_connection(conn)
    }

    /// Open an in-memory database (tests only — never touches the user's real DB)
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to open in-memory database: {}", e))?;
        Self::from_connection(conn)
    }

    /// Shared initialization: pragmas, schema, seed data
    fn from_connection(conn: Connection) -> Result<Self, String> {
        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;

        let db = Database {
            conn: Mutex::new(conn),
        };

        db.init_schema()?;
        db.seed_data()?;

        Ok(db)
    }

    fn db_path() -> Result<PathBuf, String> {
        crate::app_paths::database_path()
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<(), String> {
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
            ",
        )
        .map_err(|e| format!("Failed to create schema: {}", e))?;

        // Migrate sync_logs: check if old schema exists (has 'direction' CHECK but no 'action' column)
        let needs_migration: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('sync_logs') WHERE name = 'action'")
            .and_then(|mut stmt| {
                stmt.query_row([], |row| row.get::<_, i64>(0))
            })
            .map(|count| count == 0) // action column missing → needs migration
            .unwrap_or(true); // table doesn't exist → will be created fresh

        if needs_migration {
            // Check if old table exists
            let old_exists: bool = conn
                .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sync_logs'")
                .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
                .map(|c| c > 0)
                .unwrap_or(false);

            if old_exists {
                // Migrate: rename old table, create new, copy data, drop old
                conn.execute_batch(
                    "
                    ALTER TABLE sync_logs RENAME TO sync_logs_old;
                    ",
                )
                .map_err(|e| format!("Migration rename failed: {}", e))?;
            }
            // else: no old table, just create fresh below

            // Create new sync_logs with expanded schema
            conn.execute_batch(
                "
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
                ",
            )
            .map_err(|e| format!("Migration create failed: {}", e))?;

            if old_exists {
                // Copy data from old table
                conn.execute_batch(
                    "
                    INSERT INTO sync_logs (skill_id, tool_id, project_id, action, direction, status, detail, created_at)
                    SELECT skill_id, tool_id, project_id, 'sync', direction, status, error_message, created_at
                    FROM sync_logs_old;

                    DROP TABLE sync_logs_old;
                    ",
                )
                .map_err(|e| format!("Migration copy failed: {}", e))?;
            }
        }

        // Migrate: normalize source_path in skills table (strip \\?\ prefix on Windows)
        #[cfg(target_os = "windows")]
        {
            conn.execute_batch(
                "UPDATE skills SET source_path = SUBSTR(source_path, 5)
                 WHERE source_path LIKE '\\\\?\\%';",
            )
            .map_err(|e| format!("Source path migration failed: {}", e))?;
        }

        // Migrate: detect old (pre-JS-safe) tool IDs and clear stale data.
        // Old IDs were full i64 values that exceed JS Number.MAX_SAFE_INTEGER.
        // If any old seed tool is found, wipe all data so seed_data() can
        // re-populate with new JS-safe IDs.
        {
            let old_seed_ids: [i64; 5] = [
                -768412307910267356,  // old Claude Code
                -5387663353590988835, // old Codex CLI
                8996106060148633658,  // old OpenCode
                -1843142830140024973, // old Gemini CLI
                6180056807951602058,  // old Cline
            ];
            let placeholders = old_seed_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!("SELECT COUNT(*) FROM tools WHERE id IN ({})", placeholders);
            let count: i64 = conn
                .query_row(&query, rusqlite::params_from_iter(old_seed_ids.iter()), |row| row.get(0))
                .unwrap_or(0);
            if count > 0 {
                // Old IDs detected — clear all data (FK cascades handle dependencies)
                conn.execute_batch(
                    "DELETE FROM skill_installations;
                     DELETE FROM sync_logs;
                     DELETE FROM skills;
                     DELETE FROM tools;
                     DELETE FROM projects;",
                )
                .map_err(|e| format!("Stale data migration failed: {}", e))?;
            }
        }

        // Migrate: add core_hash column to skills table if missing
        {
            let has_core_hash: bool = conn
                .prepare("SELECT COUNT(*) FROM pragma_table_info('skills') WHERE name = 'core_hash'")
                .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
                .map(|count| count > 0)
                .unwrap_or(false);

            if !has_core_hash {
                conn.execute_batch(
                    "ALTER TABLE skills ADD COLUMN core_hash TEXT NOT NULL DEFAULT '';"
                ).map_err(|e| format!("Failed to add core_hash column: {}", e))?;

                // Backfill: compute core_hash for existing skills from their SKILL.md
                let skills_to_backfill: Vec<(i64, String)> = {
                    let mut stmt = conn
                        .prepare("SELECT id, source_path FROM skills WHERE core_hash = ''")
                        .map_err(|e| format!("Backfill query failed: {}", e))?;
                    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                        .map_err(|e| format!("Backfill query_map failed: {}", e))?;
                    let result: Vec<(i64, String)> = rows.filter_map(|r| r.ok()).collect();
                    result
                };

                for (id, source_path) in skills_to_backfill {
                    let skill_md = std::path::Path::new(&source_path).join("SKILL.md");
                    if let Ok(hash) = crate::hash::compute_core_hash(&skill_md) {
                        let _ = conn.execute(
                            "UPDATE skills SET core_hash = ?1 WHERE id = ?2",
                            params![hash, id],
                        );
                    }
                }

                // Create unique index on core_hash (excluding empty strings)
                let _ = conn.execute_batch(
                    "CREATE UNIQUE INDEX IF NOT EXISTS idx_skills_core_hash ON skills(core_hash) WHERE core_hash != '';"
                );
            }
        }

        // ==================== M4 Migration: Name-as-Identity ====================

        // Check if skills table needs M4 migration (project_id column missing)
        let needs_m4: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('skills') WHERE name = 'project_id'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count == 0)
            .unwrap_or(true);

        if needs_m4 {
            // 1. Add new columns to skills
            conn.execute_batch(
                "ALTER TABLE skills ADD COLUMN project_id INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE skills ADD COLUMN ssot_updated_at TEXT;",
            ).map_err(|e| format!("M4 add skills columns failed: {}", e))?;

            // 2. Merge duplicate skills with same name+project_id.
            //    Keep the most recently updated record, remap installations, delete rest.
            let dup_ids: Vec<i64> = conn.prepare(
                "SELECT id FROM skills WHERE id NOT IN (
                    SELECT MAX(id) FROM skills GROUP BY name, project_id
                )"
            ).map_err(|e| format!("M4 dup query failed: {}", e))?
              .query_map([], |row| row.get(0))
              .map_err(|e| format!("M4 dup query_map failed: {}", e))?
              .filter_map(|r| r.ok())
              .collect();

            for dup_id in &dup_ids {
                // Find the keeper (MAX(id) for the same name+project_id)
                let keeper: Option<i64> = conn.query_row(
                    "SELECT MAX(id) FROM skills WHERE id != ?1 AND name = (
                        SELECT name FROM skills WHERE id = ?1
                    ) AND project_id = (
                        SELECT project_id FROM skills WHERE id = ?1
                    )",
                    params![dup_id], |row| row.get(0),
                ).ok().flatten();

                if let Some(kid) = keeper {
                    // Remap installations (skip conflicts with keeper)
                    conn.execute(
                        "UPDATE OR IGNORE skill_installations SET skill_id = ?2 WHERE skill_id = ?1",
                        params![dup_id, kid],
                    ).map_err(|e| format!("M4 remap installations failed: {}", e))?;
                    // Delete remaining installations that conflicted
                    conn.execute("DELETE FROM skill_installations WHERE skill_id = ?1", params![dup_id])
                        .map_err(|e| format!("M4 delete installations failed: {}", e))?;
                    // Remap sync_logs
                    conn.execute(
                        "UPDATE sync_logs SET skill_id = ?2 WHERE skill_id = ?1",
                        params![dup_id, kid],
                    ).map_err(|e| format!("M4 remap sync_logs failed: {}", e))?;
                }
                // Delete the duplicate skill record
                conn.execute("DELETE FROM skills WHERE id = ?1", params![dup_id])
                    .map_err(|e| format!("M4 delete duplicate skill failed: {}", e))?;
            }

            // 3. Drop old UNIQUE constraint on source_path via table rebuild
            conn.execute_batch(
                "CREATE TABLE skills_new (
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
                INSERT INTO skills_new (id, name, description, source_path, content_hash, core_hash, project_id, ssot_updated_at, created_at, updated_at)
                    SELECT id, name, description, source_path, content_hash, core_hash, project_id, ssot_updated_at, created_at, updated_at FROM skills;
                DROP TABLE skills;
                ALTER TABLE skills_new RENAME TO skills;
                CREATE INDEX idx_skills_name ON skills(name);",
            ).map_err(|e| format!("M4 rebuild skills table failed: {}", e))?;

            // 4. Add installation_synced_at to skill_installations
            let has_isa: bool = conn
                .prepare("SELECT COUNT(*) FROM pragma_table_info('skill_installations') WHERE name = 'installation_synced_at'")
                .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
                .map(|count| count > 0)
                .unwrap_or(false);
            if !has_isa {
                conn.execute_batch(
                    "ALTER TABLE skill_installations ADD COLUMN installation_synced_at TEXT;"
                ).map_err(|e| format!("M4 add installation_synced_at failed: {}", e))?;
                // Backfill from synced_at
                conn.execute_batch(
                    "UPDATE skill_installations SET installation_synced_at = synced_at WHERE synced_at IS NOT NULL;"
                ).map_err(|e| format!("M4 backfill installation_synced_at failed: {}", e))?;
            }

            // 5. Drop old core_hash unique index (no longer needed — name is identity)
            conn.execute_batch("DROP INDEX IF EXISTS idx_skills_core_hash;")
                .map_err(|e| format!("M4 drop core_hash index failed: {}", e))?;
        }

        // Create skill_conflicts table.
        // NOTE: must run unconditionally — fresh databases skip the M4 branch
        // (skills is created with project_id already), so creating it only
        // inside `needs_m4` would leave new installs without this table.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS skill_conflicts (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                skill_id        INTEGER NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
                detected_at     TEXT    NOT NULL DEFAULT (datetime('now')),
                resolved_at     TEXT,
                resolved_by     TEXT,
                detail          TEXT,
                created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_conflicts_skill ON skill_conflicts(skill_id);
            CREATE INDEX IF NOT EXISTS idx_conflicts_unresolved ON skill_conflicts(skill_id) WHERE resolved_at IS NULL;",
        ).map_err(|e| format!("Create skill_conflicts failed: {}", e))?;

        // Create dismissed_updates table (for ignoring specific tool changes)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dismissed_updates (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                skill_id        INTEGER NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
                tool_id         INTEGER NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
                dismissed_hash  TEXT    NOT NULL,
                dismissed_at    TEXT    NOT NULL DEFAULT (datetime('now')),
                UNIQUE(skill_id, tool_id)
            );
            CREATE INDEX IF NOT EXISTS idx_dismissed_skill ON dismissed_updates(skill_id);
            CREATE INDEX IF NOT EXISTS idx_dismissed_tool ON dismissed_updates(tool_id);",
        )
        .map_err(|e| format!("Failed to create dismissed_updates table: {}", e))?;

        // Create remote market metadata tables.
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS markets (
                id              INTEGER PRIMARY KEY,
                provider        TEXT    NOT NULL DEFAULT 'github',
                owner           TEXT    NOT NULL,
                name            TEXT    NOT NULL,
                branch          TEXT    NOT NULL DEFAULT 'main',
                enabled         INTEGER NOT NULL DEFAULT 1,
                remote_url      TEXT    NOT NULL,
                last_indexed_at TEXT,
                last_checked_at TEXT,
                last_commit_sha TEXT,
                layout          TEXT    NOT NULL DEFAULT 'subdir',
                created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
                updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),
                UNIQUE(provider, owner, name, branch)
            );

            CREATE TABLE IF NOT EXISTS remote_skills (
                id                  INTEGER PRIMARY KEY,
                market_id           INTEGER NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
                skill_name          TEXT    NOT NULL,
                description         TEXT,
                remote_url          TEXT    NOT NULL,
                ssot_path           TEXT    NOT NULL,
                remote_content_hash TEXT    NOT NULL DEFAULT '',
                remote_core_hash    TEXT    NOT NULL DEFAULT '',
                is_installed        INTEGER NOT NULL DEFAULT 0,
                installed_at        TEXT,
                created_at          TEXT    NOT NULL DEFAULT (datetime('now')),
                updated_at          TEXT    NOT NULL DEFAULT (datetime('now')),
                UNIQUE(market_id, skill_name)
            );

            CREATE INDEX IF NOT EXISTS idx_remote_skills_market ON remote_skills(market_id);
            CREATE INDEX IF NOT EXISTS idx_remote_skills_name ON remote_skills(skill_name);
            ",
        )
        .map_err(|e| format!("Failed to create market tables: {}", e))?;

        let has_remote_url: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('markets') WHERE name = 'remote_url'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count > 0)
            .unwrap_or(false);

        if !has_remote_url {
            conn.execute_batch(
                "ALTER TABLE markets ADD COLUMN remote_url TEXT NOT NULL DEFAULT '';",
            )
            .map_err(|e| format!("Failed to migrate markets remote_url: {}", e))?;
        }

        let has_commit_sha: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('markets') WHERE name = 'last_commit_sha'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count > 0)
            .unwrap_or(false);

        if !has_commit_sha {
            conn.execute_batch(
                "ALTER TABLE markets ADD COLUMN last_commit_sha TEXT;",
            )
            .map_err(|e| format!("Failed to migrate markets last_commit_sha: {}", e))?;
        }

        let has_layout: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('markets') WHERE name = 'layout'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count > 0)
            .unwrap_or(false);
        if !has_layout {
            conn.execute_batch(
                "ALTER TABLE markets ADD COLUMN layout TEXT NOT NULL DEFAULT 'subdir';",
            )
            .map_err(|e| format!("Failed to migrate markets layout: {}", e))?;
        }

        // skills.source_market_id: tracks which remote market a skill was
        // installed from, so the global/project view can show a provenance
        // badge. Nullable (skills can still originate from local tools), and
        // uses ON DELETE SET NULL so removing a market just clears the badge.
        let has_skill_source_market: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('skills') WHERE name = 'source_market_id'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count > 0)
            .unwrap_or(false);
        if !has_skill_source_market {
            conn.execute_batch(
                "ALTER TABLE skills ADD COLUMN source_market_id INTEGER REFERENCES markets(id) ON DELETE SET NULL;",
            )
            .map_err(|e| format!("Failed to migrate skills source_market_id: {}", e))?;
        }

        Ok(())
    }

    /// Insert seed data (preset tools and global project)
    fn seed_data(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Insert global project
        conn.execute(
            "INSERT OR IGNORE INTO projects (id, name, path) VALUES (0, 'Global', '~/.agents/skills/')",
            [],
        )
        .map_err(|e| format!("Failed to seed global project: {}", e))?;

        // Preset tools with pre-computed hash IDs (JS-safe, < 2^53)
        let tools = [
            (6206827997457956i64, "Claude Code", "~/.claude/skills/", ".claude/skills/"),
            (7648999998865373i64, "Codex CLI", "~/.codex/skills/", ".codex/skills/"),
            (6921203917123642i64, "OpenCode", "~/.opencode/skills/", ".opencode/skills/"),
            (3333017081878387i64, "Gemini CLI", "~/.gemini/skills/", ".gemini/skills/"),
            (1118119199281546i64, "Cline", "~/.cline/skills/", ".cline/skills/"),
        ];

        for (id, name, global_path, project_rel_path) in tools {
            conn.execute(
                "INSERT OR IGNORE INTO tools (id, name, global_path, project_rel_path) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, global_path, project_rel_path],
            )
            .map_err(|e| format!("Failed to seed tool {}: {}", name, e))?;
        }

        Ok(())
    }

    /// List all tools
    pub fn list_tools(&self) -> Result<Vec<Tool>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT id, name, global_path, project_rel_path, created_at, updated_at FROM tools ORDER BY name")
            .map_err(|e| format!("Prepare error: {}", e))?;

        let tools = stmt
            .query_map([], |row| {
                Ok(Tool {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    global_path: row.get(2)?,
                    project_rel_path: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tools)
    }

    /// Add a new tool
    pub fn add_tool(&self, name: &str, global_path: &str, project_rel_path: &str) -> Result<Tool, String> {
        let id = compute_id_hash(name);
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.execute(
            "INSERT INTO tools (id, name, global_path, project_rel_path) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, global_path, project_rel_path],
        )
        .map_err(|e| format!("Failed to add tool: {}", e))?;

        // Fetch the inserted row to get created_at/updated_at from SQLite
        conn.query_row(
            "SELECT id, name, global_path, project_rel_path, created_at, updated_at FROM tools WHERE id = ?1",
            params![id],
            |row| {
                Ok(Tool {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    global_path: row.get(2)?,
                    project_rel_path: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .map_err(|e| format!("Failed to fetch inserted tool: {}", e))
    }

    /// Update tool path
    pub fn update_tool_path(
        &self,
        tool_id: i64,
        global_path: &str,
        project_rel_path: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.execute(
            "UPDATE tools SET global_path = ?1, project_rel_path = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![global_path, project_rel_path, tool_id],
        )
        .map_err(|e| format!("Failed to update tool: {}", e))?;

        Ok(())
    }

    /// Delete a tool (cascades to installations and sync_logs)
    pub fn delete_tool(&self, tool_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.execute("DELETE FROM tools WHERE id = ?1", params![tool_id])
            .map_err(|e| format!("Failed to delete tool: {}", e))?;

        Ok(())
    }

    /// Upsert a skill by name + project_id (name-as-identity model).
    /// Returns (skill_id, is_new).
    pub fn upsert_skill(
        &self,
        name: &str,
        description: Option<&str>,
        source_path: &str,
        content_hash: &str,
        core_hash: &str,
        project_id: i64,
    ) -> Result<(i64, bool), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Look up by (name, project_id) — the new identity
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM skills WHERE name = ?1 AND project_id = ?2",
                params![name, project_id],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            // Update existing skill record
            conn.execute(
                "UPDATE skills SET description = ?1, source_path = ?2, content_hash = ?3,
                 core_hash = ?4, ssot_updated_at = datetime('now'), updated_at = datetime('now')
                 WHERE id = ?5",
                params![description, source_path, content_hash, core_hash, id],
            )
            .map_err(|e| format!("Failed to update skill: {}", e))?;
            return Ok((id, false));
        }

        // Brand new skill — insert
        let id_input = format!("{}:{}", project_id, name);
        let id = compute_id_hash(&id_input);
        conn.execute(
            "INSERT INTO skills (id, name, description, source_path, content_hash, core_hash, project_id, ssot_updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))",
            params![id, name, description, source_path, content_hash, core_hash, project_id],
        )
        .map_err(|e| format!("Failed to insert skill: {}", e))?;
        Ok((id, true))
    }

    /// Upsert a skill with a remote-market provenance tag. Used by the market
    /// sync/install flows so the resulting `skills.source_market_id` is set.
    /// Behaves exactly like `upsert_skill` for the rest; on a hit, only the
    /// provenance column is refreshed when it was previously null (we don't
    /// want to silently steal a badge that a different market already claimed).
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_skill_from_market(
        &self,
        name: &str,
        description: Option<&str>,
        source_path: &str,
        content_hash: &str,
        core_hash: &str,
        project_id: i64,
        source_market_id: i64,
    ) -> Result<(i64, bool), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let existing: Option<(i64, Option<i64>)> = conn
            .query_row(
                "SELECT id, source_market_id FROM skills WHERE name = ?1 AND project_id = ?2",
                params![name, project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((id, existing_market)) = existing {
            conn.execute(
                "UPDATE skills SET description = ?1, source_path = ?2, content_hash = ?3,
                 core_hash = ?4, ssot_updated_at = datetime('now'), updated_at = datetime('now'),
                 source_market_id = COALESCE(source_market_id, ?5)
                 WHERE id = ?6",
                params![description, source_path, content_hash, core_hash, source_market_id, id],
            )
            .map_err(|e| format!("Failed to update skill: {}", e))?;
            let _ = existing_market;
            return Ok((id, false));
        }

        let id_input = format!("{}:{}", project_id, name);
        let id = compute_id_hash(&id_input);
        conn.execute(
            "INSERT INTO skills (id, name, description, source_path, content_hash, core_hash, project_id, ssot_updated_at, source_market_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), ?8)",
            params![id, name, description, source_path, content_hash, core_hash, project_id, source_market_id],
        )
        .map_err(|e| format!("Failed to insert skill: {}", e))?;
        Ok((id, true))
    }

    /// List all skills
    pub fn list_skills(&self) -> Result<Vec<Skill>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT id, name, description, source_path, content_hash, core_hash, project_id, ssot_updated_at, source_market_id, created_at, updated_at FROM skills ORDER BY name")
            .map_err(|e| format!("Prepare error: {}", e))?;

        let skills = stmt
            .query_map([], |row| {
                Ok(Skill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source_path: row.get(3)?,
                    content_hash: row.get(4)?,
                    core_hash: row.get(5)?,
                    project_id: row.get(6)?,
                    ssot_updated_at: row.get(7)?,
                    source_market_id: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(skills)
    }

    /// Get tool's resolved paths (global_path, project_rel_path)
    pub fn get_tool_paths(&self) -> Result<Vec<(i64, String, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT id, global_path, project_rel_path FROM tools")
            .map_err(|e| format!("Prepare error: {}", e))?;

        let paths = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(paths)
    }

    /// Get skill's existing content_hash
    #[allow(dead_code)] // Reserved for future use
    pub fn get_skill_hash(&self, source_path: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let result = conn.query_row(
            "SELECT content_hash FROM skills WHERE source_path = ?1",
            params![source_path],
            |row| row.get(0),
        );

        match result {
            Ok(hash) => Ok(Some(hash)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Query error: {}", e)),
        }
    }

    // ==================== M2: Sync & Installation ====================

    /// Get a single skill by ID
    pub fn get_skill_by_id(&self, skill_id: i64) -> Result<Skill, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.query_row(
            "SELECT id, name, description, source_path, content_hash, core_hash, project_id, ssot_updated_at, source_market_id, created_at, updated_at FROM skills WHERE id = ?1",
            params![skill_id],
            |row| {
                Ok(Skill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source_path: row.get(3)?,
                    content_hash: row.get(4)?,
                    core_hash: row.get(5)?,
                    project_id: row.get(6)?,
                    ssot_updated_at: row.get(7)?,
                    source_market_id: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .map_err(|e| format!("Failed to get skill: {}", e))
    }

    /// List skills with installation status (SkillView)
    /// Filters by project_id directly (name-as-identity model).
    pub fn list_skills_with_status(&self, project_id: i64) -> Result<Vec<SkillView>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Step 1: Get skills filtered by project_id
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, source_path, content_hash, core_hash, project_id, ssot_updated_at, source_market_id, created_at, updated_at
                 FROM skills WHERE project_id = ?1 ORDER BY name",
            )
            .map_err(|e| format!("Prepare error: {}", e))?;

        let skills: Vec<Skill> = stmt
            .query_map(params![project_id], |row| {
                Ok(Skill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source_path: row.get(3)?,
                    content_hash: row.get(4)?,
                    core_hash: row.get(5)?,
                    project_id: row.get(6)?,
                    ssot_updated_at: row.get(7)?,
                    source_market_id: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        // Step 2: For each skill, get installation info for the given project_id
        let mut views = Vec::new();
        for skill in skills {
            let mut inst_stmt = conn
                .prepare(
                    "SELECT si.tool_id, t.name, si.status, si.synced_at, si.installation_synced_at
                     FROM skill_installations si
                     JOIN tools t ON t.id = si.tool_id
                     WHERE si.skill_id = ?1 AND si.project_id = ?2",
                )
                .map_err(|e| format!("Prepare error: {}", e))?;

            let installations: Vec<InstallationInfo> = inst_stmt
                .query_map(params![skill.id, project_id], |row| {
                    Ok(InstallationInfo {
                        tool_id: row.get(0)?,
                        tool_name: row.get(1)?,
                        status: row.get(2)?,
                        synced_at: row.get(3)?,
                        installation_synced_at: row.get(4)?,
                    })
                })
                .map_err(|e| format!("Query error: {}", e))?
                .filter_map(|r| r.ok())
                .collect();

            let install_count = installations.iter().filter(|i| i.status == "active").count();

            let has_update = false; // Will be set by check_updates logic

            views.push(SkillView {
                skill,
                installed_tools: installations,
                install_count,
                has_update,
            });
        }

        Ok(views)
    }

    /// Toggle skill installation status (enable/disable)
    pub fn toggle_installation(
        &self,
        skill_id: i64,
        tool_id: i64,
        project_id: i64,
        active: bool,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let status = if active { "active" } else { "disabled" };

        if active {
            conn.execute(
                "INSERT INTO skill_installations (skill_id, tool_id, project_id, status)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(skill_id, tool_id, project_id) DO UPDATE SET
                    status = ?4,
                    updated_at = datetime('now')",
                params![skill_id, tool_id, project_id, status],
            )
            .map_err(|e| format!("Failed to toggle installation: {}", e))?;
        } else {
            // When disabling, clear synced_at so the skill appears as never-synced
            conn.execute(
                "INSERT INTO skill_installations (skill_id, tool_id, project_id, status)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(skill_id, tool_id, project_id) DO UPDATE SET
                    status = ?4,
                    synced_at = NULL,
                    installation_synced_at = NULL,
                    updated_at = datetime('now')",
                params![skill_id, tool_id, project_id, status],
            )
            .map_err(|e| format!("Failed to toggle installation: {}", e))?;
        }

        Ok(())
    }

    /// Auto-detect installation: insert an active record only if no record exists yet.
    /// Used during scan to mark skills that physically exist in a tool's directory.
    /// Does NOT override existing records (preserves user's explicit toggle-off).
    pub fn ensure_installation(
        &self,
        skill_id: i64,
        tool_id: i64,
        project_id: i64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT OR IGNORE INTO skill_installations (skill_id, tool_id, project_id, status)
             VALUES (?1, ?2, ?3, 'active')",
            params![skill_id, tool_id, project_id],
        )
        .map_err(|e| format!("Failed to ensure installation: {}", e))?;
        Ok(())
    }

    /// Get all active installations for a skill (for sync targeting)
    pub fn get_active_installations(&self, skill_id: i64, project_id: i64) -> Result<Vec<(i64, String)>, String> {
        self.get_installations_by_status(skill_id, project_id, "active")
    }

    /// Installations for a skill filtered by status, paired with the tool's global_path
    fn get_installations_by_status(&self, skill_id: i64, project_id: i64, status: &str) -> Result<Vec<(i64, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn
            .prepare(
                "SELECT si.tool_id, t.global_path
                 FROM skill_installations si
                 JOIN tools t ON t.id = si.tool_id
                 WHERE si.skill_id = ?1 AND si.project_id = ?2 AND si.status = ?3",
            )
            .map_err(|e| format!("Prepare error: {}", e))?;

        let results = stmt
            .query_map(params![skill_id, project_id, status], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Get active installation target paths, resolved per project scope.
    /// - project_id == 0 (Global): returns tool's global_path (e.g. ~/.claude/skills/)
    /// - project_id > 0 (Project): returns project_path + tool's project_rel_path
    pub fn get_active_installation_paths(
        &self,
        skill_id: i64,
        project_id: i64,
    ) -> Result<Vec<(i64, String)>, String> {
        self.get_installation_paths_by_status(skill_id, project_id, "active")
    }

    /// Get disabled installation target paths (same resolution as active).
    /// Used on sync to remove stale copies from tools the user has unchecked.
    pub fn get_disabled_installation_paths(
        &self,
        skill_id: i64,
        project_id: i64,
    ) -> Result<Vec<(i64, String)>, String> {
        self.get_installation_paths_by_status(skill_id, project_id, "disabled")
    }

    fn get_installation_paths_by_status(
        &self,
        skill_id: i64,
        project_id: i64,
        status: &str,
    ) -> Result<Vec<(i64, String)>, String> {
        if project_id == 0 {
            // Global: use global_path directly
            return self.get_installations_by_status(skill_id, 0, status);
        }

        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Get project base path
        let project_path: String = conn
            .query_row("SELECT path FROM projects WHERE id = ?1", params![project_id], |row| row.get(0))
            .map_err(|e| format!("Failed to get project path: {}", e))?;
        let project_base = scanner::expand_path(&project_path)?;

        let mut stmt = conn
            .prepare(
                "SELECT si.tool_id, t.project_rel_path
                 FROM skill_installations si
                 JOIN tools t ON t.id = si.tool_id
                 WHERE si.skill_id = ?1 AND si.project_id = ?2 AND si.status = ?3",
            )
            .map_err(|e| format!("Prepare error: {}", e))?;

        let results = stmt
            .query_map(params![skill_id, project_id, status], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .map(|(tool_id, rel_path)| {
                let full_path = if rel_path.is_empty() {
                    scanner::normalize_path(&project_base)
                } else {
                    scanner::normalize_path(&project_base.join(&rel_path))
                };
                (tool_id, full_path)
            })
            .collect();

        Ok(results)
    }

    /// Update synced_at and installation_synced_at timestamps for an installation
    pub fn update_synced_at(&self, skill_id: i64, tool_id: i64, project_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.execute(
            "UPDATE skill_installations SET synced_at = datetime('now'), installation_synced_at = datetime('now'), updated_at = datetime('now')
             WHERE skill_id = ?1 AND tool_id = ?2 AND project_id = ?3",
            params![skill_id, tool_id, project_id],
        )
        .map_err(|e| format!("Failed to update synced_at: {}", e))?;

        Ok(())
    }

    /// Refresh stored hashes for a skill after sync (clears update indicator)
    /// Also updates ssot_updated_at timestamp.
    pub fn update_skill_hashes(
        &self,
        skill_id: i64,
        content_hash: &str,
        core_hash: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE skills SET content_hash = ?1, core_hash = ?2, ssot_updated_at = datetime('now'), updated_at = datetime('now')
             WHERE id = ?3",
            params![content_hash, core_hash, skill_id],
        )
        .map_err(|e| format!("Failed to update skill hashes: {}", e))?;
        Ok(())
    }

    /// Update the stored source_path for a skill (e.g., after sync points to SSOT).
    pub fn update_skill_source_path(
        &self,
        skill_id: i64,
        source_path: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE skills SET source_path = ?1, updated_at = datetime('now')
             WHERE id = ?2",
            params![source_path, skill_id],
        )
        .map_err(|e| format!("Failed to update skill source_path: {}", e))?;
        Ok(())
    }

    /// Get all active installation paths for a skill across ALL projects.
    /// Returns Vec<(tool_id, tool_name, absolute_path)>.
    pub fn get_all_active_paths(&self, skill_id: i64) -> Result<Vec<(i64, String, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn
            .prepare(
                "SELECT si.tool_id, t.name, t.global_path, t.project_rel_path,
                        si.project_id, p.path, s.name
                 FROM skill_installations si
                 JOIN tools t ON si.tool_id = t.id
                 JOIN skills s ON si.skill_id = s.id
                 LEFT JOIN projects p ON si.project_id = p.id
                 WHERE si.skill_id = ?1 AND si.status = 'active'"
            )
            .map_err(|e| format!("Prepare error: {}", e))?;

        let results = stmt
            .query_map(params![skill_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| format!("Query error: {}", e))?;

        let mut paths = Vec::new();
        for row in results {
            let (tool_id, tool_name, global_path, rel_path, project_id, project_path, skill_name) =
                row.map_err(|e| format!("Row error: {}", e))?;
            let base = if project_id == 0 {
                scanner::expand_path(&global_path)?
            } else {
                let pp = project_path.unwrap_or_default();
                let expanded = scanner::expand_path(&pp)?;
                expanded.join(&rel_path)
            };
            let skill_dir = base.join(&skill_name);
            paths.push((tool_id, tool_name, skill_dir.to_string_lossy().to_string()));
        }
        Ok(paths)
    }

    /// Insert a sync log entry (for sync operations with direction)
    pub fn insert_sync_log(
        &self,
        skill_id: i64,
        tool_id: i64,
        project_id: i64,
        direction: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.execute(
            "INSERT INTO sync_logs (skill_id, tool_id, project_id, action, direction, status, detail)
             VALUES (?1, ?2, ?3, 'sync', ?4, ?5, ?6)",
            params![skill_id, tool_id, project_id, direction, status, error_message],
        )
        .map_err(|e| format!("Failed to insert sync log: {}", e))?;

        Ok(())
    }

    /// Insert an action log entry (for non-sync operations: scan, toggle, add, delete, etc.)
    pub fn insert_action_log(
        &self,
        action: &str,
        skill_id: Option<i64>,
        tool_id: Option<i64>,
        project_id: i64,
        status: &str,
        detail: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.execute(
            "INSERT INTO sync_logs (skill_id, tool_id, project_id, action, status, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![skill_id, tool_id, project_id, action, status, detail],
        )
        .map_err(|e| format!("Failed to insert action log: {}", e))?;

        Ok(())
    }

    /// Get a tool's name by ID (for logging before deletion)
    pub fn get_tool_name(&self, tool_id: i64) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.query_row(
            "SELECT name FROM tools WHERE id = ?1",
            params![tool_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get tool name: {}", e))
    }

    /// Get a project's name by ID (for logging before deletion)
    pub fn get_project_name(&self, project_id: i64) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.query_row(
            "SELECT name FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get project name: {}", e))
    }

    // ==================== Markets ====================

    pub fn list_markets(&self) -> Result<Vec<crate::models::Market>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, provider, owner, name, branch, enabled, last_indexed_at, last_checked_at, last_commit_sha, created_at, updated_at, layout FROM markets ORDER BY created_at")
            .map_err(|e| format!("Prepare error: {}", e))?;
        let markets = stmt
            .query_map([], |row| {
                Ok(crate::models::Market {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    owner: row.get(2)?,
                    name: row.get(3)?,
                    branch: row.get(4)?,
                    enabled: row.get(5)?,
                    last_indexed_at: row.get(6)?,
                    last_checked_at: row.get(7)?,
                    last_commit_sha: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    layout: row.get::<_, String>(11)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(markets)
    }

    pub fn get_market(&self, market_id: i64) -> Result<Option<crate::models::Market>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, provider, owner, name, branch, enabled, last_indexed_at, last_checked_at, last_commit_sha, created_at, updated_at, layout FROM markets WHERE id = ?1")
            .map_err(|e| format!("Prepare error: {}", e))?;
        let result = stmt
            .query_row(params![market_id], |row| {
                Ok(crate::models::Market {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    owner: row.get(2)?,
                    name: row.get(3)?,
                    branch: row.get(4)?,
                    enabled: row.get(5)?,
                    last_indexed_at: row.get(6)?,
                    last_checked_at: row.get(7)?,
                    last_commit_sha: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    layout: row.get::<_, String>(11)?,
                })
            });
        match result {
            Ok(market) => Ok(Some(market)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Query error: {}", e)),
        }
    }

    /// Look up a market by its natural key instead of by id.
    ///
    /// Legacy databases may contain market rows whose ids were computed with
    /// an older hashing scheme, so after an ON CONFLICT upsert the stored id
    /// can differ from the freshly computed one. Resolving by the unique
    /// natural key works for both new and legacy rows.
    pub fn get_market_by_key(&self, provider: &str, owner: &str, name: &str, branch: &str) -> Result<Option<crate::models::Market>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, provider, owner, name, branch, enabled, last_indexed_at, last_checked_at, last_commit_sha, created_at, updated_at, layout FROM markets WHERE provider = ?1 AND owner = ?2 AND name = ?3 AND branch = ?4")
            .map_err(|e| format!("Prepare error: {}", e))?;
        let result = stmt
            .query_row(params![provider, owner, name, branch], |row| {
                Ok(crate::models::Market {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    owner: row.get(2)?,
                    name: row.get(3)?,
                    branch: row.get(4)?,
                    enabled: row.get(5)?,
                    last_indexed_at: row.get(6)?,
                    last_checked_at: row.get(7)?,
                    last_commit_sha: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    layout: row.get::<_, String>(11)?,
                })
            });
        match result {
            Ok(market) => Ok(Some(market)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Query error: {}", e)),
        }
    }

    #[allow(dead_code)] // Kept as a future entry point; current callers go through `upsert_market`.
    pub fn insert_market(&self, provider: &str, owner: &str, name: &str, branch: &str, remote_url: &str, layout: &str) -> Result<crate::models::Market, String> {
        let id = crate::hash::compute_id_hash(&format!("{}:{}:{}:{}", provider, owner, name, branch));
        {
            let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
            conn.execute(
                "INSERT INTO markets (id, provider, owner, name, branch, remote_url, layout) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, provider, owner, name, branch, remote_url, layout],
            )
            .map_err(|e| format!("Failed to insert market: {}", e))?;
        }
        // Lock must be released before the follow-up query, which acquires it
        // again (std::sync::Mutex is not reentrant; nesting deadlocks).
        // Resolve by natural key so legacy rows with older ids still match.
        self.get_market_by_key(provider, owner, name, branch)?
            .ok_or_else(|| "Market not found after insert".to_string())
    }

    pub fn upsert_market(&self, provider: &str, owner: &str, name: &str, branch: &str, remote_url: &str, layout: &str) -> Result<crate::models::Market, String> {
        let id = crate::hash::compute_id_hash(&format!("{}:{}:{}:{}", provider, owner, name, branch));
        {
            let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
            conn.execute(
                "INSERT INTO markets (id, provider, owner, name, branch, remote_url, layout) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(provider, owner, name, branch) DO UPDATE SET remote_url=excluded.remote_url, layout=excluded.layout, updated_at=datetime('now')",
                params![id, provider, owner, name, branch, remote_url, layout],
            )
            .map_err(|e| format!("Failed to upsert market: {}", e))?;
        }
        // Lock must be released before the follow-up query, which acquires it
        // again (std::sync::Mutex is not reentrant; nesting deadlocks).
        // Resolve by natural key: an ON CONFLICT update keeps the stored row's
        // original id, which in legacy databases may differ from the freshly
        // computed one.
        self.get_market_by_key(provider, owner, name, branch)?
            .ok_or_else(|| "Market not found after upsert".to_string())
    }

    pub fn update_market(&self, market_id: i64, enabled: bool, last_indexed_at: Option<&str>, last_checked_at: Option<&str>, last_commit_sha: Option<&str>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE markets SET enabled = ?1, last_indexed_at = ?2, last_checked_at = ?3, last_commit_sha = ?4, updated_at = datetime('now') WHERE id = ?5",
            params![enabled as i64, last_indexed_at, last_checked_at, last_commit_sha, market_id],
        )
        .map_err(|e| format!("Failed to update market: {}", e))?;
        Ok(())
    }

    pub fn delete_market(&self, market_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute("DELETE FROM markets WHERE id = ?1", params![market_id])
            .map_err(|e| format!("Failed to delete market: {}", e))?;
        Ok(())
    }

    /// Persist a layout change for an existing market row. Layout updates are
    /// intentionally kept separate from `update_market` so the rest of the
    /// field set doesn't grow new positional parameters.
    pub fn set_market_layout(&self, market_id: i64, layout: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE markets SET layout = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![layout, market_id],
        )
        .map_err(|e| format!("Failed to update market layout: {}", e))?;
        Ok(())
    }

    #[allow(dead_code, clippy::too_many_arguments)] // Reserved for the remote-market sync pipeline; field-by-field inserts use `update_remote_skill_description`.
    pub fn upsert_remote_skill(&self, market_id: i64, skill_name: &str, description: Option<&str>, remote_url: &str, ssot_path: &str, content_hash: &str, core_hash: &str, installed: bool, installed_at: Option<&str>) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let id = crate::hash::compute_id_hash(&format!("{}:{}", market_id, skill_name));
        conn.execute(
            "INSERT INTO remote_skills (id, market_id, skill_name, description, remote_url, ssot_path, remote_content_hash, remote_core_hash, is_installed, installed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(market_id, skill_name) DO UPDATE SET
                description=excluded.description,
                remote_url=excluded.remote_url,
                ssot_path=excluded.ssot_path,
                remote_content_hash=excluded.remote_content_hash,
                remote_core_hash=excluded.remote_core_hash,
                is_installed=excluded.is_installed,
                installed_at=excluded.installed_at,
                updated_at=datetime('now')",
            params![id, market_id, skill_name, description, remote_url, ssot_path, content_hash, core_hash, installed as i64, installed_at],
        )
        .map_err(|e| format!("Failed to upsert remote skill: {}", e))?;
        Ok(id)
    }

    /// Update only the description of a remote skill row. Used when the SSOT
    /// copy has been freshly downloaded and we want to backfill the index
    /// without overwriting fields the market sync owns.
    pub fn update_remote_skill_description(&self, remote_skill_id: i64, description: Option<&str>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE remote_skills SET description = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![description, remote_skill_id],
        )
        .map_err(|e| format!("Failed to update remote skill description: {}", e))?;
        Ok(())
    }

    pub fn list_remote_skills(&self, market_id: Option<i64>) -> Result<Vec<crate::models::RemoteSkill>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut skills = Vec::new();
        if let Some(mid) = market_id {
            let mut stmt = conn.prepare("SELECT id, market_id, skill_name, description, remote_url, ssot_path, remote_content_hash, remote_core_hash, is_installed, installed_at, created_at, updated_at FROM remote_skills WHERE market_id = ?1 ORDER BY skill_name").map_err(|e| format!("Prepare error: {}", e))?;
            let rows = stmt.query_map(params![mid], remote_skill_row).map_err(|e| format!("Query error: {}", e))?;
            skills.extend(rows.filter_map(|r| r.ok()));
        } else {
            let mut stmt = conn.prepare("SELECT id, market_id, skill_name, description, remote_url, ssot_path, remote_content_hash, remote_core_hash, is_installed, installed_at, created_at, updated_at FROM remote_skills ORDER BY market_id, skill_name").map_err(|e| format!("Prepare error: {}", e))?;
            let rows = stmt.query_map([], remote_skill_row).map_err(|e| format!("Query error: {}", e))?;
            skills.extend(rows.filter_map(|r| r.ok()));
        }
        Ok(skills)
    }

    /// Look up a single remote skill by id. Returns `None` when not found.
    /// Prefer this over `list_remote_skills(None)?.find(...)` to avoid a
    /// full-table scan on every call (the Detail Modal opens often).
    pub fn get_remote_skill(&self, remote_skill_id: i64) -> Result<Option<crate::models::RemoteSkill>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare("SELECT id, market_id, skill_name, description, remote_url, ssot_path, remote_content_hash, remote_core_hash, is_installed, installed_at, created_at, updated_at FROM remote_skills WHERE id = ?1").map_err(|e| format!("Prepare error: {}", e))?;
        let mut rows = stmt.query_map(params![remote_skill_id], remote_skill_row).map_err(|e| format!("Query error: {}", e))?;
        match rows.next() {
            Some(row) => Ok(Some(row.map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    pub fn list_remote_installations(&self, project_id: i64, market_id: Option<i64>) -> Result<Vec<crate::models::RemoteInstallation>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<crate::models::RemoteInstallation> {
            Ok(crate::models::RemoteInstallation {
                id: row.get(0)?,
                remoteSkillId: row.get(0)?,
                skillName: row.get(1)?,
                remoteUrl: row.get(2)?,
                ssotPath: row.get(3)?,
                installedAt: row.get(4)?,
                marketId: row.get(5)?,
                marketTitle: format!("{}/{}", row.get::<_, String>(6)?, row.get::<_, String>(7)?),
                projectId: project_id,
                scope: "global".to_string(),
            })
        };
        if let Some(mid) = market_id {
            let mut stmt = conn.prepare("SELECT rs.id, rs.skill_name, rs.remote_url, rs.ssot_path, rs.installed_at, rs.market_id, m.owner, m.name FROM remote_skills rs JOIN markets m ON m.id = rs.market_id WHERE rs.is_installed = 1 AND rs.market_id = ?1 ORDER BY rs.skill_name").map_err(|e| format!("Prepare error: {}", e))?;
            let rows = stmt.query_map(params![mid], map_row)
                .map_err(|e| format!("Query error: {}", e))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        } else {
            let mut stmt = conn.prepare("SELECT rs.id, rs.skill_name, rs.remote_url, rs.ssot_path, rs.installed_at, rs.market_id, m.owner, m.name FROM remote_skills rs JOIN markets m ON m.id = rs.market_id WHERE rs.is_installed = 1 ORDER BY rs.market_id, rs.skill_name").map_err(|e| format!("Prepare error: {}", e))?;
            let rows = stmt.query_map([], map_row)
                .map_err(|e| format!("Query error: {}", e))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }
    }

    pub fn set_remote_skill_installed(&self, remote_skill_id: i64, installed: bool) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE remote_skills SET is_installed = ?1, installed_at = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![installed as i64, if installed { Some("datetime(now)") } else { None }, remote_skill_id],
        )
        .map_err(|e| format!("Failed to update remote install state: {}", e))?;
        Ok(())
    }

    pub fn list_market_templates(&self) -> Result<Vec<crate::models::MarketTemplate>, String> {
        Ok(vec![
            crate::models::MarketTemplate {
                id: "anthropic-skills".to_string(),
                label: "Anthropic Skills".to_string(),
                description: "Built-in Anthropic market".to_string(),
                provider: "github".to_string(),
                owner: "anthropics".to_string(),
                name: "skills".to_string(),
                branch: "main".to_string(),
                kind: "standard".to_string(),
                root_skill: true,
            },
            crate::models::MarketTemplate {
                id: "superpowers-skills".to_string(),
                label: "Superpowers Skills".to_string(),
                description: "Built-in Superpowers market".to_string(),
                provider: "github".to_string(),
                owner: "superpowers".to_string(),
                name: "skills".to_string(),
                branch: "main".to_string(),
                kind: "standard".to_string(),
                root_skill: true,
            },
        ])
    }

    /// Query sync logs with optional filters
    pub fn get_sync_logs(&self, skill_id: Option<i64>, limit: Option<i64>) -> Result<Vec<SyncLog>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let limit = limit.unwrap_or(100);

        let query = match skill_id {
            Some(sid) => format!(
                "SELECT sl.id, sl.skill_id, s.name, sl.tool_id, t.name, sl.project_id,
                        sl.action, sl.direction, sl.status, sl.detail, sl.created_at
                 FROM sync_logs sl
                 LEFT JOIN skills s ON s.id = sl.skill_id
                 LEFT JOIN tools t ON t.id = sl.tool_id
                 WHERE sl.skill_id = {}
                 ORDER BY sl.created_at DESC LIMIT {}",
                sid, limit
            ),
            None => format!(
                "SELECT sl.id, sl.skill_id, s.name, sl.tool_id, t.name, sl.project_id,
                        sl.action, sl.direction, sl.status, sl.detail, sl.created_at
                 FROM sync_logs sl
                 LEFT JOIN skills s ON s.id = sl.skill_id
                 LEFT JOIN tools t ON t.id = sl.tool_id
                 ORDER BY sl.created_at DESC LIMIT {}",
                limit
            ),
        };

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Prepare error: {}", e))?;

        let logs = stmt
            .query_map([], |row| {
                Ok(SyncLog {
                    id: row.get(0)?,
                    skill_id: row.get(1)?,
                    skill_name: row.get(2)?,
                    tool_id: row.get(3)?,
                    tool_name: row.get(4)?,
                    project_id: row.get(5)?,
                    action: row.get(6)?,
                    direction: row.get(7)?,
                    status: row.get(8)?,
                    detail: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(logs)
    }

    /// Get skills for update check, filtered by project_id
    pub fn get_project_skills_for_update_check(
        &self,
        project_id: i64,
    ) -> Result<Vec<(i64, String, String, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT id, name, source_path, content_hash FROM skills WHERE project_id = ?1")
            .map_err(|e| format!("Prepare error: {}", e))?;

        let skills = stmt
            .query_map(params![project_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(skills)
    }

    // ==================== M3: Project Management ====================

    /// List all projects
    pub fn list_projects(&self) -> Result<Vec<Project>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT id, name, path, created_at FROM projects ORDER BY id")
            .map_err(|e| format!("Prepare error: {}", e))?;

        let projects = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(projects)
    }

    /// Add a new project
    pub fn add_project(&self, name: &str, path: &str) -> Result<Project, String> {
        let id = compute_id_hash(path);
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Check if path already exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM projects WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            return Err("Project with this path already exists".to_string());
        }

        conn.execute(
            "INSERT INTO projects (id, name, path) VALUES (?1, ?2, ?3)",
            params![id, name, path],
        )
        .map_err(|e| format!("Failed to add project: {}", e))?;

        // Fetch the inserted row
        conn.query_row(
            "SELECT id, name, path, created_at FROM projects WHERE id = ?1",
            params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .map_err(|e| format!("Failed to fetch inserted project: {}", e))
    }

    /// Delete a project (cascades to installations and sync_logs)
    pub fn delete_project(&self, project_id: i64) -> Result<(), String> {
        if project_id == 0 {
            return Err("Cannot delete the Global project".to_string());
        }

        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.execute("DELETE FROM projects WHERE id = ?1", params![project_id])
            .map_err(|e| format!("Failed to delete project: {}", e))?;

        Ok(())
    }

    /// Update a project's name and/or path
    pub fn update_project(&self, project_id: i64, name: &str, path: &str) -> Result<(), String> {
        if project_id == 0 {
            return Err("Cannot modify the Global project".to_string());
        }

        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Check path uniqueness (exclude self)
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM projects WHERE path = ?1 AND id != ?2",
            params![path, project_id],
            |row| row.get(0),
        ).map_err(|e| format!("Failed to check path uniqueness: {}", e))?;
        if exists {
            return Err("Another project with this path already exists".to_string());
        }

        conn.execute(
            "UPDATE projects SET name = ?1, path = ?2 WHERE id = ?3",
            params![name, path, project_id],
        )
        .map_err(|e| format!("Failed to update project: {}", e))?;

        Ok(())
    }

    /// Get a project's path by ID
    pub fn get_project_path(&self, project_id: i64) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.query_row(
            "SELECT path FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get project path: {}", e))
    }

    /// Get tool paths scoped to a project (project_path + tool.project_rel_path)
    pub fn get_project_tool_paths(&self, project_path: &str) -> Result<Vec<(i64, String, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT id, global_path, project_rel_path FROM tools")
            .map_err(|e| format!("Prepare error: {}", e))?;

        let project_base = scanner::expand_path(project_path)?;
        let paths = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let _global: String = row.get(1)?;
                let rel: String = row.get(2)?;
                Ok((id, _global, rel))
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .map(|(id, _global, rel)| {
                // Construct full path: project_path / project_rel_path
                let full_path = if rel.is_empty() {
                    scanner::normalize_path(&project_base)
                } else {
                    scanner::normalize_path(&project_base.join(&rel))
                };
                (id, full_path, rel)
            })
            .collect();

        Ok(paths)
    }

    // ==================== M5: Conflict Management ====================

    /// Insert a conflict record for a skill
    pub fn insert_conflict(&self, skill_id: i64, detail: &str) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT INTO skill_conflicts (skill_id, detail) VALUES (?1, ?2)",
            params![skill_id, detail],
        )
        .map_err(|e| format!("Failed to insert conflict: {}", e))?;
        Ok(conn.last_insert_rowid())
    }

    /// List all unresolved conflicts for a given project scope
    pub fn list_unresolved_conflicts(&self, project_id: i64) -> Result<Vec<ConflictView>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Get unresolved conflicts for skills in this project
        let mut stmt = conn.prepare(
            "SELECT sc.id, sc.skill_id, s.name, sc.detected_at, sc.detail
             FROM skill_conflicts sc
             JOIN skills s ON s.id = sc.skill_id
             WHERE sc.resolved_at IS NULL AND s.project_id = ?1
             ORDER BY sc.detected_at DESC",
        ).map_err(|e| format!("Prepare error: {}", e))?;

        let conflicts: Vec<(i64, i64, String, String, String)> = stmt
            .query_map(params![project_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let mut views = Vec::new();
        for (id, skill_id, skill_name, detected_at, detail) in conflicts {
            // Parse versions from detail JSON
            let versions: Vec<ConflictVersion> = serde_json::from_str(&detail).unwrap_or_default();
            views.push(ConflictView {
                id,
                skill_id,
                skill_name,
                detected_at,
                versions,
            });
        }

        Ok(views)
    }

    /// Mark a conflict as resolved
    pub fn resolve_conflict_record(&self, conflict_id: i64, resolved_by: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE skill_conflicts SET resolved_at = datetime('now'), resolved_by = ?1 WHERE id = ?2",
            params![resolved_by, conflict_id],
        )
        .map_err(|e| format!("Failed to resolve conflict: {}", e))?;
        Ok(())
    }

    /// Check if a skill has unresolved conflicts
    pub fn has_unresolved_conflict(&self, skill_id: i64) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM skill_conflicts WHERE skill_id = ?1 AND resolved_at IS NULL",
            params![skill_id],
            |row| row.get(0),
        ).map_err(|e| format!("Query error: {}", e))?;
        Ok(count > 0)
    }

    // ==================== Dismissed Updates ====================

    /// Record that a specific tool's change for a skill has been dismissed.
    /// Stores the current hash so the dismissal is invalidated if the file changes again.
    pub fn dismiss_update(&self, skill_id: i64, tool_id: i64, hash: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT INTO dismissed_updates (skill_id, tool_id, dismissed_hash)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(skill_id, tool_id) DO UPDATE SET
                dismissed_hash = ?3,
                dismissed_at = datetime('now')",
            params![skill_id, tool_id, hash],
        )
        .map_err(|e| format!("Failed to dismiss update: {}", e))?;
        Ok(())
    }

    /// Check if a specific tool's change for a skill is currently dismissed.
    /// Returns true if the dismissed hash matches the current hash (same change).
    pub fn is_update_dismissed(&self, skill_id: i64, tool_id: i64, current_hash: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let result: Option<String> = conn.query_row(
            "SELECT dismissed_hash FROM dismissed_updates WHERE skill_id = ?1 AND tool_id = ?2",
            params![skill_id, tool_id],
            |row| row.get(0),
        ).ok();

        match result {
            Some(dismissed_hash) => Ok(dismissed_hash == current_hash),
            None => Ok(false),
        }
    }

    /// Clear all dismissed updates for a skill (e.g., after sync or new scan)
    pub fn clear_dismissed_updates_for_skill(&self, skill_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "DELETE FROM dismissed_updates WHERE skill_id = ?1",
            params![skill_id],
        )
        .map_err(|e| format!("Failed to clear dismissed updates: {}", e))?;
        Ok(())
    }
}

pub(crate) fn remote_skill_row(row: &rusqlite::Row) -> Result<crate::models::RemoteSkill, rusqlite::Error> {
    Ok(crate::models::RemoteSkill {
        id: row.get(0)?,
        market_id: row.get(1)?,
        skill_name: row.get(2)?,
        description: row.get(3)?,
        remote_url: row.get(4)?,
        ssot_path: row.get(5)?,
        remote_content_hash: row.get(6)?,
        remote_core_hash: row.get(7)?,
        is_installed: row.get(8)?,
        installed_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}
