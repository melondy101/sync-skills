// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Thin Tauri command glue for the remote market domain.
//!
//! Provider-specific remote knowledge (URL shapes, REST responses, raw download
//! paths, layout probing) lives behind the [`MarketProvider`](crate::providers::MarketProvider)
//! seam in `crate::providers`; indexing orchestration (the SQLite transaction
//! that persists discovered skills) lives in [`crate::ops::market`]. Adding a
//! new marketplace means writing one provider adapter — nothing here changes.

use crate::db::Database;
use crate::DbState;

use crate::models::{Market, MarketCommitUpdate, MarketSyncResult, MarketTemplate, RemoteInstallation, RemoteSkill, RemoteSkillUpdate, SyncResult};
use crate::fs::{copy_directory, replace_directory, symlink_or_copy};
use crate::providers::{provider_for, provider_for_market, provider_for_reference_probed};
use crate::settings::Settings;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
struct SyncProgress {
    completed: usize,
    total: usize,
    current: Option<String>,
}

fn default_templates() -> Vec<MarketTemplate> { vec![
    MarketTemplate {
        id: "anthropic-skills".to_string(),
        label: "Anthropic Skills".to_string(),
        description: "Built-in Anthropic market".to_string(),
        provider: "github".to_string(),
        owner: "anthropics".to_string(),
        name: "skills".to_string(),
        branch: "main".to_string(),
        kind: "standard".to_string(),
        root_skill: true,
    },
    MarketTemplate {
        id: "superpowers-skills".to_string(),
        label: "Superpowers Skills".to_string(),
        description: "Built-in Superpowers market".to_string(),
        provider: "github".to_string(),
        owner: "superpowers".to_string(),
        name: "skills".to_string(),
        branch: "main".to_string(),
        kind: "standard".to_string(),
        root_skill: true,
    },
    MarketTemplate {
        id: "mattpocock-skills".to_string(),
        label: "Matt Pocock Skills".to_string(),
        description: "Built-in Matt Pocock market".to_string(),
        provider: "github".to_string(),
        owner: "mattpocock".to_string(),
        name: "skills".to_string(),
        branch: "main".to_string(),
        kind: "standard".to_string(),
        root_skill: true,
    },
    MarketTemplate {
        id: "andrej-karpathy-skill".to_string(),
        label: "Andrej Karpathy Skill".to_string(),
        description: "Built-in Andrej Karpathy market".to_string(),
        provider: "github".to_string(),
        owner: "karpathy".to_string(),
        name: "skill".to_string(),
        branch: "main".to_string(),
        kind: "standard".to_string(),
        // karpathy/skill has SKILL.md at the repo root → treat the whole repo as one skill.
        root_skill: true,
    },
    MarketTemplate {
        id: "khazix-skills".to_string(),
        label: "Khazix Skills".to_string(),
        description: "Built-in Khazix market".to_string(),
        provider: "github".to_string(),
        owner: "khazix".to_string(),
        name: "skills".to_string(),
        branch: "main".to_string(),
        kind: "standard".to_string(),
        root_skill: true,
    },
] }

/// Resolve the layout for a market template.
fn layout_for_template(template: &MarketTemplate) -> &'static str {
    if template.root_skill { "root" } else { "subdir" }
}

/// Seed the built-in markets into the `markets` table on first run, so the
/// market tab is populated and "scan all" / "check updates" have something to
/// query. Idempotent: `upsert_market` resolves on the unique constraint
/// (provider, owner, name, branch), so repeated runs never create duplicates.
pub fn seed_default_markets(db: &Database) -> Result<(), String> {
    for template in default_templates() {
        let remote_url = provider_for(&template.provider)?
            .display_url(&template.owner, &template.name, &template.branch);
        db.upsert_market(
            &template.provider,
            &template.owner,
            &template.name,
            &template.branch,
            &remote_url,
            layout_for_template(&template),
        )?;
    }
    Ok(())
}

