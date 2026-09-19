// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The second half of roadmap v0.7.0 "MCP 集成": putting `skill-manager-mcp`
//! into each tool's own MCP config, and taking it out again.
//!
//! Scope is the *server this app owns*, not a general MCP editor: one key
//! (`skill-manager`) in a handful of files. That keeps the write set small
//! enough to make the promise that matters — the user's other servers, and every
//! unrelated key in those files, survive. Files are round-tripped as
//! `serde_json::Value` rather than a typed struct for exactly that reason.
//!
//! A target is only writable once its parent directory exists. A tool that was
//! never run has no config directory, and creating one would leave a stray file
//! the user has to find and delete themselves; such targets report
//! `registered: false` and are left alone.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The key this app owns inside each tool's `mcpServers` map.
pub const SERVER_KEY: &str = "skill-manager";

/// A stdio MCP entry, in the shape every JSON-family tool here expects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerEntry {
    pub command: String,
    pub args: Vec<String>,
}

/// Per-tool outcome of a read or a write.
#[derive(Debug, Clone, Serialize)]
pub struct McpTargetStatus {
    pub tool: String,
    pub path: String,
    /// The tool created its config directory, so writing there is wanted.
    pub registered: bool,
    pub file_exists: bool,
    /// Our key is present in the file.
    pub installed: bool,
    /// Our key is present and equal to the entry the caller described.
    pub matches: bool,
    /// Why this target could not be read or written. Other targets still run.
    pub error: Option<String>,
}

struct Target {
    tool: &'static str,
    path: PathBuf,
}

/// Every tool whose MCP config is a JSON document with a top-level `mcpServers`
/// object. Verified against real files for Claude Code/Desktop, Cursor, Qoder
/// and Gemini; Windsurf follows the documented `mcp_config.json` layout and is
/// inert until that directory exists.
fn targets() -> Vec<Target> {
    let home = dirs::home_dir();
    let config = dirs::config_dir();
    let specs: [(&str, Option<PathBuf>, &[&str]); 6] = [
        ("Claude Code", home.clone(), &[".claude.json"]),
        (
            "Claude Desktop",
            config.clone(),
            &["Claude", "claude_desktop_config.json"],
        ),
        ("Cursor", home.clone(), &[".cursor", "mcp.json"]),
        ("Qoder", home.clone(), &[".qoder", "mcp.json"]),
        ("Gemini CLI", home.clone(), &[".gemini", "settings.json"]),
        (
            "Windsurf",
            home.clone(),
            &[".codeium", "windsurf", "mcp_config.json"],
        ),
    ];
    specs
        .into_iter()
        .filter_map(|(tool, base, rest)| {
            let path = rest.iter().fold(base?, |acc, part| acc.join(part));
            Some(Target { tool, path })
        })
        .collect()
}

/// The server binary that ships beside the app executable. In a dev checkout
/// this resolves inside `target/debug`, which is the same directory the app
/// itself is running from.
pub fn default_entry() -> McpServerEntry {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or(Path::new("."));
    let name = if cfg!(windows) {
        "skill-manager-mcp.exe"
    } else {
        "skill-manager-mcp"
    };
    McpServerEntry {
        command: dir.join(name).to_string_lossy().to_string(),
        args: Vec::new(),
    }
}

/// What each target currently holds, compared against `entry`.
pub fn status(entry: &McpServerEntry) -> Vec<McpTargetStatus> {
    targets().iter().map(|t| read(t, Some(entry))).collect()
}

/// The entry the UI should prefill, plus whether that command is actually on
/// disk. Installed bundles currently carry only the main executable, so the
/// sibling server binary can be missing — the panel says so instead of writing
/// a config that points at nothing.
pub fn suggested_entry() -> McpSuggestedEntry {
    let entry = default_entry();
    McpSuggestedEntry {
        command_exists: Path::new(&entry.command).is_file(),
        entry,
    }
}

/// `default_entry` paired with the evidence the user needs to trust it.
#[derive(Debug, Clone, Serialize)]
pub struct McpSuggestedEntry {
    #[serde(flatten)]
    pub entry: McpServerEntry,
    pub command_exists: bool,
}

/// Add or replace our key in every registered target.
pub fn install(entry: &McpServerEntry) -> Vec<McpTargetStatus> {
    let value = entry_value(entry);
    targets()
        .iter()
        .map(|t| {
            mutate(t, &mut |servers| {
                servers.insert(SERVER_KEY.to_string(), value.clone());
            })
        })
        .collect()
}

