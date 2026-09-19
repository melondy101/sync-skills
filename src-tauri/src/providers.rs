// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Market provider seam (ports & adapters).
//!
//! [`MarketProvider`] is the deep interface for "a remote skill marketplace":
//! it hides every provider-specific concern (URL shapes, REST responses, raw
//! download paths, layout probing) behind a small, provider-agnostic surface of
//! seven operations. [`commands::market`](crate::commands::market) and
//! [`ops::market`](crate::ops::market) talk only to the trait, so a marketplace
//! is one adapter: [`GithubProvider`], [`GitlabProvider`] and [`BitbucketProvider`]
//! are the three shipped today, and none required a command or DB orchestration
//! change.
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
        "bitbucket" => Ok(Box::new(BitbucketProvider::new())),
        "azure" => Ok(Box::new(AzureDevopsProvider::new())),
        other => Err(format!("unsupported provider: {}", other)),
    }
}

/// The adapter for an already-registered market. GitLab is the only provider
/// whose instance is part of the identity, and the stored `remote_url` carries
/// that host, so a self-hosted market keeps working across restarts without a
/// schema change.
pub fn provider_for_market(market: &Market) -> Result<Box<dyn MarketProvider>, String> {
    if market.provider == "gitlab" {
        if let Some(base) = instance_base(&market.remote_url) {
            if base != GITLAB_COM {
                return Ok(Box::new(GitlabProvider::with_base(&base)));
            }
        }
    }
    provider_for(&market.provider)
}

/// `scheme://host` out of a display URL, or `None` when there is nothing to read.
fn instance_base(remote_url: &str) -> Option<String> {
    let without_scheme = remote_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = without_scheme.split('/').next()?.trim_end_matches('/');
    if host.is_empty() || !host.contains('.') {
        return None;
    }
    let scheme = if remote_url.starts_with("http://") { "http://" } else { "https://" };
    Some(format!("{scheme}{host}"))
}

/// Drop a leading `host/` from a reference that already had its scheme removed.
/// Only a segment that looks like a hostname (contains a dot) is treated as one,
/// so `group/repo` keeps its group.
fn strip_host(reference: &str) -> &str {
    match reference.split_once('/') {
        Some((head, rest)) if head.contains('.') && !head.contains(':') => rest,
        _ => reference,
    }
}

/// Pick an adapter from a user-entered reference. `add_market_by_url` receives a
/// URL and no provider discriminator, so the host decides here. Only the public
/// SaaS hosts are routed by name: a GitLab adapter speaks to one instance, so an
/// unknown host has to be probed (see [`provider_for_reference_probed`]).
///
/// Accepts the `https://`, `http://`, `ssh://` and bare `git@host:` spellings of
/// the same repository.
pub fn provider_kind_for_reference(raw: &str) -> (&'static str, Option<String>) {
    let trimmed = raw.trim();
    let without_scheme = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("ssh://");
    let head = without_scheme.split(['/', ':']).next().unwrap_or("");
    // `git@gitlab.com:group/repo.git` is the SSH spelling of the same host.
    let host = head.strip_prefix("git@").unwrap_or(head);
    if host.eq_ignore_ascii_case("gitlab.com") {
        return ("gitlab", None);
    }
    if host.eq_ignore_ascii_case("bitbucket.org") {
        return ("bitbucket", None);
    }
    // Azure serves git over a per-org `ssh.dev.azure.com` alias as well as the
    // canonical `dev.azure.com` web host.
    if host.eq_ignore_ascii_case("dev.azure.com")
        || host.ends_with(".dev.azure.com")
        || host.ends_with(".visualstudio.com")
    {
        return ("azure", None);
    }
    // Some other host that still looks like `owner/repo` may be a self-hosted
    // GitLab; hand back its base so the caller can probe, and let GitHub be the
    // fallback if the probe says no.
    if without_scheme.contains('/') && host.contains('.') && !host.eq_ignore_ascii_case("github.com")
    {
        return ("github", instance_base(trimmed));
    }
    ("github", None)
}

/// [`provider_for_reference`] plus one request to tell an anonymous self-hosted
/// GitLab apart from a GitHub Enterprise or arbitrary host. The probe is the
/// cheapest read GitLab answers without credentials: `GET /api/v4/projects` is
/// JSON on GitLab and an error page or 404 everywhere else.
pub async fn provider_for_reference_probed(raw: &str) -> Result<Box<dyn MarketProvider>, String> {
    let (kind, base) = provider_kind_for_reference(raw);
    if kind == "github" {
        if let Some(base) = base {
            if looks_like_gitlab(&base).await {
                return Ok(Box::new(GitlabProvider::with_base(&base)));
            }
        }
    }
    provider_for(kind)
}

