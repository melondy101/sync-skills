// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared HTTP client factory — a portable leaf module.
//!
//! This module deliberately has **no `crate::` dependencies**: it takes only
//! primitives (`Duration`, `&str`, `bool`) so the whole file can be lifted into
//! another Tauri/Rust project verbatim. It swallows the user-agent, connect
//! timeout, redirect policy and (system/manual) proxy handling that were
//! previously duplicated across `commands::market` and `commands::updater`.
//!
//! Interface invariant: the consuming crate MUST enable reqwest's
//! `system-proxy` + `rustls-tls-native-roots` features, otherwise
//! [`ProxySource::System`] silently degrades to a direct connection.

use std::time::Duration;

/// Whether the client should follow HTTP redirects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectPolicy {
    /// Follow up to reqwest's default number of redirects (the common case).
    Follow,
    /// Do not follow redirects; surface the 3xx response to the caller.
    None,
}

/// Where the proxy (if any) comes from. Normalization of a bare
/// `host:port` into `http://host:port` is hidden behind [`build_client`].
#[derive(Debug, Clone)]
pub enum ProxySource {
    /// A manually configured proxy URL, possibly without a scheme prefix.
    Manual(String),
    /// The OS/system proxy (Windows registry lookup, cached process-wide).
    System,
}

/// Everything needed to construct a client. Built once per call site from
/// already-resolved primitives — this module never reads app settings itself.
#[derive(Debug, Clone)]
pub struct HttpConfig<'a> {
    pub user_agent: &'a str,
    pub connect_timeout: Duration,
    pub redirects: RedirectPolicy,
    pub proxy: Option<ProxySource>,
}

impl<'a> HttpConfig<'a> {
    /// A sensible default: given UA and timeout, follow redirects, no proxy.
    pub fn new(user_agent: &'a str, connect_timeout: Duration) -> Self {
        Self {
            user_agent,
            connect_timeout,
            redirects: RedirectPolicy::Follow,
            proxy: None,
        }
    }

    pub fn with_proxy(mut self, proxy: Option<ProxySource>) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn with_redirects(mut self, redirects: RedirectPolicy) -> Self {
        self.redirects = redirects;
        self
    }
}

/// Collapse the three formerly-duplicated proxy branches into one pure
/// decision. Callers read their own settings and pass the values in, so this
/// module stays free of any app-specific `Settings` type.
///
/// Precedence (matching the historical behavior): on Windows a set
/// `use_system_proxy` wins outright; otherwise a manual proxy is used only when
/// `use_proxy` is set and `proxy_url` is non-empty.
pub fn resolve_proxy(
    use_system_proxy: bool,
    use_proxy: bool,
    proxy_url: Option<&str>,
) -> Option<ProxySource> {
    #[cfg(windows)]
    if use_system_proxy {
        return Some(ProxySource::System);
    }
    #[cfg(not(windows))]
    let _ = use_system_proxy;

    if !use_proxy {
        return None;
    }
    let url = proxy_url?.trim();
    if url.is_empty() {
        return None;
    }
    Some(ProxySource::Manual(url.to_string()))
}

/// Construct a pooled [`reqwest::Client`] from an [`HttpConfig`].
///
/// Invariant: never fails because of a missing proxy — if the system proxy
/// cannot be resolved the client is simply built for a direct connection.
pub fn build_client(cfg: HttpConfig<'_>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(cfg.user_agent)
        .connect_timeout(cfg.connect_timeout);

    if cfg.redirects == RedirectPolicy::None {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }

    if let Some(source) = cfg.proxy.as_ref() {
        let proxy = match source {
            ProxySource::System => system_proxy(),
            ProxySource::Manual(url) => {
                let normalized = if url.contains("://") {
                    url.clone()
                } else {
                    format!("http://{}", url)
                };
                reqwest::Proxy::all(normalized).ok()
            }
        };
        if let Some(proxy) = proxy {
            builder = builder.proxy(proxy);
        }
    }

    builder.build().map_err(|e| e.to_string())
}

/// Read the Windows system proxy from the registry, caching the result for the
/// process lifetime. Returns `None` on non-Windows platforms.
#[cfg(windows)]
fn system_proxy() -> Option<reqwest::Proxy> {
    static CACHED: std::sync::OnceLock<Option<reqwest::Proxy>> = std::sync::OnceLock::new();
    CACHED.get_or_init(|| {
        let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
            .ok()?;

        let enabled: u32 = key.get_value("ProxyEnable").ok()?;
        if enabled == 0 {
            return None;
        }

        let server: String = key.get_value("ProxyServer").ok()?;
        let server = server.trim();
        if server.is_empty() {
            return None;
        }

        let url = if server.contains("://") {
            server.to_string()
        } else {
            format!("http://{}", server)
        };
        reqwest::Proxy::all(url).ok()
    })
    .clone()
}

#[cfg(not(windows))]
fn system_proxy() -> Option<reqwest::Proxy> {
    None
}
