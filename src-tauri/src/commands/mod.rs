// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Thin Tauri command layer, grouped by domain.
//! Domain logic lives in `crate::ops`; commands only adapt State/async.

pub mod app_settings;
pub mod conflicts;
pub mod diagnostics;
pub mod logs;
pub mod market;
pub mod paths;
pub mod projects;
pub mod scan;
pub mod skills;
pub mod syncing;
pub mod tools;
pub mod updater;