async fn looks_like_gitlab(base: &str) -> bool {
    let settings = Settings::load();
    let proxy = http::resolve_proxy(
        settings.use_system_proxy,
        settings.use_proxy,
        settings.proxy_url.as_deref(),
    );
    let client = match http::build_client(
        HttpConfig::new("skill-manager", Duration::from_secs(10)).with_proxy(proxy),
    ) {
        Ok(client) => client,
        Err(_) => return false,
    };
    let url = GitlabProvider::instance_probe_url(base);
    get_json::<Vec<serde_json::Value>>(&client, &url, "application/json")
        .await
        .is_ok()
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

/// GitLab adapter — [`MarketProvider`] over GitLab's REST v4 API. Three shapes
/// force it to diverge from GitHub: the project is addressed by its URL-encoded
/// namespace path (`group%2Fsubgroup%2Frepo`) instead of `owner/repo`, a
/// directory listing is one flat paginated `tree` whose entries carry `path` +
/// `type` (`"blob"`/`"tree"`) instead of per-directory `contents` with
/// `type == "dir"`, and web links need a `/-/` separator before any ref-scoped
/// path.
///
/// `base` is the instance root, so the same adapter serves gitlab.com and a
/// self-hosted `https://gitlab.example.org`. The API paths below the root are
/// identical on both — verified against a live self-hosted instance, whose
/// `tree` and project payloads matched gitlab.com's field for field.
pub struct GitlabProvider {
    client: std::sync::OnceLock<reqwest::Client>,
    base: String,
}

/// The SaaS instance, which is what a bare `owner/repo` reference means.
const GITLAB_COM: &str = "https://gitlab.com";

/// gitlab.com caps `per_page` at 100 for anonymous tree listings.
const TREE_PAGE_SIZE: usize = 100;
/// Only a bound on runaway paging: a short page already ends the walk.
const TREE_MAX_PAGES: usize = 10;

impl GitlabProvider {
    pub fn new() -> Self {
        Self {
            client: std::sync::OnceLock::new(),
            base: GITLAB_COM.to_string(),
        }
    }

    /// Point at a self-hosted instance. Any trailing slash and path suffix are
    /// dropped so `/api/v4/...` always hangs off the instance root.
    pub fn with_base(base: &str) -> Self {
        let trimmed = base
            .trim_end_matches('/')
            .trim_end_matches("/api/v4")
            .to_string();
        Self {
            client: std::sync::OnceLock::new(),
            base: trimmed,
        }
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

    fn project_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/api/v4/projects/{}", self.base, Self::project_id(owner, repo))
    }

    /// The cheapest "is this host a GitLab at all" read: the projects list needs
    /// no credentials on instances that allow anonymous browsing and answers with
    /// a JSON array nowhere else.
    fn instance_probe_url(base: &str) -> String {
        format!("{}/api/v4/projects?per_page=1", base.trim_end_matches('/'))
    }

    fn tree_url(&self, owner: &str, repo: &str, branch: &str, path: &str, page: usize) -> String {
        format!(
            "{}/repository/tree?path={}&ref={}&per_page={}&page={}",
            self.project_url(owner, repo),
            Self::encode(path.trim_start_matches('/')),
            Self::encode(branch),
            TREE_PAGE_SIZE,
            page
        )
    }

    fn file_url(&self, owner: &str, repo: &str, branch: &str, path: &str) -> String {
        format!(
            "{}/repository/files/{}/raw?ref={}",
            self.project_url(owner, repo),
            Self::encode(path.trim_start_matches('/')),
            Self::encode(branch)
        )
    }

    fn web_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/{}/{}", self.base, owner, repo)
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
                .fetch_json(&self.tree_url(owner, repo, branch, path, page))
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
        let url = self.file_url(owner, repo, branch, skill_md_path);
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
        format!("{}/-/tree/{}", self.web_url(owner, repo), branch)
    }

    fn parse_reference(&self, raw: &str) -> Result<(String, String, Option<String>), String> {
        let without_host = strip_host(raw.trim().trim_start_matches("https://").trim_start_matches("http://"));
        let without_host = without_host
            .strip_prefix("git@")
            .map(|rest| match rest.split_once(':') {
                Some((_host, path)) => path,
                None => rest,
            })
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
            let project_url = self.project_url(owner, repo);
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
                .fetch_bytes(&self.file_url(owner, repo, branch, "/SKILL.md"))
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
            let url = self.file_url(&market.owner, &market.name, &market.branch, &path);
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
                self.project_url(owner, repo),
                Self::encode(branch)
            );
            let bytes = self.fetch_bytes(&url).await.ok()?;
            let commits: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
            commits.get(0)?.get("id")?.as_str().map(|s| s.to_string())
        })
    }
}

/// Bitbucket Cloud adapter — [`MarketProvider`] over `api.bitbucket.org/2.0`.
///
/// Bitbucket collapses what GitHub needs two endpoints for into one: `/src/{ref}
/// /{path}` returns a paginated JSON listing when the path is a directory (which
/// it must be told explicitly, with a trailing slash) and the file's raw bytes
/// when it is not. Listing entries carry `path` + `type`
/// (`commit_directory`/`commit_file`) and no `name`, so a skill's name is the
/// last segment of its path.
///
/// Anonymous access is what this adapter can offer without a token, and it runs
/// against a per-IP budget measured in dozens of requests per hour. A `subdir`
/// market costs one request per skill, so a large marketplace can exhaust that
/// budget; when it does, the API answers 429 and [`describe_status`] labels it as
/// rate-limited rather than retrying into the hole.
///
/// Verified against the live API: `src/<branch>/` lists JSON `values` whose
/// entries carry `path` + `type`, `src/<branch>/<file>` returns the raw bytes,
/// `refs/branches/<name>` wraps the commit under `target`, and the repository
/// payload names the default branch `mainbranch.name`.
pub struct BitbucketProvider {
    client: std::sync::OnceLock<reqwest::Client>,
}

/// Bitbucket caps a `pagelen` page at 100 entries.
const SRC_PAGE_SIZE: usize = 100;
/// Only a bound on runaway paging: an absent `next` ends the walk.
const SRC_MAX_PAGES: usize = 20;

