# Honest status (2026-09-05)

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
| 7 | IR v2 effects + dry-run preview | Done — `specs/07-dry-run.md`. `Task.effects` (v2, v1 still valid), `dry_run()` folds a hypothetical `dry_run: true` event stream into `effects_preview`, CLI renders the preview at simulate and re-displays it at approve. Shared fixtures: `tests/contract/v0_kernel.json` (TS + Rust). Python IR models accept v2. |
| 8 | Capability breadth | Done — `specs/08-breadth.md`. `cap.read` / `cap.append` / `cap.list` with host enforcement, sensitive-path firewall (`*.key`, `*secret*`, `.env*`) at preview + execution, risk-by-verb drives `requires_approval`, low-risk auto-approve (audited), `.imperium/grants.json` three-layer subset check (token ⊆ declared ⊆ defaults, fail-closed), WASM guest grew `read`/`append`/`list` imports (encoder in `scripts/assemble-guest.mjs`, baseline-verified). |
| 9 | The Semantic Firewall | Done — `specs/09-policy.md`. `.imperium/policy.imp` (deny matching/containing, require approval, allow) evaluated first-match-wins; fail-closed load; enforced at dry-run, proposal, and execution; `PolicyEvaluated` audit events on every fs action; `policy lint` + `policy explain`; proposer BANNED list migrated to built-in content denies (unified substring semantics, fixture-pinned). |
| 10 | The Ledger | Done — `specs/10-ledger.md`. `imperium-store` is real (workspace member): SQLite projection over the canonical records, sync-on-save, `ledger rebuild` restores it from scratch (tested); `search` / `show` / `ledger stats`; `FrictionDetected` event on the third semantically identical compile (spec 04 seed, no auto-synthesis). |
| 11 | Synthesis + scoped network | Done — `specs/11-synthesis.md`. `cap.http` (`Fetch <https-url>`): URL hardening, default-deny policy allowlist (`allow fetch to <host>`), grants with `*` ceiling and closed init, token net floor, always-approved execution via injectable transport (ureq; tests never touch the network); OpenAPI → `CapabilityManifest` synthesis with deterministic WIT, seeded property tests, `capabilities add/approve/list` registry (registration grants nothing). |
| 12 | The Simulator | Done — `specs/12-simulation.md`. Seeded Monte Carlo (`intent simulate --trials/--seed`) over retry dynamics; world facts as a pure view over the ledger (`world show`, `V0Event.at` timestamps); integer-exact probabilistic approval gate (`mc_gate`, p ≥ 0.9); hard denials pass through unchanged; Monte Carlo fixtures byte-identical across Rust (u64 LCG) and TS (BigInt LCG). |
| 13 | The Evolution Loop | Done — `specs/13-evolution.md`. Slot-aware friction key (`capability|task|target`); human-saved forms (`forms save/run/list` — run re-enters the full gauntlet and stops for approval); shadow execution (`execute --shadow` redirects destructive effects under `scratch/shadow/`, folds `ShadowVerified` promise-vs-reality diff, never advances status, fetch refused); verified badge at ≥ 3 matching shadow runs (authorizes nothing). No auto-promotion, no LLM patches. |

## Kernel consolidation (Phase 7)

- `imperium-core::v0` is the canonical kernel: rules compiler, proposer,
  scratch paths, tokens, dry-run simulation, fold.
- `web/v0` remains the TS reference implementation; drift is caught by the
  shared contract fixtures (compile, propose, token signature, simulate, fold).
- The CLI consumes the Rust kernel.

## Still scaffolding

- Rust runtime / daemon / sync / voice (CLI v0 is real)
- Python packages other than IR models
- `frontend/workbench` (mock UI)
- Specs `02` (full synthesis), `03` (causal graph), `04` (autonomy) — the
  governance-first slices of `03`/`04` are drafted as Phases 12/13 below
- Air-gap, SLSA, TPM, Sigstore

## Planned

Nothing drafted. The roadmap (Phases 7–13) is complete; specs `02`–`04`
retain deferred depth (causal graphs, A/B auto-promotion, LLM patching,
P2P, voice, TPM/SLSA) that would each need a new named spec.

Do not add crates or UI pages until they consume `imperium-core::v0` behavior.
