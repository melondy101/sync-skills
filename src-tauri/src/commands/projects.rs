// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::models::Project;
use crate::paths;
use crate::workspaces::{self, WorkspaceCandidate};
use crate::DbState;
use tauri::State;

#[tauri::command]
pub fn list_projects(db: State<DbState>) -> Result<Vec<Project>, String> {
    db.list_projects()
}

/// Workspaces the local coding tools remember, offered for import. Proposes only
/// — nothing is added to the database until the user accepts a candidate.
#[tauri::command]
pub fn discover_workspaces(db: State<DbState>) -> Result<Vec<WorkspaceCandidate>, String> {
    workspaces::discover_workspaces(&db)
}

#[tauri::command]
pub fn add_project(db: State<DbState>, name: String, path: String) -> Result<Project, String> {
    let path = validated_project_path(&path)?;
    let project = db.add_project(&name, &path)?;
    let _ = db.insert_action_log("add_project", None, None, project.id, "success", Some(&format!("{} ({})", name, path)));
    Ok(project)
}

#[tauri::command]
pub fn delete_project(db: State<DbState>, project_id: i64) -> Result<(), String> {
    // Log before deletion so we can capture the project name
    let project_name = db.get_project_name(project_id).unwrap_or_else(|_| format!("id={}", project_id));
    db.delete_project(project_id)?;
    let _ = db.insert_action_log("delete_project", None, None, 0, "success", Some(&project_name));
    Ok(())
}

#[tauri::command]
pub fn update_project(db: State<DbState>, project_id: i64, name: String, path: String) -> Result<(), String> {
    let path = validated_project_path(&path)?;
    db.update_project(project_id, &name, &path)?;
    let _ = db.insert_action_log("edit_project", None, None, project_id, "success", Some(&format!("{} ({})", name, path)));
    Ok(())
}

/// Unlike a tool path — which may point somewhere the tool has not created yet —
/// a project root has to be a directory that is already there.
fn validated_project_path(raw: &str) -> Result<String, String> {
    let check = paths::check_user_path(raw)?;
    if !check.is_dir {
        return Err(format!("path-not-found:{}", check.expanded));
    }
    Ok(check.input)
}