impl BitbucketProvider {
    pub fn new() -> Self {
        Self { client: std::sync::OnceLock::new() }
    }

    fn client(&self) -> Result<&reqwest::Client, String> {
        pooled_client(&self.client)
    }

    /// Percent-encode a path or ref. `/` stays verbatim in a path (Bitbucket
    /// takes the whole repo-relative path as free-form tail) but must be escaped
    /// in a ref, where it would otherwise read as another segment.
    fn encode(value: &str, keep_slash: bool) -> String {
        value
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    (b as char).to_string()
                }
                b'/' if keep_slash => "/".to_string(),
                _ => format!("%{:02X}", b),
            })
            .collect()
    }

    fn repo_url(owner: &str, repo: &str) -> String {
        format!(
            "https://api.bitbucket.org/2.0/repositories/{}/{}",
            Self::encode(owner, false),
            Self::encode(repo, false)
        )
    }

    /// Listing URL for a directory. The trailing slash is what asks for a
    /// listing; without it the API reads the path as a file, and the root path
    /// without it collapses to the repository resource itself.
    fn listing_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
        let trimmed = path.trim_start_matches('/').trim_end_matches('/');
        let dir = if trimmed.is_empty() {
            String::new()
        } else {
            format!("{}/", Self::encode(trimmed, true))
        };
        format!(
            "{}/src/{}/{}?pagelen={}",
            Self::repo_url(owner, repo),
            Self::encode(branch, false),
            dir,
            SRC_PAGE_SIZE
        )
    }

    fn file_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
        format!(
            "{}/src/{}/{}",
            Self::repo_url(owner, repo),
            Self::encode(branch, false),
            Self::encode(path.trim_start_matches('/'), true)
        )
    }

    async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        get_bytes(self.client()?, url, "application/json").await
    }

    /// Raw file content. `*/*` because a JSON `Accept` invites the API to answer
    /// a `src` file request with metadata instead of the bytes themselves.
    async fn fetch_raw(&self, url: &str) -> Result<Vec<u8>, String> {
        get_bytes(self.client()?, url, "*/*").await
    }

    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        let bytes = self.fetch_bytes(url).await?;
        // Bitbucket mixes plain REST with the web app: some failures arrive as a
        // 200 and an HTML body, which would otherwise surface as a cryptic
        // `expected value at line 1 column 1`.
        if bytes.starts_with(b"<") {
            return Err("HTML response where JSON was expected".to_string());
        }
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    /// Walk every page by following `next` verbatim. Its `page=` token is opaque
    /// and not a page number, so rebuilding the URL from a counter would loop on
    /// the first page forever.
    async fn fetch_listing(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut url = Self::listing_url(owner, repo, branch, path);
        let mut entries: Vec<serde_json::Value> = Vec::new();
        for _ in 0..SRC_MAX_PAGES {
            let page: serde_json::Value = self.fetch_json(&url).await?;
            if let Some(values) = page.get("values").and_then(|v| v.as_array()) {
                entries.extend(values.iter().cloned());
            }
            let next = page
                .get("next")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            match next {
                Some(next) => url = next,
                None => break,
            }
        }
        Ok(entries)
    }

    /// The listing entries that are candidate skills, as `(name, repo path)`. A
    /// skill's name is the last path segment because these entries carry no
    /// `name`; the trailing slash is trimmed defensively so it cannot leak into
    /// the `SKILL.md` URL as a doubled separator.
    fn skill_dirs(entries: &[serde_json::Value]) -> Vec<(String, String)> {
        entries
            .iter()
            .filter_map(|entry| {
                if entry.get("type").and_then(|v| v.as_str()) != Some("commit_directory") {
                    return None;
                }
                let path = entry.get("path").and_then(|v| v.as_str())?.trim_end_matches('/');
                let name = path.rsplit('/').next()?;
                if name.is_empty() {
                    return None;
                }
                Some((name.to_string(), path.to_string()))
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
        let bytes = self.fetch_raw(&url).await?;
        hash_skill_payload(skill_name, &bytes)
    }

    /// The fallback when a repo payload carries no `mainbranch`: probe the two
    /// conventional names, as the other adapters do.
    async fn probe_branch(&self, owner: &str, repo: &str, candidate: &str) -> bool {
        let url = format!(
            "{}/refs/branches/{}",
            Self::repo_url(owner, repo),
            Self::encode(candidate, false)
        );
        self.fetch_bytes(&url).await.is_ok()
    }
}

impl MarketProvider for BitbucketProvider {
    fn kind(&self) -> &'static str {
        "bitbucket"
    }

    fn display_url(&self, owner: &str, repo: &str, branch: &str) -> String {
        format!("https://bitbucket.org/{}/{}/src/{}", owner, repo, branch)
    }

    fn parse_reference(&self, raw: &str) -> Result<(String, String, Option<String>), String> {
        let trimmed = raw.trim();
        let without_host = trimmed
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let without_host = without_host
            .strip_prefix("bitbucket.org/")
            .or_else(|| without_host.strip_prefix("git@bitbucket.org:"))
            .unwrap_or(without_host)
            .trim_end_matches('/');
        let segments: Vec<&str> = without_host.split('/').filter(|s| !s.is_empty()).collect();
        // A workspace owns exactly one level, unlike GitLab's nested groups, so
        // the first two segments are the coordinates and anything after them is
        // either a ref-scoped path or a section we do not care about.
        if segments.len() < 2 {
            return Err(format!("无法解析市场地址: {}", raw));
        }
        let owner = segments[0].to_string();
        let name = segments[1].trim_end_matches(".git").to_string();
        if owner.is_empty() || name.is_empty() {
            return Err(format!("无法解析市场地址: {}", raw));
        }
        // `src` is the only verb that puts a ref first; `browse`/`raw` do too, and
        // refs with slashes are not claimable beyond their leading segment.
        let branch_hint = segments
            .iter()
            .position(|p| matches!(*p, "src" | "browse" | "raw"))
            .and_then(|idx| segments.get(idx + 1))
            .map(|s| s.to_string());
        Ok((owner, name, branch_hint))
    }

    fn resolve_branch<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let repo_url = Self::repo_url(owner, repo);
            match self.fetch_bytes(&repo_url).await {
                Ok(bytes) => {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                        // Bitbucket calls it `mainbranch`, and unlike the other
                        // two it is an object rather than a bare name.
                        if let Some(name) = json
                            .get("mainbranch")
                            .and_then(|b| b.get("name"))
                            .and_then(|v| v.as_str())
                        {
                            if !name.is_empty() {
                                return Ok(name.to_string());
                            }
                        }
                    }
                    // Metadata reachable but no mainbranch — fall through to probe.
                }
                Err(e) => {
                    return Err(format!("Cannot access {}/{}: {}", owner, repo, e));
                }
            }
            for candidate in ["main", "master"] {
                if self.probe_branch(owner, repo, candidate).await {
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
            let root_md = Self::file_url(owner, repo, branch, "/SKILL.md");
            if self.fetch_raw(&root_md).await.is_ok() {
                return Some("root");
            }
            if let Ok(entries) = self.fetch_listing(owner, repo, branch, "").await {
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

            let mut probes = Vec::new();
            let mut errors = Vec::new();
            let display_base = self.display_url(owner, repo, branch);

            if market.layout == "root" {
                let skill_name = repo.clone();
                let ssot = match ssot_path(&skill_name, 0) {
                    Ok(p) => p,
                    Err(e) => {
                        errors.push(format!("{}: ssot path error: {}", skill_name, e));
                        return Err(errors);
                    }
                };
                let path = "/SKILL.md".to_string();
                match self
                    .probe_skill_md(owner, repo, branch, &skill_name, &path)
                    .await
                {
                    Ok((content_hash, core_hash, description)) => probes.push(RemoteSkillProbe {
                        skill_name,
                        remote_url: display_base,
                        skill_md_repo_path: path,
                        ssot_path: ssot.to_string_lossy().to_string(),
                        remote_content_hash: content_hash,
                        remote_core_hash: core_hash,
                        description,
                    }),
                    Err(e) => errors.push(format!("{}: {}", skill_name, e)),
                }
                return if probes.is_empty() { Err(errors) } else { Ok(probes) };
            }

            let entries = match self.fetch_listing(owner, repo, branch, "").await {
                Ok(v) => v,
                Err(e) => return Err(vec![format!("root listing: {}", e)]),
            };
            for (skill_name, entry_path) in Self::skill_dirs(&entries) {
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
            self.fetch_raw(&url).await
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
                "{}/refs/branches/{}",
                Self::repo_url(owner, repo),
                Self::encode(branch, false)
            );
            let json: serde_json::Value = self.fetch_json(&url).await.ok()?;
            // A branch ref wraps its commit under `target`, as in the GitLab and
            // GitHub listing shapes but one level deeper here.
            json.get("target")?
                .get("hash")?
                .as_str()
                .map(|s| s.to_string())
        })
    }
}

