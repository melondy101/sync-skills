// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Market provider seam (ports & adapters).
//!
//! [`MarketProvider`] is the deep interface for "a remote skill marketplace":
//! it hides every provider-specific concern (URL shapes, REST responses, raw
//! download paths, layout probing) behind a small, provider-agnostic surface of
//! seven operations. [`commands::market`](crate::commands::market) and
//! [`ops::market`](crate::ops::market) talk only to the trait, so a marketplace
//! is one adapter: [`GithubProvider`] and [`GitlabProvider`] are the two shipped
//! today, and neither required a command or DB orchestration change.
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
        "gitlab" => Ok(Box::new(GitlabProvider::new())),
        other => Err(format!("unsupported provider: {}", other)),
    }
}

/// Pick an adapter from a user-entered reference. `add_market_by_url` receives a
/// URL and no provider discriminator, so the host decides here. Only
/// gitlab.com is routed to the GitLab adapter: that adapter speaks to the SaaS
/// API and cannot address a self-hosted instance.
pub fn provider_for_reference(raw: &str) -> Result<Box<dyn MarketProvider>, String> {
    let host = raw
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split(['/', ':'])
        .next()
        .unwrap_or("");
    // `git@gitlab.com:group/repo.git` is the SSH spelling of the same host.
    let host = host.strip_prefix("git@").unwrap_or(host);
    let kind = if host.eq_ignore_ascii_case("gitlab.com") { "gitlab" } else { "github" };
    provider_for(kind)
}

/// Builds an adapter's client on first use, reflecting the proxy settings live at
/// that moment; every later request on that provider reuses its pool.
fn pooled_client(slot: &std::sync::OnceLock<reqwest::Client>) -> Result<&reqwest::Client, String> {
    if let Some(client) = slot.get() {
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
    Ok(slot.get_or_init(|| client))
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

async fn get_bytes(
    client: &reqwest::Client,
    url: &str,
    accept: &str,
) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .header("Accept", accept)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "HTTP {} ({})",
            status.as_u16(),
            describe_status(status.as_u16())
        ));
    }
    response.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
}

