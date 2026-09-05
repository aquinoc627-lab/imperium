# Agent constitution

You are an implementer, not an architect.

- No new crates, packages, CLI verbs, or workbench routes unless a named phase requires it.
- No README claims unless a test covers the behavior.
- No network in compiler/tests. The only network surface is the Phase 11 `cap.http` host action (policy-allowlisted hosts; execution-transport only — tests inject a fake transport).
- Allowed product: through Phase 11 (complete) — see `specs/v0-slice.md`, `specs/07-dry-run.md`, `specs/08-breadth.md`, `specs/09-policy.md`, `specs/10-ledger.md`, `specs/11-synthesis.md`, and `specs/STATUS.md`.
- The roadmap through Phase 15 is complete (`specs/15-preview-tests.md`, diff preview + policy test). Phase 16 (`specs/16-schedules.md`, scheduled intents) is confirmed and complete. New work beyond it requires a new named spec confirmed by the owner first.
- IR contract: `schemas/intent_ir.schema.json` plus `tests/contract/intent_ir/` and `tests/contract/v0_kernel.json`.
- Canonical kernel: `imperium-core::v0` (Rust). `web/v0` is the TS reference; keep both green against the shared fixtures.
- If blocked, stop. Do not invent evolution, P2P, voice, OPA, TPM, or model-provider routing.
