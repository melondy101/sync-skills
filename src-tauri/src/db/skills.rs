// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Skill CRUD + installation management.
//! Extracted from db.rs for maintainability.

use super::Database;
use crate::hash::compute_id_hash;
use crate::models::{RemoteInstallation, Skill, SkillView, SkillUpdate};
use rusqlite::params;

impl Database {
    pub fn upsert_skill(
        &self,
        name: &str,
        source_path: &str,
        content_hash: &str,
        project_id: i64,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let id = compute_id_hash(source_path);
        conn.execute(
            "INSERT OR REPLACE INTO skills (id, name, source_path, content_hash, project_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![id, name, source_path, content_hash, project_id],
        )
        .map_err(|e| format!("Failed to upsert skill: {}", e))?;
        Ok(id)
    }

    pub fn upsert_skill_from_market(
        &self,
        name: &str,
        source_path: &str,
        content_hash: &str,
        core_hash: &str,
        project_id: i64,
        market_id: i64,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let id = compute_id_hash(source_path);

        // Check if skill already exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM skills WHERE name = ?1 AND project_id = ?2",
                params![name, project_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if exists {
            conn.execute(
                "UPDATE skills SET source_path = ?1, content_hash = ?2, core_hash = ?3, updated_at = datetime('now')
                 WHERE name = ?4 AND project_id = ?5",
                params![source_path, content_hash, core_hash, name, project_id],
            )
            .map_err(|e| format!("Failed to update market skill: {}", e))?;

            // Get the existing id
            let existing_id: i64 = conn
                .query_row(
                    "SELECT id FROM skills WHERE name = ?1 AND project_id = ?2",
                    params![name, project_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to get existing skill id: {}", e))?;
            Ok(existing_id)
        } else {
            // Add source_market_id column — handled in schema migration
            conn.execute(
                "INSERT INTO skills (id, name, source_path, content_hash, core_hash, project_id, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
                params![id, name, source_path, content_hash, core_hash, project_id],
            )
            .map_err(|e| format!("Failed to insert market skill: {}", e))?;
            Ok(id)
        }
    }

    pub fn list_skills(&self) -> Result<Vec<Skill>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, source_path, content_hash, core_hash, project_id, created_at, updated_at FROM skills ORDER BY name")
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Skill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source_path: row.get(3)?,
                    content_hash: row.get(4)?,
                    core_hash: row.get(5)?,
                    project_id: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    source_market_id: None,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn get_skill_by_id(&self, skill_id: i64) -> Result<Skill, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.query_row(
            "SELECT id, name, description, source_path, content_hash, core_hash, project_id, created_at, updated_at FROM skills WHERE id = ?1",
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
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    source_market_id: None,
                })
            },
        )
        .map_err(|e| format!("Failed to get skill: {}", e))
    }

    pub fn get_skill_hash(&self, source_path: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let result = conn.query_row(
            "SELECT content_hash FROM skills WHERE source_path = ?1",
            params![source_path],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(hash) => Ok(Some(hash)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to get skill hash: {}", e)),
        }
    }

    pub fn list_skills_with_status(&self, project_id: i64) -> Result<Vec<SkillView>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.name, s.description, s.source_path, s.content_hash,
                        s.core_hash, s.project_id, s.created_at, s.updated_at,
                        COUNT(CASE WHEN si.status = 'active' THEN 1 END) as active_count,
                        COUNT(CASE WHEN si.status = 'disabled' THEN 1 END) as disabled_count,
                        GROUP_CONCAT(CASE WHEN si.status = 'active' THEN si.tool_id END) as active_tools,
                        GROUP_CONCAT(CASE WHEN si.status = 'disabled' THEN si.tool_id END) as disabled_tools,
                        MAX(si.synced_at) as last_synced,
                        COALESCE(
                          (SELECT MIN(CASE
                            WHEN s.core_hash = '' THEN 'conflict'
                            WHEN s.content_hash != '' AND s.core_hash != '' AND s.content_hash != s.core_hash THEN 'pending'
                            ELSE 'synced'
                          END) FROM skills sub WHERE sub.name = s.name AND sub.project_id = s.project_id),
                          'unknown'
                        ) as sync_status
                 FROM skills s
                 LEFT JOIN skill_installations si ON si.skill_id = s.id AND si.project_id = ?1
                 GROUP BY s.id, s.name
                 ORDER BY s.name",
            )
            .map_err(|e| format!("Query prepare failed: {}", e))?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                let active_tools_str: Option<String> = row.get(11).ok().flatten();
                let disabled_tools_str: Option<String> = row.get(12).ok().flatten();
                let active_tools: Vec<i64> = active_tools_str
                    .as_ref()
                    .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
                    .unwrap_or_default();
                let disabled_tools: Vec<i64> = disabled_tools_str
                    .as_ref()
                    .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
                    .unwrap_or_default();

                Ok(SkillView {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source_path: row.get(3)?,
                    content_hash: row.get(4)?,
                    core_hash: row.get(5)?,
                    project_id: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    active_tool_count: row.get::<_, i64>(9).unwrap_or(0) as usize,
                    disabled_tool_count: row.get::<_, i64>(10).unwrap_or(0) as usize,
                    active_tools,
                    disabled_tools,
                    last_synced: row.get(14).ok().flatten(),
                    sync_status: row.get::<_, String>(15).unwrap_or_default(),
                    source_market_id: None,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn update_skill_hashes(
        &self,
        skill_id: i64,
        content_hash: &str,
        core_hash: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE skills SET content_hash = ?1, core_hash = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![content_hash, core_hash, skill_id],
        )
        .map_err(|e| format!("Failed to update skill hashes: {}", e))?;
        Ok(())
    }

    pub fn update_skill_source_path(&self, skill_id: i64, source_path: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE skills SET source_path = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![source_path, skill_id],
        )
        .map_err(|e| format!("Failed to update skill source path: {}", e))?;
        Ok(())
    }

    pub fn update_content_hash(&self, skill_id: i64, content_hash: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE skills SET content_hash = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![content_hash, skill_id],
        )
        .map_err(|e| format!("Failed to update content hash: {}", e))?;
        Ok(())
    }

    pub fn get_project_skills_for_update_check(
        &self,
        project_id: i64,
    ) -> Result<Vec<Skill>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, source_path, content_hash, core_hash, project_id, created_at, updated_at FROM skills WHERE project_id = ?1 ORDER BY name")
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(Skill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source_path: row.get(3)?,
                    content_hash: row.get(4)?,
                    core_hash: row.get(5)?,
                    project_id: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    source_market_id: None,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    // ── Installations ──────────────────────────────────────

    pub fn toggle_installation(
        &self,
        skill_id: i64,
        tool_id: i64,
        project_id: i64,
        enable: bool,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        if enable {
            conn.execute(
                "INSERT OR REPLACE INTO skill_installations (skill_id, tool_id, project_id, status, updated_at)
                 VALUES (?1, ?2, ?3, 'active', datetime('now'))",
                params![skill_id, tool_id, project_id],
            )
            .map_err(|e| format!("Failed to toggle installation: {}", e))?;
        } else {
            conn.execute(
                "UPDATE skill_installations SET status = 'disabled', updated_at = datetime('now')
                 WHERE skill_id = ?1 AND tool_id = ?2 AND project_id = ?3",
                params![skill_id, tool_id, project_id],
            )
            .map_err(|e| format!("Failed to disable installation: {}", e))?;
        }
        Ok(())
    }

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

    pub fn get_active_installations(
        &self,
        skill_id: i64,
        project_id: i64,
    ) -> Result<Vec<(i64, String)>, String> {
        self.get_installations_by_status(skill_id, project_id, "active")
    }

    fn get_installations_by_status(
        &self,
        skill_id: i64,
        project_id: i64,
        status: &str,
    ) -> Result<Vec<(i64, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT si.tool_id, COALESCE(t.global_path, '') as path
                 FROM skill_installations si
                 JOIN tools t ON t.id = si.tool_id
                 WHERE si.skill_id = ?1 AND si.status = ?2 AND si.project_id = ?3",
            )
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map(params![skill_id, status, project_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn get_active_installation_paths(
        &self,
        skill_id: i64,
        project_id: i64,
    ) -> Result<Vec<(i64, String)>, String> {
        self.get_installation_paths_by_status(skill_id, project_id, "active")
    }

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
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.global_path
                 FROM skill_installations si
                 JOIN tools t ON t.id = si.tool_id
                 WHERE si.skill_id = ?1 AND si.status = ?2 AND si.project_id = ?3",
            )
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map(params![skill_id, status, project_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn update_synced_at(
        &self,
        skill_id: i64,
        tool_id: i64,
        project_id: i64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE skill_installations SET synced_at = datetime('now'), updated_at = datetime('now')
             WHERE skill_id = ?1 AND tool_id = ?2 AND project_id = ?3",
            params![skill_id, tool_id, project_id],
        )
        .map_err(|e| format!("Failed to update synced_at: {}", e))?;
        Ok(())
    }

    pub fn get_all_active_paths(
        &self,
        skill_id: i64,
    ) -> Result<Vec<(i64, String, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT si.tool_id, t.global_path, t.name
                 FROM skill_installations si
                 JOIN tools t ON t.id = si.tool_id
                 WHERE si.skill_id = ?1 AND si.status = 'active'",
            )
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map(params![skill_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn is_update_dismissed(&self, skill_id: i64, tool_id: i64, hash: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dismissed_updates WHERE skill_id = ?1 AND tool_id = ?2 AND hash_value = ?3",
                params![skill_id, tool_id, hash],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to check dismissed update: {}", e))?;
        Ok(count > 0)
    }

    pub fn dismiss_update(&self, skill_id: i64, tool_id: i64, hash: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT OR IGNORE INTO dismissed_updates (skill_id, tool_id, hash_value) VALUES (?1, ?2, ?3)",
            params![skill_id, tool_id, hash],
        )
        .map_err(|e| format!("Failed to dismiss update: {}", e))?;
        Ok(())
    }

    pub fn get_dismissed_hashes(&self, skill_id: i64) -> Result<Vec<(i64, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT tool_id, hash_value FROM dismissed_updates WHERE skill_id = ?1")
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map(params![skill_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn get_dismissed_hashes_for_tool(&self, skill_id: i64, tool_id: i64) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT hash_value FROM dismissed_updates WHERE skill_id = ?1 AND tool_id = ?2")
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map(params![skill_id, tool_id], |row| row.get(0))
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn delete_skill(&self, skill_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute("DELETE FROM skills WHERE id = ?1", params![skill_id])
            .map_err(|e| format!("Failed to delete skill: {}", e))?;
        Ok(())
    }
}