async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    accept: &str,
) -> Result<T, String> {
    let bytes = get_bytes(client, url, accept).await?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/// Hash a fetched `SKILL.md` and pull its front-matter description. The hash
/// helpers are directory-based, so the payload has to land in a temp dir first.
fn hash_skill_payload(
    skill_name: &str,
    bytes: &[u8],
) -> Result<(String, String, Option<String>), String> {
    let tmp_dir = std::env::temp_dir().join(format!("skill-probe-{}", skill_name));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let md_path = tmp_dir.join("SKILL.md");
    std::fs::write(&md_path, bytes).map_err(|e| e.to_string())?;

    let content_hash = crate::hash::compute_content_hash(&tmp_dir).unwrap_or_default();
    let core_hash = crate::hash::compute_core_hash(&md_path).unwrap_or_default();
    let content_str = String::from_utf8_lossy(bytes).to_string();
    let description = crate::scanner::parse_front_matter(&content_str, &tmp_dir)
        .ok()
        .and_then(|(_, desc)| desc);
    Ok((content_hash, core_hash, description))
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

    fn client(&self) -> Result<&reqwest::Client, String> {
        pooled_client(&self.client)
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

    async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        get_bytes(self.client()?, url, "application/vnd.github+json").await
    }

    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        get_json(self.client()?, url, "application/vnd.github+json").await
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
        hash_skill_payload(skill_name, &bytes)
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

/// GitLab adapter — [`MarketProvider`] over gitlab.com's REST v4 API. Three
/// shapes force it to diverge from GitHub: the project is addressed by its
/// URL-encoded namespace path (`group%2Fsubgroup%2Frepo`) instead of
/// `owner/repo`, a directory listing is one flat paginated `tree` whose entries
/// carry `path` + `type` (`"blob"`/`"tree"`) instead of per-directory `contents`
/// with `type == "dir"`, and web links need a `/-/` separator before any
/// ref-scoped path.
pub struct GitlabProvider {
    client: std::sync::OnceLock<reqwest::Client>,
}

/// gitlab.com caps `per_page` at 100 for anonymous tree listings.
const TREE_PAGE_SIZE: usize = 100;
/// Only a bound on runaway paging: a short page already ends the walk.
const TREE_MAX_PAGES: usize = 10;

impl GitlabProvider {
    pub fn new() -> Self {
        Self { client: std::sync::OnceLock::new() }
    }

    fn client(&self) -> Result<&reqwest::Client, String> {
        pooled_client(&self.client)
    }

    /// Percent-encode everything outside RFC 3986 unreserved characters, `/`
    /// included: GitLab takes a namespace path or a file path as a *single*
    /// API path segment, so an unescaped slash would read as a segment break.
    fn encode(value: &str) -> String {
        value
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!("%{:02X}", b),
            })
            .collect()
    }

    /// `("group/sub", "repo")` → `group%2Fsub%2Frepo`, the `:id` of every
    /// `/projects/:id/...` route.
    fn project_id(owner: &str, repo: &str) -> String {
        format!("{}%2F{}", Self::encode(owner), Self::encode(repo))
    }

    fn project_url(owner: &str, repo: &str) -> String {
        format!("https://gitlab.com/api/v4/projects/{}", Self::project_id(owner, repo))
    }

    fn tree_url(owner: &str, repo: &str, branch: &str, path: &str, page: usize) -> String {
        format!(
            "{}/repository/tree?path={}&ref={}&per_page={}&page={}",
            Self::project_url(owner, repo),
            Self::encode(path.trim_start_matches('/')),
            Self::encode(branch),
            TREE_PAGE_SIZE,
            page
        )
    }

    fn file_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
        format!(
            "{}/repository/files/{}/raw?ref={}",
            Self::project_url(owner, repo),
            Self::encode(path.trim_start_matches('/')),
            Self::encode(branch)
        )
    }

    async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        get_bytes(self.client()?, url, "application/json").await
    }

    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        get_json(self.client()?, url, "application/json").await
    }

    /// Walk every page of a directory listing — unlike GitHub's `contents`
    /// endpoint, `tree` is paginated and a large market spans several pages.
    async fn fetch_tree(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut entries: Vec<serde_json::Value> = Vec::new();
        for page in 1..=TREE_MAX_PAGES {
            let page_entries: Vec<serde_json::Value> = self
                .fetch_json(&Self::tree_url(owner, repo, branch, path, page))
                .await?;
            let count = page_entries.len();
            entries.extend(page_entries);
            if count < TREE_PAGE_SIZE {
                break;
            }
        }
        Ok(entries)
    }

    /// The listing entries that are candidate skills: a `tree` with a name,
    /// paired with its repo-relative path (older/trimmed payloads omit `path`).
    fn skill_dirs(entries: &[serde_json::Value]) -> Vec<(&str, &str)> {
        entries
            .iter()
            .filter_map(|entry| {
                if entry.get("type").and_then(|v| v.as_str()) != Some("tree") {
                    return None;
                }
                let name = entry.get("name").and_then(|v| v.as_str())?;
                if name.is_empty() {
                    return None;
                }
                let path = entry
                    .get("path")
                    .and_then(|v| v.as_str())
                    .filter(|p| !p.is_empty())
                    .unwrap_or(name);
                Some((name, path))
            })
            .collect()
    }

    async fn probe_skill_md(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        skill_name: &str,
        skill_md_path: &str,
    ) -> Result<(String, String, Option<String>), String> {
        let url = Self::file_url(owner, repo, branch, skill_md_path);
        let bytes = self.fetch_bytes(&url).await?;
        hash_skill_payload(skill_name, &bytes)
    }
}

