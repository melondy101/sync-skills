// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Startup diagnostics the GUI asks about once. Kept separate from
//! `app_settings` because it reports what happened while opening the database,
//! which the frontend cannot observe any other way.

use crate::db::RecoveryNotice;
use crate::DbState;
use tauri::State;

/// Set only when startup had to move an unreadable `skill-manager.db` aside; the
/// payload names the file it was moved to so the user can recover it by hand.
#[tauri::command]
pub fn get_db_recovery(db: State<'_, DbState>) -> Option<RecoveryNotice> {
    db.recovered.clone()
}
