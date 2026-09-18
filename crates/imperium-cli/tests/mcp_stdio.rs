//! Phase 23 — end-to-end stdio conversation with `imperium mcp`, driven
//! exactly the way an agent host drives an MCP server: newline-delimited
//! JSON-RPC 2.0 over stdin/stdout against the real compiled binary.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_imperium-cli")
}

fn tmp_home() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("imperium-mcp-e2e-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn init(home: &Path) {
    let status = Command::new(bin())
        .arg("init")
        .env("IMPERIUM_HOME", home)
        .status()
        .expect("imperium init");
    assert!(status.success());
}

struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpClient {
    fn start(home: &Path) -> Self {
        let mut child = Command::new(bin())
            .arg("mcp")
            .env("IMPERIUM_HOME", home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn imperium mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    /// Send one request line; return the parsed response line.
    fn send(&mut self, line: &str) -> serde_json::Value {
        writeln!(self.stdin, "{line}").expect("write request");
        self.stdin.flush().expect("flush request");
        let mut buf = String::new();
        let n = self
            .stdout
            .read_line(&mut buf)
            .expect("read response line");
        assert!(n > 0, "server closed the connection without responding");
        serde_json::from_str(buf.trim()).expect("response parses as JSON")
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn tool_names(client: &mut McpClient) -> Vec<String> {
    let resp = client.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    resp.pointer("/result/tools")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn agent_host_conversation_end_to_end() {
    let home = tmp_home();
    init(&home);
    let mut client = McpClient::start(&home);

    // initialize: the host handshake.
    let resp = client.send(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
    );
    assert_eq!(resp.pointer("/result/serverInfo/name").unwrap(), "imperium");
    assert!(resp.pointer("/result/capabilities/tools").is_some());

    // The frozen tool list: compile/simulate/execute exist...
    let names = tool_names(&mut client);
    for tool in ["intent_compile", "intent_simulate", "intent_execute"] {
        assert!(names.contains(&tool.to_string()), "missing {tool}: {names:?}");
    }
    // ...and approval is still not a tool (Phase 14, re-pinned here).
    assert!(!names.contains(&"intent_approve".to_string()), "{names:?}");
    assert!(!names.contains(&"intent_revoke".to_string()), "{names:?}");

    // compile an intent, then simulate it — the real agent loop.
    let resp = client.send(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"intent_compile","arguments":{"input":"Echo this message: ping"}}}"#,
    );
    assert_eq!(resp.pointer("/result/isError").unwrap(), false);
    let text = resp.pointer("/result/content/0/text").unwrap().as_str().unwrap();
    assert!(text.contains("compiled"), "{text}");
    let id = text.split('\t').next().unwrap().to_string();

    let resp = client.send(&format!(
        r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"intent_simulate","arguments":{{"intent_id":"{id}"}}}}}}"#
    ));
    assert_eq!(resp.pointer("/result/isError").unwrap(), false);
    let text = resp.pointer("/result/content/0/text").unwrap().as_str().unwrap();
    assert!(text.contains("effects_preview"), "{text}");

    // An unknown tool is an in-band error, and the server survives it.
    let resp = client.send(
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"approve_everything","arguments":{}}}"#,
    );
    assert_eq!(resp.pointer("/result/isError").unwrap(), true);
    let text = resp.pointer("/result/content/0/text").unwrap().as_str().unwrap();
    assert!(text.contains("unknown tool"), "{text}");

    // Alive after the error: ping still answers.
    let resp = client.send(r#"{"jsonrpc":"2.0","id":6,"method":"ping"}"#);
    assert_eq!(resp.pointer("/result").unwrap(), &serde_json::json!({}));
}

#[test]
fn protocol_errors_are_standard_json_rpc() {
    let home = tmp_home();
    init(&home);
    let mut client = McpClient::start(&home);

    let resp = client.send("not json");
    assert_eq!(resp.pointer("/error/code").unwrap(), -32700);
    assert!(resp.get("id").unwrap().is_null());

    let resp = client.send(r#"{"jsonrpc":"2.0","id":7,"method":"not/a/real/method"}"#);
    assert_eq!(resp.pointer("/error/code").unwrap(), -32601);
}