impl MarketProvider for GitlabProvider {
    fn kind(&self) -> &'static str {
        "gitlab"
    }

    fn display_url(&self, owner: &str, repo: &str, branch: &str) -> String {
        // `/-/` is GitLab's marker for "what follows is ref-scoped"; GitHub has
        // no equivalent and a browser link without it 404s.
        format!("https://gitlab.com/{}/{}/-/tree/{}", owner, repo, branch)
    }

    fn parse_reference(&self, raw: &str) -> Result<(String, String, Option<String>), String> {
        let without_host = raw
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let without_host = without_host
            .strip_prefix("gitlab.com/")
            .or_else(|| without_host.strip_prefix("git@gitlab.com:"))
            .unwrap_or(without_host)
            .trim_end_matches('/');
        let segments: Vec<&str> = without_host.split('/').filter(|s| !s.is_empty()).collect();
        // The repo ends where the ref-scoped part begins. `/-/` is the canonical
        // separator and always wins; legacy links drop it and start with a bare
        // verb, which is only a separator once an owner + name precede it
        // (`gitlab.com/user/tree` is a repo named `tree`, not branch `tree`).
        let dash = segments.iter().position(|p| *p == "-");
        let verb_idx = segments.iter().position(|p| matches!(*p, "tree" | "blob" | "raw"));
        let cut = dash.or_else(|| verb_idx.filter(|idx| *idx >= 2)).unwrap_or(segments.len());
        let (repo, tail) = segments.split_at(cut);
        let tail = if tail.first() == Some(&"-") { &tail[1..] } else { tail };
        if repo.len() < 2 {
            return Err(format!("无法解析市场地址: {}", raw));
        }
        // A nested namespace keeps its slashes in `owner`; `%2F` is applied only
        // when building an API path, and web links want the slashes verbatim.
        let owner = repo[..repo.len() - 1].join("/");
        let name = repo[repo.len() - 1].trim_end_matches(".git").to_string();
        if owner.is_empty() || name.is_empty() {
            return Err(format!("无法解析市场地址: {}", raw));
        }
        // GitLab refs may contain slashes, so only the leading segment is usable
        // as a hint; anything after it is a path inside the repo.
        let branch_hint = match tail {
            [verb, reference, ..] if matches!(*verb, "tree" | "blob" | "raw") => {
                Some(reference.to_string())
            }
            _ => None,
        };
        Ok((owner, name, branch_hint))
    }

    fn resolve_branch<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let project_url = Self::project_url(owner, repo);
            match self.fetch_bytes(&project_url).await {
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
            for candidate in ["main", "master"] {
                let url = format!("{}/repository/branches/{}", project_url, candidate);
                if self.fetch_bytes(&url).await.is_ok() {
                    return Ok(candidate.to_string());
                }
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
            if self
                .fetch_bytes(&Self::file_url(owner, repo, branch, "/SKILL.md"))
                .await
                .is_ok()
            {
                return Some("root");
            }
            if let Ok(entries) = self.fetch_tree(owner, repo, branch, "").await {
                if !Self::skill_dirs(&entries).is_empty() {
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
            let entries = match self.fetch_tree(owner, repo, branch, "").await {
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
                    match self.probe_skill_md(owner, repo, branch, &skill_name, "/SKILL.md").await {
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
                    let display_base = self.display_url(owner, repo, branch);
                    for (entry_name, entry_path) in Self::skill_dirs(&entries) {
                        let skill_name = entry_name.to_string();
                        let skill_md_path = format!("/{}/SKILL.md", entry_path);
                        let remote_url = format!("{}/{}", display_base, entry_path);
                        let ssot = match ssot_path(&skill_name, 0) {
                            Ok(p) => p,
                            Err(e) => {
                                errors.push(format!("{}: ssot path error: {}", skill_name, e));
                                continue;
                            }
                        };
                        match self
                            .probe_skill_md(owner, repo, branch, &skill_name, &skill_md_path)
                            .await
                        {
                            Ok((content_hash, core_hash, description)) => probes.push(RemoteSkillProbe {
                                skill_name,
                                remote_url,
                                skill_md_repo_path: skill_md_path,
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
            // root  → /SKILL.md; subdir → /{skill_name}/SKILL.md
            let path = match market.layout.as_str() {
                "root" => "/SKILL.md".to_string(),
                _ => format!("/{}/SKILL.md", skill_name),
            };
            let url = Self::file_url(&market.owner, &market.name, &market.branch, &path);
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
                "{}/repository/commits?ref_name={}&per_page=1",
                Self::project_url(owner, repo),
                Self::encode(branch)
            );
            let bytes = self.fetch_bytes(&url).await.ok()?;
            let commits: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
            commits.get(0)?.get("id")?.as_str().map(|s| s.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gitlab() -> GitlabProvider {
        GitlabProvider::new()
    }

    fn assert_parsed(raw: &str, owner: &str, name: &str, branch: Option<&str>) {
        let (o, n, b) = gitlab()
            .parse_reference(raw)
            .unwrap_or_else(|e| panic!("{} should parse: {}", raw, e));
        assert_eq!((o.as_str(), n.as_str()), (owner, name), "coordinates of {}", raw);
        assert_eq!(b.as_deref(), branch, "branch hint of {}", raw);
    }

    #[test]
    fn provider_for_resolves_each_discriminator() {
        assert_eq!(provider_for("github").unwrap().kind(), "github");
        assert_eq!(provider_for("").unwrap().kind(), "github");
        assert_eq!(provider_for("gitlab").unwrap().kind(), "gitlab");
        // `map` because `unwrap_err` wants `Debug` on the Ok side, which a trait object lacks.
        assert_eq!(
            provider_for("bitbucket").map(|_| ()).unwrap_err(),
            "unsupported provider: bitbucket"
        );
    }

    #[test]
    fn provider_for_reference_dispatches_on_host() {
        for url in [
            "https://gitlab.com/g/r",
            "http://gitlab.com/g/r",
            "gitlab.com/g/r",
            "git@gitlab.com:g/r.git",
        ] {
            assert_eq!(provider_for_reference(url).unwrap().kind(), "gitlab", "{}", url);
        }
        for url in ["https://github.com/o/r", "owner/repo", "https://notgitlab.com/o/r"] {
            assert_eq!(provider_for_reference(url).unwrap().kind(), "github", "{}", url);
        }
    }

    #[test]
    fn gitlab_display_url_uses_the_dash_tree_separator() {
        assert_eq!(
            gitlab().display_url("group", "repo", "main"),
            "https://gitlab.com/group/repo/-/tree/main"
        );
        assert_eq!(
            gitlab().display_url("group/sub", "repo", "dev"),
            "https://gitlab.com/group/sub/repo/-/tree/dev"
        );
    }

    #[test]
    fn gitlab_parses_plain_repo_urls() {
        assert_parsed("https://gitlab.com/group/repo", "group", "repo", None);
        assert_parsed("https://gitlab.com/group/repo/", "group", "repo", None);
        assert_parsed("gitlab.com/group/repo", "group", "repo", None);
        assert_parsed("group/repo", "group", "repo", None);
        assert_parsed("https://gitlab.com/group/repo.git", "group", "repo", None);
        assert_parsed("git@gitlab.com:group/repo.git", "group", "repo", None);
    }

    #[test]
    fn gitlab_parses_tree_urls_into_a_branch_hint() {
        assert_parsed("https://gitlab.com/g/r/-/tree/main", "g", "r", Some("main"));
        assert_parsed(
            "https://gitlab.com/g/r/-/tree/main/skills/foo",
            "g",
            "r",
            Some("main"),
        );
        // A ref may itself contain slashes; only the first segment is claimable.
        assert_parsed("https://gitlab.com/g/r/-/tree/release/1.2", "g", "r", Some("release"));
        assert_parsed("https://gitlab.com/g/r/-/blob/main/SKILL.md", "g", "r", Some("main"));
        assert_parsed("https://gitlab.com/g/r/-/raw/main/SKILL.md", "g", "r", Some("main"));
        // Legacy links omit the `/-/` separator.
        assert_parsed("https://gitlab.com/g/r/tree/main", "g", "r", Some("main"));
    }

    #[test]
    fn gitlab_owns_every_segment_before_the_last_one() {
        assert_parsed("https://gitlab.com/grp/sub/repo", "grp/sub", "repo", None);
        assert_parsed(
            "https://gitlab.com/grp/sub/deep/repo/-/tree/main",
            "grp/sub/deep",
            "repo",
            Some("main"),
        );
        // Non-ref suffixes must not be read as extra namespace segments.
        assert_parsed("https://gitlab.com/grp/repo/-/issues/1", "grp", "repo", None);
        assert_parsed("https://gitlab.com/grp/repo/-/wikis/home", "grp", "repo", None);
        // A repo named after a ref verb still parses: the verb only cuts when an
        // owner + name precede it.
        assert_parsed("https://gitlab.com/user/tree", "user", "tree", None);
    }

    #[test]
    fn gitlab_rejects_unparsable_references() {
        for raw in [
            "",
            "   ",
            "gitlab.com",
            "https://gitlab.com/repo",
            "https://gitlab.com/repo/-/tree/main",
            "just-a-name",
        ] {
            assert!(gitlab().parse_reference(raw).is_err(), "{} should be rejected", raw);
        }
    }

    #[test]
    fn gitlab_encodes_the_namespace_path_for_api_ids() {
        assert_eq!(GitlabProvider::project_id("group", "repo"), "group%2Frepo");
        assert_eq!(
            GitlabProvider::project_id("group/sub/deep", "repo"),
            "group%2Fsub%2Fdeep%2Frepo"
        );
        assert_eq!(GitlabProvider::encode("a b/c"), "a%20b%2Fc");
    }

    #[test]
    fn gitlab_builds_tree_and_file_urls() {
        assert_eq!(
            GitlabProvider::tree_url("grp/sub", "repo", "main", "/skills", 2),
            "https://gitlab.com/api/v4/projects/grp%2Fsub%2Frepo/repository/tree?path=skills&ref=main&per_page=100&page=2"
        );
        assert_eq!(
            GitlabProvider::tree_url("grp", "repo", "main", "", 1),
            "https://gitlab.com/api/v4/projects/grp%2Frepo/repository/tree?path=&ref=main&per_page=100&page=1"
        );
        assert_eq!(
            GitlabProvider::file_url("grp", "repo", "main", "/SKILL.md"),
            "https://gitlab.com/api/v4/projects/grp%2Frepo/repository/files/SKILL.md/raw?ref=main"
        );
        assert_eq!(
            GitlabProvider::file_url("grp/sub", "repo", "dev/x", "/a b/SKILL.md"),
            "https://gitlab.com/api/v4/projects/grp%2Fsub%2Frepo/repository/files/a%20b%2FSKILL.md/raw?ref=dev%2Fx"
        );
    }

    #[test]
    fn gitlab_maps_tree_listing_to_skill_dirs() {
        let entries = serde_json::json!([
            { "name": "skill-a", "path": "skill-a", "type": "tree" },
            { "name": "SKILL.md", "path": "SKILL.md", "type": "blob" },
            { "name": "README.md", "path": "README.md", "type": "blob" },
            { "name": "", "path": "unnamed", "type": "tree" },
            { "path": "nameless", "type": "tree" },
            { "name": "skill-b", "type": "tree" },
        ]);
        assert_eq!(
            GitlabProvider::skill_dirs(entries.as_array().unwrap()),
            vec![("skill-a", "skill-a"), ("skill-b", "skill-b")]
        );
        assert!(GitlabProvider::skill_dirs(&[]).is_empty());
    }

    #[test]
    fn github_adapter_string_shapes_are_unchanged() {
        let github = GithubProvider::new();
        assert_eq!(github.kind(), "github");
        assert_eq!(
            github.display_url("o", "r", "main"),
            "https://github.com/o/r/tree/main"
        );
        assert_eq!(GithubProvider::raw_url("o", "r", "main", "/SKILL.md"), "https://raw.githubusercontent.com/o/r/main/SKILL.md");
        assert_eq!(
            GithubProvider::api_url("o", "r", "main", "/foo"),
            "https://api.github.com/repos/o/r/contents/foo?ref=main"
        );
        assert_eq!(
            github.parse_reference("https://github.com/o/r/tree/main").unwrap(),
            ("o".to_string(), "r".to_string(), None)
        );
        assert_eq!(
            github.parse_reference("anthropics/skills").unwrap(),
            ("anthropics".to_string(), "skills".to_string(), None)
        );
        assert_eq!(
            github.parse_reference("https://github.com/o/r.git").unwrap().1,
            "r"
        );
        assert!(github.parse_reference("nonsense").is_err());
    }
}
