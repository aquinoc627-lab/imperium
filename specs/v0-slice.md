# IMPERIUM v0 slice (normative)

Implemented in [`web/v0`](../web/v0) and canonically in `imperium-core::v0`.
Specs `02`–`04` stay aspirational. Phase 7 extends this slice with dry-run
previews — see [`07-dry-run.md`](07-dry-run.md). Phase 8 grows the capability
surface — see [`08-breadth.md`](08-breadth.md). Phase 9 adds the user-authored
semantic firewall — see [`09-policy.md`](09-policy.md). Phase 10 adds the
ledger projection — see [`10-ledger.md`](10-ledger.md). Phase 11 adds
capability synthesis and the scoped network — see
[`11-synthesis.md`](11-synthesis.md). Phase 12 adds the probabilistic
simulator — see [`12-simulation.md`](12-simulation.md). Phase 13 adds the
evolution loop — see [`13-evolution.md`](13-evolution.md).

## Allowed intents

Rules compiler (always the gate):

```
Echo this message: <text>
Write file <path> with contents <text>
```

`<path>` must resolve under `scratch/`. `..`, `.`, and absolute paths are denied.

A local proposer may map looser phrasing (`say hello`, `save notes.txt with hi`) to those forms. A model may propose only `{echo|write|reject}`. Rules still decide.

## Capabilities

| Name | Effect | Permissions |
|------|--------|-------------|
| `cap.echo` | return the message | no FS, no net, no env |
| `cap.write` | persist under `scratch/` | FS prefix `scratch/` only |

Unknown capabilities are denied. Network is always deny-all.

Implementation: a WASM guest with host imports `echo` and `write`. Imports refuse the call if the token does not grant them.

## Simulation

Static planner only:

| Field | Rule |
|-------|------|
| `success_probability` | `1` if every task capability is known, else `0` |
| `risk` | `0` if known, else `1` |
| `duration_ms` | sum of `estimated_duration_ms` |

## Execution rules

1. Compile (or propose-then-compile) → IR version 1.
2. Simulate. High-risk / unknown cap cannot be approved.
3. Approve issues a one-shot HMAC-SHA256 token (nonce, expiry, subset permissions).
4. Execute without approve, with a bad/expired/reused/revoked token, or with a capability mismatch is rejected.
5. Replay folds the event log. The fold is the source of truth.

Events: `IntentProposed?`, `IntentCompiled`, `IntentSimulated`, `IntentApproved`, `TokenIssued`, `TokenRevoked?`, `TaskStarted`, `TaskSucceeded` | `TaskFailed`, `IntentReplayed`.

## Out of scope until this stays green

Evolution loop, P2P sync, voice, OPA/Rego, OpenAPI→WASM synthesis, TPM/IMA, plugin marketplace, mock frontend pages that are not store-backed.
