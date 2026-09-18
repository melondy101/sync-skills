// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Outbound MCP server: JSON-RPC 2.0 over stdio exposing read-only tools over
//! the SQLite index, so coding agents can ask what is installed where.
//!
//! Dispatch is a pure line-in / line-out function so the whole protocol contract
//! is unit-testable without a process; [`serve_stdio`] is the only part that
//! touches the terminal. Writing other tools' MCP *config* is out of scope here.

use serde::Serialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::db::Database;

/// Newest protocol revision this server speaks. MCP tells an older server to
/// answer with its own version when the client offers a newer one.
const PROTOCOL_VERSION: &str = "2025-06-18";
const SERVER_NAME: &str = "skill-manager";
/// Same string the GUI resolves through `App::package_info().version`.
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `projects.id = 0` is the seeded "Global" scope (see `db::seed_data`).
const GLOBAL_PROJECT: i64 = 0;
const DEFAULT_LIMIT: i64 = 200;
const MAX_LIMIT: i64 = 1000;
/// `tools/list` page size; nine tools means one follow-up call for a strict client.
const TOOL_PAGE: usize = 8;

const JSONRPC: &str = "2.0";
const ERR_PARSE: i32 = -32700;
const ERR_INVALID_REQUEST: i32 = -32600;
const ERR_METHOD_NOT_FOUND: i32 = -32601;
const ERR_INVALID_PARAMS: i32 = -32602;

/// Dispatch one newline-delimited JSON-RPC message against the index.
/// `None` means nothing goes back on the wire: a blank line, a notification, or
/// a client response to a request (this server never sends requests).
pub fn handle_request(db: &Database, line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let msg: Value = match serde_json::from_str(trimmed) {
        Ok(msg) => msg,
        Err(e) => return Some(error_response(Value::Null, ERR_PARSE, &e.to_string())),
    };
    // A frame with no `method` is the client answering a request; we send none.
    let method = match msg.get("method")?.as_str() {
        Some(method) => method,
        None => return reject(&msg, ERR_INVALID_REQUEST, "method must be a string"),
    };
    if !matches!(msg.get("jsonrpc"), Some(v) if v.as_str() == Some(JSONRPC)) {
        return reject(&msg, ERR_INVALID_REQUEST, "jsonrpc must be \"2.0\"");
    }
    let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));

    let result = match method {
        "initialize" => Ok(initialize_result()),
        "notifications/initialized" | "notifications/cancelled" => return None,
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list(&params)),
        "tools/call" => match params.get("name").and_then(Value::as_str) {
            Some(name) => Ok(call_tool(db, name, &params)),
            None => Err((ERR_INVALID_PARAMS, "tools/call requires params.name".to_string())),
        },
        _ => Err((ERR_METHOD_NOT_FOUND, format!("method not found: {method}"))),
    };
    match result {
        Ok(value) => respond(&msg, value),
        Err((code, message)) => reject(&msg, code, &message),
    }
}

/// Serve the protocol on stdin/stdout until EOF, one JSON-RPC frame per line.
/// The GUI process keeps the same file open, so a concurrent write can surface as
/// `SQLITE_BUSY` here; every tool is read-only, so a client may retry freely.
pub fn serve_stdio() -> Result<(), String> {
    let db = Database::new()?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("stdin read failed: {e}"))?;
        if let Some(response) = handle_request(&db, &line) {
            writeln!(out, "{response}").map_err(|e| format!("stdout write failed: {e}"))?;
            out.flush().map_err(|e| format!("stdout flush failed: {e}"))?;
        }
    }
    Ok(())
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
    })
}

fn id_of(msg: &Value) -> Value {
    msg.get("id").cloned().unwrap_or(Value::Null)
}

/// Absent or null id marks a notification, which must never be answered.
fn respond(msg: &Value, result: Value) -> Option<String> {
    match id_of(msg) {
        Value::Null => None,
        id => Some(serialize(json!({ "jsonrpc": JSONRPC, "result": result, "id": id }))),
    }
}

fn reject(msg: &Value, code: i32, message: &str) -> Option<String> {
    match id_of(msg) {
        Value::Null => None,
        id => Some(error_response(id, code, message)),
    }
}

