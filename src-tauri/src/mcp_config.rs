// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The second half of roadmap v0.7.0 "MCP 集成": putting `skill-manager-mcp`
//! into each tool's own MCP config, and taking it out again.
//!
//! Scope is the *server this app owns*, not a general MCP editor: one key
//! (`skill-manager`) in a handful of files. That keeps the write set small enough
//! to make the promise that matters — the user's other servers, and every
//! unrelated key in those files, survive. "Survive" is not a nice-to-have here:
//! Codex keeps model settings and credentials in the same `config.toml` we write
//! to, so a document is re-serialized from its own parsed form
//! (`serde_json::Value` / `toml_edit::DocumentMut`) and never rebuilt from a
//! typed struct, and an unparseable file is reported instead of replaced.
//!
//! Two on-disk shapes exist in the wild and both are handled: JSON with a
//! top-level `mcpServers` map, and TOML with `[mcp_servers.<id>]` tables. TOML
//! needs `toml_edit` specifically because a `toml::Value` round-trip would drop
//! the user's comments and reformat their file.
//!
//! A target is only writable once its parent directory exists. A tool that was
//! never run has no config directory, and creating one would leave a stray file
//! the user has to find and delete themselves; such targets report
//! `registered: false` and are left alone.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use toml_edit::{Array, Item, Table};

/// The key this app owns inside each tool's `mcpServers` map.
pub const SERVER_KEY: &str = "skill-manager";

/// A stdio MCP entry, in the shape every supported tool expects.
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

/// How a tool stores its server list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Json,
    Toml,
}

struct Target {
    tool: &'static str,
    path: PathBuf,
    format: Format,
}

/// Every tool that can host a stdio server. The JSON shapes were verified against
/// real files for Claude Code/Desktop, Cursor, Qoder and Gemini; Windsurf follows
/// the documented `mcp_config.json` layout and is inert until that directory
/// exists; Codex is the TOML case, verified against a live `~/.codex/config.toml`
/// that also carries `[projects]` and per-server `[...env]` tables.
fn targets() -> Vec<Target> {
    let home = dirs::home_dir();
    let config = dirs::config_dir();
    let specs: [(&str, Option<PathBuf>, &[&str], Format); 7] = [
        ("Claude Code", home.clone(), &[".claude.json"], Format::Json),
        (
            "Claude Desktop",
            config.clone(),
            &["Claude", "claude_desktop_config.json"],
            Format::Json,
        ),
        ("Cursor", home.clone(), &[".cursor", "mcp.json"], Format::Json),
        ("Qoder", home.clone(), &[".qoder", "mcp.json"], Format::Json),
        (
            "Gemini CLI",
            home.clone(),
            &[".gemini", "settings.json"],
            Format::Json,
        ),
        (
            "Windsurf",
            home.clone(),
            &[".codeium", "windsurf", "mcp_config.json"],
            Format::Json,
        ),
        (
            "Codex CLI",
            home.clone(),
            &[".codex", "config.toml"],
            Format::Toml,
        ),
    ];
    specs
        .into_iter()
        .filter_map(|(tool, base, rest, format)| {
            let path = rest.iter().fold(base?, |acc, part| acc.join(part));
            Some(Target {
                tool,
                path,
                format,
            })
        })
        .collect()
}

/// The server binary that ships beside the app executable. In a dev checkout this
/// resolves inside `target/debug`, the same directory the app runs from; in an
/// installed bundle the server lands next to the app executable under the same
/// name (it is the crate's second binary, so Tauri ships it alongside the first),
/// which means one rule covers both.
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
/// disk — the panel says so instead of writing a config that points at nothing.
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
    apply_to_targets(&Edit::Set(entry.clone()))
}

/// Drop our key from every registered target, leaving the other servers alone.
pub fn uninstall() -> Vec<McpTargetStatus> {
    apply_to_targets(&Edit::Remove)
}

fn apply_to_targets(edit: &Edit) -> Vec<McpTargetStatus> {
    targets()
        .iter()
        .map(|t| write_target(t, edit))
        .collect()
}

/// The only two mutations this module performs on a config document.
#[derive(Debug, Clone)]
enum Edit {
    Set(McpServerEntry),
    Remove,
}

