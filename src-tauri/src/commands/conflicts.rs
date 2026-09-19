// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Conflict adjudication (M5).
//!
//! A conflict means two tools hold different content for the same skill name.
//! Both paths here end in the same step — promote one version to the SSOT and
//! propagate it to every other installation — and differ only in who picks that
//! version: the user, naming a tool, or a strategy, deciding from evidence.
//!
//! Strategies never guess. When the evidence is incomplete or ambiguous (an
//! unreadable timestamp, equal timestamps, or none of the preferred tools
//! actually holding a version) the conflict is left unresolved and reported, so
//! the banner still offers the manual choice. Every automatic decision is
//! recorded in `skill_conflicts.resolved_by` as `auto:<strategy>:<tool>`, which
//! keeps the audit trail distinguishable from a click.

use crate::db::Database;
use crate::lock::LockManager;
use crate::models::{ConflictVersion, ConflictView, SyncResult};
use crate::{fs, sync, DbState, LockState};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::State;

/// The strategies a caller can hand to `auto_resolve_conflicts`.
pub const STRATEGY_NEWEST: &str = "newest";
pub const STRATEGY_PREFERRED_TOOL: &str = "preferred-tool";

/// One conflict's outcome: either the sync that resolved it, or why the
/// strategy declined to decide.
#[derive(Debug, Clone, Serialize)]
pub struct AutoResolveOutcome {
    pub conflict_id: i64,
    pub skill_name: String,
    pub resolved: bool,
    /// Tool whose version was promoted, when resolved.
    pub kept_tool: Option<String>,
    /// Why nothing changed: an unknown strategy, or evidence too thin to pick.
    pub reason: Option<String>,
    pub errors: Vec<String>,
}

#[tauri::command]
pub fn list_conflicts(db: State<DbState>, project_id: Option<i64>) -> Result<Vec<ConflictView>, String> {
    let pid = project_id.unwrap_or(0);
    db.list_unresolved_conflicts(pid)
}

