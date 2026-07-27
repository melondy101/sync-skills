// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs;
use std::path::{Path, PathBuf};

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

/// Remove an installed skill (directory or symlink) from a tool's skills folder.
/// Symlinks are removed without following them, so the SSOT copy is never touched.
/// Returns Ok(true) if something was removed, Ok(false) if nothing existed.
pub fn remove_installed_skill(dir: &Path) -> Result<bool, String> {
    let meta = match fs::symlink_metadata(dir) {
        Ok(m) => m,
        Err(_) => return Ok(false), // nothing to remove
    };

    if meta.file_type().is_symlink() {
        // Delete the link itself, never its target
        #[cfg(windows)]
        let res = fs::remove_dir(dir).or_else(|_| fs::remove_file(dir));
        #[cfg(not(windows))]
        let res = fs::remove_file(dir);
        res.map_err(|e| format!("Failed to remove symlink {:?}: {}", dir, e))?;
    } else if meta.is_dir() {
        fs::remove_dir_all(dir)
            .map_err(|e| format!("Failed to remove directory {:?}: {}", dir, e))?;
    } else {
        fs::remove_file(dir)
            .map_err(|e| format!("Failed to remove file {:?}: {}", dir, e))?;
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

/// Create the `local.md` marker file in a skill directory.
pub fn create_local_marker(dir: &Path) -> Result<(), String> {
    let marker = dir.join("local.md");
    if !marker.exists() {
        fs::write(
            &marker,
            "# Local Skill\n\nThis skill is managed by Skill Manager.\n",
        )
        .map_err(|e| format!("Failed to create local.md marker: {}", e))?;
    }
    Ok(())
}

/// Check if a skill directory has the `local.md` marker (bidirectional sync).
#[allow(dead_code)] // Reserved for future use
pub fn is_local_skill(dir: &Path) -> bool {
    dir.join("local.md").exists()
}

/// Validate a skill name before using it as a single path component.
/// Skill names come from YAML front matter (untrusted input), so reject
/// anything that could escape the SSOT base directory or break the filesystem.
fn validate_skill_name(skill_name: &str) -> Result<(), String> {
    if skill_name.trim().is_empty() {
        return Err("Skill name is empty".to_string());
    }
    if skill_name == "." || skill_name == ".." {
        return Err(format!("Invalid skill name: '{}'", skill_name));
    }
    if skill_name.contains('/') || skill_name.contains('\\') || skill_name.contains('\0') {
        return Err(format!(
            "Invalid skill name '{}': path separators are not allowed",
            skill_name
        ));
    }
    Ok(())
}

/// Get the SSOT path for a skill.
/// Global (project_id=0): `~/.agents/skills/local/<skill-name>/`
/// Project (project_id>0): `~/.agents/skills/local/_p<project_id>/<skill-name>/`
pub fn ssot_path(skill_name: &str, project_id: i64) -> Result<PathBuf, String> {
    validate_skill_name(skill_name)?;
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let base = home.join(".agents").join("skills").join("local");
    if project_id == 0 {
        Ok(base.join(skill_name))
    } else {
        Ok(base.join(format!("_p{}", project_id)).join(skill_name))
    }
}

/// Ensure the SSOT base directory exists.
pub fn ensure_ssot_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let ssot_base = home.join(".agents").join("skills").join("local");
    fs::create_dir_all(&ssot_base)
        .map_err(|e| format!("Failed to create SSOT directory: {}", e))?;
    Ok(ssot_base)
}

/// Resolve a name conflict: if a skill with the same name exists at the target,
/// append `-local` suffix.
#[allow(dead_code)] // Reserved for future use
pub fn resolve_conflict(target_parent: &Path, skill_name: &str) -> PathBuf {
    let target = target_parent.join(skill_name);
    if target.exists() {
        // Check if it's a different skill (not the same source)
        target_parent.join(format!("{}-local", skill_name))
    } else {
        target
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
