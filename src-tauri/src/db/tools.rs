// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Tool CRUD operations.
//! Extracted from db.rs for maintainability.

use super::Database;
use crate::hash::compute_id_hash;
use crate::models::{Tool, ToolTemplate, TOOL_TEMPLATES};
use rusqlite::params;

impl Database {
    pub fn list_tools(&self) -> Result<Vec<Tool>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, name, global_path, project_rel_path, created_at, updated_at FROM tools ORDER BY id")
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
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
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn add_tool(&self, name: &str, global_path: &str, project_rel_path: &str) -> Result<Tool, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let id = compute_id_hash(name);
        conn.execute(
            "INSERT INTO tools (id, name, global_path, project_rel_path) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, global_path, project_rel_path],
        )
        .map_err(|e| format!("Failed to add tool: {}", e))?;
        Ok(Tool {
            id,
            name: name.to_string(),
            global_path: global_path.to_string(),
            project_rel_path: project_rel_path.to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        })
    }

    pub fn update_tool_path(&self, tool_id: i64, global_path: &str, project_rel_path: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let affected = conn
            .execute(
                "UPDATE tools SET global_path = ?1, project_rel_path = ?2, updated_at = datetime('now') WHERE id = ?3",
                params![global_path, project_rel_path, tool_id],
            )
            .map_err(|e| format!("Failed to update tool path: {}", e))?;
        if affected == 0 {
            return Err("Tool not found".to_string());
        }
        Ok(())
    }

    pub fn delete_tool(&self, tool_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute("DELETE FROM tools WHERE id = ?1", params![tool_id])
            .map_err(|e| format!("Failed to delete tool: {}", e))?;
        Ok(())
    }

    pub fn get_tool_name(&self, tool_id: i64) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.query_row(
            "SELECT name FROM tools WHERE id = ?1",
            params![tool_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get tool name: {}", e))
    }

    pub fn get_tool_paths(&self) -> Result<Vec<(i64, String, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, global_path, project_rel_path FROM tools")
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn list_tool_templates(&self) -> Vec<ToolTemplate> {
        TOOL_TEMPLATES.to_vec()
    }
}