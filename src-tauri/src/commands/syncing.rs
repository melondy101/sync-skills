// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::diff::{self, SkillDiff};
use crate::models::{SkillUpdate, SyncResult};
use crate::settings::Settings;
use crate::{ops, sync, DbState, LockState};
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn toggle_skill(
    db: State<DbState>,
    skill_id: i64,
    tool_id: i64,
    project_id: i64,
    active: bool,
) -> Result<(), String> {
    db.toggle_installation(skill_id, tool_id, project_id, active)?;

    // When disabling, immediately remove the installed skill from the tool directory
    if !active {
        if let Ok(skill) = db.get_skill_by_id(skill_id) {
            // Resolve the tool's installation path for this project scope
            if let Ok(paths) = db.get_disabled_installation_paths(skill_id, project_id) {
                for (tid, target_path) in &paths {
                    if *tid != tool_id {
                        continue;
                    }
                    if let Ok(expanded) = crate::scanner::expand_path(target_path) {
                        let target_dir = expanded.join(&skill.name);
                        let _ = sync::remove_installed_skill(&target_dir);
                    }
                }
            }
        }
    }

    // Log the toggle action
    let action = if active { "toggle_on" } else { "toggle_off" };
    let _ = db.insert_action_log(action, Some(skill_id), Some(tool_id), project_id, "success", None);

    Ok(())
}

#[tauri::command]
pub async fn sync_skill(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    skill_id: i64,
    project_id: Option<i64>,
    source_path: Option<String>,
) -> Result<SyncResult, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let settings = Settings::load();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<SyncResult, String> {
        ops::do_sync_skill(&db, &locks, skill_id, pid, settings.prefer_symlink, source_path.as_deref())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn sync_all_pending(db: State<'_, DbState>, locks: State<'_, LockState>) -> Result<Vec<SyncResult>, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let settings = Settings::load();

    tokio::task::spawn_blocking(move || -> Result<Vec<SyncResult>, String> {
        let skills = db.list_skills()?;
        let projects = db.list_projects()?;
        let mut results = Vec::new();

        // For each project (including global id=0), check active installations
        for project in &projects {
            for skill in &skills {
                let installations = db.get_active_installations(skill.id, project.id)?;
                if !installations.is_empty() {
                    match ops::do_sync_skill(&db, &locks, skill.id, project.id, settings.prefer_symlink, None) {
                        Ok(result) => results.push(result),
                        Err(e) => {
                            results.push(SyncResult {
                                skill_id: skill.id,
                                skill_name: skill.name.clone(),
                                synced_to: 0,
                                errors: vec![e],
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn check_updates(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    project_id: Option<i64>,
) -> Result<Vec<SkillUpdate>, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<Vec<SkillUpdate>, String> {
        // P0-2 fix: only check skills in the specified project
        let skills = db.get_project_skills_for_update_check(pid)?;
        let mut updates = Vec::new();

        for (skill_id, _skill_name, _source_path, _old_hash) in &skills {
            // P0-3 fix: collect ALL divergent tools per skill
            match ops::check_single_skill(&db, &locks, *skill_id, pid) {
                Ok(mut skill_updates) => updates.append(&mut skill_updates),
                Err(_) => continue,
            }
        }

        Ok(updates)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn check_skill_update(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    skill_id: i64,
    project_id: Option<i64>,
) -> Result<Vec<SkillUpdate>, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || ops::check_single_skill(&db, &locks, skill_id, pid))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn get_skill_diff(
    db: State<'_, DbState>,
    skill_id: i64,
    source_path: Option<String>,
) -> Result<SkillDiff, String> {
    let db = db.inner().clone();

    tokio::task::spawn_blocking(move || -> Result<SkillDiff, String> {
        let skill = db.get_skill_by_id(skill_id)?;
        let source = PathBuf::from(
            source_path.as_deref().unwrap_or(&skill.source_path),
        );
        let ssot = sync::ssot_path(&skill.name, skill.project_id)?;

        diff::compute_skill_diff(&source, &ssot, &skill.name)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

// ==================== Reverse Sync & Dismiss ====================

/// Push SSOT content to a specific tool directory (overwrite tool with SSOT version).
#[tauri::command]
pub async fn reverse_sync_skill(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    skill_id: i64,
    tool_id: i64,
    project_id: Option<i64>,
) -> Result<SyncResult, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<SyncResult, String> {
        let skill = db.get_skill_by_id(skill_id)?;
        // Serialize with other sync operations on this skill
        let _lock = locks.acquire_blocking(pid, &skill.name);
        let ssot = sync::ssot_path(&skill.name, pid)?;

        if !ssot.exists() {
            return Err("SSOT directory does not exist. Sync to SSOT first.".to_string());
        }

        // Find the target tool's installation directory
        let installations = db.get_all_active_paths(skill_id)?;
        let mut synced_to = 0usize;
        let mut errors = Vec::new();

        for (tid, _tool_name, install_path) in &installations {
            if *tid != tool_id {
                continue;
            }
            let target = PathBuf::from(install_path);
            let result = if target.exists() {
                sync::replace_directory(&ssot, &target)
            } else {
                sync::copy_directory(&ssot, &target)
            };

            match result {
                Ok(_) => {
                    db.update_synced_at(skill_id, *tid, pid)?;
                    db.insert_sync_log(skill_id, *tid, pid, "reverse_sync", "success", None)?;
                    synced_to += 1;
                }
                Err(e) => {
                    errors.push(format!("Tool {}: {}", tid, e));
                    db.insert_sync_log(skill_id, *tid, pid, "reverse_sync", "failed", Some(&e))?;
                }
            }
        }

        // Refresh hashes from SSOT and clear dismissed updates
        if synced_to > 0 {
            if let Ok(new_content_hash) = crate::hash::compute_content_hash(&ssot) {
                let skill_md = ssot.join("SKILL.md");
                let new_core_hash = crate::hash::compute_core_hash(&skill_md).unwrap_or_default();
                let _ = db.update_skill_hashes(skill_id, &new_content_hash, &new_core_hash);
            }
            let _ = db.clear_dismissed_updates_for_skill(skill_id);
        }

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

/// Dismiss a specific tool's change for a skill (ignore this update).
#[tauri::command]
pub async fn dismiss_skill_update(
    db: State<'_, DbState>,
    skill_id: i64,
    tool_id: i64,
    current_hash: String,
) -> Result<(), String> {
    let db = db.inner().clone();
    db.dismiss_update(skill_id, tool_id, &current_hash)
        .map_err(|e| format!("Failed to dismiss update: {}", e))
}
