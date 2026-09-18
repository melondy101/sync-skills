// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Market provider seam (ports & adapters).
//!
//! [`MarketProvider`] is the deep interface for "a remote skill marketplace":
//! it hides every provider-specific concern (URL shapes, REST responses, raw
//! download paths, layout probing) behind a small, provider-agnostic surface of
//! seven operations. [`commands::market`](crate::commands::market) and
//! [`ops::market`](crate::ops::market) talk only to the trait, so adding a second
//! marketplace (GitLab, a private registry, …) means writing one more adapter —
//! no command or DB orchestration changes.
//!
//! Design note (design-it-twice, Plan A3 + A2 connection reuse): the trait is
//! deliberately keyed 1:1 on the operations the command layer already performs,
//! so the seam is a faithful extraction rather than a speculative redesign. Each
//! adapter owns a single pooled [`reqwest::Client`] built once per operation, so
//! the many requests of a market scan share TCP connections and TLS sessions —
//! previously a fresh client (and cold pool) was built for every probe.
//!
//! Async uses boxed futures (`BoxFuture`) instead of `async fn` in the trait so
//! the trait stays object-safe for `Box<dyn MarketProvider>` runtime dispatch by
//! the `markets.provider` discriminator.

use crate::http::{self, HttpConfig};
use crate::models::Market;
use crate::settings::Settings;
use crate::sync::ssot_path;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Result of probing a single skill entry inside a remote market.
/// Provider-agnostic leaf data consumed by the DB-index orchestration.
#[allow(dead_code)] // `skill_md_repo_path` is reserved for future mirror-mode cloning.
pub struct RemoteSkillProbe {
    pub skill_name: String,
    /// Human-facing remote URL (e.g. GitHub web URL).
    pub remote_url: String,
    /// Path of the skill's SKILL.md inside the repo (e.g. "/" or "/foo").
    /// Empty when the repo itself is the skill.
    pub skill_md_repo_path: String,
    /// SSOT path on disk where this skill will live.
    pub ssot_path: String,
    pub remote_content_hash: String,
    pub remote_core_hash: String,
    pub description: Option<String>,
}

/// The seam: a remote skill marketplace adapter. Seven operations covering the
/// full lifecycle the app needs — parse a user reference, resolve coordinates,
/// discover the skill set, fetch one skill's content, and cheaply detect change.
pub trait MarketProvider: Send + Sync {
    /// The `markets.provider` discriminator this adapter handles.
    fn kind(&self) -> &'static str;

    /// Human-facing web URL for a market repo at a branch (no request needed).
    fn display_url(&self, owner: &str, repo: &str, branch: &str) -> String;

    /// Parse a free-form market URL / `owner/repo` string into coordinates.
    fn parse_reference(&self, raw: &str) -> Result<(String, String, Option<String>), String>;

    /// Resolve the default branch, erroring if the repo is unreachable.
    fn resolve_branch<'a>(&'a self, owner: &'a str, repo: &'a str) -> BoxFuture<'a, Result<String, String>>;

    /// Probe the repo to decide whether it is a single root skill or a
    /// directory-per-skill layout. `None` when undeterminable.
    fn detect_layout<'a>(&'a self, owner: &'a str, repo: &'a str, branch: &'a str)
        -> BoxFuture<'a, Option<&'static str>>;

    /// Enumerate every skill the market exposes, with hashed content metadata.
    fn discover_skills<'a>(&'a self, market: &'a Market)
        -> BoxFuture<'a, Result<Vec<RemoteSkillProbe>, Vec<String>>>;

    /// Fetch the raw `SKILL.md` bytes for one skill of the given market.
    fn fetch_skill_md<'a>(&'a self, market: &'a Market, skill_name: &'a str)
        -> BoxFuture<'a, Result<Vec<u8>, String>>;

    /// Latest commit SHA for a branch, used for cheap "did anything change" checks.
    fn latest_commit_sha<'a>(&'a self, owner: &'a str, repo: &'a str, branch: &'a str)
        -> BoxFuture<'a, Option<String>>;
}

