# Phase 12 — The Simulator (DONE)

Extends `specs/03-simulation-engine.md` with the governance-first slice.
Status: **implemented** (canonical in `imperium-core::v0`, view in
`imperium-store`, TS reference mirrored).

## From deterministic dry-run to probabilistic dry-run

Phase 7's dry-run is exact but binary (success 1 or 0). Phase 12 adds
**uncertainty with explainability**, over the same preview:

- `intent simulate --trials N [--seed S]` runs a seeded Monte Carlo over the
  intent's task graph: per-task duration sampling and per-attempt success
  sampling, honoring the IR's own `retry_policy` (max_attempts, backoff).
- `SimulationResult` gains (all serde-defaulted; the old shape stays valid):
  - `probabilistic: bool`, `trials: u64`, `seed: u64`
  - `p_success: f32`, `mc_successes: u64`
  - `p50_ms / p95_ms` — completion-time distribution
  - `factors: Vec<String>` — the per-task inputs, e.g.
    `P(cap.write) = 0.9091 (n=9)` or `P(cap.write) = 0.5000 (prior, n=0)`
- Deterministic: same IR + same world facts + same seed ⇒ identical numbers.
  Default seed derives from the intent id (FNV-1a 64; `derive_seed`).
- Events now carry `at` (epoch ms) so observed durations exist; legacy
  events without `at` parse fine and contribute samples but no durations.

## The world model is a view, not a store

World facts are computed **on the fly from the ledger's event log** — the
fold is the truth, so there is no new state to corrupt:

- per-capability success rate (Laplace-smoothed from `TaskSucceeded`/
  `TaskFailed`), observed duration samples (`TaskStarted.at`→
  `TaskSucceeded.at` deltas), sample counts.
- Only executions that reached `TaskStarted` count as samples — token and
  denial failures are not capability performance data.
- Shadow executions (`"shadow": true`) are provenance, not world facts:
  they are excluded from sample counts and duration samples (spec 13), so
  redirected runs can never inflate the stats behind the approval gate.
- Fewer than 5 duration samples ⇒ the static estimate is used per attempt
  (and the factor note is marked `prior`).
- `imperium world show` prints the facts. Implemented as
  `Ledger::world_stats()` — a pure view over the ledger (store unit tests
  cover aggregation and rebuildability; the TS reference has no store, so
  the facts themselves are not contract fixtures — the Monte Carlo math is).

## Approval gate change (the one behavioral risk, pinned here)

Today: approve requires `success_probability == 1.0`. With probabilistic
estimates that would block everything at 0.98. New rule:

- Static simulations: unchanged (`p == 1.0`).
- Probabilistic simulations: `mc_successes * 10_000 >= 9_000 * trials`
  (`mc_gate`, `APPROVAL_PROBABILITY_MILLI = 9000`) — **integer-exact**,
  because a float comparison of 0.9 diverges between f32 and f64 at
  exactly 900/1000 (the TS mirror uses BigInt the same way).
- **Hard denials still force `p_success = 0` and skip the rollout
  entirely** (`probabilistic` stays false) — uncertainty never softens a
  denial.
- The approval error prints the probability that failed the gate.

## Scope boundary (explicitly still deferred)

- No causal graph / counterfactual queries ("what if traffic 3x") — the
  world model is flat per-capability facts, not a causal graph.
- No chaos injection beyond the seeded failure sampling.
- No live filesystem probing: facts come from the event log only.
