// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::db::Database;
use crate::DbState;

use crate::models::{Market, MarketCommitUpdate, MarketSyncResult, MarketTemplate, RemoteInstallation, RemoteSkill, RemoteSkillUpdate, SyncResult};
use crate::settings::Settings;
use crate::sync::{copy_directory, replace_directory, ssot_path, symlink_or_copy};
use rusqlite::params;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
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
        let remote_url = format!(
            "https://github.com/{}/{}/tree/{}",
            template.owner, template.name, template.branch
        );
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

    if let Some(proxy) = http_proxy_from_settings(&settings) {
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(|e| e.to_string())
}

fn http_proxy_from_settings(settings: &Settings) -> Option<reqwest::Proxy> {
    #[cfg(windows)]
    if settings.use_system_proxy {
        return crate::commands::updater::http_system_proxy();
    }
    if !settings.use_proxy {
        return None;
    }
    let url = settings.proxy_url.as_ref()?.trim();
    if url.is_empty() {
        return None;
    }
    let normalized = if url.contains("://") {
        url.to_string()
    } else {
        format!("http://{}", url)
    };
    reqwest::Proxy::all(normalized).ok()
}

async fn fetch_github_bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = github_http_client()?;
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {} ({})", status.as_u16(), describe_github_status(status.as_u16())));
    }
    response.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
}

fn describe_github_status(status: u16) -> &'static str {
    match status {
        404 => "not found or private",
        403 => "rate-limited or private",
        401 => "unauthorized",
        429 => "rate-limited",
        500..=599 => "server error",
        _ => "request failed",
    }
}

async fn fetch_github_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let bytes = fetch_github_bytes(url).await?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

