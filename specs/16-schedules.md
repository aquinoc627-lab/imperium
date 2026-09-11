# Phase 16 — Scheduled Intents (audited cron) (normative)

## Problem

No way to schedule recurring intent runs. Users must manually re-run intents,
and there is no audit trail for skipped or repeated executions.

## Solution

Add a minimal scheduling layer on top of the existing CLI and kernel. Schedules
are stored as simple JSON files in `.imperium/schedules/`. A new CLI subcommand
`imperium schedule` manages them, and a `imperium schedule tick` command (run
via cron/launchd — no daemon required) processes due schedules.

Schedules are **form-bounded**: each schedule references a saved form name and a
slot value. When a schedule ticks, it re-enters the full gauntlet (compile →
simulate → execute) exactly as if the user had run `imperium forms run`. Because
execution requires the approval gate, schedules are restricted to low-risk verbs
only.

## Schema

`.imperium/schedules/<name>.json`:

```json
{
  "name": "daily-report",
  "form": "monthly-report",
  "slot": "2026-W15",
  "every_secs": 86400,
  "enabled": true,
  "created_at": "2026-01-15T10:00:00Z",
  "last_run_at": null,
  "next_run_at": "2026-01-16T10:00:00Z"
}
```

Fields:
- `name` — human-readable schedule name (unique filename)
- `form` — the saved form to run
- `slot` — the slot value substituted into the form
- `every_secs` — interval in seconds (integer > 0)
- `enabled` — true/false — pause without deleting
- `created_at` — ISO-8601 timestamp
- `last_run_at` — ISO-8601 timestamp of last tick (null if never run)
- `next_run_at` — ISO-8601 timestamp of next expected tick

## CLI subcommands

| Command | Behavior |
|---|---|
| `imperium schedule add --form <name> --slot <val> --every <secs>` | Add a new schedule (enabled by default) |
| `imperium schedule remove <name>` | Remove a schedule file |
| `imperium schedule pause <name>` | Disable (set `enabled: false`) |
| `imperium schedule resume <name>` | Re-enable (set `enabled: true`) |
| `imperium schedule list` | List all schedules with next_run_at status |
| `imperium schedule tick` | Run all due schedules (respects `enabled` and approval gate) |

**`imperium schedule tick`** scans `.imperium/schedules/`, computes `now >= next_run_at`,
and for each due + enabled schedule:
1. Runs the form via `forms run` (full gauntlet: compile → simulate → execute)
2. If the form's execution auto-approves (low-risk verb), the run succeeds and
   `last_run_at`/`next_run_at` are updated
3. If the form requires approval, the tick records the intent id and stops; the
   human must `imperium intent approve` it manually — the schedule does NOT
   self-approve
4. `next_run_at` is advanced by `every_secs` from the original value

**Safety guarantees:**
- Schedules can only contain low-risk verbs (the form gauntlet enforces this)
- Schedules never bypass the approval gate — high-risk verbs in a form cause the
  tick to halt and report "approval required"
- No daemon runs in the background — `tick` is a one-shot CLI command invoked by
  the user's own cron/launchd
- Each tick produces a ledger event `ScheduleFired` with the schedule name and
  outcome (success/approval_required/skipped_disabled)

## Tests

- Fixture schedules in `tests/contract/v0_kernel.json` (extended with schedule
  cases)
- `imperium schedule tick` integration: add a schedule with `every_secs: 1` and
  tick repeatedly, verifying `last_run_at` advances and `next_run_at` recalculates
- `imperium schedule tick` with a high-risk form verb: verify it stops with
  "approval required" and does not advance `last_run_at`
- Concurrency: `tick` is not re-entrant — if a previous tick is still running,
  subsequent ticks are skipped with a warning