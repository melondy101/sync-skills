// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! App self-update: check GitHub Releases, download the platform installer
//! to a local temp dir (with progress events) and launch it for one-click install.

use serde::Serialize;
use tauri::Emitter;

const RELEASES_API: &str =
    "https://api.github.com/repos/huang-yi-dae/sync-skills/releases/latest";
const RELEASES_PAGE: &str = "https://github.com/huang-yi-dae/sync-skills/releases";
const DOWNLOAD_PREFIX: &str =
    "https://github.com/huang-yi-dae/sync-skills/releases/download/";

#[derive(Serialize, Clone)]
pub struct AppUpdateInfo {
    pub latest_version: String,
    pub release_url: String,
    pub update_available: bool,
    pub no_releases: bool,
    /// Installer asset matching the current platform, if any.
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub asset_size: Option<u64>,
}

#[derive(Serialize, Clone)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
}

/// Compare dotted numeric versions; returns >0 if a is newer than b.
fn compare_versions(a: &str, b: &str) -> i64 {
    let parse = |v: &str| -> Vec<i64> {
        v.split('.').map(|s| s.trim().parse().unwrap_or(0)).collect()
    };
    let (pa, pb) = (parse(a), parse(b));
    for i in 0..pa.len().max(pb.len()) {
        let d = pa.get(i).copied().unwrap_or(0) - pb.get(i).copied().unwrap_or(0);
        if d != 0 {
            return d;
        }
    }
    0
}

/// Pick the release asset matching the current platform's installer format.
fn pick_asset(assets: &[serde_json::Value]) -> Option<(String, String, u64)> {
    // Ordered by preference per platform.
    let suffixes: &[&str] = if cfg!(target_os = "windows") {
        &["-setup.exe", ".exe", ".msi"]
    } else if cfg!(target_os = "macos") {
        &[".dmg"]
    } else {
        &[".appimage", ".deb"]
    };
    for suffix in suffixes {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or("");
            if !name.to_lowercase().ends_with(suffix) {
                continue;
            }
            let url = asset["browser_download_url"].as_str().unwrap_or("");
            if !url.is_empty() {
                return Some((
                    name.to_string(),
                    url.to_string(),
                    asset["size"].as_u64().unwrap_or(0),
                ));
            }
        }
    }
    None
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("skill-manager")
        .build()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_app_update(app: tauri::AppHandle) -> Result<AppUpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let resp = http_client()?
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().as_u16() == 404 {
        // Repo has no published releases yet
        return Ok(AppUpdateInfo {
            latest_version: current,
            release_url: RELEASES_PAGE.to_string(),
            update_available: false,
            no_releases: true,
            asset_name: None,
            asset_url: None,
            asset_size: None,
        });
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let rel: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let latest = rel["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    let release_url = rel["html_url"].as_str().unwrap_or(RELEASES_PAGE).to_string();
    let update_available = !latest.is_empty() && compare_versions(&latest, &current) > 0;
    let empty = Vec::new();
    let assets = rel["assets"].as_array().unwrap_or(&empty);
    let (asset_name, asset_url, asset_size) = if update_available {
        match pick_asset(assets) {
            Some((name, url, size)) => (Some(name), Some(url), Some(size)),
            None => (None, None, None),
        }
    } else {
        (None, None, None)
    };
    Ok(AppUpdateInfo {
        latest_version: if latest.is_empty() { current } else { latest },
        release_url,
        update_available,
        no_releases: false,
        asset_name,
        asset_url,
        asset_size,
    })
}

/// Download the installer asset to a local temp dir, emitting
/// `app-update-progress` events. Returns the saved file path.
#[tauri::command]
pub async fn download_app_update(
    app: tauri::AppHandle,
    url: String,
    file_name: String,
) -> Result<String, String> {
    // Only allow official release assets of this repo
    if !url.starts_with(DOWNLOAD_PREFIX) {
        return Err("Invalid download URL".to_string());
    }
    let mut resp = http_client()?
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);

    let dir = std::env::temp_dir().join("skill-manager-updates");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Keep only the plain file name to avoid path traversal
    let safe_name = std::path::Path::new(&file_name)
        .file_name()
        .map(|s| s.to_os_string())
        .ok_or_else(|| "Invalid file name".to_string())?;
    let path = dir.join(safe_name);

    use std::io::Write;
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        let _ = app.emit("app-update-progress", DownloadProgress { downloaded, total });
    }
    file.flush().map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// Launch the downloaded installer and exit the app so files can be replaced.
#[tauri::command]
pub fn install_app_update(app: tauri::AppHandle, installer_path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&installer_path);
    if !path.exists() {
        return Err("Installer file not found".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // Spawned children are detached on Windows, so they survive app exit.
        if installer_path.to_lowercase().ends_with(".msi") {
            // Passive upgrade: progress bar only, msiexec removes the old
            // version itself instead of asking the user to uninstall first
            std::process::Command::new("msiexec")
                .args(["/i", &installer_path, "/passive", "/norestart"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            // Tauri NSIS installer: /P = passive upgrade-in-place (silently
            // replaces the old version), /R = relaunch the app when done
            std::process::Command::new(&installer_path)
                .args(["/P", "/R"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        // Mounts the dmg in Finder
        std::process::Command::new("open")
            .arg(&installer_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // AppImage needs the executable bit before it can run
        if installer_path.to_lowercase().ends_with(".appimage") {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
        }
        std::process::Command::new("xdg-open")
            .arg(&installer_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    // Exit so the installer can replace the running binary
    app.exit(0);
    Ok(())
}
