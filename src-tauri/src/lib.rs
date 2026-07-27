// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

mod db;
mod diff;
mod discovery;
#[cfg(test)]
mod edge_tests;
mod hash;
mod lock;
mod models;
mod scanner;
mod settings;
mod sync;

use db::Database;
use diff::SkillDiff;
use discovery::ToolTemplate;
use lock::LockManager;
use models::{
    ConflictView, Project, ScanDetail, ScanResult, SkillUpdate, SkillView, SyncLog, SyncResult, Tool,
};
use settings::Settings;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

type DbState = Arc<Database>;
type LockState = Arc<LockManager>;

// ==================== Shared Helpers ====================

/// Core scan logic: scan a list of (tool_id, path) pairs, upsert to DB.
/// Name-as-identity: same skill name across different tools → one skill record, multiple installations.
/// After scan, detects conflicts (different core_hash across tools for same skill name).
fn scan_tool_paths(
    db: &Database,
    tool_paths: &[(i64, String, String)],
    project_id: i64,
) -> Result<ScanResult, String> {
    let mut all_skills: Vec<(i64, models::DiscoveredSkill)> = Vec::new(); // (tool_id, skill)
    let mut all_errors = Vec::new();
    let mut skills_new = 0usize;
    let mut skills_updated = 0usize;
    let mut details: Vec<ScanDetail> = Vec::new();

    // Pre-build tool_id → tool_name map
    let all_tools = db.list_tools().unwrap_or_default();
    let tool_name_map: std::collections::HashMap<i64, String> =
        all_tools.into_iter().map(|t| (t.id, t.name)).collect();

    // Determine scope label
    let scope = if project_id == 0 {
        "Global".to_string()
    } else {
        db.get_project_name(project_id).unwrap_or_else(|_| format!("Project#{}", project_id))
    };

    for (tool_id, global_path, _project_rel_path) in tool_paths {
        let expanded = match scanner::expand_path(global_path) {
            Ok(p) => p,
            Err(e) => {
                all_errors.push(format!("Path '{}' expand failed: {}", global_path, e));
                continue;
            }
        };

        // Skip non-existent directories silently (not an error — tool may not be installed)
        if !expanded.exists() {
            continue;
        }

        match scanner::scan_directory(&expanded) {
            Ok(skills) => {
                for skill in skills {
                    all_skills.push((*tool_id, skill));
                }
            }
            Err(errs) => all_errors.extend(errs),
        }
    }

    // Group by skill name — name-as-identity: same name = same skill
    let mut by_name: std::collections::HashMap<String, Vec<(i64, models::DiscoveredSkill)>> =
        std::collections::HashMap::new();
    for (tool_id, skill) in &all_skills {
        by_name.entry(skill.name.clone()).or_default().push((*tool_id, skill.clone()));
    }

    // Detect within-tool duplicates: same skill name appearing multiple times
    // in the same tool's directory (e.g., two directories with the same YAML name)
    for (name, entries) in &by_name {
        let mut per_tool: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
        for (tid, _) in entries {
            *per_tool.entry(*tid).or_insert(0) += 1;
        }
        for (tid, count) in per_tool {
            if count > 1 {
                let tname = tool_name_map.get(&tid).cloned().unwrap_or_default();
                all_errors.push(format!(
                    "Warning: skill name '{}' found {} times in tool '{}'. Only the first occurrence is used.",
                    name, count, tname
                ));
            }
        }
    }

    // Upsert one record per unique skill name, then create installations for each tool
    for (name, entries) in &by_name {
        // Use the first discovered entry for metadata
        let first = &entries[0].1;

        let skill_id = match db.upsert_skill(
            name,
            first.description.as_deref(),
            &first.source_path,
            &first.content_hash,
            &first.core_hash,
            project_id,
        ) {
            Ok((id, is_new)) => {
                if is_new {
                    skills_new += 1;
                } else {
                    skills_updated += 1;
                }
                id
            }
            Err(e) => {
                all_errors.push(format!("DB error for {}: {}", name, e));
                continue;
            }
        };

        // Record scan details and create installations for each tool that has this skill
        for (tool_id, skill) in entries {
            let tname = tool_name_map.get(tool_id).cloned().unwrap_or_else(|| format!("Tool#{}", tool_id));
            let status = if skills_new > skills_updated { "new" } else { "updated" };
            details.push(ScanDetail {
                skill_name: name.clone(),
                tool_name: tname.clone(),
                scope: scope.clone(),
                status: status.to_string(),
                source_path: skill.source_path.clone(),
            });

            // Auto-detect: skill physically exists in this tool's directory
            let _ = db.ensure_installation(skill_id, *tool_id, project_id);
        }

        // Conflict detection: if different tools have different core_hash for same skill
        if entries.len() > 1 {
            let unique_hashes: std::collections::HashSet<&str> =
                entries.iter().map(|(_, s)| s.core_hash.as_str()).collect();
            if unique_hashes.len() > 1 {
                // Build version info for the conflict
                let versions: Vec<models::ConflictVersion> = entries.iter().map(|(tool_id, s)| {
                    let tname = tool_name_map.get(tool_id).cloned().unwrap_or_default();
                    models::ConflictVersion {
                        tool_id: *tool_id,
                        tool_name: tname,
                        core_hash: s.core_hash.clone(),
                        source_path: s.source_path.clone(),
                    }
                }).collect();
                let detail = serde_json::to_string(&versions).unwrap_or_default();

                // Only insert if no existing unresolved conflict
                if let Ok(false) = db.has_unresolved_conflict(skill_id) {
                    let _ = db.insert_conflict(skill_id, &detail);
                }
            }
        }
    }

    // Log scan operation
    let status = if all_errors.is_empty() { "success" } else { "failed" };
    let log_detail = if all_errors.is_empty() {
        format!("found={}, new={}, updated={}", all_skills.len(), skills_new, skills_updated)
    } else {
        all_errors.join("; ")
    };
    let _ = db.insert_action_log(
        "scan",
        None,
        None,
        project_id,
        status,
        Some(&log_detail),
    );

    Ok(ScanResult {
        skills_found: all_skills.len(),
        skills_new,
        skills_updated,
        errors: all_errors,
        details,
    })
}

