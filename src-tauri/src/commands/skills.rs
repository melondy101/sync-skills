// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::lint::{self, SkillLint};
use crate::models::{SkillView, SyncResult};
use crate::settings::Settings;
use crate::{ops, sync, DbState, LockState};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn list_skills(db: State<DbState>, project_id: Option<i64>) -> Result<Vec<SkillView>, String> {
    let pid = project_id.unwrap_or(0);
    db.list_skills_with_status(pid)
}

// ==================== SKILL.md editor ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFile {
    pub path: String,
    pub content: String,
}

/// Resolve the canonical directory for a skill: SSOT copy when present,
/// otherwise the recorded source path.
fn resolve_skill_dir(
    db: &crate::db::Database,
    skill_id: i64,
    project_id: i64,
) -> Result<(String, PathBuf), String> {
    let skill = db.get_skill_by_id(skill_id)?;
    let ssot = sync::ssot_path(&skill.name, project_id)?;
    let dir = if ssot.exists() {
        ssot
    } else {
        let recorded = PathBuf::from(&skill.source_path);
        if !recorded.exists() {
            return Err(format!("Skill directory not found: {}", skill.source_path));
        }
        recorded
    };
    Ok((skill.name, dir))
}

/// Read a skill's SKILL.md from its canonical location (SSOT preferred).
#[tauri::command]
pub async fn read_skill_md(
    db: State<'_, DbState>,
    skill_id: i64,
    project_id: Option<i64>,
) -> Result<SkillFile, String> {
    let db = db.inner().clone();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<SkillFile, String> {
        let (_name, dir) = resolve_skill_dir(&db, skill_id, pid)?;
        let file = dir.join("SKILL.md");
        let content = fs::read_to_string(&file)
            .map_err(|e| format!("Cannot read {}: {}", file.display(), e))?;
        Ok(SkillFile {
            path: crate::scanner::normalize_path(&file),
            content,
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Save SKILL.md to the skill's canonical location, then sync so the change
/// flows into the SSOT and distributes to every checked tool (SSOT spec:
/// writing to SSOT naturally triggers distribution).
#[tauri::command]
pub async fn save_skill_md(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    skill_id: i64,
    project_id: Option<i64>,
    content: String,
) -> Result<SyncResult, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let settings = Settings::load();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<SyncResult, String> {
        let (name, dir) = resolve_skill_dir(&db, skill_id, pid)?;
        {
            // Hold the skill lock only for the write; do_sync_skill re-acquires it.
            let _lock = locks.acquire_blocking(pid, &name);
            let file = dir.join("SKILL.md");
            fs::write(&file, &content)
                .map_err(|e| format!("Cannot write {}: {}", file.display(), e))?;
            let _ = db.insert_action_log(
                "edit",
                Some(skill_id),
                None,
                pid,
                "success",
                Some(&crate::scanner::normalize_path(&file)),
            );
        }
        let dir_str = dir.to_string_lossy().to_string();
        ops::do_sync_skill(&db, &locks, skill_id, pid, settings.prefer_symlink, Some(&dir_str))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

// ==================== Lint ====================

/// Lint one skill, resolving its canonical directory (SSOT preferred).
/// Any unexpected panic inside the rules is caught and surfaced as an
/// `internal_error` issue for that skill instead of failing the whole check.
fn lint_one_skill(
    skill: &crate::models::Skill,
    pid: i64,
) -> Result<Vec<crate::lint::LintIssue>, String> {
    let ssot = sync::ssot_path(&skill.name, pid)?;
    let dir = if ssot.exists() {
        ssot
    } else {
        PathBuf::from(&skill.source_path)
    };
    if !dir.exists() {
        return Ok(vec![crate::lint::LintIssue {
            code: "file_missing".to_string(),
            severity: "error".to_string(),
            param: None,
            fixable: false,
        }]);
    }
    let issues = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        lint::lint_skill_dir(&dir, &skill.name)
    }))
    .unwrap_or_else(|payload| {
        let detail = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()));
        vec![crate::lint::LintIssue {
            code: "internal_error".to_string(),
            severity: "error".to_string(),
            param: detail,
            fixable: false,
        }]
    });
    Ok(issues)
}

/// Lint every skill in the given project scope against quality rules.
#[tauri::command]
pub async fn lint_skills(
    db: State<'_, DbState>,
    project_id: Option<i64>,
) -> Result<Vec<SkillLint>, String> {
    let db = db.inner().clone();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<Vec<SkillLint>, String> {
        let skills = db.list_skills_with_status(pid)?;
        let mut results = Vec::new();

        for view in &skills {
            let skill = &view.skill;
            let issues = lint_one_skill(skill, pid)?;
            if !issues.is_empty() {
                results.push(SkillLint {
                    skill_id: skill.id,
                    skill_name: skill.name.clone(),
                    issues,
                });
            }
        }

        Ok(results)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Lint a single skill (per-card health check). Always returns an entry,
/// even when the skill is healthy (empty issues).
#[tauri::command]
pub async fn lint_skill(
    db: State<'_, DbState>,
    skill_id: i64,
    project_id: Option<i64>,
) -> Result<SkillLint, String> {
    let db = db.inner().clone();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<SkillLint, String> {
        let skill = db.get_skill_by_id(skill_id)?;
        let issues = lint_one_skill(&skill, pid)?;
        Ok(SkillLint {
            skill_id: skill.id,
            skill_name: skill.name.clone(),
            issues,
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Auto-fix a skill's fixable lint issues, distribute the change through the
/// SSOT (same flow as save_skill_md), then re-lint and return the fresh result
/// so the frontend can update the health check list in place.
#[tauri::command]
pub async fn fix_skill(
    db: State<'_, DbState>,
    locks: State<'_, LockState>,
    skill_id: i64,
    project_id: Option<i64>,
) -> Result<SkillLint, String> {
    let db = db.inner().clone();
    let locks = locks.inner().clone();
    let settings = Settings::load();
    let pid = project_id.unwrap_or(0);

    tokio::task::spawn_blocking(move || -> Result<SkillLint, String> {
        let (name, dir) = resolve_skill_dir(&db, skill_id, pid)?;
        let fixed = {
            // Hold the skill lock only for the write; do_sync_skill re-acquires it.
            let _lock = locks.acquire_blocking(pid, &name);
            lint::fix_skill_dir(&dir, &name)?
        };
        if !fixed.is_empty() {
            let _ = db.insert_action_log(
                "fix",
                Some(skill_id),
                None,
                pid,
                "success",
                Some(&fixed.join(",")),
            );
            let dir_str = dir.to_string_lossy().to_string();
            ops::do_sync_skill(&db, &locks, skill_id, pid, settings.prefer_symlink, Some(&dir_str))?;
        }
        let skill = db.get_skill_by_id(skill_id)?;
        let issues = lint_one_skill(&skill, pid)?;
        Ok(SkillLint {
            skill_id: skill.id,
            skill_name: skill.name.clone(),
            issues,
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}