/// A parsed config file, kept in the format it arrived in so writing it back
/// cannot lose anything this module did not intend to touch.
enum Doc {
    Json(Value),
    Toml(toml_edit::DocumentMut),
}

impl Doc {
    fn empty(format: Format) -> Doc {
        match format {
            Format::Json => Doc::Json(Value::Object(Map::new())),
            Format::Toml => Doc::Toml(toml_edit::DocumentMut::new()),
        }
    }

    /// The entry currently registered under our key, if any.
    fn ours(&self) -> Option<McpServerEntry> {
        match self {
            Doc::Json(root) => root
                .get("mcpServers")?
                .as_object()?
                .get(SERVER_KEY)
                .and_then(entry_from_json),
            Doc::Toml(root) => root
                .get("mcp_servers")?
                .as_table()?
                .get(SERVER_KEY)?
                .as_table()
                .and_then(entry_from_toml),
        }
    }

    /// Apply `edit`, or return why the file's shape is not ours to guess at. The
    /// caller must not save anything when this fails.
    fn apply(&mut self, edit: &Edit) -> Result<(), String> {
        match self {
            Doc::Json(root) => apply_json(root, edit),
            Doc::Toml(root) => apply_toml(root, edit),
        }
    }
}

fn entry_from_json(value: &Value) -> Option<McpServerEntry> {
    let map = value.as_object()?;
    let command = map.get("command")?.as_str()?.to_string();
    Some(McpServerEntry {
        command,
        args: string_list(map.get("args")),
    })
}

fn entry_from_toml(table: &Table) -> Option<McpServerEntry> {
    let command = table.get("command")?.as_str()?.to_string();
    let args = table
        .get("args")
        .and_then(|value| value.as_array())
        .map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    Some(McpServerEntry { command, args })
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|a| a.as_array())
        .map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

fn entry_table_json(entry: &McpServerEntry) -> Value {
    let mut map = Map::new();
    map.insert("command".to_string(), Value::String(entry.command.clone()));
    map.insert(
        "args".to_string(),
        Value::Array(entry.args.iter().cloned().map(Value::String).collect()),
    );
    Value::Object(map)
}

fn entry_table_toml(entry: &McpServerEntry) -> Table {
    let mut table = Table::new();
    table["command"] = Item::Value(entry.command.clone().into());
    let mut args = Array::new();
    for arg in &entry.args {
        args.push(arg.clone());
    }
    table["args"] = Item::Value(args.into());
    table
}

/// Guarded JSON edit. A root or `mcpServers` that is not an object is left
/// untouched: rewriting it would guess at a shape the tool defined, and could
/// break it.
fn apply_json(root: &mut Value, edit: &Edit) -> Result<(), String> {
    let object = root
        .as_object_mut()
        .ok_or("config-root-is-not-an-object")?;
    if matches!(object.get("mcpServers"), Some(v) if !v.is_object()) {
        return Err("mcpServers-is-not-an-object".to_string());
    }
    let servers = object
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()));
    let servers = servers
        .as_object_mut()
        .ok_or("mcpServers-is-not-an-object")?;
    match edit {
        Edit::Set(entry) => {
            servers.insert(SERVER_KEY.to_string(), entry_table_json(entry));
        }
        Edit::Remove => {
            servers.remove(SERVER_KEY);
        }
    }
    Ok(())
}

/// Guarded TOML edit — the Codex shape, where each server is its own table and
/// `toml_edit` leaves the surrounding comments, key order and quoting intact.
fn apply_toml(root: &mut toml_edit::DocumentMut, edit: &Edit) -> Result<(), String> {
    if root
        .get("mcp_servers")
        .is_some_and(|item| !item.is_table())
    {
        return Err("mcp_servers-is-not-a-table".to_string());
    }
    if root
        .get("mcp_servers")
        .and_then(|item| item.as_table())
        .is_some_and(|table| {
            table
                .get(SERVER_KEY)
                .is_some_and(|item| !item.is_table())
        })
    {
        return Err(format!("{SERVER_KEY}-is-not-a-table"));
    }
    let servers = root
        .entry("mcp_servers")
        .or_insert_with(|| Item::Table(Table::new()));
    let servers = servers
        .as_table_mut()
        .ok_or("mcp_servers-is-not-a-table")?;
    match edit {
        Edit::Set(entry) => {
            servers[SERVER_KEY] = Item::Table(entry_table_toml(entry));
        }
        Edit::Remove => {
            servers.remove(SERVER_KEY);
        }
    }
    Ok(())
}