/// Core sync logic: sync a skill to all its active installation targets.
fn do_sync_skill(
    db: &Database,
    locks: &LockManager,
    skill_id: i64,
    project_id: i64,
    prefer_symlink: bool,
    source_override: Option<&str>,
) -> Result<SyncResult, String> {
    let skill = db.get_skill_by_id(skill_id)?;
    // Serialize all operations touching this skill's directories (SSOT + tools)
    let _lock = locks.acquire_blocking(project_id, &skill.name);
    let source = PathBuf::from(source_override.unwrap_or(&skill.source_path));

    if !source.exists() {
        return Err(format!("Source path does not exist: {}", source.display()));
    }

    // Ensure SSOT directory exists
    sync::ensure_ssot_dir()?;
    let ssot_target = sync::ssot_path(&skill.name, project_id)?;

    // Step 1: Copy to SSOT (replace if target already exists)
    let to_ssot_result = if prefer_symlink {
        sync::symlink_or_copy(&source, &ssot_target)
    } else if ssot_target.exists() {
        sync::replace_directory(&source, &ssot_target).map(|_| "replace".to_string())
    } else {
        sync::copy_directory(&source, &ssot_target).map(|_| "copy".to_string())
    };

    match &to_ssot_result {
        Ok(method) => {
            log::info!("Synced {} to SSOT via {}", skill.name, method);
            // Create local.md marker in SSOT
            let _ = sync::create_local_marker(&ssot_target);
        }
        Err(e) => {
            log::error!("Failed to sync {} to SSOT: {}", skill.name, e);
            return Ok(SyncResult {
                skill_id,
                skill_name: skill.name.clone(),
                synced_to: 0,
                errors: vec![e.clone()],
            });
        }
    }

    // Step 2: Copy from SSOT to all active installation targets (project-aware paths)
    let installations = db.get_active_installation_paths(skill_id, project_id)?;
    let mut synced_to = 0usize;
    let mut errors = Vec::new();

    for (tool_id, target_path) in &installations {
        let expanded = match scanner::expand_path(target_path) {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("Path expansion failed for tool {}: {}", tool_id, e));
                db.insert_sync_log(
                    skill_id,
                    *tool_id,
                    project_id,
                    "from_ssot",
                    "failed",
                    Some(&e),
                )?;
                continue;
            }
        };

        let target_dir = expanded.join(&skill.name);

        let result = if prefer_symlink {
            sync::symlink_or_copy(&ssot_target, &target_dir)
        } else if target_dir.exists() {
            sync::replace_directory(&ssot_target, &target_dir).map(|_| "replace".to_string())
        } else {
            sync::copy_directory(&ssot_target, &target_dir).map(|_| "copy".to_string())
        };

        match result {
            Ok(method) => {
                log::info!(
                    "Synced {} to tool {} via {}",
                    skill.name,
                    tool_id,
                    method
                );
                db.update_synced_at(skill_id, *tool_id, project_id)?;
                db.insert_sync_log(skill_id, *tool_id, project_id, "from_ssot", "success", None)?;
                synced_to += 1;
            }
            Err(e) => {
                log::error!(
                    "Failed to sync {} to tool {}: {}",
                    skill.name,
                    tool_id,
                    e
                );
                errors.push(format!("Tool {}: {}", tool_id, e));
                db.insert_sync_log(
                    skill_id,
                    *tool_id,
                    project_id,
                    "from_ssot",
                    "failed",
                    Some(&e),
                )?;
            }
        }
    }

    // Step 3: Remove the skill from tools the user has unchecked (disabled
    // installations), so stale copies don't linger after a sync.
    let disabled = db.get_disabled_installation_paths(skill_id, project_id)?;
    for (tool_id, target_path) in &disabled {
        let expanded = match scanner::expand_path(target_path) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let target_dir = expanded.join(&skill.name);
        match sync::remove_installed_skill(&target_dir) {
            Ok(true) => {
                log::info!("Removed {} from disabled tool {}", skill.name, tool_id);
                let _ = db.insert_action_log(
                    "remove",
                    Some(skill_id),
                    Some(*tool_id),
                    project_id,
                    "success",
                    Some(&format!("removed {}", target_dir.display())),
                );
            }
            Ok(false) => {} // nothing installed there — no-op
            Err(e) => {
                log::error!("Failed to remove {} from tool {}: {}", skill.name, tool_id, e);
                errors.push(format!("Tool {}: {}", tool_id, e));
                let _ = db.insert_action_log(
                    "remove",
                    Some(skill_id),
                    Some(*tool_id),
                    project_id,
                    "failed",
                    Some(&e),
                );
            }
        }
    }

    // After successful sync, refresh stored hashes from SSOT (the canonical copy)
    // so check_updates doesn't flag it as "needs update" anymore
    if synced_to > 0 || ssot_target.exists() {
        if let Ok(new_content_hash) = hash::compute_content_hash(&ssot_target) {
            let skill_md = ssot_target.join("SKILL.md");
            let new_core_hash = hash::compute_core_hash(&skill_md).unwrap_or_default();
            let _ = db.update_skill_hashes(skill_id, &new_content_hash, &new_core_hash);
        }
        // Update source_path to SSOT so future operations use the canonical location
        let ssot_str = ssot_target.to_string_lossy().to_string();
        let _ = db.update_skill_source_path(skill_id, &ssot_str);
    }

    Ok(SyncResult {
        skill_id,
        skill_name: skill.name,
        synced_to,
        errors,
    })
}