#[allow(dead_code)] // Kept for future use: write a SKILL.md + aux files into SSOT.
fn write_skill_directory(ssot_dir: &Path, bytes: Vec<u8>) -> Result<(), String> {
    fs::create_dir_all(ssot_dir).map_err(|e| format!("Failed to create SSOT directory: {}", e))?;
    fs::write(ssot_dir.join("SKILL.md"), bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Result of probing a single skill entry inside a remote market.
#[allow(dead_code)] // `skill_md_repo_path` is reserved for future mirror-mode cloning.
struct RemoteSkillProbe {
    skill_name: String,
    /// Human-facing remote URL (e.g. GitHub web URL).
    remote_url: String,
    /// Path of the skill's SKILL.md inside the repo (e.g. "/" or "/foo").
    /// Empty when the repo itself is the skill.
    skill_md_repo_path: String,
    /// SSOT path on disk where this skill will live.
    ssot_path: String,
    remote_content_hash: String,
    remote_core_hash: String,
    description: Option<String>,
}

async fn probe_github_skill_md(
    owner: &str,
    repo: &str,
    branch: &str,
    skill_name: &str,
    repo_subpath: &str,
) -> Result<(String, String, Option<String>), String> {
    // We need both the bytes (for hashing + saving) and the parsed front matter
    // (for description). fetch_github_bytes once, then derive everything from it.
    let raw_url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/SKILL.md",
        owner,
        repo,
        branch.trim_start_matches('/'),
    );
    let bytes = fetch_github_bytes(&raw_url).await?;

    // Write into a temp dir so hash functions (which expect a directory) work.
    let tmp_dir = std::env::temp_dir().join(format!("skill-probe-{}", skill_name));
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let md_path = tmp_dir.join("SKILL.md");
    fs::write(&md_path, &bytes).map_err(|e| e.to_string())?;
    // Capture the relative path used for this probe so we can attribute errors.
    let _ = repo_subpath;

    let content_hash = crate::hash::compute_content_hash(&tmp_dir).unwrap_or_default();
    let core_hash = crate::hash::compute_core_hash(&md_path).unwrap_or_default();
    let content_str = String::from_utf8_lossy(&bytes).to_string();
    let description = crate::scanner::parse_front_matter(&content_str, &tmp_dir)
        .ok()
        .and_then(|(_, desc)| desc);
    Ok((content_hash, core_hash, description))
}

async fn discover_github_skills(market: &Market) -> Result<Vec<RemoteSkillProbe>, Vec<String>> {
    let owner = market.owner.clone();
    let repo = market.name.clone();
    let branch = market.branch.clone();
    let root_url = github_api_url(&owner, &repo, &branch, "/");
    let contents: Vec<serde_json::Value> = match fetch_github_json(&root_url).await {
        Ok(v) => v,
        Err(e) => return Err(vec![format!("root listing: {}", e)]),
    };

    let mut probes = Vec::new();
    let mut errors = Vec::new();

    match market.layout.as_str() {
        "root" => {
            // The whole repo is one skill. Skill name defaults to the repo
            // name (matches directory-style identity), but the YAML front matter
            // can override it via `name:` once we download SKILL.md.
            let skill_name = repo.clone();
            let remote_url = format!("https://github.com/{}/{}/tree/{}", owner, repo, branch);
            let ssot = match ssot_path(&skill_name, 0) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(format!("{}: ssot path error: {}", skill_name, e));
                    return Err(errors);
                }
            };
            match probe_github_skill_md(&owner, &repo, &branch, &skill_name, "/").await {
                Ok((content_hash, core_hash, description)) => probes.push(RemoteSkillProbe {
                    skill_name,
                    remote_url,
                    skill_md_repo_path: "/SKILL.md".to_string(),
                    ssot_path: ssot.to_string_lossy().to_string(),
                    remote_content_hash: content_hash,
                    remote_core_hash: core_hash,
                    description,
                }),
                Err(e) => errors.push(format!("{}: {}", skill_name, e)),
            }
        }
        _ => {
            // Default layout: each subdirectory of the repo root is a skill.
            // Iterate the root entries; for every directory, look for SKILL.md.
            for entry in contents {
                let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let entry_name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if entry_type != "dir" || entry_name.is_empty() {
                    continue;
                }
                let skill_name = entry_name.to_string();
                let remote_url = format!(
                    "https://github.com/{}/{}/tree/{}/{}",
                    owner, repo, branch, skill_name
                );
                let ssot = match ssot_path(&skill_name, 0) {
                    Ok(p) => p,
                    Err(e) => {
                        errors.push(format!("{}: ssot path error: {}", skill_name, e));
                        continue;
                    }
                };
                let subpath = format!("/{}", skill_name);
                match probe_github_skill_md(&owner, &repo, &branch, &skill_name, &subpath).await {
                    Ok((content_hash, core_hash, description)) => probes.push(RemoteSkillProbe {
                        skill_name,
                        remote_url,
                        skill_md_repo_path: format!("{}/SKILL.md", subpath),
                        ssot_path: ssot.to_string_lossy().to_string(),
                        remote_content_hash: content_hash,
                        remote_core_hash: core_hash,
                        description,
                    }),
                    Err(e) => errors.push(format!("{}: {}", skill_name, e)),
                }
            }
        }
    }

    if probes.is_empty() && !errors.is_empty() {
        Err(errors)
    } else {
        Ok(probes)
    }
}

