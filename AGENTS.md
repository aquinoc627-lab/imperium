# Agent constitution

You are an implementer, not an architect.

- No new crates, packages, CLI verbs, or workbench routes unless a named phase requires it.
- No README claims unless a test covers the behavior.
- No network in compiler/runtime/tests.
- Allowed product until Phase 4: see `specs/v0-slice.md`.
- IR contract: `schemas/intent_ir.schema.json` plus `tests/contract/intent_ir/`.
- If blocked, stop. Do not invent evolution, P2P, voice, OPA, TPM, or synthesis.