fn error_response(id: Value, code: i32, message: &str) -> String {
    serialize(json!({
        "jsonrpc": JSONRPC,
        "error": { "code": code, "message": message },
        "id": id
    }))
}

fn serialize(body: Value) -> String {
    serde_json::to_string(&body).unwrap_or_else(|e| {
        format!(
            "{{\"jsonrpc\":\"{JSONRPC}\",\"error\":{{\"code\":-32603,\"message\":\"cannot serialize response: {e}\"}},\"id\":null}}"
        )
    })
}

// ==================== tool catalogue ====================

/// One row of the catalogue: `tools/list` renders it, `tools/call` runs it.
/// Adding a tool means one entry here plus its handler below.
struct Tool {
    name: &'static str,
    description: &'static str,
    schema: Value,
    handler: fn(&Database, &Value) -> Result<Value, String>,
}

fn catalog() -> Vec<Tool> {
    vec![
        Tool {
            name: "list_skills",
            description: "List indexed skills with per-tool installation status. Scope 0 is global, any \
                          other value is a project id from list_projects.",
            schema: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "integer",
                        "description": "Scope to read; omit or 0 for the global SSOT."
                    },
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_LIMIT, "default": DEFAULT_LIMIT }
                }
            }),
            handler: list_skills,
        },
        Tool {
            name: "get_skill",
            description: "Read one skill record by numeric id, with the tool directories of its \
                          active installations.",
            schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Skill id as returned by list_skills." }
                },
                "required": ["id"]
            }),
            handler: get_skill,
        },
        Tool {
            name: "list_tools",
            description: "List the registered AI coding tools and the directories they sync from.",
            schema: json!({ "type": "object", "properties": {} }),
            handler: list_tools,
        },
        Tool {
            name: "list_projects",
            description: "List registered projects; id 0 is the synthetic Global scope.",
            schema: json!({ "type": "object", "properties": {} }),
            handler: list_projects,
        },
        Tool {
            name: "list_markets",
            description: "List configured market sources (provider, owner, repository, branch, layout) \
                          and their index state.",
            schema: json!({ "type": "object", "properties": {} }),
            handler: list_markets,
        },
        Tool {
            name: "list_market_skills",
            description: "List skills discovered in market indexes. Pass market_id to filter, or omit it \
                          for every source. Reading does not refresh an index.",
            schema: json!({
                "type": "object",
                "properties": {
                    "market_id": { "type": "integer", "description": "Market id from list_markets." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_LIMIT, "default": DEFAULT_LIMIT }
                }
            }),
            handler: list_market_skills,
        },
        Tool {
            name: "list_conflicts",
            description: "List unresolved version conflicts (same skill edited differently in several \
                          tools) for one scope, with the competing versions per tool.",
            schema: json!({
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "integer",
                        "description": "Scope to read; omit or 0 for the global SSOT."
                    },
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_LIMIT, "default": DEFAULT_LIMIT }
                }
            }),
            handler: list_conflicts,
        },
        Tool {
            name: "get_sync_logs",
            description: "Recent sync / scan / edit activity from the audit log, newest first.",
            schema: json!({
                "type": "object",
                "properties": {
                    "skill_id": { "type": "integer", "description": "Filter to one skill; omit for all." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_LIMIT, "default": DEFAULT_LIMIT }
                }
            }),
            handler: get_sync_logs,
        },
        Tool {
            name: "get_stats",
            description: "Counts of the whole index: skills, tools, projects, markets, remote skills \
                          and unresolved conflicts per scope.",
            schema: json!({ "type": "object", "properties": {} }),
            handler: get_stats,
        },
    ]
}

fn tools_list(params: &Value) -> Value {
    let defs = catalog();
    let start = params
        .get("cursor")
        .and_then(Value::as_str)
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(0)
        .min(defs.len());
    let tools: Vec<Value> = defs
        .iter()
        .skip(start)
        .take(TOOL_PAGE)
        .map(|t| json!({
            "name": t.name,
            "description": t.description,
            "inputSchema": t.schema
        }))
        .collect();
    let next = if start + TOOL_PAGE < defs.len() {
        Value::String((start + TOOL_PAGE).to_string())
    } else {
        Value::Null
    };
    json!({ "tools": tools, "nextCursor": next })
}