/// Construct the provider for a `markets.provider` discriminator. Unknown or
/// empty values default to GitHub (the historical behavior); genuinely unknown
/// providers surface as an error rather than silently mis-resolving.
///
/// Construction is cheap: the HTTP client is built lazily on first request, so
/// callers that only need provider-specific string shapes (URL formatting,
/// reference parsing) never pay for a TLS context.
pub fn provider_for(kind: &str) -> Result<Box<dyn MarketProvider>, String> {
    match kind {
        "github" | "" => Ok(Box::new(GithubProvider::new())),
        other => Err(format!("unsupported provider: {}", other)),
    }
}

/// GitHub adapter — the concrete implementation of [`MarketProvider`] over the
/// GitHub REST API plus `raw.githubusercontent.com`. Owns one pooled client so
/// every request within an operation shares the same connection/TLS cache.
pub struct GithubProvider {
    client: std::sync::OnceLock<reqwest::Client>,
}

impl GithubProvider {
    pub fn new() -> Self {
        Self { client: std::sync::OnceLock::new() }
    }

    /// Build the client on first use, reflecting the proxy settings live at
    /// that moment; every later request on this provider reuses its pool.
    fn client(&self) -> Result<&reqwest::Client, String> {
        if let Some(client) = self.client.get() {
            return Ok(client);
        }
        let settings = Settings::load();
        let proxy = http::resolve_proxy(
            settings.use_system_proxy,
            settings.use_proxy,
            settings.proxy_url.as_deref(),
        );
        let client = http::build_client(
            HttpConfig::new("skill-manager", Duration::from_secs(15)).with_proxy(proxy),
        )?;
        Ok(self.client.get_or_init(|| client))
    }

    fn api_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
            owner,
            repo,
            path.trim_start_matches('/'),
            branch
        )
    }

    fn raw_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}{}",
            owner,
            repo,
            branch.trim_start_matches('/'),
            path
        )
    }

    fn describe_status(status: u16) -> &'static str {
        match status {
            404 => "not found or private",
            403 => "rate-limited or private",
            401 => "unauthorized",
            429 => "rate-limited",
            500..=599 => "server error",
            _ => "request failed",
        }
    }

    async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        let response = self
            .client()?
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "HTTP {} ({})",
                status.as_u16(),
                Self::describe_status(status.as_u16())
            ));
        }
        response.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
    }

    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        let bytes = self.fetch_bytes(url).await?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    /// Fetch a skill's SKILL.md, hash it, and extract its front-matter
    /// description. Uses the shared client so N skills in one scan reuse
    /// connections instead of rebuilding a client per probe.
    async fn probe_skill_md(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        skill_name: &str,
    ) -> Result<(String, String, Option<String>), String> {
        let raw = Self::raw_url(owner, repo, branch, "/SKILL.md");
        let bytes = self.fetch_bytes(&raw).await?;

        // Write into a temp dir so the directory-based hash functions work.
        let tmp_dir = std::env::temp_dir().join(format!("skill-probe-{}", skill_name));
        std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
        let md_path = tmp_dir.join("SKILL.md");
        std::fs::write(&md_path, &bytes).map_err(|e| e.to_string())?;

        let content_hash = crate::hash::compute_content_hash(&tmp_dir).unwrap_or_default();
        let core_hash = crate::hash::compute_core_hash(&md_path).unwrap_or_default();
        let content_str = String::from_utf8_lossy(&bytes).to_string();
        let description = crate::scanner::parse_front_matter(&content_str, &tmp_dir)
            .ok()
            .and_then(|(_, desc)| desc);
        Ok((content_hash, core_hash, description))
    }
}