async fn scan_github_market(db: &Database, market: &Market) -> Result<MarketSyncResult, String> {
    let probes = match discover_github_skills(market).await {
        Ok(p) => p,
        Err(errs) => {
            return Ok(MarketSyncResult {
                market_id: market.id,
                skills_found: 0,
                skills_new: 0,
                skills_updated: 0,
                errors: errs,
            });
        }
    };

    let skills_found = probes.len();
    let mut skills_new = 0usize;
    let mut skills_updated = 0usize;

    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    conn.execute_batch("BEGIN").ok();

    for probe in probes {
        let existing: Option<(i64, String, String)> = conn
            .query_row(
                "SELECT id, remote_content_hash, remote_core_hash FROM remote_skills WHERE market_id = ?1 AND skill_name = ?2",
                params![market.id, probe.skill_name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        match existing {
            Some((id, old_content_hash, old_core_hash)) => {
                if old_content_hash != probe.remote_content_hash || old_core_hash != probe.remote_core_hash {
                    skills_updated += 1;
                }
                conn.execute(
                    "UPDATE remote_skills SET remote_url = ?1, ssot_path = ?2, remote_content_hash = ?3, remote_core_hash = ?4, description = ?5, updated_at = datetime('now') WHERE id = ?6",
                    params![probe.remote_url, probe.ssot_path, probe.remote_content_hash, probe.remote_core_hash, probe.description, id],
                ).map_err(|e| format!("Failed to update remote skill: {}", e))?;
            }
            None => {
                skills_new += 1;
                let id = crate::hash::compute_id_hash(&format!("{}:{}", market.id, probe.skill_name));
                conn.execute(
                    "INSERT INTO remote_skills (id, market_id, skill_name, description, remote_url, ssot_path, remote_content_hash, remote_core_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![id, market.id, probe.skill_name, probe.description, probe.remote_url, probe.ssot_path, probe.remote_content_hash, probe.remote_core_hash],
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
        errors: Vec::new(),
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
    let market = db.upsert_market(&provider, &owner, &name, &branch, &remote_url, "subdir")?;
    Ok(market)
}

#[tauri::command]
pub fn update_market(db: State<'_, DbState>, id: i64, provider: String, owner: String, name: String, branch: String, enabled: bool) -> Result<Market, String> {
    let remote_url = format!("https://github.com/{}/{}/tree/{}", owner, name, branch);
    db.update_market(id, enabled, None, None, None)?;
    // Preserve the current layout: passing the existing value keeps a user
    // who toggled "root_skill" from having it silently reset on edit.
    let layout = db.get_market(id)?.map(|m| m.layout).unwrap_or_else(|| "subdir".to_string());
    db.upsert_market(&provider, &owner, &name, &branch, &remote_url, &layout)?;
    db.get_market(id).map(|m| m.unwrap())
}

/// Parse a GitHub market reference from a free-form URL or `owner/repo` string.
/// Returns (owner, name, optional_branch).
fn parse_github_market_input(raw: &str) -> Result<(String, String, Option<String>), String> {
    let trimmed = raw.trim();
    let without_scheme = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("github.com/")
        .trim_end_matches('/');
    // Drop any path/branch suffix after owner/repo (e.g. /tree/main, /blob/..., .git)
    let path_part = without_scheme.split('/').collect::<Vec<&str>>();
    let (owner, name) = match &path_part[..] {
        [owner, name, ..] => (owner.to_string(), name.trim_end_matches(".git").to_string()),
        [single] => {
            // `owner/repo` without slashes already handled; this is a bare token
            if let Some((o, n)) = single.split_once('/') {
                (o.to_string(), n.trim_end_matches(".git").to_string())
            } else {
                return Err(format!("无法解析市场地址: {}", raw));
            }
        }
        _ => return Err(format!("无法解析市场地址: {}", raw)),
    };
    if owner.is_empty() || name.is_empty() {
        return Err(format!("无法解析市场地址: {}", raw));
    }
    Ok((owner, name, None))
}

/// Resolve the default branch for a GitHub repo. Verifies the repo is reachable
/// (404/403/etc. surface as errors instead of silently falling back to "main"),
/// prefers the repo's declared `default_branch`, and only falls back to the
/// main/master probe when the repo metadata is unavailable.
async fn resolve_github_branch(owner: &str, repo: &str) -> Result<String, String> {
    let repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
    match fetch_github_bytes(&repo_url).await {
        Ok(bytes) => {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if let Some(default_branch) = json.get("default_branch").and_then(|v| v.as_str()) {
                    if !default_branch.is_empty() {
                        return Ok(default_branch.to_string());
                    }
                }
            }
            // Repo metadata is reachable but no default_branch — fall through
            // to main/master probe so non-standard repos still work.
        }
        Err(e) => {
            // If we can't read the repo info, branches/main and branches/master
            // will also fail (the same access controls apply). Skip the extra
            // round-trips and surface the cause immediately.
            return Err(format!("Cannot access {}/{}: {}", owner, repo, e));
        }
    }
    let main_url = format!("https://api.github.com/repos/{}/{}/branches/main", owner, repo);
    if fetch_github_bytes(&main_url).await.is_ok() {
        return Ok("main".to_string());
    }
    let master_url = format!("https://api.github.com/repos/{}/{}/branches/master", owner, repo);
    if fetch_github_bytes(&master_url).await.is_ok() {
        return Ok("master".to_string());
    }
    Err(format!("Cannot find default branch for {}/{}", owner, repo))
}

#[tauri::command]
pub async fn add_market_by_url(db: State<'_, DbState>, url: String, branch: Option<String>) -> Result<Market, String> {
    let (owner, name, branch_hint) = parse_github_market_input(&url)?;
    let resolved_branch = match (branch, branch_hint) {
        (Some(b), _) => b,
        (None, Some(b)) => b,
        (None, None) => resolve_github_branch(&owner, &name).await?,
    };
    let remote_url = format!("https://github.com/{}/{}/tree/{}", owner, name, resolved_branch);
    let market = db.upsert_market("github", &owner, &name, &resolved_branch, &remote_url, "subdir")?;
    Ok(market)
}

#[tauri::command]
pub fn delete_market(db: State<'_, DbState>, id: i64) -> Result<(), String> {
    db.delete_market(id)
}

/// Fetch the latest commit SHA for a branch (used for cheap update checks).
async fn latest_commit_sha(owner: &str, repo: &str, branch: &str) -> Option<String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/commits?sha={}&per_page=1",
        owner, repo, branch
    );
    let bytes = fetch_github_bytes(&url).await.ok()?;
    let commits: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    commits.get(0)?.get("sha")?.as_str().map(|s| s.to_string())
}

#[tauri::command]
pub async fn sync_market_index(db: State<'_, DbState>, market_id: i64) -> Result<MarketSyncResult, String> {
    let market = db.get_market(market_id)?.ok_or_else(|| format!("market {market_id} not found"))?;
    let result = scan_github_market(&db, &market).await?;
    let commit_sha = latest_commit_sha(&market.owner, &market.name, &market.branch).await;
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
        let new_sha = match latest_commit_sha(&market.owner, &market.name, &market.branch).await {
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

    // Look up the parent market so we know the GitHub coordinates + layout.
    let market = db.get_market(skill.market_id)?
        .ok_or_else(|| format!("market {} not found", skill.market_id))?;

    let ssot = PathBuf::from(&skill.ssot_path);
    fs::create_dir_all(&ssot).map_err(|e| e.to_string())?;

    // Build the SKILL.md raw URL based on the market's layout.
    // root  → raw.githubusercontent.com/{owner}/{repo}/{branch}/SKILL.md
    // subdir→ raw.githubusercontent.com/{owner}/{repo}/{branch}/{skill_name}/SKILL.md
    let raw_url = match market.layout.as_str() {
        "root" => format!(
            "https://raw.githubusercontent.com/{}/{}/{}/SKILL.md",
            market.owner, market.name, market.branch
        ),
        _ => format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}/SKILL.md",
            market.owner, market.name, market.branch, skill.skill_name
        ),
    };

    let bytes = match fetch_github_bytes(&raw_url).await {
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
    // list_skills(0) sees the skill right away. Per-tool installations are
    // written when sync_remote_skill_to_tools actually copies to a tool.
    let (skill_db_id, _) = db.upsert_skill(
        &effective_name,
        description.as_deref(),
        &skill.ssot_path,
        &content_hash,
        &core_hash,
        0,
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
            // away. The skills row is owned by the global scope (project_id=0)
            // so the same identity is shared across all tools — we just
            // attach a fresh skill_installations row keyed by the chosen
            // (tool, project) so the toggle UI knows the tool has it.
            let content_hash = crate::hash::compute_content_hash(&ssot_dir).unwrap_or_default();
            let core_hash = crate::hash::compute_core_hash(&ssot_dir.join("SKILL.md")).unwrap_or_default();
            let (skill_db_id, _) = db.upsert_skill(
                &remote.skill_name,
                None,
                &remote.ssot_path,
                &content_hash,
                &core_hash,
                project_id,
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