/// Tool failures — an unknown name or a failed read — come back as an
/// `isError` result rather than a JSON-RPC error, which is what MCP clients
/// render as tool output. Only an unusable request frame is a protocol error.
fn call_tool(db: &Database, name: &str, params: &Value) -> Value {
    let Some(tool) = catalog().into_iter().find(|t| t.name == name) else {
        return error_payload(&format!("unknown tool: {name}"));
    };
    match (tool.handler)(db, &arguments(params)) {
        Ok(payload) => json!({
            "content": [{ "type": "text", "text": payload.to_string() }],
            "isError": false
        }),
        Err(e) => error_payload(&e),
    }
}

fn error_payload(message: &str) -> Value {
    json!({
        "content": [{ "type": "text", "text": message }],
        "isError": true
    })
}

/// `params.arguments` is optional in MCP; anything but an object is ignored.
fn arguments(params: &Value) -> Value {
    match params.get("arguments") {
        Some(v @ Value::Object(_)) => v.clone(),
        _ => json!({}),
    }
}

// ==================== tool handlers ====================

fn list_skills(db: &Database, args: &Value) -> Result<Value, String> {
    let project_id = int_arg(args, "project_id").unwrap_or(GLOBAL_PROJECT);
    let views = db.list_skills_with_status(project_id)?;
    let mut page = envelope(views.len(), items_of(&views, args)?);
    page["project_id"] = json!(project_id);
    Ok(page)
}

fn get_skill(db: &Database, args: &Value) -> Result<Value, String> {
    let id = int_arg(args, "id").ok_or("get_skill requires an integer \"id\"")?;
    let skill = db.get_skill_by_id(id)?;
    let installations = db.get_active_installations(id, skill.project_id)?;
    Ok(json!({
        "skill": skill,
        "active_installations": installations
            .iter()
            .map(|(tool_id, global_path)| json!({ "tool_id": tool_id, "tool_global_path": global_path }))
            .collect::<Vec<_>>()
    }))
}

fn list_tools(db: &Database, args: &Value) -> Result<Value, String> {
    let tools = db.list_tools()?;
    Ok(envelope(tools.len(), items_of(&tools, args)?))
}

fn list_projects(db: &Database, args: &Value) -> Result<Value, String> {
    let projects = db.list_projects()?;
    Ok(envelope(projects.len(), items_of(&projects, args)?))
}

fn list_markets(db: &Database, args: &Value) -> Result<Value, String> {
    let markets = db.list_markets()?;
    Ok(envelope(markets.len(), items_of(&markets, args)?))
}

fn list_market_skills(db: &Database, args: &Value) -> Result<Value, String> {
    let market_id = int_arg(args, "market_id");
    let skills = db.list_remote_skills(market_id)?;
    let mut page = envelope(skills.len(), items_of(&skills, args)?);
    page["market_id"] = json!(market_id);
    Ok(page)
}

fn list_conflicts(db: &Database, args: &Value) -> Result<Value, String> {
    let project_id = int_arg(args, "project_id").unwrap_or(GLOBAL_PROJECT);
    let conflicts = db.list_unresolved_conflicts(project_id)?;
    let mut page = envelope(conflicts.len(), items_of(&conflicts, args)?);
    page["project_id"] = json!(project_id);
    Ok(page)
}

fn get_sync_logs(db: &Database, args: &Value) -> Result<Value, String> {
    let limit = limit_arg(args);
    let logs = db.get_sync_logs(int_arg(args, "skill_id"), Some(limit))?;
    Ok(envelope(logs.len(), items_of(&logs, args)?))
}

/// No aggregate queries exist on `Database`, so this counts the same lists the
/// app renders. The index is small enough that the extra reads are cheap.
fn get_stats(db: &Database, _args: &Value) -> Result<Value, String> {
    let projects = db.list_projects()?;
    let mut conflicts = 0usize;
    for project in &projects {
        conflicts += db.list_unresolved_conflicts(project.id)?.len();
    }
    Ok(json!({
        "skills": db.list_skills()?.len(),
        "tools": db.list_tools()?.len(),
        "projects": projects.len(),
        "markets": db.list_markets()?.len(),
        "remote_skills": db.list_remote_skills(None)?.len(),
        "unresolved_conflicts": conflicts
    }))
}

// ==================== shared helpers ====================

fn int_arg(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(Value::as_i64)
}

