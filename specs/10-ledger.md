# Phase 10 — The Ledger (normative)

Implemented in `imperium-store` (first real use of the crate; now a
workspace member) + `imperium-cli`.

## The projection rule

The canonical records (`.imperium/intents/<id>.json`, with their embedded
event logs) remain the **only source of truth**. `ledger.db` is a SQLite
**projection** — a searchable index over those records. Deleting the
database and running `imperium ledger rebuild` reproduces it entirely from
the canonical records (tested). There is no in-memory truth.

Every `save` syncs the record into the projection; a `sync` failure fails
the operation loudly (a silently diverging index is worse than a visible
one).

## Schema

```sql
intents(id PK, name, status, nl_source, risk_score, requires_approval,
        output, capabilities, compiled_at, record)   -- record = full JSON
events(intent_id, seq, kind, payload, PK(intent_id, seq))
```

## Friction signal (spec 04 seed — no auto-synthesis)

The **third** compile of a semantically identical intent (same name + task
name + description + target — phrasing-independent, so "say ping" and
"Echo this message: ping" count together) appends:

```json
{"kind": "FrictionDetected",
 "payload": {"count": 3, "key": "...", "suggestion": "consider saving this as a reusable form"}}
```

The event is recorded in the intent's log; nothing is executed or generated
automatically.

## CLI

- `imperium search <query>` — LIKE over name, source, and output; prints
  `id  status  name  nl_source`.
- `imperium show --intent-id <id>` — the full trace: nl → goal → risk →
  task/effects → simulation → token → output → numbered events.
- `imperium ledger stats` — totals, by-status, success/failure counts,
  top capabilities, top denials (`TaskFailed` reasons + `PolicyEvaluated`
  deny decisions; allowed policy decisions are not counted as denials).
- `imperium ledger rebuild` — drop the projection, reproject every
  canonical record, print the count.
