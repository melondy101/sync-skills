// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::models::SyncLog;
use crate::DbState;
use tauri::State;

#[tauri::command]
pub fn get_sync_logs(
    db: State<DbState>,
    skill_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<SyncLog>, String> {
    db.get_sync_logs(skill_id, limit)
}
