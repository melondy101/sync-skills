// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Core domain operations shared by multiple commands.
//! These functions are pure logic over Database/LockManager — no Tauri types.

pub mod remote_skill_detail;

use crate::db::Database;
use crate::lock::LockManager;
use crate::models::{self, ScanDetail, ScanResult, SkillUpdate, SyncResult};
use crate::{hash, scanner, sync};
use std::path::PathBuf;

/// Core scan logic: scan a list of (tool_id, path) pairs, upsert to DB.
/// Name-as-identity: same skill name across different tools → one skill record, multiple installations.
/// After scan, detects conflicts (different core_hash across tools for same skill name).
pub fn scan_tool_paths(
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
pub fn do_sync_skill(
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

    // Ensure SSOT directory exists
    sync::ensure_ssot_dir()?;
    let ssot_target = sync::ssot_path(&skill.name, project_id)?;

    // Resolve the sync source. An explicit override must exist; otherwise fall
    // back gracefully so a stale source_path (e.g. pointing at a missing SSOT
    // copy) doesn't wedge the skill in a permanent sync-error state.
    let source = if let Some(over) = source_override {
        let p = PathBuf::from(over);
        if !p.exists() {
            return Err(format!("Source path does not exist: {}", p.display()));
        }
        p
    } else {
        let recorded = PathBuf::from(&skill.source_path);
        if recorded.exists() {
            recorded
        } else if ssot_target.exists() {
            // Recorded source is gone but SSOT is intact: distribute from SSOT.
            ssot_target.clone()
        } else {
            // Neither source nor SSOT exists: rebuild SSOT from the first
            // surviving tool copy (self-heal after a corrupted source_path).
            db.get_active_installation_paths(skill_id, project_id)?
                .iter()
                .filter_map(|(_, target_path)| scanner::expand_path(target_path).ok())
                .map(|expanded| expanded.join(&skill.name))
                .find(|dir| dir.exists())
                .ok_or_else(|| {
                    format!(
                        "Source path does not exist and no tool copy of '{}' was found: {}",
                        skill.name, skill.source_path
                    )
                })?
        }
    };

    // Step 1: Copy to SSOT (replace if target already exists).
    // Skip when the source IS the SSOT — replacing a directory with itself
    // would delete it first and lose the content.
    let to_ssot_result = if source == ssot_target {
        Ok("noop".to_string())
    } else if prefer_symlink {
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

/// Check if any tool directory for a skill differs from SSOT.
/// Returns a Vec of SkillUpdate — one per divergent tool (P0-3 fix: no early return).
/// Scoped to project_id (P0-2 fix: only checks installations in the given project).
pub fn check_single_skill(db: &Database, locks: &LockManager, skill_id: i64, project_id: i64) -> Result<Vec<SkillUpdate>, String> {
    let skill = db.get_skill_by_id(skill_id)?;
    // Skip skills that are being synced right now — hashing a half-written
    // directory would produce bogus "update available" results
    let _lock = match locks.try_acquire_blocking(project_id, &skill.name) {
        Some(guard) => guard,
        None => return Ok(Vec::new()),
    };
    let ssot = sync::ssot_path(&skill.name, project_id)?;
    let ssot_skill_md = ssot.join("SKILL.md");

    // Compute SSOT core_hash (SKILL.md only — auxiliary file changes won't
    // trigger false "update available" notifications). Falls back to None
    // when SSOT doesn't exist yet.
    let ssot_hash = if ssot_skill_md.exists() {
        hash::compute_core_hash(&ssot_skill_md).ok()
    } else {
        None
    };

    let mut updates = Vec::new();

    // Check all active installation paths (scoped to project) against SSOT
    if let Ok(installations) = db.get_active_installation_paths(skill_id, project_id) {
        for (tool_id, install_path) in &installations {
            // install_path is the tool's skills ROOT (e.g. ~/.claude/skills/);
            // expand ~ and join the skill name, mirroring do_sync_skill's targeting.
            let expanded = match scanner::expand_path(install_path) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let skill_dir = expanded.join(&skill.name);
            let skill_md = skill_dir.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }

            if let Ok(install_hash) = hash::compute_core_hash(&skill_md) {
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
    let source_skill_md = source.join("SKILL.md");
    if source_skill_md.exists() {
        if let Ok(source_hash) = hash::compute_core_hash(&source_skill_md) {
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

    // Auto-refresh DB hashes from SSOT if everything is in sync
    if updates.is_empty() {
        if let Some(sh) = &ssot_hash {
            if sh != &skill.core_hash {
                // Refresh both hashes: core_hash for update detection,
                // content_hash gets recomputed alongside it
                if let Ok(content_hash) = hash::compute_content_hash(&ssot) {
                    let _ = db.update_skill_hashes(skill_id, &content_hash, sh);
                }
            }
        }
    }

    Ok(updates)
}