/// Drop our key from every registered target, leaving the other servers alone.
pub fn uninstall() -> Vec<McpTargetStatus> {
    targets()
        .iter()
        .map(|t| {
            mutate(t, &mut |servers| {
                servers.remove(SERVER_KEY);
            })
        })
        .collect()
}

fn entry_value(entry: &McpServerEntry) -> Value {
    let mut map = Map::new();
    map.insert("command".to_string(), Value::String(entry.command.clone()));
    map.insert(
        "args".to_string(),
        Value::Array(entry.args.iter().cloned().map(Value::String).collect()),
    );
    Value::Object(map)
}

fn is_registered(target: &Target) -> bool {
    target.path.parent().map(|p| p.is_dir()).unwrap_or(false)
}

fn status_of(target: &Target, installed: bool, matches: bool, error: Option<String>) -> McpTargetStatus {
    McpTargetStatus {
        tool: target.tool.to_string(),
        path: target.path.to_string_lossy().to_string(),
        registered: is_registered(target),
        file_exists: target.path.is_file(),
        installed,
        matches,
        error,
    }
}

/// Report one target without touching it. An unparseable file is a status with
/// an error, never a reason to skip the other tools.
fn read(target: &Target, entry: Option<&McpServerEntry>) -> McpTargetStatus {
    let doc = match load(target) {
        Ok(doc) => doc,
        Err(error) => return status_of(target, false, false, Some(error)),
    };
    let ours = doc
        .and_then(|doc| servers_of(&doc))
        .and_then(|s| s.get(SERVER_KEY).cloned());
    let installed = ours.is_some();
    let matches = match (ours, entry) {
        (Some(ours), Some(entry)) => ours == entry_value(entry),
        _ => false,
    };
    status_of(target, installed, matches, None)
}

/// Apply `edit` to the `mcpServers` map and write the document back, keeping the
/// rest of the file exactly as it was.
fn mutate(
    target: &Target,
    edit: &mut dyn FnMut(&mut Map<String, Value>),
) -> McpTargetStatus {
    if !is_registered(target) {
        return status_of(target, false, false, Some("not-registered".to_string()));
    }
    let mut doc = match load(target) {
        Ok(Some(doc)) => doc,
        Ok(None) => Value::Object(Map::new()),
        Err(error) => return status_of(target, false, false, Some(error)),
    };
    // A file whose `mcpServers` is present but not an object is left untouched:
    // rewriting it would guess at a shape the tool defined, and could break it.
    if matches!(doc.get("mcpServers"), Some(v) if !v.is_object()) {
        return status_of(target, false, false, Some("mcpServers-is-not-an-object".to_string()));
    }
    if let Value::Object(root) = &mut doc {
        let servers = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(servers) = servers {
            edit(servers);
        }
    }
    if let Err(error) = save(target, &doc) {
        return status_of(target, false, false, Some(error));
    }
    read(target, None)
}

/// Parse a config file, or `None` when it does not exist yet.
fn load(target: &Target) -> Result<Option<Value>, String> {
    if !target.path.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&target.path)
        .map_err(|e| format!("read failed: {}", e))?;
    let doc: Value = serde_json::from_str(&raw).map_err(|e| format!("parse failed: {}", e))?;
    if !doc.is_object() {
        return Err("config-root-is-not-an-object".to_string());
    }
    Ok(Some(doc))
}

fn servers_of(doc: &Value) -> Option<Map<String, Value>> {
    doc.get("mcpServers")
        .and_then(|s| s.as_object())
        .cloned()
}

