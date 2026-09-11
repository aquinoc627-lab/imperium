# Phase 20 — Monte Carlo factor breadth (normative)

Status: implementing
Gate: `just v0` plus the Monte Carlo unit and contract cases below

## Goal

Probabilistic dry-run already reports a success probability. Operators also
need **why**. Phase 20 widens the `factors: Vec<String>` channel so each
task contributes success, duration, and retry notes — still flat explainability,
not a causal graph.

## Normative behavior

For each task in the IR, `dry_run_monte_carlo` appends three factor lines (in
order), unless the run is a hard-denial passthrough (`factors` stays empty):

1. **Success** — unchanged Laplace note:
   - enough samples: `P(cap) = r.rrrr (n=N)`
   - else: `P(cap) = r.rrrr (prior, n=N)`
2. **Duration**:
   - `durations_ms.len() >= MIN_DURATION_SAMPLES` (5): sorted p50 →
     `D(cap) = p50=Xms (n=N)`
   - else: `D(cap) = estimate Yms (prior, n=M)` where `Y` is
     `task.estimated_duration_ms` or 1000
3. **Retry**: `retry(cap) max_attempts=K` with `K = max(1, retry_policy.max_attempts)`

The Monte Carlo **sampling order is unchanged** (duration draw, then success
draw, per attempt). Only the explanatory `factors` list grows. `p_success`,
`p50_ms`, `p95_ms`, and `mc_gate` are unaffected by this phase.

TS mirror (`web/v0/src/simulate.ts`) must emit identical factor strings for
shared contract cases.

## Fail-closed cases

- Hard denial still returns the static denial result: `probabilistic = false`,
  empty `factors`, `p_success = 0`. Uncertainty never softens a denial.
- Missing world stats → prior success + estimate duration notes (not an error).

## Audit events

No new event kinds. Existing `IntentSimulated` payload may include the richer
`factors` array when the CLI stores the simulation result.

## Tests

- Rust unit `monte_carlo_factors_and_percentiles` expects the three-line set
  for history and prior cases
- Contract `tests/contract/v0_kernel.json` `monte_carlo_cases` expected
  `factors` arrays updated the same way
- TS mirror stays green under `just v0`

## Non-goals

- Causal graphs / counterfactuals (still deferred from specs 03 / 12)
- Per-path or per-host world-model dimensions
- Changing Laplace smoothing, seed derivation, or approval milli-threshold
- Chaos injection beyond existing failure sampling
- Workbench UI chrome for factors (CLI / stored simulation already surfaces them)

## Honesty notes

Factors are **descriptive notes**, not causal attributions. `D(cap)` is the
observed sample p50 used as a pool for draws, not a guarantee of wall-clock
time on the next execute.