fn is_registered(target: &Target) -> bool {
    target.path.parent().map(|p| p.is_dir()).unwrap_or(false)
}

fn status_of(
    target: &Target,
    installed: bool,
    matches: bool,
    error: Option<String>,
) -> McpTargetStatus {
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

/// Report one target without touching it. An unparseable file is a status with an
/// error, never a reason to skip the other tools.
fn read(target: &Target, entry: Option<&McpServerEntry>) -> McpTargetStatus {
    let doc = match load(target) {
        Ok(doc) => doc,
        Err(error) => return status_of(target, false, false, Some(error)),
    };
    let ours = doc.and_then(|doc| doc.ours());
    let installed = ours.is_some();
    let matches = match (ours, entry) {
        (Some(ours), Some(entry)) => ours == *entry,
        _ => false,
    };
    status_of(target, installed, matches, None)
}

/// Apply `edit` to one target and write the document back, keeping the rest of
/// the file exactly as it was.
fn write_target(target: &Target, edit: &Edit) -> McpTargetStatus {
    if !is_registered(target) {
        return status_of(target, false, false, Some("not-registered".to_string()));
    }
    let mut doc = match load(target) {
        Ok(Some(doc)) => doc,
        Ok(None) => Doc::empty(target.format),
        Err(error) => return status_of(target, false, false, Some(error)),
    };
    // Removing a key the file does not have cannot change anything, and saving
    // would still rewrite the file — the JSON round-trip reformats it, so a
    // no-op removal has to stay a no-op.
    if matches!(edit, Edit::Remove) && doc.ours().is_none() {
        return read(target, None);
    }
    if let Err(error) = doc.apply(edit) {
        return status_of(target, false, false, Some(error));
    }
    if let Err(error) = save(target, &doc) {
        return status_of(target, false, false, Some(error));
    }
    read(target, None)
}

/// Parse a config file, or `None` when it does not exist yet.
fn load(target: &Target) -> Result<Option<Doc>, String> {
    if !target.path.is_file() {
        return Ok(None);
    }
    let raw =
        std::fs::read_to_string(&target.path).map_err(|e| format!("read failed: {}", e))?;
    match target.format {
        Format::Json => {
            let doc: Value =
                serde_json::from_str(&raw).map_err(|e| format!("parse failed: {}", e))?;
            if !doc.is_object() {
                return Err("config-root-is-not-an-object".to_string());
            }
            Ok(Some(Doc::Json(doc)))
        }
        Format::Toml => {
            let doc: toml_edit::DocumentMut =
                raw.parse().map_err(|e| format!("parse failed: {}", e))?;
            Ok(Some(Doc::Toml(doc)))
        }
    }
}

/// Write through a sibling temp file and rename, so a crash mid-write cannot
/// leave a tool with a truncated config.
fn save(target: &Target, doc: &Doc) -> Result<(), String> {
    let raw = match doc {
        Doc::Json(value) => {
            serde_json::to_string_pretty(value).map_err(|e| format!("serialize failed: {}", e))?
        }
        Doc::Toml(document) => document.to_string(),
    };
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

/// A target backed by a real file in `dir`, in the given format.
#[cfg(test)]
fn temp_target(dir: &Path, name: &str, body: &str, format: Format) -> Target {
    std::fs::write(dir.join(name), body).expect("write config");
    Target {
        tool: "Test Tool",
        path: dir.join(name),
        format,
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;

    fn entry(command: &str) -> McpServerEntry {
        McpServerEntry {
            command: command.to_string(),
            args: vec!["--flag".to_string()],
        }
    }

    fn install_in(target: &Target, wanted: &McpServerEntry) -> McpTargetStatus {
        write_target(target, &Edit::Set(wanted.clone()))
    }

    fn read_json(path: &Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(path).expect("read back")).expect("parse")
    }

    /// Clicking 移除 on a tool that never held our key must not touch the file:
    /// saving would re-serialize it, and a user's compact or idiosyncratic
    /// formatting would silently become ours.
    #[test]
    fn a_removal_with_nothing_to_remove_leaves_the_file_untouched() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(
            dir.path(),
            "mcp.json",
            r#"{"mcpServers":{"theirs":{"command":"other"}},"theme":"dark"}"#,
            Format::Json,
        );
        let before = std::fs::read_to_string(&target.path).expect("read");

        let status = write_target(&target, &Edit::Remove);

        assert!(!status.installed);
        assert_eq!(
            std::fs::read_to_string(&target.path).expect("read again"),
            before,
            "a no-op removal must not rewrite the file"
        );
    }

    #[test]
    fn install_creates_the_key_and_keeps_everything_else() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(
            dir.path(),
            "mcp.json",
            r#"{ "mcpServers": { "theirs": { "command": "other", "args": [] } }, "theme": "dark" }"#,
            Format::Json,
        );

        let status = install_in(&target, &entry("bin/mcp"));

        assert!(status.installed);
        assert!(status.error.is_none(), "{:?}", status.error);
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
            Format::Json,
        );

        let status = write_target(&target, &Edit::Remove);

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
            format: Format::Json,
        };

        let status = install_in(&target, &entry("bin/mcp"));

        assert!(status.file_exists);
        assert!(status.installed);
        assert_eq!(
            read_json(&target.path)["mcpServers"][SERVER_KEY]["command"],
            "bin/mcp"
        );
    }

    #[test]
    fn a_tool_that_was_never_run_is_left_alone() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = Target {
            tool: "Absent Tool",
            path: dir.path().join("not-created").join("mcp.json"),
            format: Format::Json,
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
        let target = temp_target(dir.path(), "mcp.json", &before, Format::Json);

        let status = install_in(&target, &entry("bin/mcp"));

        assert!(status.error.unwrap().starts_with("parse failed"));
        assert_eq!(
            std::fs::read_to_string(&target.path).expect("still readable"),
            before
        );
    }

    #[test]
    fn a_non_object_mcp_servers_value_is_not_guessed_at() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(dir.path(), "mcp.json", r#"{ "mcpServers": [] }"#, Format::Json);

        let status = install_in(&target, &entry("bin/mcp"));

        assert_eq!(
            status.error.as_deref(),
            Some("mcpServers-is-not-an-object"),
            "a shape we do not understand must be reported, not rewritten"
        );
        assert_eq!(read_json(&target.path)["mcpServers"], Value::Array(vec![]));
    }

    #[test]
    fn status_compares_the_entry_we_would_write() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(dir.path(), "mcp.json", "{}", Format::Json);
        let wanted = entry("bin/mcp");

        assert!(
            !read(&target, Some(&wanted)).matches,
            "empty file cannot match"
        );
        install_in(&target, &wanted);
        let after = read(&target, Some(&wanted));
        assert!(after.installed);
        assert!(after.matches);
        assert!(
            !read(&target, Some(&entry("different/bin"))).matches,
            "a different command must not read as current"
        );
    }
}

