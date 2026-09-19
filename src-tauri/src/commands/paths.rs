// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Path preflight for the forms that take a path. The mutating commands run the
//! same validation, so this only answers the question before a submit: the UI
//! shows "this is not resolvable" or "this directory is not there yet" while the
//! user is still editing.

use crate::paths::{self, PathCheck};

#[tauri::command]
pub fn check_path(path: String) -> Result<PathCheck, String> {
    paths::check_user_path(&path)
}
