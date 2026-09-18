# IMPERIUM as an MCP server (Phase 23)

`imperium mcp` serves the kernel as Model Context Protocol tools over
stdio (hand-rolled JSON-RPC 2.0). The tools: `intent_compile`,
`intent_simulate`, `intent_execute` (+shadow), `intent_search`,
`ledger_stats`, `world_show`, `forms_list`, `forms_run`, `policy_lint`,
`policy_explain`.

**Approval is deliberately not a tool.** An agent can propose and
simulate; the human approves in the CLI (`imperium intent approve`).
High-risk verbs therefore stop at the human gate even when an agent
drives the server.

## Requirements

- `imperium` on `PATH` (release tarball `bin/`, `cargo install`, or
  `cargo build --release`).
- A local `.imperium` home (run `imperium init` once) — policies,
  grants, and the token secret all live there and are never exposed to
  the agent.

## Host setup

| Host | Config file | Use this repo file |
|------|-------------|--------------------|
| Claude Desktop | `claude_desktop_config.json` | `mcp/clients/claude_desktop.json` |
| Cursor | `.cursor/mcp.json` | `mcp/clients/cursor.json` |
| Windsurf | `mcp_config.json` | `mcp/clients/windsurf.json` |
| VS Code | `.vscode/mcp.json` | `mcp/clients/vscode.json` |

Every config is `{"command": "imperium", "args": ["mcp"]}` and nothing
else — no secrets, no environment. `scripts/test-mcp-clients.sh` machine-
checks the shipped configs (valid JSON, `imperium mcp` command, and no
approval verbs anywhere).

## Verification

- `crates/imperium-cli/tests/mcp_stdio.rs` spawns the real binary and
  drives a full agent-host conversation (initialize → tools/list →
  compile → simulate → unknown-tool error) — runs in the `just v0` gate.
- The tool list is frozen: the test asserts `intent_approve` and
  `intent_revoke` never appear.
