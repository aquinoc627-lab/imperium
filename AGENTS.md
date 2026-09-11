# Agent constitution

You are an implementer, not an architect.

- No new crates, packages, CLI verbs, or workbench routes unless a named phase requires it.
- No README claims unless a test covers the behavior.
- No network in compiler/tests. The only network surface is the Phase 11 `cap.http` host action (policy-allowlisted hosts; execution-transport only — tests inject a fake transport).
- Allowed product: through Phase 20 (complete) — see `specs/STATUS.md`. Specs `07`–`20` and `specs/v0-slice.md` describe what shipped. Specs `02`–`04` are deferred depth; do not implement them.
- Next candidates: none queued. Owner must name a phase and spec path before new product work.
- IR contract: `schemas/intent_ir.schema.json` plus `tests/contract/intent_ir/` and `tests/contract/v0_kernel.json`.
- Canonical kernel: `imperium-core::v0` (Rust). `web/v0` is the TS reference; keep both green against the shared fixtures.
- Gate: `just v0`. Do not revive full-stack CI for frontend/daemon/unused crates.
- If blocked, stop. Do not invent evolution, P2P, OPA, TPM, TTS, or model-provider routing.
- MCP must not grow an `approve`, `secret bind`, or `secret rotate` tool.