// ==================== Tool Commands ====================

#[tauri::command]
fn list_tools(db: State<DbState>) -> Result<Vec<Tool>, String> {
    db.list_tools()
}

#[tauri::command]
fn add_tool(
    db: State<DbState>,
    name: String,
    global_path: String,
    project_rel_path: String,
) -> Result<Tool, String> {
    let tool = db.add_tool(&name, &global_path, &project_rel_path)?;
    let _ = db.insert_action_log("add_tool", None, Some(tool.id), 0, "success", Some(&name));
    Ok(tool)
}

#[tauri::command]
fn update_tool_path(
    db: State<DbState>,
    tool_id: i64,
    global_path: String,
    project_rel_path: String,
) -> Result<(), String> {
    db.update_tool_path(tool_id, &global_path, &project_rel_path)
}

#[tauri::command]
fn delete_tool(db: State<DbState>, tool_id: i64, tool_name: Option<String>) -> Result<(), String> {
    let name = tool_name.unwrap_or_else(|| {
        db.get_tool_name(tool_id).unwrap_or_else(|_| format!("id={}", tool_id))
    });
    db.delete_tool(tool_id)?;
    let _ = db.insert_action_log("delete_tool", None, None, 0, "success", Some(&name));
    Ok(())
}

#[tauri::command]
fn list_tool_templates() -> Vec<ToolTemplate> {
    discovery::get_all_templates()
}

