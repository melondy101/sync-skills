// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::models::ScanResult;
use crate::{ops, DbState};
use tauri::State;

#[tauri::command]
pub async fn full_scan(db: State<'_, DbState>) -> Result<ScanResult, String> {
    let db = db.inner().clone();

    tokio::task::spawn_blocking(move || -> Result<ScanResult, String> {
        let tool_paths = db.get_tool_paths()?;
        ops::scan_tool_paths(&db, &tool_paths, 0)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn scan_scope(
    db: State<'_, DbState>,
    tool_id: Option<i64>,
    project_id: Option<i64>,
) -> Result<ScanResult, String> {
    let db = db.inner().clone();

    tokio::task::spawn_blocking(move || -> Result<ScanResult, String> {
        let pid = project_id.unwrap_or(0);

        // Determine scan paths based on scope
        let all_tool_paths = if pid != 0 {
            // Project-level scan: use project path + tool's project_rel_path
            let project_path = db.get_project_path(pid)?;
            db.get_project_tool_paths(&project_path)?
        } else {
            // Global scan: use global paths
            db.get_tool_paths()?
        };

        let tool_paths = match tool_id {
            Some(tid) => all_tool_paths
                .into_iter()
                .filter(|(id, _, _)| *id == tid)
                .collect::<Vec<_>>(),
            None => all_tool_paths,
        };

        ops::scan_tool_paths(&db, &tool_paths, pid)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}
