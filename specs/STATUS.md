# Honest status (2026-09-11)

The v0 vertical slice is implemented in [`web/v0`](../web/v0) as the TS
reference; the **canonical kernel is Rust** (`imperium-core::v0`). Both are
validated against the shared fixtures in `tests/contract/v0_kernel.json`.

| Phase | Claim | Status |
|-------|--------|--------|
| 0 | IR contract + honesty | Done in repo (schema + fixtures). Rust/Python IR types exist. |
| 1 | Echo loop | Done in `web/v0` and `imperium-core` (compile → simulate → token → execute → replay). |
| 2 | Signed HMAC tokens, fail-closed | Done (`token.ts` + `v0.rs`); canonical signature pinned by shared fixture. |
| 3 | `cap.write` under `scratch/` | Done (`scratch.ts` + `v0.rs`). |
| 4 | WASM guest + host imports | Done (`wasm-host.ts`, `guest.wat`). |
| 5 | Propose then rules | Done locally (`propose.ts` + `v0.rs`). Optional remote model is app-only. |
| 6 | Event-log fold is truth | Done (`replay.ts` + `v0.rs` + CLI `intent replay`). |
| 7 | IR v2 effects + dry-run preview | Done — `specs/07-dry-run.md`. |
| 8 | Capability breadth | Done — `specs/08-breadth.md`. |
| 9 | The Semantic Firewall | Done — `specs/09-policy.md`. |
| 10 | The Ledger | Done — `specs/10-ledger.md`. `imperium-store` is a workspace member. |
| 11 | Synthesis + scoped network | Done — `specs/11-synthesis.md`. |
| 12 | The Simulator | Done — `specs/12-simulation.md`. Seeded Monte Carlo; not the causal-graph engine in spec 03. |
| 13 | The Evolution Loop | Done — `specs/13-evolution.md`. No auto-promotion, no LLM patches. |
| 14 | MCP tool surface | Done — `specs/14-mcp.md`. Approval is not a tool. |
| 15 | Diff preview + policy test | Done — `specs/15-preview-tests.md`. `diff_preview` is a capped set difference, not a unified diff. |
| 16 | Scheduled intents | Done — `specs/16-schedules.md`. High-risk forms never self-approve. |
| 17 | Policy impact & coverage | Done — `specs/17-policy-impact.md`. Allow-rules rejected fail-closed. |
| 18 | Secret binding | Done — `specs/18-secret-binding.md`. Keychain is OS-bound, not TPM-sealed. |
| 19 | Voice input (workbench) | Done — `specs/19-voice-input.md`. Input-only; no TTS; browser STT may leave the device. |

## Kernel consolidation (Phase 7)

- `imperium-core::v0` is the canonical kernel: rules compiler, proposer,
  scratch paths, tokens, dry-run simulation, fold.
- `web/v0` remains the TS reference implementation; drift is caught by the
  shared contract fixtures (compile, propose, token signature, simulate, fold).
- The CLI consumes the Rust kernel.

## Still scaffolding (inert — do not extend)

- `crates/imperium-{daemon,runtime,sync,voice,ffi,crypto,policy}` — not workspace members
- `frontend/` — leftover app-builder tree; the mock workbench was deleted
- `docker/Dockerfile.daemon` and daemon compose services
- Python packages other than the Intent IR models
- Specs `02` (full synthesis), `03` (causal graph), `04` (autonomy) — deferred depth.
  The governance-first slices of those ideas shipped as Phases 11–13.
- Air-gap, SLSA, TPM sealing, Sigstore

## Planned

Phase 19 (voice input-only, workbench) is shipped. Specs `02`–`04` retain
deferred depth (causal graphs, A/B auto-promotion, LLM patching, P2P,
offline/on-device STT, TPM/SLSA) that would each need a new named spec.

Owner-named next candidate (spec required first): Monte Carlo factor breadth.

Do not add crates or UI pages until they consume `imperium-core::v0` behavior.
