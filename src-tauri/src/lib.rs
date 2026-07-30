// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

mod commands;
mod db;
mod diff;
mod discovery;
#[cfg(test)]
mod edge_tests;
mod hash;
mod lint;
mod lock;
mod models;
mod ops;
mod scanner;
mod settings;
mod sync;

use db::Database;
use lock::LockManager;
use std::sync::Arc;

pub(crate) type DbState = Arc<Database>;
pub(crate) type LockState = Arc<LockManager>;

// ==================== Application Entry ====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger
    env_logger::init();

    // Initialize database
    let db = Database::new().expect("Failed to initialize database");
    let db_state = Arc::new(db);

    // One-time SSOT relocation: ~/.agents/skills/local -> ~/.agents/skill-manager/ssot.
    // Tools scanning ~/.agents/skills/ (Codex CLI, OpenCode) were loading the old
    // SSOT store as duplicate skills; move it out and fix stored source_paths.
    if let Some((old, new)) = sync::migrate_legacy_ssot() {
        let old_s = scanner::normalize_path(&old);
        let new_s = scanner::normalize_path(&new);
        match db_state.migrate_ssot_prefix(&old_s, &new_s) {
            Ok(n) if n > 0 => log::info!("Rewrote {} skill source_path entries to new SSOT", n),
            Ok(_) => {}
            Err(e) => log::error!("SSOT source_path migration failed: {}", e),
        }
    }

    // Per-skill lock manager: serializes sync/check operations on the same skill
    let lock_state: LockState = Arc::new(LockManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_state)
        .manage(lock_state)
        .invoke_handler(tauri::generate_handler![
            // Tools
            commands::tools::list_tools,
            commands::tools::add_tool,
            commands::tools::update_tool_path,
            commands::tools::delete_tool,
            commands::tools::list_tool_templates,
            commands::tools::discover_tools,
            // Skills
            commands::skills::list_skills,
            commands::skills::read_skill_md,
            commands::skills::save_skill_md,
            commands::skills::lint_skills,
            commands::skills::lint_skill,
            commands::skills::fix_skill,
            // Scan
            commands::scan::full_scan,
            commands::scan::scan_scope,
            // Sync (M2)
            commands::syncing::toggle_skill,
            commands::syncing::sync_skill,
            commands::syncing::sync_all_pending,
            commands::syncing::check_updates,
            commands::syncing::check_skill_update,
            commands::syncing::get_skill_diff,
            // Settings (M2)
            commands::app_settings::get_settings,
            commands::app_settings::update_settings,
            // Projects (M3)
            commands::projects::list_projects,
            commands::projects::add_project,
            commands::projects::delete_project,
            commands::projects::update_project,
            // Sync Logs (M3)
            commands::logs::get_sync_logs,
            // Conflicts (M5)
            commands::conflicts::list_conflicts,
            commands::conflicts::resolve_conflict,
            // Reverse sync & dismiss
            commands::syncing::reverse_sync_skill,
            commands::syncing::dismiss_skill_update,
            // App self-update
            commands::updater::check_app_update,
            commands::updater::download_app_update,
            commands::updater::install_app_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