fn limit_arg(args: &Value) -> i64 {
    int_arg(args, "limit")
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_LIMIT)
        .min(MAX_LIMIT)
}

/// Serialize a result set, capped by the caller's `limit` so the JSON payload
/// stays a sane size for a context window.
fn items_of<T: Serialize>(rows: &[T], args: &Value) -> Result<Vec<Value>, String> {
    rows.iter()
        .take(limit_arg(args) as usize)
        .map(|row| serde_json::to_value(row).map_err(|e| format!("serialize failed: {e}")))
        .collect()
}

fn envelope(total: usize, items: Vec<Value>) -> Value {
    json!({ "total": total, "returned": items.len(), "items": items })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(db: &Database, frame: Value) -> Option<String> {
        handle_request(db, &frame.to_string())
    }

    /// A full response frame for a well-formed request.
    fn reply(db: &Database, frame: Value) -> Value {
        parse(&request(db, frame).expect("expected a response"))
    }

    fn parse(line: &str) -> Value {
        serde_json::from_str(line).unwrap()
    }

    fn call(db: &Database, name: &str, args: Value) -> Value {
        reply(
            db,
            json!({ "jsonrpc": JSONRPC, "id": 7, "method": "tools/call",
                    "params": { "name": name, "arguments": args } }),
        )["result"]
            .clone()
    }

    /// The JSON payload a successful tool call put into its text block.
    fn payload(db: &Database, name: &str, args: Value) -> Value {
        let result = call(db, name, args);
        assert!(!result["isError"].as_bool().unwrap(), "{name} failed: {result}");
        parse(result["content"][0]["text"].as_str().unwrap())
    }

    fn seed_skill(db: &Database, name: &str, project_id: i64) -> i64 {
        db.upsert_skill(
            name,
            Some("a description"),
            &format!("/tmp/ssot/{name}"),
            "content-hash",
            "core-hash",
            project_id,
        )
        .unwrap()
        .0
    }

    fn tool_names() -> Vec<String> {
        catalog().iter().map(|t| t.name.to_string()).collect()
    }

    #[test]
    fn initialize_announces_version_capabilities_and_server_info() {
        let db = Database::new_in_memory().unwrap();
        let res = reply(
            &db,
            json!({ "jsonrpc": JSONRPC, "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2099-01-01", "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" } } }),
        );
        assert_eq!(res["jsonrpc"], JSONRPC);
        assert_eq!(res["id"], 1);
        // A client offering a revision we do not know gets our newest one back.
        assert_eq!(res["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(res["result"]["capabilities"], json!({ "tools": {} }));
        assert_eq!(res["result"]["serverInfo"]["name"], SERVER_NAME);
        assert_eq!(res["result"]["serverInfo"]["version"], SERVER_VERSION);
    }

    #[test]
    fn notifications_and_blank_lines_produce_no_response() {
        let db = Database::new_in_memory().unwrap();
        for frame in [
            json!({ "jsonrpc": JSONRPC, "method": "notifications/initialized" }),
            json!({ "jsonrpc": JSONRPC, "method": "notifications/cancelled", "params": { "requestId": 1 } }),
            json!({ "jsonrpc": JSONRPC, "method": "notifications/unknown" }),
            json!({ "jsonrpc": JSONRPC, "method": "ping" }),
            json!({ "jsonrpc": JSONRPC, "method": "tools/list" }),
            json!({ "jsonrpc": JSONRPC, "method": "no/such/method" }),
            json!({ "jsonrpc": JSONRPC, "result": {}, "id": 1 }),
        ] {
            assert!(request(&db, frame).is_none(), "frame must stay unanswered");
        }
        assert!(handle_request(&db, "").is_none());
        assert!(handle_request(&db, "  \n").is_none());
    }

    #[test]
    fn ping_request_is_answered() {
        let db = Database::new_in_memory().unwrap();
        let res = reply(&db, json!({ "jsonrpc": JSONRPC, "id": 3, "method": "ping" }));
        assert_eq!(res["result"], json!({}));
    }

    #[test]
    fn tools_list_names_every_catalogued_tool_with_a_schema() {
        let db = Database::new_in_memory().unwrap();
        let mut listed: Vec<String> = Vec::new();
        let mut cursor: Option<String> = None;
        // The catalogue is paged, so walk the pages to see the whole list.
        loop {
            let mut params = json!({});
            if let Some(token) = &cursor {
                params["cursor"] = json!(token);
            }
            let res = reply(&db, json!({ "jsonrpc": JSONRPC, "id": 2, "method": "tools/list", "params": params }));
            let tools = res["result"]["tools"].as_array().unwrap().clone();
            assert!(!tools.is_empty());
            for tool in &tools {
                assert_eq!(tool["inputSchema"]["type"], "object", "{tool} has no input schema");
                assert!(tool["description"].as_str().unwrap().len() > 20);
                listed.push(tool["name"].as_str().unwrap().to_string());
            }
            cursor = res["result"]["nextCursor"].as_str().map(str::to_string);
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(listed, tool_names());
        assert_eq!(
            listed,
            vec![
                "list_skills",
                "get_skill",
                "list_tools",
                "list_projects",
                "list_markets",
                "list_market_skills",
                "list_conflicts",
                "get_sync_logs",
                "get_stats"
            ]
        );
    }

    #[test]
    fn tools_list_rejects_an_out_of_range_cursor_page_gracefully() {
        let db = Database::new_in_memory().unwrap();
        let res = reply(
            &db,
            json!({ "jsonrpc": JSONRPC, "id": 2, "method": "tools/list", "params": { "cursor": "999" } }),
        );
        assert_eq!(res["result"]["tools"].as_array().unwrap().len(), 0);
        assert!(res["result"]["nextCursor"].is_null());
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let db = Database::new_in_memory().unwrap();
        let res = reply(&db, json!({ "jsonrpc": JSONRPC, "id": "a", "method": "resources/list" }));
        assert_eq!(res["error"]["code"], ERR_METHOD_NOT_FOUND);
        assert!(res["error"]["message"].as_str().unwrap().contains("resources/list"));
        assert_eq!(res["id"], "a");
        assert!(res.get("result").is_none());
    }

    #[test]
    fn malformed_input_returns_jsonrpc_protocol_errors() {
        let db = Database::new_in_memory().unwrap();
        let truncated = parse(&handle_request(&db, "{\"jsonrpc\": \"2.0\", \"id\": 1,").unwrap());
        assert_eq!(truncated["error"]["code"], ERR_PARSE);
        assert!(truncated["id"].is_null());

        let no_version = reply(&db, json!({ "id": 4, "method": "initialize" }));
        assert_eq!(no_version["error"]["code"], ERR_INVALID_REQUEST);
        assert_eq!(no_version["id"], 4);

        let old_version = reply(&db, json!({ "jsonrpc": "1.0", "id": 5, "method": "initialize" }));
        assert_eq!(old_version["error"]["code"], ERR_INVALID_REQUEST);
    }

    #[test]
    fn tool_call_without_a_name_is_invalid_params() {
        let db = Database::new_in_memory().unwrap();
        let res = reply(
            &db,
            json!({ "jsonrpc": JSONRPC, "id": 6, "method": "tools/call", "params": { "arguments": {} } }),
        );
        assert_eq!(res["error"]["code"], ERR_INVALID_PARAMS);
    }

    #[test]
    fn unknown_tool_is_an_error_result_not_a_protocol_error() {
        let db = Database::new_in_memory().unwrap();
        let result = call(&db, "drop_all_skills", json!({}));
        assert!(result["isError"].as_bool().unwrap());
        assert_eq!(result["content"][0]["type"], "text");
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("unknown tool: drop_all_skills"));
    }

    #[test]
    fn every_tool_answers_on_a_fresh_index() {
        let db = Database::new_in_memory().unwrap();
        for tool in catalog() {
            let result = call(&db, tool.name, json!({}));
            if tool.name == "get_skill" {
                assert!(result["isError"].as_bool().unwrap(), "get_skill requires an id");
                continue;
            }
            assert!(!result["isError"].as_bool().unwrap(), "{} failed: {result}", tool.name);
            assert_eq!(result["content"][0]["type"], "text");
            parse(result["content"][0]["text"].as_str().unwrap());
        }
    }

    #[test]
    fn list_skills_scopes_by_project_and_counts_installs() {
        let db = Database::new_in_memory().unwrap();
        let empty = payload(&db, "list_skills", json!({}));
        assert_eq!(empty["total"], 0);
        assert_eq!(empty["project_id"], GLOBAL_PROJECT);
        assert!(empty["items"].as_array().unwrap().is_empty());

        let skill_id = seed_skill(&db, "pdf-tools", 0);
        let tools = db.list_tools().unwrap();
        db.toggle_installation(skill_id, tools[0].id, 0, true).unwrap();
        db.toggle_installation(skill_id, tools[1].id, 0, false).unwrap();
        let project = db.add_project("web", "/tmp/web").unwrap();
        let project_skill = seed_skill(&db, "pdf-tools", project.id);

        let global = payload(&db, "list_skills", json!({}));
        assert_eq!(global["total"], 1);
        assert_eq!(global["returned"], 1);
        assert_eq!(global["items"][0]["name"], "pdf-tools");
        assert_eq!(global["items"][0]["id"], skill_id);
        assert_eq!(global["items"][0]["install_count"], 1, "only active installs count");
        assert_eq!(global["items"][0]["installed_tools"][0]["tool_name"], tools[0].name);

        let scoped = payload(&db, "list_skills", json!({ "project_id": project.id }));
        assert_eq!(scoped["project_id"], project.id);
        assert_eq!(scoped["items"][0]["id"], project_skill);
        assert_ne!(scoped["items"][0]["id"], global["items"][0]["id"]);

        let capped = payload(&db, "list_skills", json!({ "limit": 100000 }));
        assert_eq!(capped["total"], 1);
        assert_eq!(capped["returned"], 1);
    }

    #[test]
    fn get_skill_returns_record_and_active_installations() {
        let db = Database::new_in_memory().unwrap();
        let skill_id = seed_skill(&db, "review-bot", 0);
        let tools = db.list_tools().unwrap();
        db.toggle_installation(skill_id, tools[0].id, 0, true).unwrap();

        let body = payload(&db, "get_skill", json!({ "id": skill_id }));
        assert_eq!(body["skill"]["name"], "review-bot");
        assert_eq!(body["skill"]["project_id"], 0);
        assert_eq!(body["active_installations"].as_array().unwrap().len(), 1);
        assert_eq!(body["active_installations"][0]["tool_global_path"], tools[0].global_path);
    }

    #[test]
    fn a_failing_read_comes_back_as_an_error_result() {
        let db = Database::new_in_memory().unwrap();
        for (args, expected) in [
            (json!({ "id": 987654 }), "Failed to get skill"),
            (json!({}), "requires an integer"),
        ] {
            let result = call(&db, "get_skill", args);
            assert!(result["isError"].as_bool().unwrap());
            assert!(result["content"][0]["text"].as_str().unwrap().contains(expected));
        }
    }

    #[test]
    fn list_tools_and_list_projects_mirror_the_seeded_index() {
        let db = Database::new_in_memory().unwrap();
        let tools = payload(&db, "list_tools", json!({}));
        assert_eq!(tools["total"], 5, "five preset tools are seeded on first open");
        assert!(tools["items"].as_array().unwrap().iter().any(|t| t["name"] == "Claude Code"));

        let projects = payload(&db, "list_projects", json!({}));
        assert_eq!(projects["total"], 1);
        assert_eq!(projects["items"][0]["id"], GLOBAL_PROJECT);
        assert_eq!(projects["items"][0]["name"], "Global");

        db.add_project("cli", "/tmp/cli").unwrap();
        assert_eq!(payload(&db, "list_projects", json!({}))["total"], 2);
    }

    #[test]
    fn markets_and_remote_skills_are_readable() {
        let db = Database::new_in_memory().unwrap();
        assert_eq!(payload(&db, "list_markets", json!({}))["total"], 0);

        let market = db
            .insert_market(
                "github",
                "anthropics",
                "skills",
                "main",
                "https://github.com/anthropics/skills",
                "subdir",
            )
            .unwrap();
        for (name, installed) in [("pdf", false), ("docx", true)] {
            db.upsert_remote_skill(
                market.id,
                name,
                Some("desc"),
                &format!("https://x/{name}"),
                &format!("/ssot/{name}"),
                "content-hash",
                "core-hash",
                installed,
                None,
            )
            .unwrap();
        }

        let markets = payload(&db, "list_markets", json!({}));
        assert_eq!(markets["total"], 1);
        assert_eq!(markets["items"][0]["owner"], "anthropics");
        assert_eq!(markets["items"][0]["provider"], "github");

        let all = payload(&db, "list_market_skills", json!({}));
        assert_eq!(all["total"], 2);
        assert!(all["market_id"].is_null(), "no filter means every source");
        assert!(all["items"].as_array().unwrap().iter().any(|s| s["is_installed"] == true));

        let filtered = payload(&db, "list_market_skills", json!({ "market_id": market.id }));
        assert_eq!(filtered["market_id"], market.id);
        assert_eq!(filtered["total"], 2);

        let one = payload(&db, "list_market_skills", json!({ "limit": 1 }));
        assert_eq!(one["total"], 2, "total is the whole result set");
        assert_eq!(one["returned"], 1);
        assert_eq!(one["items"].as_array().unwrap().len(), 1);

        assert_eq!(payload(&db, "list_market_skills", json!({ "market_id": 4242 }))["total"], 0);
    }

    #[test]
    fn list_conflicts_reports_unresolved_versions() {
        let db = Database::new_in_memory().unwrap();
        let none = payload(&db, "list_conflicts", json!({}));
        assert_eq!(none["total"], 0);
        assert_eq!(none["project_id"], GLOBAL_PROJECT);

        let skill_id = seed_skill(&db, "conflicted", 0);
        db.insert_conflict(
            skill_id,
            r#"[{"tool_id":1,"tool_name":"Claude Code","core_hash":"aaa","source_path":"/x"},
                {"tool_id":2,"tool_name":"Codex CLI","core_hash":"bbb","source_path":"/y"}]"#,
        )
        .unwrap();

        let list = payload(&db, "list_conflicts", json!({}));
        assert_eq!(list["total"], 1);
        assert_eq!(list["items"][0]["skill_name"], "conflicted");
        assert_eq!(list["items"][0]["versions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn get_sync_logs_reads_the_audit_trail() {
        let db = Database::new_in_memory().unwrap();
        let skill_id = seed_skill(&db, "logged", 0);
        let tool_id = db.list_tools().unwrap()[0].id;
        db.insert_sync_log(skill_id, tool_id, 0, "to_ssot", "success", None).unwrap();
        db.insert_sync_log(skill_id, tool_id, 0, "from_ssot", "failed", Some("boom"))
            .unwrap();

        let logs = payload(&db, "get_sync_logs", json!({}));
        assert_eq!(logs["total"], 2);
        assert!(logs["items"].as_array().unwrap().iter().any(|l| l["status"] == "failed"));
        assert_eq!(logs["items"][0]["skill_name"], "logged");

        let capped = payload(&db, "get_sync_logs", json!({ "limit": 1 }));
        assert_eq!(capped["returned"], 1, "the limit reaches the SQL query");
        assert_eq!(payload(&db, "get_sync_logs", json!({ "skill_id": 999 }))["total"], 0);
    }

    #[test]
    fn get_stats_counts_every_scope() {
        let db = Database::new_in_memory().unwrap();
        let fresh = payload(&db, "get_stats", json!({}));
        assert_eq!(fresh["skills"], 0);
        assert_eq!(fresh["tools"], 5);
        assert_eq!(fresh["projects"], 1);
        assert_eq!(fresh["markets"], 0);
        assert_eq!(fresh["remote_skills"], 0);
        assert_eq!(fresh["unresolved_conflicts"], 0);

        let skill_id = seed_skill(&db, "one", 0);
        db.insert_conflict(skill_id, "[]").unwrap();
        let after = payload(&db, "get_stats", json!({}));
        assert_eq!(after["skills"], 1);
        assert_eq!(after["unresolved_conflicts"], 1);
    }

    #[test]
    fn responses_are_one_json_object_per_line() {
        let db = Database::new_in_memory().unwrap();
        for frame in [
            json!({ "jsonrpc": JSONRPC, "id": 1, "method": "initialize" }),
            json!({ "jsonrpc": JSONRPC, "id": 2, "method": "tools/list" }),
            json!({ "jsonrpc": JSONRPC, "id": 3, "method": "bogus" }),
        ] {
            let line = request(&db, frame).unwrap();
            assert!(!line.contains('\n'), "frame must stay on one line: {line}");
            assert!(line.starts_with('{') && line.ends_with('}'));
        }
    }
}