/// Write through a sibling temp file and rename, so a crash mid-write cannot
/// leave a tool with a truncated config.
fn save(target: &Target, doc: &Value) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(doc).map_err(|e| format!("serialize failed: {}", e))?;
    let mut tmp = target.path.as_os_str().to_os_string();
    tmp.push(format!(
        ".tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, raw.as_bytes()).map_err(|e| format!("write failed: {}", e))?;
    match std::fs::rename(&tmp, &target.path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(format!("replace failed: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(command: &str) -> McpServerEntry {
        McpServerEntry {
            command: command.to_string(),
            args: vec!["--flag".to_string()],
        }
    }

    fn temp_target(dir: &Path, name: &str, body: &str) -> Target {
        std::fs::write(dir.join(name), body).expect("write config");
        Target {
            tool: "Test Tool",
            path: dir.join(name),
        }
    }

    fn read_json(path: &Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(path).expect("read back")).expect("parse")
    }

    /// What `install` does to one target, without the real targets list.
    fn install_in(target: &Target, wanted: &McpServerEntry) -> McpTargetStatus {
        let value = entry_value(wanted);
        mutate(target, &mut |s| {
            s.insert(SERVER_KEY.to_string(), value.clone());
        })
    }

    #[test]
    fn install_creates_the_key_and_keeps_everything_else() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(
            dir.path(),
            "mcp.json",
            r#"{ "mcpServers": { "theirs": { "command": "other", "args": [] } }, "theme": "dark" }"#,
        );

        let status = install_in(&target, &entry("bin/mcp"));

        assert!(status.installed);
        assert!(status.error.is_none());
        let doc = read_json(&target.path);
        assert_eq!(doc["theme"], "dark", "unrelated keys must survive");
        assert_eq!(doc["mcpServers"]["theirs"]["command"], "other");
        assert_eq!(doc["mcpServers"][SERVER_KEY]["command"], "bin/mcp");
        assert_eq!(doc["mcpServers"][SERVER_KEY]["args"][0], "--flag");
    }

    #[test]
    fn uninstall_removes_only_our_key() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(
            dir.path(),
            "mcp.json",
            r#"{ "mcpServers": { "theirs": { "command": "other" }, "skill-manager": { "command": "bin/mcp" } } }"#,
        );

        let status = mutate(&target, &mut |s| {
            s.remove(SERVER_KEY);
        });

        assert!(!status.installed);
        let doc = read_json(&target.path);
        assert!(doc["mcpServers"].get(SERVER_KEY).is_none());
        assert_eq!(doc["mcpServers"]["theirs"]["command"], "other");
    }

    #[test]
    fn a_missing_file_is_created_beside_the_existing_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = Target {
            tool: "Fresh Tool",
            path: dir.path().join("mcp.json"),
        };

        let status = install_in(&target, &entry("bin/mcp"));

        assert!(status.file_exists);
        assert!(status.installed);
        assert_eq!(read_json(&target.path)["mcpServers"][SERVER_KEY]["command"], "bin/mcp");
    }

    #[test]
    fn a_tool_that_was_never_run_is_left_alone() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = Target {
            tool: "Absent Tool",
            path: dir.path().join("not-created").join("mcp.json"),
        };

        let status = install_in(&target, &entry("bin/mcp"));

        assert_eq!(status.error.as_deref(), Some("not-registered"));
        assert!(!status.registered);
        assert!(
            !target.path.exists(),
            "must not create the tool's config directory"
        );
    }

    #[test]
    fn a_corrupt_config_reports_rather_than_being_overwritten() {
        let dir = tempfile::tempdir().expect("temp dir");
        let before = "{ this is not json ".to_string();
        let target = temp_target(dir.path(), "mcp.json", &before);

        let status = install_in(&target, &entry("bin/mcp"));

        assert!(status.error.unwrap().starts_with("parse failed"));
        assert_eq!(std::fs::read_to_string(&target.path).expect("still readable"), before);
    }

    #[test]
    fn a_non_object_mcp_servers_value_is_not_guessed_at() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(dir.path(), "mcp.json", r#"{ "mcpServers": [] }"#);

        let status = install_in(&target, &entry("bin/mcp"));

        assert_eq!(status.error.as_deref(), Some("mcpServers-is-not-an-object"));
        assert_eq!(read_json(&target.path)["mcpServers"], Value::Array(vec![]));
    }

    #[test]
    fn status_compares_the_entry_we_would_write() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(dir.path(), "mcp.json", "{}");
        let wanted = entry("bin/mcp");

        assert!(!read(&target, Some(&wanted)).matches, "empty file cannot match");
        install_in(&target, &wanted);
        let after = read(&target, Some(&wanted));
        assert!(after.installed);
        assert!(after.matches);
        assert!(
            !read(&target, Some(&entry("different/bin"))).matches,
            "a different command must not read as current"
        );
    }

    #[test]
    fn the_suggestion_points_at_the_bundled_server_binary() {
        let suggested = suggested_entry();
        let file_name = Path::new(&suggested.entry.command)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let expected = if cfg!(windows) {
            "skill-manager-mcp.exe"
        } else {
            "skill-manager-mcp"
        };
        assert_eq!(file_name, expected);
        assert!(suggested.entry.args.is_empty());
    }

    #[test]
    fn the_targets_list_uses_the_documented_config_locations() {
        let listed: Vec<String> = targets().iter().map(|t| t.tool.to_string()).collect();
        assert_eq!(
            listed,
            vec![
                "Claude Code",
                "Claude Desktop",
                "Cursor",
                "Qoder",
                "Gemini CLI",
                "Windsurf",
            ]
        );
    }
}
