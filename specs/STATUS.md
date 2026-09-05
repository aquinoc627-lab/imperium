# Honest status (2026-09-05)

The v0 vertical slice is implemented in [`web/v0`](../web/v0) and
`imperium-core` / `imperium-cli`. Rust is the source of truth for behavior.
JS is a port checked by `tests/contract/v0/`.

| Phase | Claim | Status |
|-------|--------|--------|
| 0 | IR contract + honesty | Done in repo (schema + fixtures). Rust/Python IR types exist. |
| 1 | Echo loop | Done (compile \u2192 simulate \u2192 token \u2192 WASM \u2192 replay). |
| 2 | Signed HMAC tokens, fail-closed | Done. Cross-language fixtures in `tests/contract/v0/`. |
| 3 | `cap.write` under `scratch/` | Done. CLI writes through the guest host. |
| 4 | WASM guest + host imports | Done in JS and the Rust CLI (`echo`/`write`/`read`). |
| 5 | Propose then rules | Done locally. |
| 6 | Event-log fold is truth | Done. CLI appends `events/<id>.jsonl` then snapshots. |
| 7 | `cap.read` under `scratch/` | Done. |
| CLI | v0 verbs on disk | compile/simulate/approve/execute/revoke/replay. Secret mode 0600. No re-approve after execute. |

## Still scaffolding

- Rust runtime / daemon / store / sync / voice (not workspace members)
- Python packages other than IR models
- `frontend/workbench` (mock UI)
- Specs `02`\u2013`04` (do not implement yet)
- Air-gap, SLSA, TPM, Sigstore