impl MarketProvider for GithubProvider {
    fn kind(&self) -> &'static str {
        "github"
    }

    fn display_url(&self, owner: &str, repo: &str, branch: &str) -> String {
        format!("https://github.com/{}/{}/tree/{}", owner, repo, branch)
    }

    fn parse_reference(&self, raw: &str) -> Result<(String, String, Option<String>), String> {
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

    fn resolve_branch<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
            match self.fetch_bytes(&repo_url).await {
                Ok(bytes) => {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                        if let Some(default_branch) =
                            json.get("default_branch").and_then(|v| v.as_str())
                        {
                            if !default_branch.is_empty() {
                                return Ok(default_branch.to_string());
                            }
                        }
                    }
                    // Metadata reachable but no default_branch — fall through to probe.
                }
                Err(e) => {
                    return Err(format!("Cannot access {}/{}: {}", owner, repo, e));
                }
            }
            let main_url = format!("https://api.github.com/repos/{}/{}/branches/main", owner, repo);
            if self.fetch_bytes(&main_url).await.is_ok() {
                return Ok("main".to_string());
            }
            let master_url =
                format!("https://api.github.com/repos/{}/{}/branches/master", owner, repo);
            if self.fetch_bytes(&master_url).await.is_ok() {
                return Ok("master".to_string());
            }
            Err(format!("Cannot find default branch for {}/{}", owner, repo))
        })
    }

    fn detect_layout<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
        branch: &'a str,
    ) -> BoxFuture<'a, Option<&'static str>> {
        Box::pin(async move {
            let root_raw = Self::raw_url(owner, repo, branch, "/SKILL.md");
            if self.fetch_bytes(&root_raw).await.is_ok() {
                return Some("root");
            }
            let root_url = Self::api_url(owner, repo, branch, "/");
            if let Ok(contents) = self.fetch_json::<Vec<serde_json::Value>>(&root_url).await {
                let has_dir = contents
                    .iter()
                    .any(|entry| entry.get("type").and_then(|v| v.as_str()) == Some("dir"));
                if has_dir {
                    return Some("subdir");
                }
            }
            None
        })
    }

    fn discover_skills<'a>(
        &'a self,
        market: &'a Market,
    ) -> BoxFuture<'a, Result<Vec<RemoteSkillProbe>, Vec<String>>> {
        Box::pin(async move {
            let owner = &market.owner;
            let repo = &market.name;
            let branch = &market.branch;
            let root_url = Self::api_url(owner, repo, branch, "/");
            let contents: Vec<serde_json::Value> = match self.fetch_json(&root_url).await {
                Ok(v) => v,
                Err(e) => return Err(vec![format!("root listing: {}", e)]),
            };

            let mut probes = Vec::new();
            let mut errors = Vec::new();

            match market.layout.as_str() {
                "root" => {
                    let skill_name = repo.clone();
                    let remote_url = self.display_url(owner, repo, branch);
                    let ssot = match ssot_path(&skill_name, 0) {
                        Ok(p) => p,
                        Err(e) => {
                            errors.push(format!("{}: ssot path error: {}", skill_name, e));
                            return Err(errors);
                        }
                    };
                    match self.probe_skill_md(owner, repo, branch, &skill_name).await {
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
                    for entry in contents {
                        let entry_type =
                            entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let entry_name =
                            entry.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        if entry_type != "dir" || entry_name.is_empty() {
                            continue;
                        }
                        let skill_name = entry_name.to_string();
                        let remote_url =
                            format!("{}/{}", self.display_url(owner, repo, branch), skill_name);
                        let ssot = match ssot_path(&skill_name, 0) {
                            Ok(p) => p,
                            Err(e) => {
                                errors.push(format!("{}: ssot path error: {}", skill_name, e));
                                continue;
                            }
                        };
                        match self.probe_skill_md(owner, repo, branch, &skill_name).await {
                            Ok((content_hash, core_hash, description)) => probes.push(RemoteSkillProbe {
                                skill_name,
                                remote_url,
                                skill_md_repo_path: format!("/{}/SKILL.md", entry_name),
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
        })
    }

    fn fetch_skill_md<'a>(
        &'a self,
        market: &'a Market,
        skill_name: &'a str,
    ) -> BoxFuture<'a, Result<Vec<u8>, String>> {
        Box::pin(async move {
            // root  → {branch}/SKILL.md; subdir → {branch}/{skill_name}/SKILL.md
            let path = match market.layout.as_str() {
                "root" => "/SKILL.md".to_string(),
                _ => format!("/{}/SKILL.md", skill_name),
            };
            let url = Self::raw_url(&market.owner, &market.name, &market.branch, &path);
            self.fetch_bytes(&url).await
        })
    }

    fn latest_commit_sha<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
        branch: &'a str,
    ) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            let url = format!(
                "https://api.github.com/repos/{}/{}/commits?sha={}&per_page=1",
                owner, repo, branch
            );
            let bytes = self.fetch_bytes(&url).await.ok()?;
            let commits: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
            commits.get(0)?.get("sha")?.as_str().map(|s| s.to_string())
        })
    }
}
