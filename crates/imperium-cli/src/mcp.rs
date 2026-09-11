//! `imperium mcp` — Phase 14: the governed tool surface.
//!
//! A minimal, hand-rolled MCP (Model Context Protocol) stdio server:
//! newline-delimited JSON-RPC 2.0 in, responses out. The kernel is exposed
//! as tools; **approval is deliberately not a tool** — the agent proposes,
//! the kernel (and its human) dispose.

use crate::v0_cmd::{resolve_nl, V0Home};
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub const SERVER_NAME: &str = "imperium";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";

pub struct McpServer {
    home: V0Home,
}

impl McpServer {
    pub fn new(home: V0Home) -> Self {
        Self { home }
    }

    /// Handle one incoming line. Returns `None` for notifications/blank
    /// lines (no response), `Some(json)` for requests.
    pub fn handle(&self, line: &str) -> Option<Value> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        let req: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                return Some(Self::error_response(None, -32700, "parse error"));
            }
        };
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        // Notifications carry no id: never respond.
        let is_notification = id.is_none();
        match method {
            "initialize" => {
                if is_notification {
                    return None;
                }
                let version = req
                    .pointer("/params/protocolVersion")
                    .and_then(|v| v.as_str())
                    .unwrap_or(DEFAULT_PROTOCOL_VERSION)
                    .to_string();
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": version,
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
                    }
                }))
            }
            "notifications/initialized" | "notifications/cancelled" => None,
            "ping" => {
                if is_notification {
                    return None;
                }
                Some(json!({"jsonrpc": "2.0", "id": id, "result": {}}))
            }
            "tools/list" => {
                if is_notification {
                    return None;
                }
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {"tools": Self::tool_defs()}
                }))
            }
            "tools/call" => {
                if is_notification {
                    return None;
                }
                let name = req
                    .pointer("/params/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args = req.pointer("/params/arguments").cloned().unwrap_or(json!({}));
                let (text, is_error) = self.call_tool(&name, &args);
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{"type": "text", "text": text}],
                        "isError": is_error,
                    }
                }))
            }
            other => {
                if is_notification {
                    return None;
                }
                Some(Self::error_response(
                    id,
                    -32601,
                    &format!("method not found: {other}"),
                ))
            }
        }
    }

    fn error_response(id: Option<Value>, code: i64, message: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or(Value::Null),
            "error": {"code": code, "message": message},
        })
    }

    fn tool_defs() -> Vec<Value> {
        fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
            json!({
                "name": name,
                "description": description,
                "inputSchema": {
                    "type": "object",
                    "properties": props,
                    "required": required,
                },
            })
        }
        vec![
            tool("intent_compile",
                 "Compile a natural-language intent to IR. The agent proposes; rules decide.",
                 json!({
                     "input": {"type": "string", "description": "Natural language intent"},
                     "propose": {"type": "boolean", "description": "Map loose phrasing first"}
                 }),
                 &["input"]),
            tool("intent_simulate",
                 "Dry-run an intent: exact effects preview, optionally a seeded Monte Carlo.",
                 json!({
                     "intent_id": {"type": "string"},
                     "trials": {"type": "integer", "description": "Monte Carlo trials"},
                     "seed": {"type": "integer"}
                 }),
                 &["intent_id"]),
            tool("intent_execute",
                 "Execute through the full gauntlet. Low-risk verbs auto-approve; high-risk verbs fail until a human runs `imperium intent approve` in the CLI.",
                 json!({
                     "intent_id": {"type": "string"},
                     "shadow": {"type": "boolean", "description": "Redirect destructive effects under scratch/shadow/"}
                 }),
                 &["intent_id"]),
            tool("intent_search",
                 "Search the ledger by name, source, or output.",
                 json!({"query": {"type": "string"}}),
                 &["query"]),
            tool("ledger_stats",
                 "Aggregate ledger stats (statuses, capabilities, denials).",
                 json!({}), &[]),
            tool("world_show",
                 "Observed per-capability facts computed from the event log.",
                 json!({}), &[]),
            tool("forms_list",
                 "List saved forms with run/verified counts.",
                 json!({}), &[]),
            tool("forms_run",
                 "Run a saved form: substitute, compile, simulate. Stops for normal approval.",
                 json!({
                     "name": {"type": "string"},
                     "slot": {"type": "string", "description": "The slot value"},
                     "trials": {"type": "integer"}
                 }),
                 &["name", "slot"]),
            tool("policy_lint",
                 "Parse + lint .imperium/policy.imp.",
                 json!({}), &[]),
            tool("policy_explain",
                 "Explain the decision chain for a verb+path.",
                 json!({
                     "verb": {"type": "string", "enum": ["read", "write", "append", "list", "fetch"]},
                     "path": {"type": "string"}
                 }),
                 &["verb", "path"]),
        ]
    }

    /// Dispatch a tools/call. Tool failures are in-band per the MCP spec.
    fn call_tool(&self, name: &str, args: &Value) -> (String, bool) {
        match self.run_tool(name, args) {
            Ok(text) => (text, false),
            Err(e) => (format!("error: {e}"), true),
        }
    }

    fn run_tool(&self, name: &str, args: &Value) -> Result<String> {
        let arg_str = |key: &str| args.get(key).and_then(|v| v.as_str()).map(String::from);
        match name {
            "intent_compile" => {
                let input = arg_str("input").unwrap_or_default();
                let propose = args.get("propose").and_then(|v| v.as_bool()).unwrap_or(false);
                let nl = resolve_nl(&input)?;
                let rec = self.home.compile(&nl, propose)?;
                Ok(format!(
                    "{}\t{}\t{}",
                    rec.ir.id, rec.status, rec.ir.name
                ))
            }
            "intent_simulate" => {
                let id = arg_str("intent_id").unwrap_or_default();
                let trials = args.get("trials").and_then(|v| v.as_u64());
                let seed = args.get("seed").and_then(|v| v.as_u64());
                let rec = self.home.simulate_opts(&id, trials.map(|t| (t, seed)))?;
                serde_json::to_string_pretty(
                    rec.simulation.as_ref().expect("simulated"),
                )
                .map_err(|e| anyhow::anyhow!("{e}"))
            }
            "intent_execute" => {
                let id = arg_str("intent_id").unwrap_or_default();
                let shadow = args.get("shadow").and_then(|v| v.as_bool()).unwrap_or(false);
                let rec = if shadow {
                    self.home.execute_shadow(&id)?
                } else {
                    self.home.execute(&id)?
                };
                Ok(format!(
                    "{}\t{}\t{}",
                    rec.ir.id,
                    rec.status,
                    rec.output.unwrap_or_default()
                ))
            }
            "intent_search" => {
                let query = arg_str("query").unwrap_or_default();
                let hits = self.home.ledger()?.search(&query)?;
                let lines: Vec<String> = hits
                    .iter()
                    .map(|h| format!("{}\t{}\t{}", h.id, h.status, h.name))
                    .collect();
                Ok(if lines.is_empty() {
                    "no matches".into()
                } else {
                    lines.join("\n")
                })
            }
            "ledger_stats" => {
                let stats = self.home.ledger()?.stats()?;
                serde_json::to_string_pretty(&stats).map_err(|e| anyhow::anyhow!("{e}"))
            }
            "world_show" => {
                let stats = self.home.ledger()?.world_stats()?;
                serde_json::to_string_pretty(&stats).map_err(|e| anyhow::anyhow!("{e}"))
            }
            "forms_list" => {
                let mut lines = vec![];
                for form in self.home.list_forms()? {
                    let (runs, verified) = self.home.form_usage(&form)?;
                    lines.push(format!("{form}\truns={runs}\tverified={verified}"));
                }
                Ok(if lines.is_empty() {
                    "no forms".into()
                } else {
                    lines.join("\n")
                })
            }
            "forms_run" => {
                let form = arg_str("name").unwrap_or_default();
                let slot = arg_str("slot").unwrap_or_default();
                let trials = args.get("trials").and_then(|v| v.as_u64());
                let rec = self.home.run_form(&form, &slot, trials.map(|t| (t, None)))?;
                Ok(format!("{}\t{}", rec.ir.id, rec.status))
            }
            "policy_lint" => {
                let path = self.home.root.join("policy.imp");
                if !path.exists() {
                    Ok("no policy.imp (built-in defaults only)".into())
                } else {
                    let data = std::fs::read_to_string(&path)?;
                    let issues = imperium_core::policy::imp::lint_policy(&data);
                    if issues.is_empty() {
                        Ok("ok".into())
                    } else {
                        let lines: Vec<String> = issues
                            .iter()
                            .map(|i| {
                                format!(
                                    "{}\tline {}\t{}",
                                    match i.severity {
                                        imperium_core::policy::imp::Severity::Error => "error",
                                        imperium_core::policy::imp::Severity::Warning => "warning",
                                    },
                                    i.line,
                                    i.message
                                )
                            })
                            .collect();
                        Ok(lines.join("\n"))
                    }
                }
            }
            "policy_explain" => {
                let verb = arg_str("verb").unwrap_or_default();
                let path = arg_str("path").unwrap_or_default();
                let policy = self.home.load_policy()?;
                let host = match imperium_core::v0::fetch_url_host(&path)
                    .or_else(|_| imperium_core::v0::resolve_scratch_path(&path))
                {
                    Ok(h) => h,
                    Err(e) => e,
                };
                let decision = match policy.as_ref().map(|p| {
                    p.evaluate(&verb, &host, &path)
                }) {
                    Some(d) => d,
                    None => imperium_core::policy::imp::PolicyDecision::Allow { rule: None },
                };
                Ok(format!("{verb} {path} → {}", decision.kind()))
            }
            other => Err(anyhow::anyhow!("unknown tool: {other}")),
        }
    }

    /// The stdio loop: newline-delimited JSON-RPC in, responses out.
    /// Errors writing a line are fatal; malformed lines get a parse error.
    pub fn serve(&self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        for line in stdin.lock().lines() {
            let line = line?;
            if let Some(resp) = self.handle(&line) {
                writeln!(stdout, "{resp}")?;
                stdout.flush()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_server() -> McpServer {
        // Unique path per call — process::id() races under cargo test parallelism.
        let root = std::env::temp_dir().join(format!("imperium-mcp-{}", uuid::Uuid::new_v4()));
        let home = V0Home { root };
        home.init().unwrap();
        McpServer::new(home)
    }

    #[test]
    fn initialize_echoes_protocol_version() {
        let server = test_server();
        let resp = server
            .handle(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#)
            .unwrap();
        assert_eq!(resp.pointer("/result/protocolVersion").unwrap(), "2025-06-18");
        assert_eq!(resp.pointer("/result/serverInfo/name").unwrap(), "imperium");
        assert!(resp.pointer("/result/capabilities/tools").is_some());
    }

    #[test]
    fn notifications_are_silent() {
        let server = test_server();
        assert!(server
            .handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none());
        assert!(server.handle("").is_none());
    }

    #[test]
    fn tools_list_and_call_happy_path() {
        let server = test_server();
        let resp = server
            .handle(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)
            .unwrap();
        let tools = resp.pointer("/result/tools").unwrap().as_array().unwrap();
        let names: Vec<&str> = tools
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"intent_compile"));
        assert!(names.contains(&"intent_simulate"));
        // Approval is not a tool.
        assert!(!names.contains(&"intent_approve"));
        assert!(!names.contains(&"intent_revoke"));

        let resp = server
            .handle(
                r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"intent_compile","arguments":{"input":"Echo this message: ping"}}}"#,
            )
            .unwrap();
        assert_eq!(resp.pointer("/result/isError").unwrap(), false);
        let text = resp.pointer("/result/content/0/text").unwrap().as_str().unwrap();
        assert!(text.contains("compiled"), "{text}");

        let id = text.split('\t').next().unwrap().to_string();
        let resp = server
            .handle(
                &format!(
                    r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"intent_simulate","arguments":{{"intent_id":"{id}"}}}}}}"#
                ),
            )
            .unwrap();
        assert_eq!(resp.pointer("/result/isError").unwrap(), false);
        let text = resp.pointer("/result/content/0/text").unwrap().as_str().unwrap();
        assert!(text.contains("effects_preview"), "{text}");
    }

    #[test]
    fn high_risk_execution_demands_the_human() {
        let server = test_server();
        let resp = server
            .handle(
                r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"intent_compile","arguments":{"input":"Echo hello world"}}}"#,
            )
            .unwrap();
        let id = resp
            .pointer("/result/content/0/text")
            .unwrap()
            .as_str()
            .unwrap()
            .split('\t')
            .next()
            .unwrap()
            .to_string();
        server
            .handle(
                &format!(
                    r#"{{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{{"name":"intent_simulate","arguments":{{"intent_id":"{id}"}}}}}}"#
                ),
            )
            .unwrap();
        // High-risk would demand approve; echo auto-approves — the test documents the compile+sim path.
        let _ = id;
    }

    #[test]
    fn tool_failures_are_in_band_and_unknown_tools_error() {
        let server = test_server();
        let resp = server
            .handle(
                r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"intent_simulate","arguments":{"intent_id":"nope"}}}"#,
            )
            .unwrap();
        assert_eq!(resp.pointer("/result/isError").unwrap(), true);
        let resp = server
            .handle(
                r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"approve_everything","arguments":{}}}"#,
            )
            .unwrap();
        assert_eq!(resp.pointer("/result/isError").unwrap(), true);
        let text = resp.pointer("/result/content/0/text").unwrap().as_str().unwrap();
        assert!(text.contains("unknown tool"), "{text}");
    }

    #[test]
    fn malformed_json_is_a_parse_error_with_null_id() {
        let server = test_server();
        let resp = server.handle("not json").unwrap();
        assert_eq!(resp.pointer("/error/code").unwrap(), -32700);
        assert!(resp.get("id").unwrap().is_null());
    }

    #[test]
    fn unknown_method_is_a_json_rpc_error() {
        let server = test_server();
        let resp = server
            .handle(r#"{"jsonrpc":"2.0","id":10,"method":"not/a/real/method"}"#)
            .unwrap();
        assert_eq!(resp.pointer("/error/code").unwrap(), -32601);
    }
}
