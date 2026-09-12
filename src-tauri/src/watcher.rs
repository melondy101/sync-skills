// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! File-system watcher for SSOT SKILL.md changes.
//!
//! Phase 4 / M12: monitors `~/.agents/skill-manager/ssot/` recursively and
//! emits a `skill-file-changed` event to the frontend when any SKILL.md is
//! created, modified, or deleted. The frontend decides whether to auto-sync
//! based on the user's sync mode (full-auto vs semi-auto).
//!
//! Debouncing: events within the same DEBOUNCE_MS window for the same path
//! are coalesced into a single emission.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use crate::app_paths;

/// Debounce window in milliseconds — file writes often fire multiple events
/// (create → modify → attribute-change), so we wait this long before emitting.
const DEBOUNCE_MS: u64 = 500;

/// Coalesced state: maps a watched file path to the last time we saw an event
/// for it. If a new event arrives within DEBOUNCE_MS of the previous one, it
/// resets the timer instead of emitting immediately.
type DebounceMap = Arc<Mutex<std::collections::HashMap<PathBuf, Instant>>>;

/// Starts the file-system watcher on a background thread. The watcher monitors
/// the SSOT directory and emits `skill-file-changed` Tauri events. It runs
/// until the `shutdown` channel is signalled or the app exits.
pub fn start_watcher(app_handle: AppHandle) {
    let ssot_base: PathBuf = match app_paths::ssot_path() {
        Ok(p) => p,
        Err(e) => {
            log::error!("[watcher] Cannot determine SSOT path: {e}");
            return;
        }
    };
    if !ssot_base.exists() {
        log::warn!("[watcher] SSOT base does not exist yet: {ssot_base:?} — watcher not started");
        return;
    }

    let debounce: DebounceMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

    // Channel: notify → our thread
    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();

    // Build the watcher. `RecommendedWatcher` picks the best backend for the
    // platform (inotify on Linux, FSEvent on macOS, ReadDirectoryChanges on
    // Windows).
    let mut watcher: RecommendedWatcher = match RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            log::error!("[watcher] Failed to create file watcher: {e}");
            return;
        }
    };

    if let Err(e) = watcher.watch(&ssot_base, RecursiveMode::Recursive) {
        log::error!("[watcher] Failed to watch SSOT directory {ssot_base:?}: {e}");
        return;
    }

    log::info!("[watcher] Started watching: {ssot_base:?}");

    // Spawn a dedicated thread for event processing
    std::thread::Builder::new()
        .name("skill-watcher".into())
        .spawn(move || {
            loop {
                match rx.recv() {
                    Ok(Ok(event)) => {
                        handle_event(&app_handle, &debounce, &event);
                    }
                    Ok(Err(e)) => {
                        log::debug!("[watcher] notify error: {e}");
                    }
                    Err(mpsc::RecvError) => {
                        // Sender dropped — watcher was stopped or app is shutting down
                        log::info!("[watcher] Event channel closed — shutting down");
                        break;
                    }
                }
            }
        })
        .expect("Failed to spawn watcher thread");
}

/// Process a single notify event: filter for SKILL.md changes, debounce, emit.
fn handle_event(app_handle: &AppHandle, debounce: &DebounceMap, event: &Event) {
    // Only care about actual content changes.
    // Ignore Access, AnyOther, Other and pure metadata events.
    match event.kind {
        EventKind::Create(_)
        | EventKind::Modify(notify::event::ModifyKind::Data(_))
        | EventKind::Modify(notify::event::ModifyKind::Any)
        | EventKind::Remove(_) => {}
        _ => return,
    }

    for path in &event.paths {
        // Only fire for files named SKILL.md
        if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
            continue;
        }

        // Debounce: if we've seen this path in the last DEBOUNCE_MS, skip.
        {
            let mut map = debounce.lock().unwrap();
            if let Some(last) = map.get(path) {
                if last.elapsed() < Duration::from_millis(DEBOUNCE_MS) {
                    // Reset the timer — keep waiting for writes to settle
                    let _ = map.insert(path.clone(), Instant::now());
                    return;
                }
            }
            let _ = map.insert(path.clone(), Instant::now());
        }

        log::debug!("[watcher] SKILL.md changed: {path:?}");

        // Extract the skill name from the path:
        //   ssot/<skill-name>/SKILL.md  →  "skill-name"
        let skill_name = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Emit event to frontend
        let _ = app_handle.emit("skill-file-changed", serde_json::json!({
            "skill_name": skill_name,
            "path": path.to_string_lossy(),
        }));
    }
}