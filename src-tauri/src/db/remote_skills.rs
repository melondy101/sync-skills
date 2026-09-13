// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Remote skill CRUD.
//! Extracted from db.rs for maintainability.

use super::Database;
use crate::models::{RemoteInstallation, RemoteSkill};
use rusqlite::params;

impl Database {
    pub fn upsert_remote_skill(
        &self,
        market_id: i64,
        skill_name: &str,
        file_path: &str,
        content_hash: &str,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let id = crate::hash::compute_id_hash(&format!("{}/{}", market_id, skill_name));
        conn.execute(
            "INSERT OR REPLACE INTO remote_skills (id, market_id, skill_name, file_path, content_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![id, market_id, skill_name, file_path, content_hash],
        )
        .map_err(|e| format!("Failed to upsert remote skill: {}", e))?;
        Ok(id)
    }

    pub fn update_remote_skill_description(
        &self,
        remote_skill_id: i64,
        description: &str,
        tags_str: &str,
        core_hash: &str,
        updated_at: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE remote_skills SET description = ?1, tags = ?2, core_hash = ?3, updated_at = ?4 WHERE id = ?5",
            params![description, tags_str, core_hash, updated_at, remote_skill_id],
        )
        .map_err(|e| format!("Failed to update remote skill description: {}", e))?;
        Ok(())
    }

    pub fn list_remote_skills(
        &self,
        market_id: Option<i64>,
    ) -> Result<Vec<RemoteSkill>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(mid) = market_id {
            (
                "SELECT rs.id, rs.market_id, rs.skill_name, rs.description, rs.tags,
                        rs.file_path, rs.content_hash, rs.core_hash, rs.installed, rs.updated_at
                 FROM remote_skills rs WHERE rs.market_id = ?1 ORDER BY rs.skill_name".into(),
                vec![Box::new(mid)],
            )
        } else {
            (
                "SELECT rs.id, rs.market_id, rs.skill_name, rs.description, rs.tags,
                        rs.file_path, rs.content_hash, rs.core_hash, rs.installed, rs.updated_at
                 FROM remote_skills rs JOIN markets m ON m.id = rs.market_id
                 WHERE m.enabled = 1 ORDER BY rs.skill_name".into(),
                vec![],
            )
        };

        let mut stmt = conn.prepare(&sql).map_err(|e| format!("Query prepare failed: {}", e))?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(RemoteSkill {
                    id: row.get(0)?,
                    market_id: row.get(1)?,
                    skill_name: row.get(2)?,
                    description: row.get(3)?,
                    tags: row.get::<_, Option<String>>(4).ok().flatten(),
                    file_path: row.get(5)?,
                    content_hash: row.get(6)?,
                    core_hash: row.get(7)?,
                    installed: row.get::<_, i64>(8).unwrap_or(0) != 0,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn get_remote_skill(&self, remote_skill_id: i64) -> Result<RemoteSkill, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.query_row(
            "SELECT id, market_id, skill_name, description, tags, file_path, content_hash,
                    core_hash, installed, updated_at
             FROM remote_skills WHERE id = ?1",
            params![remote_skill_id],
            |row| {
                Ok(RemoteSkill {
                    id: row.get(0)?,
                    market_id: row.get(1)?,
                    skill_name: row.get(2)?,
                    description: row.get(3)?,
                    tags: row.get::<_, Option<String>>(4).ok().flatten(),
                    file_path: row.get(5)?,
                    content_hash: row.get(6)?,
                    core_hash: row.get(7)?,
                    installed: row.get::<_, i64>(8).unwrap_or(0) != 0,
                    updated_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| format!("Failed to get remote skill: {}", e))
    }

    pub fn set_remote_skill_installed(
        &self,
        remote_skill_id: i64,
        installed: bool,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE remote_skills SET installed = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![installed as i64, remote_skill_id],
        )
        .map_err(|e| format!("Failed to set remote skill installed: {}", e))?;
        Ok(())
    }

    pub fn list_remote_installations(
        &self,
        project_id: i64,
        market_id: Option<i64>,
    ) -> Result<Vec<RemoteInstallation>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let (where_clause, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(mid) = market_id {
            ("WHERE si.project_id = ?1 AND rs.market_id = ?2".into(),
             vec![Box::new(project_id), Box::new(mid)])
        } else {
            ("WHERE si.project_id = ?1 AND rs.id IS NOT NULL".into(),
             vec![Box::new(project_id)])
        };

        let sql = format!(
            "SELECT s.id, s.name, s.source_path, s.content_hash, s.updated_at, rs.id, rs.market_id, rs.skill_name, rs.description
             FROM skills s
             JOIN skill_installations si ON si.skill_id = s.id AND si.status = 'active'
             JOIN remote_skills rs ON rs.skill_name = s.name AND rs.installed = 1
             {}",
            where_clause
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("Query prepare failed: {}", e))?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(RemoteInstallation {
                    local_skill_id: row.get(0)?,
                    skill_name: row.get(1)?,
                    source_path: row.get(2)?,
                    content_hash: row.get(3)?,
                    updated_at: row.get(4)?,
                    remote_skill_id: row.get(5)?,
                    market_id: row.get(6)?,
                    remote_skill_name: row.get(7)?,
                    description: row.get(8)?,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }
}