/// Azure DevOps adapter — [`MarketProvider`] over the `dev.azure.com` Git REST
/// API.
///
/// Coordinates are three levels deep here (organisation → project → repository),
/// so `owner` carries `"org/project"` the same way the GitLab adapter carries a
/// nested namespace, and `name` is the repository.
///
/// Read access is gated per organisation, and this is the one adapter that
/// cannot be exercised anonymously: measured on 2026-09-19, `azure-sdk` answers
/// with a 302 to the sign-in page (HTML), and `dnceng/public` lists repositories
/// but returns `TF401019 … you do not have permissions` for repository detail,
/// items and commits. So a token is required to get anywhere useful and it is
/// taken from [`ADO_TOKEN_ENV`] rather than the settings file, which is
/// plaintext. Everything below the auth boundary — URL shapes, payload fields —
/// follows the documented contract, and the failure modes observed live are
/// surfaced as their own errors instead of a bare 404.
pub struct AzureDevopsProvider {
    client: std::sync::OnceLock<reqwest::Client>,
}

/// Set this to a PAT with `Code (read)` to index an organisation that does not
/// allow anonymous access.
pub const ADO_TOKEN_ENV: &str = "AZURE_DEVOPS_PAT";
const ADO_API_VERSION: &str = "7.1";
/// A ref of this shape is how ADO spells "branch" in its version descriptors.
const ADO_BRANCH_PREFIX: &str = "refs/heads/";

impl AzureDevopsProvider {
    pub fn new() -> Self {
        Self { client: std::sync::OnceLock::new() }
    }

    fn client(&self) -> Result<&reqwest::Client, String> {
        pooled_client(&self.client)
    }

