// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::models::{ConflictView, SyncResult};
use crate::{fs, sync, DbState, LockState};
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn list_conflicts(db: State<DbState>, project_id: Option<i64>) -> Result<Vec<ConflictView>, String> {
    let pid = project_id.unwrap_or(0);
    db.list_unresolved_conflicts(pid)
}

#[tauri::command]
pub async fn resolve_conflict(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    conflict_id: i64,
    keep_tool_name: String,
    project_id: Option<i64>,
) -> Result<SyncResult, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<SyncResult, String> {
        // Get the conflict to find the skill
        let conflicts = db.list_unresolved_conflicts(pid)?;
        let conflict = conflicts.iter().find(|c| c.id == conflict_id)
            .ok_or_else(|| format!("Conflict {} not found", conflict_id))?;

        let skill_id = conflict.skill_id;
        let skill = db.get_skill_by_id(skill_id)?;
        // Serialize with other sync operations on this skill
        let _lock = locks.acquire_blocking(pid, &skill.name);

        // Find the version to keep
        let keep_version = conflict.versions.iter()
            .find(|v| v.tool_name == keep_tool_name)
            .ok_or_else(|| format!("Tool '{}' not found in conflict versions", keep_tool_name))?;

        let source = PathBuf::from(&keep_version.source_path);
        if !source.exists() {
            return Err(format!("Source path does not exist: {}", keep_version.source_path));
        }

        // Sync the kept version to SSOT
        sync::ensure_ssot_dir()?;
        let ssot_target = sync::ssot_path(&skill.name, pid)?;

        if ssot_target.exists() {
            fs::replace_directory(&source, &ssot_target)?;
        } else {
            fs::copy_directory(&source, &ssot_target)?;
        }
        let _ = sync::create_local_marker(&ssot_target);

        // Propagate from SSOT to all other active installations
        let installations = db.get_active_installation_paths(skill_id, pid)?;
        let mut synced_to = 0usize;
        let mut errors = Vec::new();

        for (tool_id, target_path) in &installations {
            let expanded = match crate::scanner::expand_path(target_path) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(format!("Path expansion failed for tool {}: {}", tool_id, e));
                    continue;
                }
            };
            let target_dir = expanded.join(&skill.name);

            let result = if target_dir.exists() {
                fs::replace_directory(&ssot_target, &target_dir).map(|_| "replace".to_string())
            } else {
                fs::copy_directory(&ssot_target, &target_dir).map(|_| "copy".to_string())
            };

            match result {
                Ok(_) => {
                    db.update_synced_at(skill_id, *tool_id, pid)?;
                    db.insert_sync_log(skill_id, *tool_id, pid, "from_ssot", "success", None)?;
                    synced_to += 1;
                }
                Err(e) => {
                    errors.push(format!("Tool {}: {}", tool_id, e));
                    db.insert_sync_log(skill_id, *tool_id, pid, "from_ssot", "failed", Some(&e))?;
                }
            }
        }

        // Refresh hashes from SSOT (the canonical copy after conflict resolution)
        if let Ok(new_content_hash) = crate::hash::compute_content_hash(&ssot_target) {
            let skill_md = ssot_target.join("SKILL.md");
            let new_core_hash = crate::hash::compute_core_hash(&skill_md).unwrap_or_default();
            let _ = db.update_skill_hashes(skill_id, &new_content_hash, &new_core_hash);
        }
        // Update source_path to SSOT
        let ssot_str = ssot_target.to_string_lossy().to_string();
        let _ = db.update_skill_source_path(skill_id, &ssot_str);

        // Mark conflict as resolved
        db.resolve_conflict_record(conflict_id, &keep_tool_name)?;

        Ok(SyncResult {
            skill_id,
            skill_name: skill.name,
            synced_to,
            errors,
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}
