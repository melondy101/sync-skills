// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Standalone MCP server over stdio: the GUI never owns this process' terminal,
//! so it can be spawned as a child by any MCP client (Claude Code, Codex, ...).

fn main() {
    if let Err(e) = skill_manager_lib::run_mcp_stdio_server() {
        // stdout is the protocol channel: diagnostics go to stderr only.
        eprintln!("skill-manager mcp server: {e}");
        std::process::exit(1);
    }
}
