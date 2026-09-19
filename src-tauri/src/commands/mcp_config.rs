// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Commands for managing this app's own MCP server entry in each tool's config.
//! Thin by design: the read/merge/write rules live in `crate::mcp_config`.

use crate::mcp_config::{self, McpServerEntry, McpSuggestedEntry, McpTargetStatus};

/// The command line the panel should prefill, and whether it exists on disk.
#[tauri::command]
pub fn mcp_suggested_entry() -> McpSuggestedEntry {
    mcp_config::suggested_entry()
}

/// Current state of every known target, compared against `entry` (or the binary
/// that ships beside the app when the caller has no preference).
#[tauri::command]
pub fn mcp_status(entry: Option<McpServerEntry>) -> Vec<McpTargetStatus> {
    mcp_config::status(&entry.unwrap_or_else(mcp_config::default_entry))
}

/// Point each tool's config at the server binary. Idempotent: running it twice
/// changes nothing the second time.
#[tauri::command]
pub fn mcp_install(entry: Option<McpServerEntry>) -> Vec<McpTargetStatus> {
    mcp_config::install(&entry.unwrap_or_else(mcp_config::default_entry))
}

/// Remove our entry from every target, leaving the user's other servers alone.
#[tauri::command]
pub fn mcp_uninstall() -> Vec<McpTargetStatus> {
    mcp_config::uninstall()
}
