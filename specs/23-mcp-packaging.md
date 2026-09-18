# Phase 23 — MCP packaging & agent onboarding (normative)

Status: done (2026-09-12)
Gate: `just v0` plus the stdio conversation test and the client-config
checks below

## Goal

`imperium mcp` is already a governed tool surface (Phase 14). Phase 23
makes it **consumable by real agent hosts**: ready-to-paste client
configurations for Claude Desktop, Cursor, Windsurf, and VS Code, and a
true end-to-end stdio test that drives the server the way an agent host
does. No new MCP tools; **approval stays deliberately not a tool**.

## Normative behavior

### Client configs (`mcp/clients/`)

One JSON file per host, each declaring `imperium mcp` over stdio:

- `claude_desktop.json` — `mcpServers` entry for
  `claude_desktop_config.json`.
- `cursor.json` — `.cursor/mcp.json` shape.
- `windsurf.json` — `mcp_config.json` shape.
- `vscode.json` — `.vscode/mcp.json` shape (`servers` key).

The command is `imperium` with `args: ["mcp"]` — resolved from `PATH`
(installed via release tarball or `cargo install`). No environment
secrets, no flags beyond the subcommand.

### Config checks (`scripts/test-mcp-clients.sh`)

Every shipped config must:

1. parse as JSON;
2. contain a `command` (or equivalent) of `imperium` whose args include
   exactly `mcp` under the host's key;
3. never mention `approve`, `revoke`, `secret bind`, or `secret rotate`
   anywhere in the file — the agent-facing surface must stay
   approval-free (Phase 14 invariant, now machine-checked).

### Stdio conversation test (`crates/imperium-cli/tests/mcp_stdio.rs`)

Spawns the real `imperium-cli` binary (`CARGO_BIN_EXE_imperium-cli mcp`,
isolated `IMPERIUM_HOME`), sends newline-delimited JSON-RPC exactly as an
agent host does, and asserts:

1. `initialize` responds with serverInfo + tools capability.
2. `tools/list` returns the ten tool names — and **no**
   `intent_approve` / `intent_revoke` tool.
3. `tools/call` `intent_compile` produces a compiled intent id.
4. `tools/call` `intent_simulate` on that id returns an effects preview.
5. `tools/call` on a nonexistent tool (`approve_everything`) is an
   in-band error, not a crash; the server stays alive for the next
   request.
6. Malformed JSON yields `-32700`; unknown method yields `-32601`.

## Fail-closed cases

- A config referencing approval verbs fails `test-mcp-clients.sh` (exit
  1) — the gate goes red before such a config could ship.
- The stdio test fails if the server exits early, returns empty lines
  for requests, or exposes an approval tool.

## Scope boundary (explicitly still deferred)

- Registry publication, OAuth/remote MCP transports, and any new MCP
  tool (approval, secret bind/rotate) — the tool list is frozen by
  Phase 14 and this phase re-pins it with tests.