#[tauri::command]
fn discover_tools(db: State<DbState>) -> Result<Vec<ToolTemplate>, String> {
    discovery::discover_tools(&db)
}

// ==================== Skill Commands ====================

#[tauri::command]
fn list_skills(db: State<DbState>, project_id: Option<i64>) -> Result<Vec<SkillView>, String> {
    let pid = project_id.unwrap_or(0);
    db.list_skills_with_status(pid)
}

// ==================== Scan Commands ====================

#[tauri::command]
async fn full_scan(db: State<'_, DbState>) -> Result<ScanResult, String> {
    let db = db.inner().clone();

    tokio::task::spawn_blocking(move || -> Result<ScanResult, String> {
        let tool_paths = db.get_tool_paths()?;
        scan_tool_paths(&db, &tool_paths, 0)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn scan_scope(
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

        scan_tool_paths(&db, &tool_paths, pid)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

// ==================== Sync Commands (M2) ====================

#[tauri::command]
fn toggle_skill(
    db: State<DbState>,
    skill_id: i64,
    tool_id: i64,
    project_id: i64,
    active: bool,
) -> Result<(), String> {
    db.toggle_installation(skill_id, tool_id, project_id, active)?;

    // Log the toggle action
    let action = if active { "toggle_on" } else { "toggle_off" };
    let _ = db.insert_action_log(action, Some(skill_id), Some(tool_id), project_id, "success", None);

    Ok(())
}

#[tauri::command]
async fn sync_skill(
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
        do_sync_skill(&db, &locks, skill_id, pid, settings.prefer_symlink, source_path.as_deref())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn sync_all_pending(db: State<'_, DbState>, locks: State<'_, LockState>) -> Result<Vec<SyncResult>, String> {
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
                    match do_sync_skill(&db, &locks, skill.id, project.id, settings.prefer_symlink, None) {
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

/// Check if any tool directory for a skill differs from SSOT.
/// Returns a Vec of SkillUpdate — one per divergent tool (P0-3 fix: no early return).
/// Scoped to project_id (P0-2 fix: only checks installations in the given project).
fn check_single_skill(db: &Database, locks: &LockManager, skill_id: i64, project_id: i64) -> Result<Vec<SkillUpdate>, String> {
    let skill = db.get_skill_by_id(skill_id)?;
    // Skip skills that are being synced right now — hashing a half-written
    // directory would produce bogus "update available" results
    let _lock = match locks.try_acquire_blocking(project_id, &skill.name) {
        Some(guard) => guard,
        None => return Ok(Vec::new()),
    };
    let ssot = sync::ssot_path(&skill.name, project_id)?;

    // Compute SSOT hash (None if SSOT doesn't exist yet)
    let ssot_hash = if ssot.exists() {
        hash::compute_content_hash(&ssot).ok()
    } else {
        None
    };

    let mut updates = Vec::new();

    // Check all active installation paths (scoped to project) against SSOT
    if let Ok(installations) = db.get_active_installation_paths(skill_id, project_id) {
        for (tool_id, install_path) in &installations {
            // install_path is the tool's skills ROOT (e.g. ~/.claude/skills/);
            // expand ~ and join the skill name, mirroring do_sync_skill's targeting.
            // Hashing the root directly would compare ALL skills against one SSOT
            // and flag a phantom update forever.
            let expanded = match scanner::expand_path(install_path) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let skill_dir = expanded.join(&skill.name);
            if !skill_dir.exists() {
                continue;
            }

            if let Ok(install_hash) = hash::compute_content_hash(&skill_dir) {
                match &ssot_hash {
                    Some(sh) if *sh == install_hash => continue, // in sync
                    _ => {
                        // Check if this change was dismissed (same hash = still dismissed)
                        if db.is_update_dismissed(skill_id, *tool_id, &install_hash).unwrap_or(false) {
                            continue;
                        }
                        let tool_name = db.get_tool_name(*tool_id).unwrap_or_default();
                        let old_hash = ssot_hash.clone().unwrap_or_default();
                        updates.push(SkillUpdate {
                            skill_id,
                            skill_name: skill.name.clone(),
                            source_path: scanner::normalize_path(&skill_dir),
                            old_hash,
                            new_hash: install_hash,
                            changed_tool: Some(tool_name),
                            changed_tool_id: Some(*tool_id),
                        });
                    }
                }
            }
        }
    }

    // Also check source_path against SSOT (backward compat)
    let source = PathBuf::from(&skill.source_path);
    if source.exists() {
        if let Ok(source_hash) = hash::compute_content_hash(&source) {
            match &ssot_hash {
                Some(sh) if *sh == source_hash => {} // in sync
                _ => {
                    let old_hash = ssot_hash.clone().unwrap_or_default();
                    // Only add if not already covered by an installation check
                    if !updates.iter().any(|u| u.source_path == skill.source_path) {
                        updates.push(SkillUpdate {
                            skill_id,
                            skill_name: skill.name.clone(),
                            source_path: skill.source_path.clone(),
                            old_hash,
                            new_hash: source_hash,
                            changed_tool: None,
                            changed_tool_id: None,
                        });
                    }
                }
            }
        }
    }

    // Auto-refresh DB content_hash from SSOT if everything is in sync
    if updates.is_empty() {
        if let Some(sh) = &ssot_hash {
            if sh != &skill.content_hash {
                let _ = db.update_content_hash(skill_id, sh);
            }
        }
    }

    Ok(updates)
}

#[tauri::command]
async fn check_updates(
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
            match check_single_skill(&db, &locks, *skill_id, pid) {
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
async fn check_skill_update(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    skill_id: i64,
    project_id: Option<i64>,
) -> Result<Vec<SkillUpdate>, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || check_single_skill(&db, &locks, skill_id, pid))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
async fn get_skill_diff(
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

// ==================== Conflict Commands (M5) ====================

#[tauri::command]
fn list_conflicts(db: State<DbState>, project_id: Option<i64>) -> Result<Vec<ConflictView>, String> {
    let pid = project_id.unwrap_or(0);
    db.list_unresolved_conflicts(pid)
}

#[tauri::command]
async fn resolve_conflict(
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
            sync::replace_directory(&source, &ssot_target)?;
        } else {
            sync::copy_directory(&source, &ssot_target)?;
        }
        let _ = sync::create_local_marker(&ssot_target);

        // Propagate from SSOT to all other active installations
        let installations = db.get_active_installation_paths(skill_id, pid)?;
        let mut synced_to = 0usize;
        let mut errors = Vec::new();

        for (tool_id, target_path) in &installations {
            let expanded = match scanner::expand_path(target_path) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(format!("Path expansion failed for tool {}: {}", tool_id, e));
                    continue;
                }
            };
            let target_dir = expanded.join(&skill.name);

            let result = if target_dir.exists() {
                sync::replace_directory(&ssot_target, &target_dir).map(|_| "replace".to_string())
            } else {
                sync::copy_directory(&ssot_target, &target_dir).map(|_| "copy".to_string())
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
        if let Ok(new_content_hash) = hash::compute_content_hash(&ssot_target) {
            let skill_md = ssot_target.join("SKILL.md");
            let new_core_hash = hash::compute_core_hash(&skill_md).unwrap_or_default();
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

// ==================== Reverse Sync & Dismiss ====================

/// Push SSOT content to a specific tool directory (overwrite tool with SSOT version).
#[tauri::command]
async fn reverse_sync_skill(
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
            if let Ok(new_content_hash) = hash::compute_content_hash(&ssot) {
                let skill_md = ssot.join("SKILL.md");
                let new_core_hash = hash::compute_core_hash(&skill_md).unwrap_or_default();
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
async fn dismiss_skill_update(
    db: State<'_, DbState>,
    skill_id: i64,
    tool_id: i64,
    current_hash: String,
) -> Result<(), String> {
    let db = db.inner().clone();
    db.dismiss_update(skill_id, tool_id, &current_hash)
        .map_err(|e| format!("Failed to dismiss update: {}", e))
}

// ==================== Settings Commands (M2) ====================

#[tauri::command]
fn get_settings() -> Settings {
    Settings::load()
}

#[tauri::command]
fn update_settings(new_settings: Settings) -> Result<(), String> {
    new_settings.save()
}

// ==================== Project Commands (M3) ====================

#[tauri::command]
fn list_projects(db: State<DbState>) -> Result<Vec<Project>, String> {
    db.list_projects()
}

#[tauri::command]
fn add_project(db: State<DbState>, name: String, path: String) -> Result<Project, String> {
    let project = db.add_project(&name, &path)?;
    let _ = db.insert_action_log("add_project", None, None, project.id, "success", Some(&format!("{} ({})", name, path)));
    Ok(project)
}

#[tauri::command]
fn delete_project(db: State<DbState>, project_id: i64) -> Result<(), String> {
    // Log before deletion so we can capture the project name
    let project_name = db.get_project_name(project_id).unwrap_or_else(|_| format!("id={}", project_id));
    db.delete_project(project_id)?;
    let _ = db.insert_action_log("delete_project", None, None, 0, "success", Some(&project_name));
    Ok(())
}

#[tauri::command]
fn update_project(db: State<DbState>, project_id: i64, name: String, path: String) -> Result<(), String> {
    db.update_project(project_id, &name, &path)?;
    let _ = db.insert_action_log("edit_project", None, None, project_id, "success", Some(&format!("{} ({})", name, path)));
    Ok(())
}

// ==================== Sync Log Commands (M3) ====================

#[tauri::command]
fn get_sync_logs(
    db: State<DbState>,
    skill_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<SyncLog>, String> {
    db.get_sync_logs(skill_id, limit)
}

// ==================== Application Entry ====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger
    env_logger::init();

    // Initialize database
    let db = Database::new().expect("Failed to initialize database");
    let db_state = Arc::new(db);
    // Per-skill lock manager: serializes sync/check operations on the same skill
    let lock_state: LockState = Arc::new(LockManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_state)
        .manage(lock_state)
        .invoke_handler(tauri::generate_handler![
            // Tools
            list_tools,
            add_tool,
            update_tool_path,
            delete_tool,
            list_tool_templates,
            discover_tools,
            // Skills
            list_skills,
            // Scan
            full_scan,
            scan_scope,
            // Sync (M2)
            toggle_skill,
            sync_skill,
            sync_all_pending,
            check_updates,
        check_skill_update,
        get_skill_diff,
            // Settings (M2)
            get_settings,
            update_settings,
            // Projects (M3)
            list_projects,
            add_project,
            delete_project,
            update_project,
            // Sync Logs (M3)
            get_sync_logs,
            // Conflicts (M5)
            list_conflicts,
            resolve_conflict,
            // Reverse sync & dismiss
            reverse_sync_skill,
            dismiss_skill_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
