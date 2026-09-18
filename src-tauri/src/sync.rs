// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! SSOT identity + layout logic ("name-as-identity", per-domain directory
//! isolation, `local.md` markers). The generic, domain-free filesystem helpers
//! that used to live here now live in the portable `crate::fs` leaf module.

use std::fs;
use std::path::{Path, PathBuf};

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

/// Base directory of the SSOT store: `~/.skill-manager/ssot/`.
/// Deliberately kept OUTSIDE any `skills/` tree: tools like Codex CLI and
/// OpenCode scan `~/.agents/skills/` as a shared skills directory, so an
/// SSOT store located there would be double-loaded as duplicate skills.
pub fn ssot_base() -> Result<PathBuf, String> {
    crate::app_paths::ssot_path()
}

/// Get the SSOT path for a skill.
/// Global (project_id=0): `~/.skill-manager/ssot/<skill-name>/`
/// Project (project_id>0): `~/.skill-manager/ssot/_p<project_id>/<skill-name>/`
pub fn ssot_path(skill_name: &str, project_id: i64) -> Result<PathBuf, String> {
    validate_skill_name(skill_name)?;
    let base = ssot_base()?;
    if project_id == 0 {
        Ok(base.join(skill_name))
    } else {
        Ok(base.join(format!("_p{}", project_id)).join(skill_name))
    }
}

/// Ensure the SSOT base directory exists.
pub fn ensure_ssot_dir() -> Result<PathBuf, String> {
    let ssot_base = ssot_base()?;
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
