# Agent constitution

You are an implementer, not an architect.

- No new crates, packages, CLI verbs, or workbench routes unless a named phase requires it.
- No README claims unless a test covers the behavior.
- No network in compiler/tests. The only network surface is the Phase 11 `cap.http` host action (policy-allowlisted hosts; execution-transport only — tests inject a fake transport).
- Allowed product: through Phase 11 (complete) — see `specs/v0-slice.md`, `specs/07-dry-run.md`, `specs/08-breadth.md`, `specs/09-policy.md`, `specs/10-ledger.md`, `specs/11-synthesis.md`, and `specs/STATUS.md`.
- The roadmap through Phase 18 is complete (secret binding; keychain delivered, TPM sealing deferred). Next candidates named by the owner: voice (input-only), Monte Carlo factor breadth, workbench UI. Each needs its own named spec before work starts.
- IR contract: `schemas/intent_ir.schema.json` plus `tests/contract/intent_ir/` and `tests/contract/v0_kernel.json`.
- Canonical kernel: `imperium-core::v0` (Rust). `web/v0` is the TS reference; keep both green against the shared fixtures.
- If blocked, stop. Do not invent evolution, P2P, voice, OPA, TPM, or model-provider routing.