#[cfg(test)]
mod toml_tests {
    use super::*;

    fn entry(command: &str) -> McpServerEntry {
        McpServerEntry {
            command: command.to_string(),
            args: Vec::new(),
        }
    }

    fn install_in(target: &Target, wanted: &McpServerEntry) -> McpTargetStatus {
        write_target(target, &Edit::Set(wanted.clone()))
    }

    fn read_toml(target: &Target) -> String {
        std::fs::read_to_string(&target.path).expect("read back")
    }

    /// Shaped like the live Codex file: comments above and beside keys, an
    /// unrelated `[projects]` table with a quoted backslashed key, and a nested
    /// `[mcp_servers.<id>.env]` table.
    const CODEX_LIKE: &str = r#"# my model settings
model = "gpt-5"
service_tier = "default" # inline comment

[projects.'d:\dev\app']
trust_level = "trusted"

[mcp_servers]
# the one I wrote by hand
[mcp_servers.cloudbase]
command = "cloudbase-mcp"
args = ["--stdio"]

[mcp_servers.cloudbase.env]
LOG_LEVEL = "debug"
"#;

    #[test]
    fn install_keeps_comments_other_servers_and_nested_tables() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(dir.path(), "config.toml", CODEX_LIKE, Format::Toml);
        let wanted = entry(r"C:\apps\skill-manager-mcp.exe");

        let status = install_in(&target, &wanted);

