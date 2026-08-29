// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::{Path, PathBuf};

pub(crate) fn app_data_root_from(home: &Path) -> PathBuf {
    home.join(".skill-manager")
}

pub(crate) fn database_path_from(home: &Path) -> PathBuf {
    app_data_root_from(home).join("skill-manager.db")
}

pub(crate) fn database_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(database_path_from(&home))
}

pub(crate) fn settings_path_from(home: &Path) -> PathBuf {
    app_data_root_from(home)
        .join("config")
        .join("settings.json")
}

pub(crate) fn settings_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(settings_path_from(&home))
}

pub(crate) fn ssot_path_from(home: &Path) -> PathBuf {
    app_data_root_from(home).join("ssot")
}

pub(crate) fn ssot_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(ssot_path_from(&home))
}

pub(crate) fn market_cache_path_from(home: &Path) -> PathBuf {
    app_data_root_from(home).join("cache").join("markets")
}

#[allow(dead_code)] // Used by the market snapshot cache introduced in #31.
pub(crate) fn market_cache_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(market_cache_path_from(&home))
}
