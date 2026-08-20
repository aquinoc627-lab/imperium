# IMPERIUM v0 slice (normative)

This is the only product that may be implemented until Phase 4 exit.
Specs `02`–`04` are aspirational and must not grow code until this slice works.

## Allowed intent

Natural language is compiled by a **rules/template compiler** (no LLM in v0):

```
Echo this message: <text>
```

Optional later (still Phase 1, still no LLM):

```
Write file <path> with contents <text>
```

`<path>` must resolve inside a scratch directory. No workspace mutation.

## Allowed capability

- Name: `cap.echo`
- Effect: return the message text (or write it to scratch)
- Permissions: no network, no shell, no secrets
- Implementation: host-enforced stub is acceptable until WASM wiring is real

Unknown capabilities are denied.

## Simulation (trivial)

Not Monte Carlo. Static planner only:

| Field | Rule |
|-------|------|
| `success_probability` | `1.0` if every task capability is `cap.echo`, else `0.0` |
| `risk_score` / sim risk | `0.0` if capability present, else `1.0` |
| duration | sum of `estimated_duration_ms` or retry defaults |
| cost | omit (no cost model) |

## Execution rules

1. Compile → validate IR (schema + DAG + version = 1).
2. Simulate.
3. `execute` without `approve` is rejected.
4. Append events: `IntentCompiled`, `IntentSimulated`, `IntentApproved`, `TaskStarted`, `TaskSucceeded` or `TaskFailed`.
5. Replay by `intent_id` reproduces the same terminal state.

## CLI verbs that may become real

- `imperium intent compile`
- `imperium intent simulate`
- `imperium intent approve`
- `imperium intent execute`
- `imperium intent list`

All other commands stay unimplemented.

## Out of scope

Evolution loop, P2P sync, voice, OPA/Rego embedding, OpenAPI→WASM synthesis,
TPM/IMA, plugin marketplace, 10k rollouts, frontend work beyond displaying real store state.
