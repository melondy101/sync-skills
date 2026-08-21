// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Domain logic for the T4 Remote Skill Detail Modal.
//!
//! Resolves a remote-skill row into a `RemoteSkillDetail` payload by reading
//! the on-disk SSOT directory (file tree + SKILL.md body + hashes). Lives in
//! `ops/` rather than `commands/` so the four-layer split (commands = thin
//! adapters; ops = domain logic) stays intact.

use crate::hash;
use crate::models::{RemoteSkill, RemoteSkillDetail};
use std::fs;
use std::path::{Path, PathBuf};

/// Names excluded from the file-tree view (already shown elsewhere in the
/// Modal — `SKILL.md` is rendered above, `local.md` is internal-only).
const TREE_EXCLUDED_NAMES: &[&str] = &["SKILL.md", "local.md"];

/// Build the Modal payload for a remote skill. Reads SSOT-on-disk when the
/// directory exists; when it does not, returns a payload with
/// `ssot_missing = true` and `None` for the SSOT-only fields.
pub fn build_remote_skill_detail(skill: &RemoteSkill) -> Result<RemoteSkillDetail, String> {
    let ssot_dir = PathBuf::from(&skill.ssot_path);
    if !ssot_dir.exists() {
        return Ok(RemoteSkillDetail {
            remote_skill_id: skill.id,
            skill_name: skill.skill_name.clone(),
            description: skill.description.clone(),
            remote_url: skill.remote_url.clone(),
            ssot_path: skill.ssot_path.clone(),
            remote_content_hash: skill.remote_content_hash.clone(),
            remote_core_hash: skill.remote_core_hash.clone(),
            local_content_hash: None,
            local_core_hash: None,
            skill_md_content: None,
            files: Vec::new(),
            ssot_missing: true,
        });
    }

    let skill_md_content = read_ssot_skill_md(&ssot_dir)?;
    let files = collect_relative_files(&ssot_dir)?;
    let local_content_hash = hash::compute_content_hash(&ssot_dir).unwrap_or_default();
    let local_core_hash = hash::compute_core_hash(&ssot_dir.join("SKILL.md")).unwrap_or_default();

    Ok(RemoteSkillDetail {
        remote_skill_id: skill.id,
        skill_name: skill.skill_name.clone(),
        description: skill.description.clone(),
        remote_url: skill.remote_url.clone(),
        ssot_path: skill.ssot_path.clone(),
        remote_content_hash: skill.remote_content_hash.clone(),
        remote_core_hash: skill.remote_core_hash.clone(),
        local_content_hash: empty_to_none(local_content_hash),
        local_core_hash: empty_to_none(local_core_hash),
        skill_md_content,
        files,
        ssot_missing: false,
    })
}

/// Read the SSOT `SKILL.md` body; returns `None` when absent.
fn read_ssot_skill_md(ssot_dir: &Path) -> Result<Option<String>, String> {
    let file = ssot_dir.join("SKILL.md");
    if !file.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&file).map_err(|e| format!("read {}: {}", file.display(), e))?;
    Ok(Some(content))
}

/// Walk `dir` and return sorted, relative paths for every file. Hidden /
/// dot-prefixed entries and entries in `TREE_EXCLUDED_NAMES` are skipped.
/// Directories are implied by their contents (no separate dir entries).
fn collect_relative_files(dir: &Path) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    walk(dir, dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(root: &Path, current: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(current).map_err(|e| format!("read_dir({:?}): {}", current, e))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || TREE_EXCLUDED_NAMES.iter().any(|excluded| *excluded == name) {
            continue;
        }
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            walk(root, &path, out)?;
        } else if metadata.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| format!("strip_prefix: {}", e))?
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
        }
    }
    Ok(())
}

fn empty_to_none(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}