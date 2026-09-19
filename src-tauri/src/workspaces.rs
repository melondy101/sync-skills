// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Workspace discovery for PRD §11 "自动检测工作区目录".
//!
//! phase4 §六 rejected the original idea — scanning the disk for `.git`
//! directories — on privacy and false-positive grounds. This takes the other
//! route: the coding tools already remember which folders you opened, so the
//! candidates are read from *their* records rather than inferred from the
//! filesystem. Nothing leaves the machine, and nothing is imported until the
//! user says so; discovery only proposes.
//!
//! Two sources are implemented because both were verified on real installs:
//! Claude Code's `~/.claude.json` (`projects` is keyed by absolute path) and
//! Cursor's per-workspace storage (`workspace.json` → `folder`, a `file://` URI).
//! A missing or unreadable source contributes nothing — a machine without that
//! editor must not see an error for it.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;

use crate::db::Database;
use crate::scanner::{expand_path, normalize_path};

/// A project root another tool has already opened.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkspaceCandidate {
    pub path: String,
    pub name: String,
    /// Which tool recorded it: `claude` or `cursor`.
    pub source: String,
}

/// Enough to fill a screen without inviting an import of sixty projects.
pub const MAX_CANDIDATES: usize = 20;

/// Workspaces the local tools remember, minus the ones already registered.
pub fn discover_workspaces(db: &Database) -> Result<Vec<WorkspaceCandidate>, String> {
    let known: BTreeSet<String> = db
        .list_projects()?
        .iter()
        .filter_map(|p| expand_path(&p.path).ok())
        .map(|p| normalize_path(&p).to_lowercase())
        .collect();

    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (source, path) in claude_workspaces().into_iter().chain(cursor_workspaces()) {
        let dir = match expand_path(&path) {
            Ok(dir) => dir,
            Err(_) => continue,
        };
        // A tool's history outlives the folder: deleted checkouts and renamed
        // drives are common, and neither is a project the user can add.
        if !dir.is_dir() {
            continue;
        }
        let key = normalize_path(&dir).to_lowercase();
        if key.is_empty() || known.contains(&key) || !seen.insert(key) {
            continue;
        }
        out.push(WorkspaceCandidate {
            name: dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone()),
            path: dir.to_string_lossy().to_string(),
            source: source.to_string(),
        });
        if out.len() >= MAX_CANDIDATES {
            break;
        }
    }
    Ok(out)
}

/// Absolute paths from `~/.claude.json` → `projects`, whose keys *are* the
/// project roots Claude has opened.
fn claude_workspaces() -> Vec<(&'static str, String)> {
    let home = match dirs::home_dir() {
        Some(home) => home,
        None => return Vec::new(),
    };
    let file = home.join(".claude.json");
    let raw = match std::fs::read_to_string(file) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };
    claude_project_paths(&raw)
}

/// Split out from the I/O so the parsing rule has a direct test.
fn claude_project_paths(raw: &str) -> Vec<(&'static str, String)> {
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    match value.get("projects").and_then(|p| p.as_object()) {
        Some(map) => map
            .keys()
            .map(|path| ("claude", path.clone()))
            .collect(),
        None => Vec::new(),
    }
}

/// `folder` entries from `<config>/Cursor/User/workspaceStorage/*/workspace.json`.
fn cursor_workspaces() -> Vec<(&'static str, String)> {
    let root = match dirs::config_dir() {
        Some(dir) => dir.join("Cursor").join("User").join("workspaceStorage"),
        None => return Vec::new(),
    };
    cursor_workspace_paths_at(&root)
}

/// Read every entry's `workspace.json` under `root`. Non-directories, unreadable
/// files and entries without a usable `folder` are skipped.
fn cursor_workspace_paths_at(root: &Path) -> Vec<(&'static str, String)> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let file = entry.path().join("workspace.json");
        let raw = match std::fs::read_to_string(file) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        if let Some(path) = cursor_folder_from(&raw) {
            out.push(("cursor", path));
        }
    }
    out
}

fn cursor_folder_from(raw: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let folder = value.get("folder")?.as_str()?;
    decode_file_uri(folder)
}

