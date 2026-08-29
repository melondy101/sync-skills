// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application settings, persisted as JSON at ~/.skill-manager/config/settings.json
///
/// Per-field `#[serde(default = "...")]` attributes preserve legacy-tolerance
/// for older settings.json payloads that omit any given field. Dropping them
/// in favour of a struct-level `#[serde(default)]` would silently revert every
/// absent field to its default, wiping the user's other configured values on
/// minor schema drift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Sync mode: "semi-auto" (default) or "full-auto"
    #[serde(default = "default_sync_mode")]
    pub sync_mode: String,
    /// Whether to prefer symlinks over copies
    #[serde(default)]
    pub prefer_symlink: bool,
    /// Theme: "light" (default), "dark", or "system"
    pub theme: String,
    /// Language: "zh" (default) or "en"
    pub language: String,
    /// Window close behavior: "exit" (default), "minimize", or "tray"
    #[serde(default = "default_close_action")]
    pub close_action: String,
    /// Whether to automatically honor the system proxy when checking/downloading updates
    #[serde(default = "default_true")]
    pub use_system_proxy: bool,
    /// Whether to use a custom proxy for update checks
    #[serde(default)]
    pub use_proxy: bool,
    /// Custom proxy URL for update checks, e.g. http://127.0.0.1:10090
    #[serde(default)]
    pub proxy_url: Option<String>,
    /// When true (default), the Market update modal shows a diff view before
    /// installing. When false, only the commit message + hash are shown.
    /// Phase 4 / issue #5 (T1 — Settings extension).
    #[serde(default = "default_true")]
    pub view_market_diff_before_update: bool,
    /// When true, the file-system watcher (Phase 4 / M12) auto-syncs on
    /// SKILL.md change. Mirrors `sync_mode == "full-auto"`; kept separate so
    /// the UI can label it clearly. The canonical OR is exposed via
    /// [`Settings::effective_auto_sync_on_file_change`] for downstream callers
    /// (file watcher, IPC, UI).
    /// Phase 4 / issue #5 (T1 — Settings extension).
    #[serde(default)]
    pub auto_sync_on_file_change: bool,
}

fn default_sync_mode() -> String {
    "semi-auto".to_string()
}

fn default_close_action() -> String {
    "exit".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            sync_mode: default_sync_mode(),
            prefer_symlink: false,
            theme: "light".to_string(),
            language: "zh".to_string(),
            close_action: default_close_action(),
            use_system_proxy: default_true(),
            use_proxy: false,
            proxy_url: None,
            view_market_diff_before_update: default_true(),
            auto_sync_on_file_change: false,
        }
    }
}

impl Settings {
    fn settings_path() -> Result<PathBuf, String> {
        crate::app_paths::settings_path()
    }

    /// Load settings from disk, or return defaults. Older JSON files that
    /// omit any field fall back to that field's default via per-field
    /// `#[serde(default = "...")]` attributes, while preserving the rest of
    /// the user's configuration.
    pub fn load() -> Self {
        match Self::settings_path() {
            Ok(path) if path.exists() => {
                match fs::read_to_string(&path) {
                    Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                    Err(_) => Settings::default(),
                }
            }
            _ => Settings::default(),
        }
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::settings_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create settings directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&path, json)
            .map_err(|e| format!("Failed to write settings: {}", e))?;

        Ok(())
    }

    /// Canonical source of truth for "should the file watcher auto-sync on
    /// SKILL.md changes?". The watcher (Phase 4 / M12, landed in a later
    /// ticket) reads this OR directly. Keeping the derivation here (rather
    /// than in the UI or in command sites) ensures every consumer agrees.
    // `dead_code` allowed: the watcher is not in T1 (Settings extension).
    // The next ticket (M12) adds the first caller; until then the helper
    // still serves as the canonical contract for `Settings` ↔ watcher and is
    // exercised by the unit tests in this module.
    #[allow(dead_code)]
    pub fn effective_auto_sync_on_file_change(&self) -> bool {
        self.auto_sync_on_file_change || self.sync_mode == "full-auto"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        let s = Settings::default();
        assert_eq!(s.sync_mode, "semi-auto");
        assert_eq!(s.theme, "light");
        assert_eq!(s.language, "zh");
        assert_eq!(s.close_action, "exit");
        assert!(s.use_system_proxy);
        assert!(!s.use_proxy);
        assert_eq!(s.proxy_url, None);
        // Phase 4 / T1 additions
        assert!(
            s.view_market_diff_before_update,
            "Market diff should default to on"
        );
        assert!(
            !s.auto_sync_on_file_change,
            "Auto-sync on file change should default to off"
        );
    }

    #[test]
    fn effective_auto_sync_full_auto_wins() {
        // full-auto mode implies auto-sync even when the explicit flag is off.
        let s = Settings {
            sync_mode: "full-auto".to_string(),
            ..Settings::default()
        };
        assert!(
            s.effective_auto_sync_on_file_change(),
            "full-auto sync_mode should enable auto-sync regardless of the flag"
        );
    }

    #[test]
    fn effective_auto_sync_explicit_flag_works() {
        // Explicit flag enables auto-sync even in semi-auto mode.
        let s = Settings {
            sync_mode: "semi-auto".to_string(),
            auto_sync_on_file_change: true,
            ..Settings::default()
        };
        assert!(
            s.effective_auto_sync_on_file_change(),
            "explicit flag should enable auto-sync even in semi-auto"
        );
    }

    #[test]
    fn effective_auto_sync_off_by_default_in_semi_auto() {
        let s = Settings::default();
        assert_eq!(s.sync_mode, "semi-auto");
        assert!(!s.auto_sync_on_file_change);
        assert!(!s.effective_auto_sync_on_file_change());
    }

    #[test]
    fn loads_legacy_settings_without_new_fields() {
        // Older settings.json (pre-Phase 4) omits the two new fields AND
        // omits one of the older fields (`prefer_symlink`). Loading should
        // fall back to defaults for the missing fields and preserve the rest.
        let legacy = r#"{
            "sync_mode": "full-auto",
            "theme": "dark",
            "language": "en",
            "close_action": "minimize",
            "use_system_proxy": false,
            "use_proxy": true,
            "proxy_url": "http://127.0.0.1:7890"
        }"#;
        let s: Settings = serde_json::from_str(legacy).expect("legacy parse");
        assert_eq!(s.sync_mode, "full-auto");
        assert!(!s.prefer_symlink, "missing prefer_symlink should default to false");
        assert_eq!(s.theme, "dark");
        assert_eq!(s.language, "en");
        assert_eq!(s.close_action, "minimize");
        assert!(!s.use_system_proxy);
        assert!(s.use_proxy);
        assert_eq!(s.proxy_url.as_deref(), Some("http://127.0.0.1:7890"));
        assert!(
            s.view_market_diff_before_update,
            "missing field should default to true"
        );
        assert!(
            !s.auto_sync_on_file_change,
            "missing field should default to false"
        );
    }

    #[test]
    fn roundtrip_serialize_then_deserialize_preserves_new_fields() {
        let s = Settings {
            view_market_diff_before_update: false,
            auto_sync_on_file_change: true,
            ..Settings::default()
        };
        let json = serde_json::to_string_pretty(&s).expect("serialize");
        let restored: Settings = serde_json::from_str(&json).expect("deserialize");
        assert!(!restored.view_market_diff_before_update);
        assert!(restored.auto_sync_on_file_change);
    }
}