#[tauri::command]
pub async fn resolve_conflict(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    conflict_id: i64,
    keep_tool_name: String,
    project_id: Option<i64>,
) -> Result<SyncResult, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<SyncResult, String> {
        let conflict = find_unresolved(&db, pid, conflict_id)?;
        let version = conflict
            .versions
            .iter()
            .find(|v| v.tool_name == keep_tool_name)
            .ok_or_else(|| format!("Tool '{}' not found in conflict versions", keep_tool_name))?;

        promote_version(&db, &locks, &conflict, version, &keep_tool_name, pid)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Resolve every open conflict that a strategy can decide, and report each one
/// separately so a skipped conflict doesn't hide the ones that were resolved.
#[tauri::command]
pub async fn auto_resolve_conflicts(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    strategy: String,
    preferred_tools: Option<Vec<String>>,
    project_id: Option<i64>,
) -> Result<Vec<AutoResolveOutcome>, String> {
    if strategy != STRATEGY_NEWEST && strategy != STRATEGY_PREFERRED_TOOL {
        return Err(format!("unsupported auto-resolve strategy: {}", strategy));
    }
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let pid = project_id.unwrap_or(0);
    let preferred = preferred_tools.unwrap_or_default();

    tokio::task::spawn_blocking(move || -> Result<Vec<AutoResolveOutcome>, String> {
        let conflicts = db.list_unresolved_conflicts(pid)?;
        let mut outcomes = Vec::with_capacity(conflicts.len());
        for conflict in conflicts {
            let picked = pick_version(&conflict, &strategy, &preferred, &version_mtime);
            let Some(version) = picked else {
                outcomes.push(AutoResolveOutcome {
                    conflict_id: conflict.id,
                    skill_name: conflict.skill_name.clone(),
                    resolved: false,
                    kept_tool: None,
                    reason: Some(format!(
                        "strategy '{}' cannot decide between {} versions",
                        strategy,
                        conflict.versions.len()
                    )),
                    errors: Vec::new(),
                });
                continue;
            };
            // The banner shows tool names, so the audit column records the same.
            let resolved_by = format!("auto:{}:{}", strategy, version.tool_name);
            let result = promote_version(&db, &locks, &conflict, version, &resolved_by, pid);
            outcomes.push(match result {
                Ok(sync) => AutoResolveOutcome {
                    conflict_id: conflict.id,
                    skill_name: conflict.skill_name.clone(),
                    resolved: sync.errors.is_empty(),
                    kept_tool: Some(version.tool_name.clone()),
                    reason: if sync.errors.is_empty() {
                        None
                    } else {
                        Some("some installations could not be updated".to_string())
                    },
                    errors: sync.errors,
                },
                Err(error) => AutoResolveOutcome {
                    conflict_id: conflict.id,
                    skill_name: conflict.skill_name.clone(),
                    resolved: false,
                    kept_tool: None,
                    reason: Some(error),
                    errors: Vec::new(),
                },
            });
        }
        Ok(outcomes)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// The version a strategy keeps, or `None` when it must not decide.
///
/// `mtime_of` is a parameter rather than a call to the filesystem so the rule
/// itself is a pure comparison and can be tested on synthetic timestamps.
fn pick_version<'a>(
    conflict: &'a ConflictView,
    strategy: &str,
    preferred: &[String],
    mtime_of: &dyn Fn(&ConflictVersion) -> Option<SystemTime>,
) -> Option<&'a ConflictVersion> {
    if conflict.versions.len() < 2 {
        return None;
    }
    match strategy {
        // The first preferred tool that actually holds a version wins, so the
        // user's ordering is the whole decision and no clock is involved.
        STRATEGY_PREFERRED_TOOL => preferred
            .iter()
            .find_map(|tool| conflict.versions.iter().find(|v| &v.tool_name == tool)),
        STRATEGY_NEWEST => {
            let dated: Vec<(SystemTime, &ConflictVersion)> = conflict
                .versions
                .iter()
                .filter_map(|v| mtime_of(v).map(|t| (t, v)))
                .collect();
            // A version whose clock can't be read is not evidence that it is
            // older, so ranking the rest would be a guess.
            if dated.len() != conflict.versions.len() {
                return None;
            }
            let newest = dated.iter().map(|(t, _)| *t).max()?;
            // Two versions changed at the same instant: nothing ranks them, and
            // an arbitrary winner would silently discard one tool's edits.
            let mut winners = dated.iter().filter(|(t, _)| *t == newest).map(|(_, v)| *v);
            let first = winners.next()?;
            if winners.next().is_some() {
                None
            } else {
                Some(first)
            }
        }
        _ => None,
    }
}

/// Last modification time of a version's content: the skill directory's
/// `SKILL.md` when the source is a directory, otherwise the file itself.
fn version_mtime(version: &ConflictVersion) -> Option<SystemTime> {
    let path = Path::new(&version.source_path);
    let candidate = if path.is_dir() {
        path.join("SKILL.md")
    } else {
        path.to_path_buf()
    };
    std::fs::metadata(candidate)
        .and_then(|m| m.modified())
        .or_else(|_| std::fs::metadata(path).and_then(|m| m.modified()))
        .ok()
}

fn find_unresolved(db: &Database, pid: i64, conflict_id: i64) -> Result<ConflictView, String> {
    db.list_unresolved_conflicts(pid)?
        .into_iter()
        .find(|c| c.id == conflict_id)
        .ok_or_else(|| format!("Conflict {} not found", conflict_id))
}

/// Make `version` canonical: copy it over the SSOT, push the SSOT out to every
/// other active installation, then close the conflict record.
///
/// The kept version's files are the only input; nothing is deleted from the
/// losing tools, they are simply overwritten by the propagation step.
fn promote_version(
    db: &Database,
    locks: &LockManager,
    conflict: &ConflictView,
    version: &ConflictVersion,
    resolved_by: &str,
    pid: i64,
) -> Result<SyncResult, String> {
    let skill_id = conflict.skill_id;
    let skill = db.get_skill_by_id(skill_id)?;
    // Serialize with other sync operations on this skill
    let _lock = locks.acquire_blocking(pid, &skill.name);

    let source = PathBuf::from(&version.source_path);
    if !source.exists() {
        return Err(format!("Source path does not exist: {}", version.source_path));
    }

    sync::ensure_ssot_dir()?;
    let ssot_target = sync::ssot_path(&skill.name, pid)?;

    if ssot_target.exists() {
        fs::replace_directory(&source, &ssot_target)?;
    } else {
        fs::copy_directory(&source, &ssot_target)?;
    }
    let _ = sync::create_local_marker(&ssot_target);

    // Propagate from SSOT to all other active installations
    let installations = db.get_active_installation_paths(skill_id, pid)?;
    let mut synced_to = 0usize;
    let mut errors = Vec::new();

    for (tool_id, target_path) in &installations {
        let expanded = match crate::scanner::expand_path(target_path) {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("Path expansion failed for tool {}: {}", tool_id, e));
                continue;
            }
        };
        let target_dir = expanded.join(&skill.name);

        let result = if target_dir.exists() {
            fs::replace_directory(&ssot_target, &target_dir).map(|_| "replace".to_string())
        } else {
            fs::copy_directory(&ssot_target, &target_dir).map(|_| "copy".to_string())
        };

        match result {
            Ok(_) => {
                db.update_synced_at(skill_id, *tool_id, pid)?;
                db.insert_sync_log(skill_id, *tool_id, pid, "from_ssot", "success", None)?;
                synced_to += 1;
            }
            Err(e) => {
                errors.push(format!("Tool {}: {}", tool_id, e));
                db.insert_sync_log(skill_id, *tool_id, pid, "from_ssot", "failed", Some(&e))?;
            }
        }
    }

    // Refresh hashes from SSOT (the canonical copy after conflict resolution)
    if let Ok(new_content_hash) = crate::hash::compute_content_hash(&ssot_target) {
        let skill_md = ssot_target.join("SKILL.md");
        let new_core_hash = crate::hash::compute_core_hash(&skill_md).unwrap_or_default();
        let _ = db.update_skill_hashes(skill_id, &new_content_hash, &new_core_hash);
    }
    // Update source_path to SSOT
    let ssot_str = ssot_target.to_string_lossy().to_string();
    let _ = db.update_skill_source_path(skill_id, &ssot_str);

    db.resolve_conflict_record(conflict.id, resolved_by)?;

    Ok(SyncResult {
        skill_id,
        skill_name: skill.name,
        synced_to,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(tool: &str) -> ConflictVersion {
        ConflictVersion {
            tool_id: 1,
            tool_name: tool.to_string(),
            core_hash: format!("hash-{}", tool),
            source_path: format!("/tmp/{}", tool),
        }
    }

    fn conflict(tools: &[&str]) -> ConflictView {
        ConflictView {
            id: 7,
            skill_id: 3,
            skill_name: "review".to_string(),
            detected_at: "2026-09-19 00:00:00".to_string(),
            versions: tools.iter().map(|t| version(t)).collect(),
        }
    }

    /// A clock we control: each tool name maps to its own timestamp.
    fn clock<'a>(map: &'a [(&'a str, u64)]) -> impl Fn(&ConflictVersion) -> Option<SystemTime> + 'a {
        move |v: &ConflictVersion| {
            map.iter()
                .find(|(name, _)| *name == v.tool_name)
                .map(|(_, secs)| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(*secs))
        }
    }

    #[test]
    fn newest_wins_and_a_tie_declines_to_guess() {
        let conflict = conflict(&["claude", "cursor"]);
        let picked = pick_version(
            &conflict,
            STRATEGY_NEWEST,
            &[],
            &clock(&[("claude", 100), ("cursor", 200)]),
        );
        assert_eq!(picked.map(|v| v.tool_name.as_str()), Some("cursor"));

        assert_eq!(
            pick_version(
                &conflict,
                STRATEGY_NEWEST,
                &[],
                &clock(&[("claude", 200), ("cursor", 200)])
            ),
            None,
            "identical timestamps are not evidence either way"
        );
        assert_eq!(
            pick_version(&conflict, STRATEGY_NEWEST, &[], &clock(&[("claude", 100)])),
            None,
            "one readable timestamp among two versions cannot rank them"
        );
    }

    #[test]
    fn preferred_tool_takes_the_first_listed_tool_that_holds_a_version() {
        let conflict = conflict(&["claude", "cursor"]);
        let none_missing: Vec<String> = vec!["codex".to_string(), "cursor".to_string()];
        assert_eq!(
            pick_version(&conflict, STRATEGY_PREFERRED_TOOL, &none_missing, &clock(&[]))
                .map(|v| v.tool_name.as_str()),
            Some("cursor"),
            "codex holds nothing here, so the next preference wins"
        );
        assert_eq!(
            pick_version(&conflict, STRATEGY_PREFERRED_TOOL, &["codex".to_string()], &clock(&[])),
            None,
            "a preference list that matches nothing is not a decision"
        );
    }

    #[test]
    fn an_unknown_strategy_or_a_single_version_picks_nothing() {
        let subject = conflict(&["claude", "cursor"]);
        assert_eq!(
            pick_version(&subject, "coin-flip", &[], &clock(&[("claude", 1), ("cursor", 2)])),
            None
        );
        assert_eq!(
            pick_version(&conflict(&["claude"]), STRATEGY_NEWEST, &[], &clock(&[("claude", 1)])),
            None,
            "one version is not a conflict"
        );
    }

    #[test]
    fn mtime_reads_the_skill_file_inside_a_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let skill_dir = dir.path().join("review");
        std::fs::create_dir_all(&skill_dir).expect("create skill dir");
        std::fs::write(skill_dir.join("SKILL.md"), "# review").expect("write skill");

        let mut v = version("claude");
        v.source_path = skill_dir.to_string_lossy().to_string();
        assert!(version_mtime(&v).is_some());

        let missing = version("claude");
        assert!(version_mtime(&missing).is_none(), "a path that isn't there has no mtime");
    }
}