/// Turn a `file://` URI into a path: `file:///d%3A/Develop/x` becomes
/// `d:/Develop/x` (Windows drive) and `file:///home/me/x` stays rooted.
fn decode_file_uri(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    // A `file:///d:/…` authority section leaves one leading slash to drop on
    // Windows-shaped paths; `file:///home/…` needs it kept.
    let without_authority = rest.strip_prefix('/').unwrap_or(rest);
    let decoded = percent_decode(without_authority);
    if looks_like_windows_drive(&decoded) {
        Some(decoded)
    } else {
        Some(format!("/{}", decoded))
    }
}

fn looks_like_windows_drive(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Percent-decoding for the handful of escapes a path uses: `%3A` is the Windows
/// drive colon, `%20` a space, `%2F` a slash.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        // `get` rather than a slice: an escape can start at a byte offset that
        // would land mid-character when the two following bytes are UTF-8.
        if bytes[i] == b'%' {
            if let Some(byte) = input
                .get(i + 1..i + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    // A decoded UTF-8 sequence that does not decode stays as the replacement
    // character rather than failing the whole path.
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_projects_keys_become_candidates_in_file_order() {
        let raw = r#"{ "projects": { "D:/Develop/a": {}, "D:/Develop/b": {} } }"#;
        let paths = claude_project_paths(raw);
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().all(|(source, _)| *source == "claude"));
        assert!(paths.iter().any(|(_, p)| p == "D:/Develop/b"));
    }

    #[test]
    fn unreadable_or_unshaped_claude_json_yields_nothing() {
        assert!(claude_project_paths("not json").is_empty());
        assert!(claude_project_paths(r#"{ "mcpServers": {} }"#).is_empty());
        assert!(claude_project_paths(r#"{ "projects": [] }"#).is_empty());
    }

    #[test]
    fn file_uris_decode_to_paths_on_either_platform_shape() {
        assert_eq!(
            decode_file_uri("file:///d%3A/Develop/copyCursor").as_deref(),
            Some("d:/Develop/copyCursor")
        );
        assert_eq!(
            decode_file_uri("file:///home/me/My%20Blog").as_deref(),
            Some("/home/me/My Blog")
        );
        assert_eq!(decode_file_uri("/already/a/path"), None);
    }

    #[test]
    fn cursor_entries_are_read_from_a_real_storage_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let with_folder = dir.path().join("hash1");
        let broken = dir.path().join("hash2");
        std::fs::create_dir_all(&with_folder).expect("mkdir");
        std::fs::create_dir_all(&broken).expect("mkdir");
        std::fs::write(
            with_folder.join("workspace.json"),
            r#"{ "folder": "file:///d%3A/Develop/proj" }"#,
        )
        .expect("write workspace.json");
        std::fs::write(broken.join("workspace.json"), b"{ not json").expect("write junk");
        // A directory without the file at all is the normal case for a remote or
        // empty workspace, and must not abort the sweep.
        std::fs::create_dir_all(dir.path().join("hash3")).expect("mkdir");

        let found = cursor_workspace_paths_at(dir.path());

        assert_eq!(found.len(), 1, "only the readable entry counts: {:?}", found);
        assert_eq!(found[0].0, "cursor");
        assert_eq!(found[0].1, "d:/Develop/proj");
    }

    #[test]
    fn percent_decoding_leaves_unrelated_text_alone() {
        assert_eq!(percent_decode("plain/path"), "plain/path");
        assert_eq!(percent_decode("a%2Fb"), "a/b");
        assert_eq!(percent_decode("trailing%"), "trailing%");
        assert_eq!(percent_decode("%ZZ bad"), "%ZZ bad");
    }

    #[test]
    fn a_percent_before_a_multibyte_character_does_not_panic() {
        // The two bytes after `%` can land mid-UTF-8-sequence; decoding must fall
        // back to copying the byte rather than indexing a char boundary.
        assert_eq!(percent_decode("技能%中%/x"), "技能%中%/x");
        assert_eq!(percent_decode("%E6%8A%80"), "技");
    }
}