#[tauri::command]
pub async fn scan_all_remote_repositories(db: State<'_, DbState>) -> Result<Vec<MarketSyncResult>, String> {
    let markets = db.list_markets()?
        .into_iter()
        .filter(|market| market.enabled)
        .collect::<Vec<_>>();

    let mut results = Vec::new();
    for market in markets {
        match crate::ops::market::scan_market(&db, &market).await {
            Ok(result) => results.push(result),
            Err(error) => results.push(MarketSyncResult {
                market_id: market.id,
                skills_found: 0,
                skills_new: 0,
                skills_updated: 0,
                errors: vec![error],
            }),
        }
    }

    Ok(results)
}

#[tauri::command]
pub fn list_markets(db: State<'_, DbState>) -> Result<Vec<Market>, String> {
    db.list_markets()
}

#[tauri::command]
pub fn list_market_templates(db: State<'_, DbState>) -> Result<Vec<MarketTemplate>, String> {
    let mut templates = db.list_market_templates()?;
    if templates.is_empty() {
        templates = default_templates();
    }
    Ok(templates)
}

#[tauri::command]
pub fn add_market(db: State<'_, DbState>, provider: String, owner: String, name: String, branch: String) -> Result<Market, String> {
    let remote_url = provider_for(&provider)?.display_url(&owner, &name, &branch);
    let market = db.upsert_market(&provider, &owner, &name, &branch, &remote_url, "subdir")?;
    Ok(market)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // `update_market` mirrors the persistent shape of the markets table; splitting it would be churn for callers.
pub fn update_market(
    db: State<'_, DbState>,
    id: i64,
    provider: String,
    owner: String,
    name: String,
    branch: String,
    enabled: bool,
    // Optional layout override. `None` (or "auto") preserves the current
    // stored layout; "root" / "subdir" force the value (validated server-side).
    layout: Option<String>,
) -> Result<Market, String> {
    let remote_url = provider_for(&provider)?.display_url(&owner, &name, &branch);
    db.update_market(id, enabled, None, None, None)?;
    let current_layout = db.get_market(id)?.map(|m| m.layout).unwrap_or_else(|| "subdir".to_string());
    let next_layout = match layout.as_deref() {
        Some("root") | Some("subdir") => layout.unwrap(),
        _ => current_layout.clone(),
    };
    db.upsert_market(&provider, &owner, &name, &branch, &remote_url, &next_layout)?;
    // upsert_market touches remote_url/layout on the same row, but if the
    // caller kept owner/name/branch unchanged, set_market_layout is a no-op
    // extra write that guarantees a flip from the preserved value still
    // sticks even when other fields didn't change.
    if next_layout != current_layout {
        db.set_market_layout(id, &next_layout)?;
    }
    db.get_market(id).map(|m| m.unwrap())
}

#[tauri::command]
pub async fn add_market_by_url(
    db: State<'_, DbState>,
    url: String,
    branch: Option<String>,
    // `"root"`, `"subdir"`, or `"auto"` (probe SKILL.md locations and pick).
    layout: Option<String>,
) -> Result<Market, String> {
    let provider = provider_for_reference_probed(&url).await?;
    let (owner, name, branch_hint) = provider.parse_reference(&url)?;
    let resolved_branch = match (branch, branch_hint) {
        (Some(b), _) => b,
        (None, Some(b)) => b,
        (None, None) => provider.resolve_branch(&owner, &name).await?,
    };
    let remote_url = provider.display_url(&owner, &name, &resolved_branch);

    // Resolve the layout: explicit user choice wins; "auto" lets the provider
    // probe the repo; anything unknown falls back to "subdir" so a fresh row is
    // still scannable.
    let chosen_layout: String = match layout.as_deref() {
        Some("root") | Some("subdir") => layout.unwrap(),
        _ => match provider.detect_layout(&owner, &name, &resolved_branch).await {
            Some(l) => l.to_string(),
            None => "subdir".to_string(),
        },
    };

    let market = db.upsert_market(provider.kind(), &owner, &name, &resolved_branch, &remote_url, &chosen_layout)?;
    Ok(market)
}

#[tauri::command]
pub fn delete_market(db: State<'_, DbState>, id: i64) -> Result<(), String> {
    db.delete_market(id)
}

#[tauri::command]
pub async fn sync_market_index(db: State<'_, DbState>, market_id: i64) -> Result<MarketSyncResult, String> {
    let market = db.get_market(market_id)?.ok_or_else(|| format!("market {market_id} not found"))?;
    let result = crate::ops::market::scan_market(&db, &market).await?;
    let commit_sha = provider_for_market(&market)?
        .latest_commit_sha(&market.owner, &market.name, &market.branch)
        .await;
    db.update_market(
        market_id,
        market.enabled,
        Some(&chrono::Utc::now().to_rfc3339()),
        None,
        commit_sha.as_deref(),
    )?;
    Ok(result)
}

#[tauri::command]
pub async fn sync_all_market_indices(db: State<'_, DbState>) -> Result<Vec<MarketSyncResult>, String> {
    let markets = db.list_markets()?;
    let mut results = Vec::new();
    for market in markets {
        if !market.enabled {
            continue;
        }
        match crate::ops::market::scan_market(&db, &market).await {
            Ok(result) => results.push(result),
            Err(e) => results.push(MarketSyncResult {
                market_id: market.id,
                skills_found: 0,
                skills_new: 0,
                skills_updated: 0,
                errors: vec![e],
            }),
        }
    }
    Ok(results)
}

/// Lightweight update check: compare the remote repo's latest commit with the
/// stored `last_commit_sha`. Only markets with a new commit are returned, so a
/// full (and slower) index re-sync can be skipped when nothing changed.
#[tauri::command]
pub async fn check_market_commits(db: State<'_, DbState>) -> Result<Vec<MarketCommitUpdate>, String> {
    let markets = db
        .list_markets()?
        .into_iter()
        .filter(|market| market.enabled)
        .collect::<Vec<_>>();

    let mut updates = Vec::new();
    for market in markets {
        let new_sha = match provider_for_market(&market)?
            .latest_commit_sha(&market.owner, &market.name, &market.branch)
            .await
        {
            Some(sha) => sha,
            None => continue, // remote unreachable — skip silently
        };
        let changed = match &market.last_commit_sha {
            Some(stored) => stored != &new_sha,
            None => true, // never indexed yet → treat as changed
        };
        if changed {
            updates.push(MarketCommitUpdate {
                market_id: market.id,
                market_title: format!("{}/{}", market.owner, market.name),
                last_commit_sha: market.last_commit_sha.clone(),
                new_commit_sha: new_sha,
            });
        }
    }

    Ok(updates)
}

#[tauri::command]
pub fn list_remote_skills(db: State<'_, DbState>, market_id: Option<i64>) -> Result<Vec<RemoteSkill>, String> {
    db.list_remote_skills(market_id)
}

#[tauri::command]
pub async fn download_remote_skill_to_ssot(db: State<'_, DbState>, remote_skill_id: i64) -> Result<SyncResult, String> {
    let skill = db.list_remote_skills(None)?
        .into_iter()
        .find(|s| s.id == remote_skill_id)
        .ok_or_else(|| format!("remote skill {remote_skill_id} not found"))?;

    // Look up the parent market so the provider knows the coordinates + layout.
    let market = db.get_market(skill.market_id)?
        .ok_or_else(|| format!("market {} not found", skill.market_id))?;

    let ssot = PathBuf::from(&skill.ssot_path);
    fs::create_dir_all(&ssot).map_err(|e| e.to_string())?;

    let bytes = match provider_for_market(&market)?
        .fetch_skill_md(&market, &skill.skill_name)
        .await
    {
        Ok(b) => b,
        Err(error) => {
            return Ok(SyncResult {
                skill_id: 0,
                skill_name: skill.skill_name,
                synced_to: 0,
                errors: vec![error],
            });
        }
    };

    fs::write(ssot.join("SKILL.md"), &bytes).map_err(|e| e.to_string())?;

    // Re-hash the freshly-written SSOT so skills.source_path points at the
    // canonical SSOT and the content_hash reflects what's actually on disk.
    let content_hash = crate::hash::compute_content_hash(&ssot).unwrap_or_default();
    let core_hash = crate::hash::compute_core_hash(&ssot.join("SKILL.md")).unwrap_or_default();
    let content_str = String::from_utf8_lossy(&bytes).to_string();
    let (parsed_name, description) = crate::scanner::parse_front_matter(&content_str, &ssot)
        .map(|(n, d)| (Some(n), d))
        .unwrap_or((None, None));
    // The skill name we expose in the global/project view is the YAML `name:`
    // when present, otherwise the directory/remote name. This matches the
    // scanner's behavior so name-as-identity stays consistent.
    let effective_name = parsed_name.unwrap_or_else(|| skill.skill_name.clone());

    // Upsert into the `skills` table at the global scope (project_id=0) so
    // list_skills(0) sees the skill right away, and stamp the source market
    // so the global/project view can render a provenance badge. Per-tool
    // installations are written when sync_remote_skill_to_tools copies to a
    // tool.
    let (skill_db_id, _) = db.upsert_skill_from_market(
        &effective_name,
        description.as_deref(),
        &skill.ssot_path,
        &content_hash,
        &core_hash,
        0,
        skill.market_id,
    )?;

    // Update remote_skills description if it was empty (don't clobber richer
    // descriptions that a fresh market index already filled in).
    if skill.description.is_none() {
        if let Some(desc) = description.as_deref() {
            let _ = db.update_remote_skill_description(remote_skill_id, Some(desc));
        }
    }

    db.set_remote_skill_installed(remote_skill_id, true)?;

    Ok(SyncResult {
        skill_id: skill_db_id,
        skill_name: effective_name,
        synced_to: 1,
        errors: vec![],
    })
}

/// Look up a tool by its resolved `global_path`. The market UI passes back
/// the same `path` strings produced in `SkillMarketPanel`, so matching by
/// `global_path` is the most stable contract.
fn find_tool_id_by_path(db: &Database, tool_path: &str) -> Result<Option<i64>, String> {
    let tools = db.list_tools()?;
    Ok(tools.into_iter().find(|t| t.global_path == tool_path).map(|t| t.id))
}

#[tauri::command]
pub async fn sync_remote_skill_to_tools(
    db: State<'_, DbState>,
    lock_state: State<'_, crate::LockState>,
    remote_skill_id: i64,
    project_id: i64,
    tool_path: String,
) -> Result<SyncResult, String> {
    let remote = db.list_remote_skills(None)?
        .into_iter()
        .find(|s| s.id == remote_skill_id)
        .ok_or_else(|| format!("remote skill {remote_skill_id} not found"))?;

    // Gate: the user must have marked the skill as installed for this project
    // before we copy it onto a tool's skills directory.
    let installed = db.list_remote_installations(project_id, None)?
        .into_iter()
        .find(|item| item.remoteSkillId == remote_skill_id);
    if installed.is_none() {
        return Ok(SyncResult {
            skill_id: 0,
            skill_name: remote.skill_name.clone(),
            synced_to: 0,
            errors: vec![],
        });
    }

    let tool_id = match find_tool_id_by_path(&db, &tool_path)? {
        Some(id) => id,
        None => {
            return Ok(SyncResult {
                skill_id: 0,
                skill_name: remote.skill_name,
                synced_to: 0,
                errors: vec![format!("Tool with path '{}' not registered", tool_path)],
            });
        }
    };

    // Acquire the per-skill lock to mirror do_sync_skill and avoid races
    // between the SSOT download happening elsewhere and the tool copy here.
    let _lock = lock_state.acquire_blocking(project_id, &remote.skill_name);

    let ssot_dir = PathBuf::from(&remote.ssot_path);
    if !ssot_dir.exists() {
        return Ok(SyncResult {
            skill_id: 0,
            skill_name: remote.skill_name,
            synced_to: 0,
            errors: vec!["SSOT directory not found, download it first".to_string()],
        });
    }

    let expanded = crate::scanner::expand_path(&tool_path)
        .map_err(|e| format!("Path expand failed: {}", e))?;
    let target_dir = expanded.join(&remote.skill_name);

    let result = if Settings::load().prefer_symlink {
        symlink_or_copy(&ssot_dir, &target_dir)
    } else if target_dir.exists() {
        replace_directory(&ssot_dir, &target_dir).map(|_| "replace".to_string())
    } else {
        copy_directory(&ssot_dir, &target_dir).map(|_| "copy".to_string())
    };

    match result {
        Ok(_) => {
            // Make the skill visible in the global/project skill list right
            // away. The skills row at project_id=0 was already upserted by
            // download_remote_skill_to_ssot; here we also ensure a row in the
            // chosen project scope (so the project's tab sees it under the
            // chosen tool) and stamp the source market on whichever row was
            // empty. Per-tool installation rows are what toggle the UI.
            let content_hash = crate::hash::compute_content_hash(&ssot_dir).unwrap_or_default();
            let core_hash = crate::hash::compute_core_hash(&ssot_dir.join("SKILL.md")).unwrap_or_default();
            let (skill_db_id, _) = db.upsert_skill_from_market(
                &remote.skill_name,
                None,
                &remote.ssot_path,
                &content_hash,
                &core_hash,
                project_id,
                remote.market_id,
            )?;
            db.ensure_installation(skill_db_id, tool_id, project_id)?;
            db.update_synced_at(skill_db_id, tool_id, project_id)?;
            db.set_remote_skill_installed(remote_skill_id, true)?;
            Ok(SyncResult {
                skill_id: skill_db_id,
                skill_name: remote.skill_name,
                synced_to: 1,
                errors: vec![],
            })
        }
        Err(error) => Ok(SyncResult {
            skill_id: 0,
            skill_name: remote.skill_name,
            synced_to: 0,
            errors: vec![error],
        }),
    }
}

#[tauri::command]
pub async fn sync_remote_installations_to_tools(app: AppHandle, db: State<'_, DbState>, lock_state: State<'_, crate::LockState>, project_id: i64, market_id: Option<i64>, tool_path: String) -> Result<SyncResult, String> {
    let installations = db.list_remote_installations(project_id, market_id)?;
    let total = installations.len();
    let _ = app.emit(
        "market:sync-progress",
        SyncProgress { completed: 0, total, current: None },
    );

    let mut synced_to = 0usize;
    let mut errors = Vec::new();

    for (i, installation) in installations.iter().enumerate() {
        let current = installation.skillName.clone();
        let _ = app.emit(
            "market:sync-progress",
            SyncProgress { completed: i, total, current: Some(current.clone()) },
        );
        match sync_remote_skill_to_tools(db.clone(), lock_state.clone(), installation.remoteSkillId, project_id, tool_path.clone()).await {
            Ok(result) => {
                synced_to += result.synced_to;
                errors.extend(result.errors);
            }
            Err(error) => errors.push(error),
        }
        let _ = app.emit(
            "market:sync-progress",
            SyncProgress { completed: i + 1, total, current: Some(current) },
        );
    }

    Ok(SyncResult {
        skill_id: 0,
        skill_name: String::new(),
        synced_to,
        errors,
    })
}

#[tauri::command]
pub fn list_remote_installations(db: State<'_, DbState>, project_id: i64, market_id: Option<i64>) -> Result<Vec<RemoteInstallation>, String> {
    db.list_remote_installations(project_id, market_id)
}

#[tauri::command]
pub fn toggle_remote_installation(db: State<'_, DbState>, remote_skill_id: i64, project_id: i64, _scope: String, active: bool) -> Result<RemoteInstallation, String> {
    db.set_remote_skill_installed(remote_skill_id, active)?;
    let scope = if project_id == 0 { "global" } else { "project" };
    let installation = db.list_remote_installations(project_id, None)?.into_iter().find(|i| i.remoteSkillId == remote_skill_id).ok_or_else(|| "installation not found".to_string())?;
    Ok(RemoteInstallation {
        id: installation.id,
        remoteSkillId: installation.remoteSkillId,
        skillName: installation.skillName,
        remoteUrl: installation.remoteUrl,
        ssotPath: installation.ssotPath,
        installedAt: installation.installedAt,
        marketId: installation.marketId,
        marketTitle: installation.marketTitle,
        projectId: project_id,
        scope: scope.to_string(),
    })
}

#[tauri::command]
pub fn sync_remote_installations(_project_id: i64, _market_id: Option<i64>) -> Result<SyncResult, String> {
    Ok(SyncResult {
        skill_id: 0,
        skill_name: String::new(),
        synced_to: 0,
        errors: vec![],
    })
}

#[tauri::command]
pub async fn check_remote_updates(db: State<'_, DbState>, market_id: Option<i64>, market_filter: Option<i64>) -> Result<Vec<RemoteSkillUpdate>, String> {
    // "Remote updates" = the remote repository's latest SKILL.md hash differs
    // from the SSOT copy currently on disk. We compute against the on-disk
    // SSOT whenever it exists (regardless of is_installed), so a skill the
    // user downloaded then unsubscribed-from still surfaces pending updates.
    let skills = db.list_remote_skills(market_id)?;
    let mut updates = Vec::new();

    for skill in skills {
        if let Some(filter_market_id) = market_filter {
            if skill.market_id != filter_market_id {
                continue;
            }
        }
        let ssot_dir = PathBuf::from(&skill.ssot_path);
        if !ssot_dir.exists() {
            // No local copy → nothing to compare; the user hasn't downloaded
            // this skill yet. Don't surface it as an "update".
            continue;
        }
        let local_hash = crate::hash::compute_content_hash(&ssot_dir).unwrap_or_default();
        if local_hash != skill.remote_content_hash {
            updates.push(RemoteSkillUpdate {
                id: skill.id,
                market_id: skill.market_id,
                skill_name: skill.skill_name,
                remote_url: skill.remote_url,
                ssot_path: skill.ssot_path,
                old_hash: local_hash,
                new_hash: skill.remote_content_hash.clone(),
                has_changes: true,
            });
        }
    }

    Ok(updates)
}

#[tauri::command]
pub async fn check_remote_ssot_updates(db: State<'_, DbState>, market_id: Option<i64>, market_filter: Option<i64>) -> Result<Vec<RemoteSkillUpdate>, String> {
    // "Installed SSOT updates" = the SSOT copy on disk differs from the
    // remote hash AND the user has marked the skill as installed. This is
    // the "I have a copy and the upstream moved" view.
    let skills = db.list_remote_skills(market_id)?;
    let mut updates = Vec::new();

    for skill in skills {
        if !skill.is_installed {
            continue;
        }
        if let Some(filter_market_id) = market_filter {
            if skill.market_id != filter_market_id {
                continue;
            }
        }
        let ssot_dir = PathBuf::from(&skill.ssot_path);
        if !ssot_dir.exists() {
            continue;
        }
        let local_hash = crate::hash::compute_content_hash(&ssot_dir).unwrap_or_default();
        if local_hash != skill.remote_content_hash {
            updates.push(RemoteSkillUpdate {
                id: skill.id,
                market_id: skill.market_id,
                skill_name: skill.skill_name,
                remote_url: skill.remote_url,
                ssot_path: skill.ssot_path,
                old_hash: local_hash,
                new_hash: skill.remote_content_hash.clone(),
                has_changes: true,
            });
        }
    }

    Ok(updates)
}

#[tauri::command]
pub async fn set_all_remote_skills_installed(db: State<'_, DbState>, _project_id: i64, market_id: Option<i64>, active: bool) -> Result<crate::models::RemoteSkillInstalledResult, String> {
    let skills = if let Some(mid) = market_id {
        db.list_remote_skills(Some(mid))?
    } else {
        db.list_remote_skills(None)?
    };
    let mut updated = 0i64;
    for skill in skills {
        if skill.is_installed != active {
            db.set_remote_skill_installed(skill.id, active)?;
            updated += 1;
        }
    }
    Ok(crate::models::RemoteSkillInstalledResult { updated })
}

// ==================== Remote Skill Detail (T4) ====================

/// Detail payload for the T4 Market: RemoteSkillDetailModal. Thin adapter —
/// all domain logic (SSOT walk, hash, file tree) lives in
/// `ops::remote_skill_detail`. Never triggers a download.
#[tauri::command]
pub async fn get_remote_skill_detail(
    db: State<'_, DbState>,
    remote_skill_id: i64,
) -> Result<crate::models::RemoteSkillDetail, String> {
    let skill = db
        .get_remote_skill(remote_skill_id)?
        .ok_or_else(|| format!("remote skill {remote_skill_id} not found"))?;
    crate::ops::remote_skill_detail::build_remote_skill_detail(&skill)
}
