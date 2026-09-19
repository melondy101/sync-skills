// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

mod app_paths;
mod commands;
mod db;
mod diff;
mod discovery;
mod fs;
#[cfg(test)]
mod edge_tests;
mod hash;
mod http;
mod lint;
mod lock;
mod market;
mod mcp;
mod mcp_config;
mod models;
mod ops;
mod paths;
mod providers;
mod scanner;
mod settings;
mod sync;
mod watcher;
mod workspaces;

use db::Database;
use lock::LockManager;
use std::sync::Arc;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

pub(crate) type DbState = Arc<Database>;
pub(crate) type LockState = Arc<LockManager>;

/// Entry point for the standalone `skill-manager-mcp` binary: read-only MCP
/// tools over stdio, serving the same SQLite index the GUI writes to.
pub fn run_mcp_stdio_server() -> Result<(), String> {
    mcp::serve_stdio()
}

// ==================== Application Entry ====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger
    env_logger::init();

    // Initialize database
    let db = Database::new().expect("Failed to initialize database");
    let db_state = Arc::new(db);

    // Seed built-in markets so the market tab is populated on first run.
    if let Err(e) = crate::commands::market::seed_default_markets(db_state.as_ref()) {
        log::error!("Failed to seed default markets: {}", e);
    }

    // Per-skill lock manager: serializes sync/check operations on the same skill
    let lock_state: LockState = Arc::new(LockManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_state)
        .manage(lock_state)
        .setup(|app| {
            // Build tray menu
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            // Create tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .tooltip("Skill Manager")
                .menu(&tray_menu)
                .on_menu_event(|app, event| {
                    if event.id() == "show" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    } else if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // Single click on tray icon shows the window
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Start file-system watcher (T3 / M12) for auto-sync
            watcher::start_watcher(app.handle().clone());

            Ok(())
        })
        // Honor the "close_action" setting: minimize, hide to tray, or exit
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let close_action = settings::Settings::load().close_action;
                match close_action.as_str() {
                    "minimize" => {
                        api.prevent_close();
                        let _ = window.minimize();
                    }
                    "tray" => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    _ => {} // "exit" — default, let it close
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Tools
            commands::tools::list_tools,
            commands::tools::add_tool,
            commands::tools::update_tool_path,
            commands::tools::delete_tool,
            commands::tools::list_tool_templates,
            commands::tools::discover_tools,
            commands::paths::check_path,
            commands::diagnostics::get_db_recovery,
            commands::mcp_config::mcp_suggested_entry,
            commands::mcp_config::mcp_status,
            commands::mcp_config::mcp_install,
            commands::mcp_config::mcp_uninstall,
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
            commands::projects::discover_workspaces,
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
            // Skill market
            commands::market::list_markets,
            commands::market::add_market,
            commands::market::add_market_by_url,
            commands::market::check_market_commits,
            commands::market::update_market,
            commands::market::delete_market,
            commands::market::sync_market_index,
            commands::market::sync_all_market_indices,
            commands::market::scan_all_remote_repositories,
            commands::market::list_market_templates,
            commands::market::list_remote_skills,
            commands::market::download_remote_skill_to_ssot,
            commands::market::sync_remote_skill_to_tools,
            commands::market::sync_remote_installations_to_tools,
            commands::market::check_remote_updates,
            commands::market::check_remote_ssot_updates,
            commands::market::set_all_remote_skills_installed,
            commands::market::get_remote_skill_detail,
            // commands::market::get_remote_skill_diff removed with remote diff redesign
            commands::market::list_remote_installations,
            commands::market::toggle_remote_installation,
            commands::market::sync_remote_installations,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

