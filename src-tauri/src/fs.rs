// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Portable filesystem helpers — a leaf module with **no `crate::` dependencies**
//! and **no domain knowledge** (it knows nothing about skills, SSOT or tools).
//! This is intentionally the most liftable module in the backend: it can be
//! copied into another project verbatim.
//!
//! Previously these lived in `sync.rs` alongside the SSOT identity logic; they
//! are unrelated to the "name-as-identity" domain rules, so they now sit behind
//! their own small interface. `remove_tree_or_link` is the semantic,
//! domain-word-free name for what used to be `remove_installed_skill`.

use std::fs;
use std::path::Path;

/// Recursively copy all files from `src` directory to `dst` directory.
/// Creates `dst` if it doesn't exist. Overwrites existing files.
pub fn copy_directory(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        return Err(format!("Source directory does not exist: {:?}", src));
    }
    if !src.is_dir() {
        return Err(format!("Source is not a directory: {:?}", src));
    }

    fs::create_dir_all(dst).map_err(|e| format!("Failed to create target directory: {}", e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read source directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let entry_path = entry.path();

        // Skip hidden files
        if is_hidden(&entry_path) {
            continue;
        }

        let dest_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_directory(&entry_path, &dest_path)?;
        } else {
            fs::copy(&entry_path, &dest_path).map_err(|e| {
                format!(
                    "Failed to copy {:?} to {:?}: {}",
                    entry_path, dest_path, e
                )
            })?;
        }
    }

    Ok(())
}

/// Replace the `dst` directory with contents from `src`.
/// Simple strategy: delete old dst if it exists, then copy src to dst.
pub fn replace_directory(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() || !src.is_dir() {
        return Err(format!("Invalid source directory: {:?}", src));
    }

    let parent = dst.parent().ok_or("Target has no parent directory")?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create parent directory: {}", e))?;

    // Remove old destination if it exists
    if dst.exists() {
        fs::remove_dir_all(dst)
            .map_err(|e| format!("Failed to remove old target {:?}: {}", dst, e))?;
    }

    // Copy source to destination
    copy_directory(src, dst)
}

/// Remove a directory or symlink at `path`, without ever following a symlink to
/// its target. Returns `Ok(true)` if something was removed, `Ok(false)` if
/// nothing existed at `path`.
pub fn remove_tree_or_link(path: &Path) -> Result<bool, String> {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return Ok(false), // nothing to remove
    };

    if meta.file_type().is_symlink() {
        // Delete the link itself, never its target
        #[cfg(windows)]
        let res = fs::remove_dir(path).or_else(|_| fs::remove_file(path));
        #[cfg(not(windows))]
        let res = fs::remove_file(path);
        res.map_err(|e| format!("Failed to remove symlink {:?}: {}", path, e))?;
    } else if meta.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to remove directory {:?}: {}", path, e))?;
    } else {
        fs::remove_file(path)
            .map_err(|e| format!("Failed to remove file {:?}: {}", path, e))?;
    }
    Ok(true)
}

/// Try to create a symlink from `link_path` pointing to `target`.
/// On failure (e.g., Windows without developer mode), falls back to copy.
pub fn symlink_or_copy(target: &Path, link_path: &Path) -> Result<String, String> {
    if !target.exists() {
        return Err(format!("Symlink target does not exist: {:?}", target));
    }

    // Remove existing link_path if it exists
    if link_path.exists() {
        if link_path.is_dir() {
            fs::remove_dir_all(link_path)
                .map_err(|e| format!("Failed to remove existing directory: {}", e))?;
        } else {
            fs::remove_file(link_path)
                .map_err(|e| format!("Failed to remove existing file: {}", e))?;
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    // Try symlink
    #[cfg(target_os = "windows")]
    let symlink_result = if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, link_path)
    } else {
        std::os::windows::fs::symlink_file(target, link_path)
    };

    #[cfg(not(target_os = "windows"))]
    let symlink_result = std::os::unix::fs::symlink(target, link_path);

    match symlink_result {
        Ok(()) => Ok("symlink".to_string()),
        Err(_) => {
            // Symlink failed, fallback to copy
            if target.is_dir() {
                copy_directory(target, link_path)?;
            } else {
                fs::copy(target, link_path)
                    .map_err(|e| format!("Fallback copy failed: {}", e))?;
            }
            Ok("copy".to_string())
        }
    }
}

/// Check if a path is hidden
fn is_hidden(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        if name.to_string_lossy().starts_with('.') {
            return true;
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(metadata) = path.metadata() {
            const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
            return metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0;
        }
    }

    false
}