        assert!(status.installed);
        assert!(status.error.is_none(), "{:?}", status.error);
        let after = read_toml(&target);
        assert!(after.contains("# my model settings"), "top comment lost");
        assert!(
            after.contains(r#"service_tier = "default" # inline comment"#),
            "inline comment lost: {after}"
        );
        assert!(
            after.contains("# the one I wrote by hand"),
            "server comment lost"
        );
        assert!(
            after.contains(r#"[projects.'d:\dev\app']"#),
            "projects table lost"
        );
        assert!(after.contains(r#"trust_level = "trusted""#), "projects value lost");
        assert!(after.contains("LOG_LEVEL = \"debug\""), "nested env lost");
        assert!(
            after.contains("[mcp_servers.skill-manager]"),
            "our table missing: {after}"
        );
        assert!(read(&target, Some(&wanted)).matches, "our entry unreadable");
    }

    #[test]
    fn a_second_install_replaces_rather_than_duplicates() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(dir.path(), "config.toml", CODEX_LIKE, Format::Toml);

        install_in(&target, &entry("first/bin"));
        install_in(&target, &entry("second/bin"));

        let after = read_toml(&target);
        assert_eq!(
            after.matches("[mcp_servers.skill-manager]").count(),
            1,
            "install must be idempotent: {after}"
        );
        assert!(read(&target, Some(&entry("second/bin"))).matches);
    }

    #[test]
    fn uninstall_removes_only_our_table() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(dir.path(), "config.toml", CODEX_LIKE, Format::Toml);
        install_in(&target, &entry("bin/mcp"));

        let status = write_target(&target, &Edit::Remove);

        assert!(!status.installed);
        let after = read_toml(&target);
        assert!(
            !after.contains(SERVER_KEY),
            "our table must be gone: {after}"
        );
        assert!(
            after.contains("[mcp_servers.cloudbase]"),
            "their server lost"
        );
        assert!(
            after.contains("# the one I wrote by hand"),
            "their comment lost"
        );
    }

    #[test]
    fn a_missing_file_is_created_beside_the_existing_directory_for_toml_too() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = Target {
            tool: "Fresh Codex",
            path: dir.path().join("config.toml"),
            format: Format::Toml,
        };

        let status = install_in(&target, &entry("bin/mcp"));

        assert!(status.file_exists);
        assert!(read(&target, Some(&entry("bin/mcp"))).matches);
        assert!(read_toml(&target).contains("[mcp_servers.skill-manager]"));
    }

    #[test]
    fn a_malformed_toml_is_reported_not_rewritten() {
        let dir = tempfile::tempdir().expect("temp dir");
        let before = "model =\n[mcp_servers".to_string();
        let target = temp_target(dir.path(), "config.toml", &before, Format::Toml);

        let status = install_in(&target, &entry("bin/mcp"));

        assert!(status.error.unwrap().starts_with("parse failed"));
        assert_eq!(read_toml(&target), before);
    }

    #[test]
    fn an_mcp_servers_key_that_is_not_a_table_is_not_guessed_at() {
        let dir = tempfile::tempdir().expect("temp dir");
        let body = "mcp_servers = \"not-a-table\"\n";
        let target = temp_target(dir.path(), "config.toml", body, Format::Toml);

        let status = install_in(&target, &entry("bin/mcp"));

        assert_eq!(status.error.as_deref(), Some("mcp_servers-is-not-a-table"));
        assert_eq!(read_toml(&target), body);
    }

    #[test]
    fn args_survive_the_toml_round_trip() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = temp_target(dir.path(), "config.toml", CODEX_LIKE, Format::Toml);
        let wanted = McpServerEntry {
            command: "bin/mcp".to_string(),
            args: vec!["--project".to_string(), "d:/dev/app".to_string()],
        };

        install_in(&target, &wanted);

        assert!(read(&target, Some(&wanted)).matches);
    }
}

#[cfg(test)]
mod targets_tests {
    use super::*;

    #[test]
    fn the_targets_list_covers_the_json_tools_and_codex() {
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
                "Codex CLI",
            ]
        );
    }

    #[test]
    fn only_codex_is_written_as_toml() {
        let toml_tools: Vec<String> = targets()
            .iter()
            .filter(|t| t.format == Format::Toml)
            .map(|t| t.tool.to_string())
            .collect();
        assert_eq!(toml_tools, vec!["Codex CLI".to_string()]);
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
}
