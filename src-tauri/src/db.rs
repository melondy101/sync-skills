// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Database module root — struct definition + constructors.
// Methods are organized into submodules under db/.

use crate::hash::compute_id_hash;
use crate::models::Project;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

pub mod logs;
pub mod market;
pub mod remote_skills;
pub mod schema;
pub mod skills;
pub mod tools;

/// Database wrapper with mutex for thread safety
pub struct Database {
    pub(crate) conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the database at the default location
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create DB directory: {}", e))?;
        }
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;
        Self::from_connection(conn)
    }

    /// Open an in-memory database (tests only)
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to open in-memory database: {}", e))?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, String> {
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

    // ── Projects ────────────────────────────────────────────────

    pub fn list_projects(&self) -> Result<Vec<Project>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, name, path, created_at FROM projects ORDER BY id")
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn upsert_project(&self, name: &str, path: &str) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let id = compute_id_hash(path);
        conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, path) VALUES (?1, ?2, ?3)",
            params![id, name, path],
        )
        .map_err(|e| format!("Failed to upsert project: {}", e))?;
        Ok(id)
    }

    pub fn delete_project(&self, project_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute("DELETE FROM projects WHERE id = ?1", params![project_id])
            .map_err(|e| format!("Failed to delete project: {}", e))?;
        Ok(())
    }

    pub fn get_project_name(&self, project_id: i64) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.query_row(
            "SELECT name FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get project name: {}", e))
    }
}