    fn token() -> Option<String> {
        std::env::var(ADO_TOKEN_ENV)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    fn encode(value: &str) -> String {
        value
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                    (b as char).to_string()
                }
                _ => format!("%{:02X}", b),
            })
            .collect()
    }

    /// `"org/proj"` + `"repo"` → the repository-scoped API root.
    fn repo_url(owner: &str, repo: &str) -> String {
        format!(
            "https://dev.azure.com/{}/_apis/git/repositories/{}",
            Self::encode(owner),
            Self::encode(repo)
        )
    }

    fn items_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
        format!(
            "{}/items?path={}&versionDescriptor.version={}&versionDescriptor.versionType=branch&recursionLevel=OneLevel&api-version={}",
            Self::repo_url(owner, repo),
            Self::encode(path),
            Self::encode(branch),
            ADO_API_VERSION
        )
    }

    fn blob_url(owner: &str, repo: &str, object_id: &str) -> String {
        format!(
            "{}/objects/{}?api-version={}",
            Self::repo_url(owner, repo),
            Self::encode(object_id),
            ADO_API_VERSION
        )
    }

    fn commits_url(owner: &str, repo: &str) -> String {
        format!(
            "{}/commits?$top=1&api-version={}",
            Self::repo_url(owner, repo),
            ADO_API_VERSION
        )
    }

    /// Folders only; a skill is a directory containing `SKILL.md`, and the entry
    /// keeps its full repo path so nested layouts survive.
    fn skill_dirs(entries: &[serde_json::Value]) -> Vec<(String, String)> {
        entries
            .iter()
            .filter(|entry| entry.get("isFolder").and_then(|v| v.as_bool()) == Some(true))
            .filter_map(|entry| {
                let path = entry.get("path")?.as_str()?;
                let name = path.trim_end_matches('/').rsplit('/').next()?;
                if name.is_empty() {
                    return None;
                }
                Some((name.to_string(), path.trim_start_matches('/').to_string()))
            })
            .collect()
    }

    /// One authenticated GET. The auth-free failure modes observed live are
    /// translated into something the user can act on.
    async fn fetch(&self, url: &str, accept: &str) -> Result<Vec<u8>, String> {
        let mut request = self
            .client()?
            .get(url)
            .header("Accept", accept);
        if let Some(token) = Self::token() {
            // Azure DevOps accepts HTTP basic with the PAT as the password and
            // any (or empty) user name.
            use base64::Engine as _;
            let credentials = base64::engine::general_purpose::STANDARD.encode(format!(":{token}"));
            request = request.header("Authorization", format!("Basic {}", credentials));
        }
        let response = request
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| e.to_string())?.to_vec();
        if !status.is_success() {
            if matches!(status.as_u16(), 401 | 403 | 404) && Self::token().is_none() {
                return Err(format!(
                    "HTTP {} — Azure DevOps needs a read token for this organisation; set {}",
                    status.as_u16(),
                    ADO_TOKEN_ENV
                ));
            }
            return Err(format!(
                "HTTP {} ({})",
                status.as_u16(),
                describe_status(status.as_u16())
            ));
        }
        if bytes.starts_with(b"<") {
            return Err("sign-in page instead of data; this organisation does not allow anonymous access".to_string());
        }
        Ok(bytes)
    }

    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        serde_json::from_slice(&self.fetch(url, "application/json").await?)
            .map_err(|e| e.to_string())
    }

    /// The item's metadata, which is where the blob id for its content lives.
    async fn item_object_id(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
    ) -> Result<String, String> {
        let url = Self::items_url(owner, repo, branch, path);
        let body: serde_json::Value = self.fetch_json(&url).await?;
        body.get("objectId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "item has no objectId".to_string())
    }

    /// Two reads: the item's metadata for its blob id, then the blob itself.
    /// ADO serves file bytes only from the objects endpoint, as octet-stream.
    async fn fetch_item(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
    ) -> Result<Vec<u8>, String> {
        let object_id = self.item_object_id(owner, repo, branch, path).await?;
        self.fetch(
            &Self::blob_url(owner, repo, &object_id),
            "application/octet-stream",
        )
        .await
    }

    async fn probe_skill_md(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        skill_name: &str,
        skill_md_path: &str,
    ) -> Result<(String, String, Option<String>), String> {
        let bytes = self.fetch_item(owner, repo, branch, skill_md_path).await?;
        hash_skill_payload(skill_name, &bytes)
    }
}

