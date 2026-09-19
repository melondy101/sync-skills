// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::discovery::{self, ToolTemplate};
use crate::models::Tool;
use crate::paths;
use crate::DbState;
use tauri::State;

#[tauri::command]
pub fn list_tools(db: State<DbState>) -> Result<Vec<Tool>, String> {
    db.list_tools()
}

#[tauri::command]
pub fn add_tool(
    db: State<DbState>,
    name: String,
    global_path: String,
    project_rel_path: String,
) -> Result<Tool, String> {
    // Validate the shape, store what the user typed: paths stay in `~/…` form so
    // the seeded templates and the display text keep matching.
    let global_path = require_input_path(&global_path)?;
    let project_rel_path = paths::require_configurable_rel_path(&project_rel_path)?;
    let tool = db.add_tool(&name, &global_path, &project_rel_path)?;
    let _ = db.insert_action_log("add_tool", None, Some(tool.id), 0, "success", Some(&name));
    Ok(tool)
}

#[tauri::command]
pub fn update_tool_path(
    db: State<DbState>,
    tool_id: i64,
    global_path: String,
    project_rel_path: String,
) -> Result<(), String> {
    let global_path = require_input_path(&global_path)?;
    let project_rel_path = paths::require_configurable_rel_path(&project_rel_path)?;
    db.update_tool_path(tool_id, &global_path, &project_rel_path)
}

/// A tool's global path must be resolvable; the trimmed input is what gets
/// stored, so `~/…` stays `~/…` on screen and in the database.
fn require_input_path(raw: &str) -> Result<String, String> {
    paths::require_resolvable_path(raw)?;
    Ok(raw.trim().to_string())
}

#[tauri::command]
pub fn delete_tool(db: State<DbState>, tool_id: i64, tool_name: Option<String>) -> Result<(), String> {
    let name = tool_name.unwrap_or_else(|| {
        db.get_tool_name(tool_id).unwrap_or_else(|_| format!("id={}", tool_id))
    });
    db.delete_tool(tool_id)?;
    let _ = db.insert_action_log("delete_tool", None, None, 0, "success", Some(&name));
    Ok(())
}

#[tauri::command]
pub fn list_tool_templates() -> Vec<ToolTemplate> {
    discovery::get_all_templates()
}

#[tauri::command]
pub fn discover_tools(db: State<DbState>) -> Result<Vec<ToolTemplate>, String> {
    discovery::discover_tools(&db)
}
