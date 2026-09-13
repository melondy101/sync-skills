// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Sync & action log CRUD.
//! Extracted from db.rs for maintainability.

use super::Database;
use crate::models::SyncLog;
use rusqlite::params;

impl Database {
    pub fn insert_sync_log(
        &self,
        skill_id: Option<i64>,
        tool_id: Option<i64>,
        project_id: i64,
        action: &str,
        direction: Option<&str>,
        status: &str,
        detail: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT INTO sync_logs (skill_id, tool_id, project_id, action, direction, status, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![skill_id, tool_id, project_id, action, direction, status, detail],
        )
        .map_err(|e| format!("Failed to insert sync log: {}", e))?;
        Ok(())
    }

    pub fn insert_action_log(
        &self,
        skill_id: Option<i64>,
        tool_id: Option<i64>,
        project_id: i64,
        action: &str,
        status: &str,
        detail: Option<&str>,
    ) -> Result<(), String> {
        self.insert_sync_log(skill_id, tool_id, project_id, action, None, status, detail)
    }

    pub fn get_sync_logs(
        &self,
        limit: usize,
        offset: usize,
        action_filter: Option<&str>,
    ) -> Result<Vec<SyncLog>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let (where_clause, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            if let Some(filter) = action_filter {
                ("WHERE action = ?1".into(), vec![Box::new(filter.to_string())])
            } else {
                (String::new(), vec![])
            };

        let sql = format!(
            "SELECT id, skill_id, tool_id, project_id, action, direction, status, detail, created_at
             FROM sync_logs {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
            where_clause,
            if action_filter.is_some() { 2 } else { 1 },
            if action_filter.is_some() { 3 } else { 2 },
        );

        let mut stmt = conn.prepare(&sql).map_err(|e| format!("Query prepare failed: {}", e))?;

        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = params_vec;
        all_params.push(Box::new(limit as i64));
        all_params.push(Box::new(offset as i64));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(SyncLog {
                    id: row.get(0)?,
                    skill_id: row.get(1)?,
                    tool_id: row.get(2)?,
                    project_id: row.get(3)?,
                    action: row.get(4)?,
                    direction: row.get(5)?,
                    status: row.get(6)?,
                    detail: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn get_recent_logs(&self, limit: usize) -> Result<Vec<SyncLog>, String> {
        self.get_sync_logs(limit, 0, None)
    }

    pub fn clear_logs(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute("DELETE FROM sync_logs", [])
            .map_err(|e| format!("Failed to clear logs: {}", e))?;
        Ok(())
    }
}