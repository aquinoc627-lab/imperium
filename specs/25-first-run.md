# Phase 25 — First-run demo path (normative)

Status: implementing
Gate: `just v0` plus the demo checks listed below

## Goal

A stranger can complete one full gauntlet in under a minute without reading a
handover document. The path is fixed, audible, and fail-closed: if execute
does not return the expected echo text, the command exits non-zero.

## Normative behavior

### CLI: `imperium demo`

1. Resolve the runtime home (`IMPERIUM_HOME` or `.imperium`).
2. Call `init` (idempotent) so grants and scratch exist.
3. Compile the canonical sentence: `Echo this message: ping`.
4. Simulate (static dry-run).
5. Approve (issues an HMAC token).
6. Execute.
7. Print a numbered step log. Final line must be exactly `demo ok` on success.
8. Exit non-zero if the execute output does not contain the substring `ping`.

The command does not take a free-form sentence. It is not a general `run`
wrapper. High-risk verbs, network (`cap.http`), and propose are out of scope.

### README

The top of `README.md` gains a **First 60 seconds** section that shows, in
order:

```bash
just v0          # optional: prove the tree is green
cargo run -p imperium-cli -- demo
```

and, for the browser slice:

```bash
cd web/workbench && python3 -m http.server 8080
# Compile the pre-filled "Echo this message: ping" → Simulate → Execute
```

Claims in that section must match behavior covered by the gate.

### Workbench

No new kernel surface. The Source box may keep the pre-filled echo sentence
(already present). A **First-run** control is optional; if present it only
fills that sentence (or runs the same low-risk chain the user could click).
It must not skip Simulate for high-risk forms.

## Fail-closed cases

- Compile/simulate/approve/execute errors propagate; no silent success.
- Execute output without `ping` → non-zero exit and no `demo ok` line.
- Demo never enables network, never writes outside `scratch/`, never rotates
  secrets.

## Audit events

Same events as a manual echo path (`IntentCompiled`, `IntentSimulated`,
`IntentApproved`, `TokenIssued`, `TaskStarted`, `TaskSucceeded`). No new
event kinds.

## Tests

- `scripts/v0-smoke.sh` invokes `"$BIN" demo` under a temp `IMPERIUM_HOME`
  and requires the output to contain `demo ok` and `ping`.
- `cargo test -p imperium-cli` may include a focused unit/integration test of
  `demo::run_first_run` against a temp home.

## Non-goals

- Guided multi-intent tutorials or TUI wizards
- Auto-running Monte Carlo trials on first run
- Policy-explain as part of demo (already a separate CLI verb)
- Workbench session export/import
- Changing the IR, token format, or approval gate

## Honesty notes

`demo` exercises the **low-risk echo** path. This phase still performs an
**explicit** approve step so the gauntlet is visible in the log, even though
some low-risk verbs can auto-approve on `intent execute`.