impl MarketProvider for AzureDevopsProvider {
    fn kind(&self) -> &'static str {
        "azure"
    }

    fn display_url(&self, owner: &str, repo: &str, branch: &str) -> String {
        // ADO's web UI spells a branch selector as `version=GB<branch>`.
        format!(
            "https://dev.azure.com/{}/_git/{}?version=GB{}",
            owner, repo, branch
        )
    }

    fn parse_reference(&self, raw: &str) -> Result<(String, String, Option<String>), String> {
        let trimmed = raw.trim();
        let without_scheme = trimmed
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("ssh://");
        let without_user = match without_scheme.split_once('@') {
            Some((_user, rest)) => rest,
            None => without_scheme,
        };
        let (_host, path) = without_user
            .split_once('/')
            .ok_or_else(|| format!("无法解析市场地址: {}", raw))?;
        let (path, query) = match path.split_once('?') {
            Some((path, query)) => (path, Some(query)),
            None => (path, None),
        };
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        // `ssh://…@ssh.dev.azure.com/v3/{org}/{project}/{repo}` carries no `_git`
        // marker, so drop the leading API version segment when it is present.
        let segments = match segments.first() {
            Some(head) if *head == "v3" => &segments[1..],
            _ => &segments[..],
        };
        let git_idx = segments.iter().position(|p| *p == "_git");
        let (coordinates, tail) = match git_idx {
            // `_git` is the separator, so everything before it is org + project.
            Some(idx) if idx >= 2 => segments.split_at(idx),
            Some(_) | None if segments.len() == 3 => segments.split_at(2),
            _ => return Err(format!("无法解析市场地址: {}", raw)),
        };
        if tail.first() == Some(&"_git") {
            return Err(format!("无法解析市场地址: {}", raw));
        }
        let owner = coordinates.join("/");
        let name = match tail.first() {
            Some(name) => name.trim_end_matches(".git").to_string(),
            None => return Err(format!("无法解析市场地址: {}", raw)),
        };
        if owner.is_empty() || name.is_empty() {
            return Err(format!("无法解析市场地址: {}", raw));
        }
        // `?version=GBmain`, `GBrefs/heads/main` and `GBrelease%2F1.2` all appear
        // in links the web UI hands out.
        let branch_hint = query.and_then(|query| {
            query.split('&').find_map(|pair| {
                let (key, value) = pair.split_once('=')?;
                if key != "version" {
                    return None;
                }
                let value = value.strip_prefix("GB").unwrap_or(value);
                let value = value.replace("%2F", "/");
                let value = value
                    .strip_prefix(ADO_BRANCH_PREFIX)
                    .unwrap_or(&value)
                    .to_string();
                Some(value)
            })
        });
        Ok((owner, name, branch_hint))
    }

    fn resolve_branch<'a>(&'a self, owner: &'a str, repo: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let url = format!("{}?api-version={}", Self::repo_url(owner, repo), ADO_API_VERSION);
            let body: serde_json::Value = self.fetch_json(&url).await?;
            let default_branch = body
                .get("defaultBranch")
                .and_then(|v| v.as_str())
                .map(|s| s.trim_start_matches(ADO_BRANCH_PREFIX).to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| format!("no defaultBranch for {}/{}", owner, repo))?;
            Ok(default_branch)
        })
    }

    fn detect_layout<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
        branch: &'a str,
    ) -> BoxFuture<'a, Option<&'static str>> {
        Box::pin(async move {
            let url = Self::items_url(owner, repo, branch, "/SKILL.md");
            if self.fetch_json::<serde_json::Value>(&url).await.is_ok() {
                return Some("root");
            }
            let entries: Vec<serde_json::Value> = self
                .fetch_json(&Self::items_url(owner, repo, branch, ""))
                .await
                .ok()?;
            if entries.is_empty() {
                return None;
            }
            Some("subdir")
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
            let display_base = self.display_url(owner, repo, branch);

            if market.layout == "root" {
                let skill_name = repo.clone();
                let ssot = match ssot_path(&skill_name, 0) {
                    Ok(p) => p,
                    Err(e) => return Err(vec![format!("{}: ssot path error: {}", skill_name, e)]),
                };
                let path = "/SKILL.md".to_string();
                return match self
                    .probe_skill_md(owner, repo, branch, &skill_name, &path)
                    .await
                {
                    Ok((content_hash, core_hash, description)) => Ok(vec![RemoteSkillProbe {
                        skill_name,
                        remote_url: display_base,
                        skill_md_repo_path: path,
                        ssot_path: ssot.to_string_lossy().to_string(),
                        remote_content_hash: content_hash,
                        remote_core_hash: core_hash,
                        description,
                    }]),
                    Err(e) => Err(vec![e]),
                };
            }

            let entries: Vec<serde_json::Value> =
                match self.fetch_json(&Self::items_url(owner, repo, branch, "")).await {
                    Ok(v) => v,
                    Err(e) => return Err(vec![format!("root listing: {}", e)]),
                };
            let mut probes = Vec::new();
            let mut errors = Vec::new();
            for (skill_name, entry_path) in Self::skill_dirs(&entries) {
                let skill_md_path = format!("/{}/SKILL.md", entry_path);
                let remote_url = format!("{}&path=%2F{}", display_base, entry_path);
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
            self.fetch_item(&market.owner, &market.name, &market.branch, &path)
                .await
        })
    }

    fn latest_commit_sha<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
        _branch: &'a str,
    ) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            let body: serde_json::Value = self.fetch_json(&Self::commits_url(owner, repo)).await.ok()?;
            body.get("value")?
                .as_array()?
                .first()?
                .get("commitId")?
                .as_str()
                .map(|s| s.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gitlab() -> GitlabProvider {
        GitlabProvider::new()
    }

    fn bitbucket() -> BitbucketProvider {
        BitbucketProvider::new()
    }

    fn assert_parsed(raw: &str, owner: &str, name: &str, branch: Option<&str>) {
        let (o, n, b) = gitlab()
            .parse_reference(raw)
            .unwrap_or_else(|e| panic!("{} should parse: {}", raw, e));
        assert_eq!((o.as_str(), n.as_str()), (owner, name), "coordinates of {}", raw);
        assert_eq!(b.as_deref(), branch, "branch hint of {}", raw);
    }

    /// The adapter a reference would resolve to, without the network probe.
    fn kind_of(url: &str) -> &'static str {
        provider_kind_for_reference(url).0
    }

    #[test]
    fn provider_for_resolves_each_discriminator() {
        assert_eq!(provider_for("github").unwrap().kind(), "github");
        assert_eq!(provider_for("").unwrap().kind(), "github");
        assert_eq!(provider_for("gitlab").unwrap().kind(), "gitlab");
        assert_eq!(provider_for("bitbucket").unwrap().kind(), "bitbucket");
        assert_eq!(provider_for("azure").unwrap().kind(), "azure");
        // `map` because `unwrap_err` wants `Debug` on the Ok side, which a trait object lacks.
        assert_eq!(
            provider_for("azure-devops").map(|_| ()).unwrap_err(),
            "unsupported provider: azure-devops"
        );
    }

    #[test]
    fn azure_references_route_to_the_azure_adapter() {
        for url in [
            "https://dev.azure.com/org/proj/_git/repo",
            "ssh://git@ssh.dev.azure.com/v3/org/proj/repo",
            "https://org@org.visualstudio.com/proj/_git/repo",
        ] {
            assert_eq!(kind_of(url), "azure", "{}", url);
        }
    }

    #[test]
    fn provider_for_reference_dispatches_on_host() {
        for url in [
            "https://gitlab.com/g/r",
            "http://gitlab.com/g/r",
            "gitlab.com/g/r",
            "git@gitlab.com:g/r.git",
        ] {
            assert_eq!(kind_of(url), "gitlab", "{}", url);
        }
        for url in [
            "https://bitbucket.org/ws/repo",
            "http://bitbucket.org/ws/repo",
            "bitbucket.org/ws/repo",
            "git@bitbucket.org:ws/repo.git",
        ] {
            assert_eq!(kind_of(url), "bitbucket", "{}", url);
        }
        for url in ["https://github.com/o/r", "owner/repo", "https://notgitlab.com/o/r"] {
            assert_eq!(kind_of(url), "github", "{}", url);
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
            gitlab().tree_url("grp/sub", "repo", "main", "/skills", 2),
            "https://gitlab.com/api/v4/projects/grp%2Fsub%2Frepo/repository/tree?path=skills&ref=main&per_page=100&page=2"
        );
        assert_eq!(
            gitlab().tree_url("grp", "repo", "main", "", 1),
            "https://gitlab.com/api/v4/projects/grp%2Frepo/repository/tree?path=&ref=main&per_page=100&page=1"
        );
        assert_eq!(
            gitlab().file_url("grp", "repo", "main", "/SKILL.md"),
            "https://gitlab.com/api/v4/projects/grp%2Frepo/repository/files/SKILL.md/raw?ref=main"
        );
        assert_eq!(
            gitlab().file_url("grp/sub", "repo", "dev/x", "/a b/SKILL.md"),
            "https://gitlab.com/api/v4/projects/grp%2Fsub%2Frepo/repository/files/a%20b%2FSKILL.md/raw?ref=dev%2Fx"
        );
    }

    #[test]
    fn a_self_hosted_instance_only_moves_the_root() {
        let gnome = GitlabProvider::with_base("https://gitlab.gnome.org/");
        assert_eq!(gnome.base, "https://gitlab.gnome.org");
        assert_eq!(
            gnome.file_url("roi824", "manuals", "main", "/SKILL.md"),
            "https://gitlab.gnome.org/api/v4/projects/roi824%2Fmanuals/repository/files/SKILL.md/raw?ref=main"
        );
        assert_eq!(
            gnome.display_url("roi824", "manuals", "main"),
            "https://gitlab.gnome.org/roi824/manuals/-/tree/main"
        );
        // Pasting the API root instead of the instance root must not double it.
        assert_eq!(
            GitlabProvider::with_base("https://gitlab.example.org/api/v4").base,
            "https://gitlab.example.org"
        );
    }

    #[test]
    fn a_self_hosted_reference_parses_like_a_saas_one() {
        let gnome = GitlabProvider::with_base("https://gitlab.gnome.org");
        let (owner, name, branch) = gnome
            .parse_reference("https://gitlab.gnome.org/roi824/manuals/-/tree/main/doc")
            .expect("self-hosted link");
        assert_eq!((owner.as_str(), name.as_str(), branch.as_deref()), ("roi824", "manuals", Some("main")));
    }

    #[test]
    fn the_instance_probe_is_the_anonymous_projects_list() {
        assert_eq!(
            GitlabProvider::instance_probe_url("https://gitlab.gnome.org/"),
            "https://gitlab.gnome.org/api/v4/projects?per_page=1"
        );
    }

    #[test]
    fn provider_kind_routes_known_hosts_without_the_network() {
        assert_eq!(provider_kind_for_reference("https://gitlab.com/g/r"), ("gitlab", None));
        assert_eq!(
            provider_kind_for_reference("https://bitbucket.org/ws/r"),
            ("bitbucket", None)
        );
        assert_eq!(
            provider_kind_for_reference("https://dev.azure.com/org/proj/_git/repo"),
            ("azure", None)
        );
        assert_eq!(
            provider_kind_for_reference("https://org.visualstudio.com/proj/_git/repo"),
            ("azure", None)
        );
        // A bare `owner/repo` and a github.com URL need no probe at all.
        assert_eq!(provider_kind_for_reference("owner/repo"), ("github", None));
        assert_eq!(
            provider_kind_for_reference("https://github.com/o/r"),
            ("github", None)
        );
        // An unknown host that looks like owner/repo is offered for probing.
        assert_eq!(
            provider_kind_for_reference("https://gitlab.gnome.org/roi824/manuals"),
            ("github", Some("https://gitlab.gnome.org".to_string()))
        );
    }

    #[test]
    fn instance_base_reads_a_stored_display_url() {
        assert_eq!(
            instance_base("https://gitlab.gnome.org/roi824/manuals/-/tree/main"),
            Some("https://gitlab.gnome.org".to_string())
        );
        assert_eq!(instance_base("http://git.local/grp/r/-/tree/main"), Some("http://git.local".to_string()));
        assert_eq!(instance_base("not a url"), None);
    }

    #[test]
    fn provider_for_market_keeps_the_instance_from_the_stored_url() {
        let market = Market {
            id: 1,
            provider: "gitlab".to_string(),
            owner: "roi824".to_string(),
            name: "manuals".to_string(),
            branch: "main".to_string(),
            layout: "subdir".to_string(),
            enabled: true,
            remote_url: "https://gitlab.gnome.org/roi824/manuals/-/tree/main".to_string(),
            last_commit_sha: None,
            last_indexed_at: None,
            last_checked_at: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        assert_eq!(provider_for_market(&market).unwrap().kind(), "gitlab");
        // The SaaS URL keeps the default base rather than being re-derived.
        let saas = Market {
            remote_url: "https://gitlab.com/g/r/-/tree/main".to_string(),
            ..market
        };
        assert_eq!(provider_for_market(&saas).unwrap().kind(), "gitlab");
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
    fn bitbucket_urls_and_coordinates() {
        assert_eq!(
            bitbucket().display_url("ws", "repo", "main"),
            "https://bitbucket.org/ws/repo/src/main"
        );
        for raw in [
            "https://bitbucket.org/ws/repo",
            "http://bitbucket.org/ws/repo/",
            "bitbucket.org/ws/repo",
            "git@bitbucket.org:ws/repo.git",
            "ws/repo",
        ] {
            let (o, n, b) = bitbucket()
                .parse_reference(raw)
                .unwrap_or_else(|e| panic!("{} should parse: {}", raw, e));
            assert_eq!((o.as_str(), n.as_str()), ("ws", "repo"), "{}", raw);
            assert_eq!(b, None, "no ref to read from {}", raw);
        }
        // `src/<ref>` is where a branch can be claimed; the rest is a repo path.
        let (o, n, b) = bitbucket()
            .parse_reference("https://bitbucket.org/ws/repo/src/main/skills/foo")
            .unwrap();
        assert_eq!((o.as_str(), n.as_str(), b.as_deref()), ("ws", "repo", Some("main")));
        let (_, _, b) = bitbucket()
            .parse_reference("https://bitbucket.org/ws/repo/browse/master/SKILL.md")
            .unwrap();
        assert_eq!(b.as_deref(), Some("master"));
        for raw in ["", "   ", "bitbucket.org", "https://bitbucket.org/ws", "just-a-name"] {
            assert!(bitbucket().parse_reference(raw).is_err(), "{} should be rejected", raw);
        }
    }

    #[test]
    fn bitbucket_asks_for_a_listing_with_a_trailing_slash() {
        assert_eq!(
            BitbucketProvider::listing_url("ws", "repo", "main", ""),
            "https://api.bitbucket.org/2.0/repositories/ws/repo/src/main/?pagelen=100"
        );
        assert_eq!(
            BitbucketProvider::listing_url("ws", "repo", "main", "/skills"),
            "https://api.bitbucket.org/2.0/repositories/ws/repo/src/main/skills/?pagelen=100"
        );
        assert_eq!(
            BitbucketProvider::file_url("ws", "repo", "main", "/skills/a/SKILL.md"),
            "https://api.bitbucket.org/2.0/repositories/ws/repo/src/main/skills/a/SKILL.md"
        );
        // A slash in a ref must not become another path segment, while a slash in
        // a path stays a path separator and a space still gets escaped.
        assert_eq!(
            BitbucketProvider::file_url("ws", "repo", "release/1.2", "/a b/SKILL.md"),
            "https://api.bitbucket.org/2.0/repositories/ws/repo/src/release%2F1.2/a%20b/SKILL.md"
        );
    }

    #[test]
    fn bitbucket_paths_survive_a_personal_workspace_slug() {
        // `~<account id>` is the legacy spelling of a personal workspace; `~` is
        // an unreserved character and must reach the API untouched.
        assert_eq!(
            BitbucketProvider::repo_url("~melondy", "repo"),
            "https://api.bitbucket.org/2.0/repositories/~melondy/repo"
        );
    }

    #[test]
    fn bitbucket_reads_skill_dirs_from_path_not_name() {
        let entries = serde_json::json!([
            { "path": "skill-a/", "type": "commit_directory" },
            { "path": "skills/deep/skill-b", "type": "commit_directory" },
            { "path": "SKILL.md", "type": "commit_file" },
            { "type": "commit_directory" },
            { "path": "/", "type": "commit_directory" },
        ]);
        assert_eq!(
            BitbucketProvider::skill_dirs(entries.as_array().unwrap()),
            vec![
                ("skill-a".to_string(), "skill-a".to_string()),
                ("skill-b".to_string(), "skills/deep/skill-b".to_string()),
            ]
        );
        assert!(BitbucketProvider::skill_dirs(&[]).is_empty());
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
