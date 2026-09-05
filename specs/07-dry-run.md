# Phase 7 — Dry-run preview (normative)

Implemented in `imperium-core` (canonical) and mirrored by the `web/v0` TS
reference. Both validate the shared fixtures in `tests/contract/v0_kernel.json`.

## IR v2: declarative effects

`Task.effects` is a list of typed effects:

```json
{ "type": "echo", "text": "ping" }
{ "type": "write", "path": "scratch/notes.txt", "size_bytes": 5 }
```

- Protocol version is now `2`. Validation accepts `1` (legacy, read-only) and `2`.
- The rules compiler emits effects for every task it compiles.
- Legacy IRs without `effects` remain valid; their preview is unavailable and
  the simulator notes this.

## Dry-run semantics

`dry_run(ir) -> SimulationResult` builds a hypothetical event stream where
every event carries `"dry_run": true`:

```
IntentSimulated(dry_run) → TaskStarted(dry_run) → (TaskSucceeded | TaskFailed)(dry_run)*
```

It then folds that stream (the fold is the truth) and produces:

- `effects_preview`: one `EffectPreview` per declared effect
  - `{"kind": "echo", "text": ...}`
  - `{"kind": "write", "path": ..., "bytes": n}`
  - `{"kind": "write_denied", "path": ..., "reason": ...}`
- Any denial sets `success_probability = 0`, `risk = 1`, and appends a note —
  high-risk simulations cannot be approved (existing policy).
- Dry-run events are **never persisted** to the intent's event log.
- The persisted `IntentSimulated` event records the resulting preview.

## CLI

- `imperium intent simulate --intent-id <id>` renders the preview:
  `write scratch/notes.txt (5 bytes)` / `write DENIED <path>: <reason>`.
- `imperium intent simulate --intent-id <id> --json` prints the full
  `SimulationResult` JSON including `effects_preview`.
- `imperium intent approve --intent-id <id>` re-displays the preview: consent
  is always to a specific diff.

## Contract

`tests/contract/v0_kernel.json` pins: compile outputs (normalized ids and
timestamps), compile errors, proposer mappings, the canonical HMAC token
signature, dry-run simulations, and event-log folds. The Rust canonical
kernel and the TS reference must both satisfy every case.
