# Honest status (2026-08-19)

The v0 vertical slice is implemented in [`web/v0`](../web/v0). That is the source of truth for behavior.

| Phase | Claim | Status |
|-------|--------|--------|
| 0 | IR contract + honesty | Done in repo (schema + fixtures). Rust/Python IR types exist. |
| 1 | Echo loop | Done in `web/v0` (compile → simulate → token → execute → replay). |
| 2 | Signed HMAC tokens, fail-closed | Done (`token.ts`). |
| 3 | `cap.write` under `scratch/` | Done (`scratch.ts`). |
| 4 | WASM guest + host imports | Done (`wasm-host.ts`, `guest.wat`). |
| 5 | Propose then rules | Done locally (`propose.ts`). Optional remote model is app-only. |
| CLI v0 verbs | Done in `imperium-cli` (`intent compile|simulate|approve|execute|list|replay`). Store is `.imperium/`. Other CLI verbs removed / fail closed. |

## Still scaffolding

- Rust runtime / CLI / daemon / store / sync / voice
- Python packages other than IR models
- `frontend/workbench` (mock UI)
- Specs `02`–`04` (do not implement yet)
- Air-gap, SLSA, TPM, Sigstore

Do not add crates or UI pages until they consume `web/v0` behavior.
