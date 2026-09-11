# Phase 14 — The Governed Tool Surface (MCP) (normative)

`imperium mcp` — a minimal, hand-rolled MCP (Model Context Protocol) stdio
server that exposes the v0 kernel as **tools any MCP-capable agent can
call**. The thesis, deployed: *the agent proposes, the kernel disposes*.

## Transport and framing

Newline-delimited JSON-RPC 2.0 over stdio. No network, no SDK — the
protocol surface is small and the implementation is auditable (same
philosophy as `scripts/assemble-guest.mjs`).

Methods handled:

| Method | Behavior |
|---|---|
| `initialize` | echoes the client's `protocolVersion` (else server default), returns `capabilities: {tools: {}}` + `serverInfo` |
| `notifications/initialized` | notification — no response |
| `tools/list` | the tool table below |
| `tools/call` | dispatch; tool failures are **in-band** (`isError: true`), never JSON-RPC errors |
| `ping` | `{}` |

## The tool table — approval is not a tool

| Tool | Maps to | Governance |
|---|---|---|
| `intent_compile` | `compile [--propose]` | built-in content ceiling; policy content denies |
| `intent_simulate` | `simulate [--trials/--seed]` | dry-run preview; hard denials surface here |
| `intent_execute` | `execute` | **the gauntlet runs**: low-risk verbs auto-approve (audited); high-risk verbs fail with *approval required* — the human approves via the CLI, never through MCP |
| `intent_execute_shadow` | `execute --shadow` | full gauntlet, redirected + verified |
| `intent_search` | `search` | ledger view |
| `ledger_stats` | `ledger stats` | ledger view |
| `world_show` | `world` | ledger view |
| `forms_list` / `forms_run` | `forms list` / `forms run` | run re-enters the full gauntlet |
| `policy_lint` / `policy_explain` | `policy lint` / `explain` | policy views |

**Deliberately absent: `intent_approve`, `intent_revoke`.** Approval is the
human gate; a tool that approves would make the agent its own authority.
The `approve` verb stays CLI-only.

## Contract

The dispatcher is pure and unit-tested: `McpServer::handle(line) ->
Option<response>` — initialize handshake, tools/list shape, tools/call
dispatch (happy path + in-band tool errors), notification silence, and an
integration smoke (lines piped through the real binary). Tools reuse the
same `V0Home` code paths the CLI uses — no second kernel.
