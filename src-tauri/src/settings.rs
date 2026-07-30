// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application settings, persisted as JSON at ~/.agents/settings.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Sync mode: "semi-auto" (default) or "full-auto"
    pub sync_mode: String,
    /// Whether to prefer symlinks over copies
    pub prefer_symlink: bool,
    /// Theme: "light" (default), "dark", or "system"
    pub theme: String,
    /// Language: "zh" (default) or "en"
    pub language: String,
    /// Window close behavior: "exit" (default) or "minimize"
    #[serde(default = "default_close_action")]
    pub close_action: String,
}

fn default_close_action() -> String {
    "exit".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            sync_mode: "semi-auto".to_string(),
            prefer_symlink: false,
            theme: "light".to_string(),
            language: "zh".to_string(),
            close_action: default_close_action(),
        }
    }
}

impl Settings {
    fn settings_path() -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or("Cannot find home directory")?;
        Ok(home.join(".agents").join("settings.json"))
    }

    /// Load settings from disk, or return defaults
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
}
