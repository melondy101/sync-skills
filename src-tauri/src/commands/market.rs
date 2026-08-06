// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::db::Database;
use crate::DbState;

use crate::models::{Market, MarketSyncResult, MarketTemplate, RemoteInstallation, RemoteSkill, RemoteSkillUpdate, SyncResult};
use crate::settings::Settings;
use crate::sync::{copy_directory, replace_directory, ssot_path, symlink_or_copy};
use rusqlite::params;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

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

fn github_api_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
    format!(
        "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
        owner,
        repo,
        path.trim_start_matches('/'),
        branch
    )
}

fn github_http_client() -> Result<reqwest::Client, String> {
    let settings = Settings::load();
    let mut builder = reqwest::Client::builder()
        .user_agent("skill-manager")
        .connect_timeout(std::time::Duration::from_secs(15));
    if settings.use_proxy {
        if let Some(url) = settings.proxy_url.filter(|u| !u.trim().is_empty()) {
            if let Some(proxy) = reqwest::Proxy::all(url).ok() {
                builder = builder.proxy(proxy);
            }
        }
    }
    builder.build().map_err(|e| e.to_string())
}

async fn fetch_github_bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = github_http_client()?;
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("GitHub HTTP {}", response.status()));
    }
    response.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
}

async fn scan_github_market(db: &Database, market: &Market) -> Result<MarketSyncResult, String> {
    let owner = market.owner.clone();
    let repo = market.name.clone();
    let branch = market.branch.clone();
    let root_url = github_api_url(&owner, &repo, &branch, "/");
    let contents: Vec<serde_json::Value> = fetch_github_json(&root_url).await?;

    let mut skills_found = 0usize;
    let mut skills_new = 0usize;
    let mut skills_updated = 0usize;
    let mut errors = Vec::new();
    let mut updates = Vec::new();

    for entry in contents {
        let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let entry_name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if entry_type != "dir" || entry_name.is_empty() {
            continue;
        }

        let skill_name = entry_name;
        let api_url = if let Some(url) = entry.get("url").and_then(|v| v.as_str()) { url.to_string() } else { github_api_url(&owner, &repo, &branch, &format!("/{}", skill_name)) };
        let remote_url = format!("https://github.com/{}/{}/tree/{}/{}", owner, repo, branch, skill_name);
        let ssot = ssot_path(skill_name, 0).map_err(|e| e.to_string())?;
        let ssot_path_str = ssot.to_string_lossy().to_string();

        let mut remote_content_hash = String::new();
        let mut remote_core_hash = String::new();
        let mut has_skill_md = false;

        let files = fetch_github_json::<Vec<serde_json::Value>>(&api_url).await;
        match files {
            Ok(files) => {
                for file in files {
                    if file.get("type").and_then(|v| v.as_str()) == Some("file")
                        && file.get("name").and_then(|v| v.as_str()) == Some("SKILL.md")
                    {
                        has_skill_md = true;
                        if let Some(download_url) = file.get("download_url").and_then(|v| v.as_str()) {
                            if let Ok(bytes) = fetch_github_bytes(download_url).await {
                                remote_content_hash = crate::hash::compute_content_hash(&PathBuf::from(skill_name)).unwrap_or_default();
                                remote_core_hash = remote_content_hash.clone();
                            }
                        }
                    }
                }
            }
            Err(e) => {
                errors.push(format!("{}: {}", skill_name, e));
                continue;
            }
        }

        if !has_skill_md {
            errors.push(format!("{}: missing SKILL.md", skill_name));
            continue;
        }

        updates.push((skill_name.to_string(), remote_url, ssot_path_str, remote_content_hash, remote_core_hash));
        skills_found += 1;
    }

    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    conn.execute_batch("BEGIN").ok();

    for (skill_name, remote_url, ssot_path_str, remote_content_hash, remote_core_hash) in updates {
        let existing: Option<(i64, String, String)> = conn
            .query_row(
                "SELECT id, remote_content_hash, remote_core_hash FROM remote_skills WHERE market_id = ?1 AND skill_name = ?2",
                params![market.id, skill_name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        match existing {
            Some((id, old_content_hash, old_core_hash)) => {
                if old_content_hash != remote_content_hash || old_core_hash != remote_core_hash {
                    skills_updated += 1;
                }
                conn.execute(
                    "UPDATE remote_skills SET remote_url = ?1, ssot_path = ?2, remote_content_hash = ?3, remote_core_hash = ?4, updated_at = datetime('now') WHERE id = ?5",
                    params![remote_url, ssot_path_str, remote_content_hash, remote_core_hash, id],
                ).map_err(|e| format!("Failed to update remote skill: {}", e))?;
            }
            None => {
                skills_new += 1;
                let id = crate::hash::compute_id_hash(&format!("{}:{}", market.id, skill_name));
                conn.execute(
                    "INSERT INTO remote_skills (id, market_id, skill_name, description, remote_url, ssot_path, remote_content_hash, remote_core_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![id, market.id, skill_name, None::<String>, remote_url, ssot_path_str, remote_content_hash, remote_core_hash],
                ).map_err(|e| format!("Failed to insert remote skill: {}", e))?;
            }
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE markets SET last_indexed_at = ?1, updated_at = ?2 WHERE id = ?3",
        params![now, now, market.id],
    ).map_err(|e| format!("Failed to update market: {}", e))?;

    conn.execute_batch("COMMIT").ok();

    Ok(MarketSyncResult {
        market_id: market.id,
        skills_found,
        skills_new,
        skills_updated,
        errors,
    })
}


#[tauri::command]
pub async fn scan_all_remote_repositories(db: State<'_, DbState>) -> Result<Vec<MarketSyncResult>, String> {
    let markets = db.list_markets()?
        .into_iter()
        .filter(|market| market.enabled)
        .collect::<Vec<_>>();

    let mut results = Vec::new();
    for market in markets {
        match scan_github_market(&db, &market).await {
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
    let remote_url = format!("https://github.com/{}/{}/tree/{}", owner, name, branch);
    let market = db.upsert_market(&provider, &owner, &name, &branch, &remote_url)?;
    Ok(market)
}

#[tauri::command]
pub fn update_market(db: State<'_, DbState>, id: i64, provider: String, owner: String, name: String, branch: String, enabled: bool) -> Result<Market, String> {
    let remote_url = format!("https://github.com/{}/{}/tree/{}", owner, name, branch);
    db.update_market(id, enabled, None, None)?;
    db.upsert_market(&provider, &owner, &name, &branch, &remote_url)?;
    db.get_market(id).map(|m| m.unwrap())
}

#[tauri::command]
pub fn delete_market(db: State<'_, DbState>, id: i64) -> Result<(), String> {
    db.delete_market(id)
}

#[tauri::command]
pub async fn sync_market_index(db: State<'_, DbState>, market_id: i64) -> Result<MarketSyncResult, String> {
    let market = db.get_market(market_id)?.ok_or_else(|| format!("market {market_id} not found"))?;
    let result = scan_github_market(&db, &market).await?;
    db.update_market(market_id, market.enabled, Some(&chrono::Utc::now().to_rfc3339()), None)?;
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
        match scan_github_market(&db, &market).await {
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

#[tauri::command]
pub fn list_remote_skills(db: State<'_, DbState>, market_id: Option<i64>) -> Result<Vec<RemoteSkill>, String> {
    db.list_remote_skills(market_id)
}

#[tauri::command]
pub async fn download_remote_skill_to_ssot(db: State<'_, DbState>, remote_skill_id: i64) -> Result<SyncResult, String> {
    let skill = db.list_remote_skills(None)?.into_iter().find(|s| s.id == remote_skill_id).ok_or_else(|| format!("remote skill {remote_skill_id} not found"))?;
    let ssot = PathBuf::from(&skill.ssot_path);
    fs::create_dir_all(&ssot).map_err(|e| e.to_string())?;

    match fetch_github_bytes(&skill.remote_url).await {
        Ok(bytes) => {
            write_skill_directory(&ssot, bytes)?;
            db.set_remote_skill_installed(remote_skill_id, true)?;
            Ok(SyncResult {
                skill_id: 0,
                skill_name: skill.skill_name,
                synced_to: 1,
                errors: vec![],
            })
        }
        Err(error) => Ok(SyncResult {
            skill_id: 0,
            skill_name: skill.skill_name,
            synced_to: 0,
            errors: vec![error],
        }),
    }
}

#[tauri::command]
pub async fn sync_remote_skill_to_tools(db: State<'_, DbState>, lock_state: State<'_, crate::LockState>, remote_skill_id: i64, project_id: i64, tool_path: String) -> Result<SyncResult, String> {
    let installed = db.list_remote_installations(project_id, None)?.into_iter().find(|item| item.remoteSkillId == remote_skill_id);
    let remote = db.list_remote_skills(None)?.into_iter().find(|s| s.id == remote_skill_id).ok_or_else(|| format!("remote skill {remote_skill_id} not found"))?;

    if installed.is_none() {
        return Ok(SyncResult {
            skill_id: 0,
            skill_name: remote.skill_name.clone(),
            synced_to: 0,
            errors: vec![],
        });
    }

    let expanded = crate::scanner::expand_path(&tool_path).map_err(|e| format!("Path expand failed: {}", e))?;
    let target_dir = expanded.join(&remote.skill_name);
    let ssot_dir = PathBuf::from(&remote.ssot_path);

    if !ssot_dir.exists() {
        return Ok(SyncResult {
            skill_id: 0,
            skill_name: remote.skill_name,
            synced_to: 0,
            errors: vec!["SSOT directory not found, download it first".to_string()],
        });
    }

    let result = if Settings::load().prefer_symlink {
        symlink_or_copy(&ssot_dir, &target_dir)
    } else if target_dir.exists() {
        replace_directory(&ssot_dir, &target_dir).map(|_| "replace".to_string())
    } else {
        copy_directory(&ssot_dir, &target_dir).map(|_| "copy".to_string())
    };

    match result {
        Ok(_) => {
            db.set_remote_skill_installed(remote_skill_id, true)?;
            Ok(SyncResult {
                skill_id: 0,
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
pub async fn sync_remote_installations_to_tools(db: State<'_, DbState>, lock_state: State<'_, crate::LockState>, project_id: i64, market_id: Option<i64>, tool_path: String) -> Result<SyncResult, String> {
    let installations = db.list_remote_installations(project_id, market_id)?;
    let mut synced_to = 0usize;
    let mut errors = Vec::new();

    for installation in installations {
        match sync_remote_skill_to_tools(db.clone(), lock_state.clone(), installation.remoteSkillId, project_id, tool_path.clone()).await {
            Ok(result) => {
                synced_to += result.synced_to;
                errors.extend(result.errors);
            }
            Err(error) => errors.push(error),
        }
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
pub async fn check_remote_updates(db: State<'_, DbState>, market_id: Option<i64>) -> Result<Vec<RemoteSkillUpdate>, String> {
    let skills = db.list_remote_skills(market_id)?;
    let mut updates = Vec::new();

    for skill in skills {
        if !skill.is_installed {
            continue;
        }

        let ssot_dir = PathBuf::from(&skill.ssot_path);
        let local_hash = if ssot_dir.exists() {
            crate::hash::compute_content_hash(&ssot_dir).ok()
        } else {
            None
        };

        if local_hash.as_deref() != Some(&skill.remote_content_hash) {
            updates.push(RemoteSkillUpdate {
                id: skill.id,
                market_id: skill.market_id,
                skill_name: skill.skill_name,
                remote_url: skill.remote_url,
                ssot_path: skill.ssot_path,
                old_hash: local_hash.unwrap_or_default(),
                new_hash: skill.remote_content_hash.clone(),
                has_changes: true,
            });
        }
    }

    Ok(updates)
}

#[tauri::command]
pub fn get_remote_skill_diff(remote_skill_id: i64) -> Result<serde_json::Value, String> {
    Ok(json!({
        "source_path": format!("remote://{}/SKILL.md", remote_skill_id),
        "ssot_path": format!(
            "{}\\.agents\\skill-manager\\ssot\\skill\\SKILL.md",
            std::env::var("USERPROFILE").unwrap_or_default()
        ),
        "files": [],
        "has_changes": false
    }))
}

fn write_skill_directory(ssot_dir: &Path, bytes: Vec<u8>) -> Result<(), String> {
    fs::create_dir_all(ssot_dir).map_err(|e| format!("Failed to create SSOT directory: {}", e))?;
    fs::write(ssot_dir.join("SKILL.md"), bytes).map_err(|e| format!("Failed to write SKILL.md: {}", e))?;
    Ok(())
}

async fn fetch_github_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let client = github_http_client()?;
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("GitHub HTTP {}", response.status()));
    }
    response.json().await.map_err(|e| e.to_string())
}
