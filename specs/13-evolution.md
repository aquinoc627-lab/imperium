# Phase 13 — The Evolution Loop (DONE)

Extends `specs/04-evolution-loop.md` with the governance-first slice:
**friction → form → shadow verification — every step human-gated, no
autonomy**. Status: **implemented** (canonical in `imperium-core::forms` +
`imperium-cli`, TS reference in `web/v0/src/forms.ts`).

## The friction signal gets a slot-aware key (Phase 10 amendment, as built)

The semantic key is now `capability|task_name|target_path` (name and
description excluded): three intents that differ only in content
(`Echo this message: a/b/c`) count as friction, and the varying field is
the form's slot. Exact repeats still trigger (superset); the Phase 10
tests stayed valid unchanged.

## Slot inference (as built)

Canonical forms have exactly one free-text field — the task description
(for read/list it is the path) — so the v0 slot is always `description`,
and templates are built **structurally** from the IR (never by substring
replacement; a description like "e" would otherwise corrupt the template).
Fetch has no free text and cannot become a form.

## Forms: reusable, human-saved templates (as built)

- `imperium forms save <intent-id> --name <name>` writes
  `.imperium/forms/<name>.json`:
  `{name, template (canonical string with {{slot}}), slot, created_from, created_at}`.
  Duplicates are refused. The slot is `description` (see inference above);
  templates are built structurally from the IR.
- `imperium forms run <name> --slot <value> [--trials N]` substitutes,
  compiles, records a `FormRun {form, slot_value}` event, and simulates —
  then **stops for the normal human approval**. The gauntlet is never
  skipped: low-risk verbs auto-approve exactly as before; high-risk verbs
  demand explicit approval; policy/grants/dry-run denials all apply.
- `forms list` shows each form with `runs` and `verified` counts computed
  as a **view over the ledger** (FormRun events; matching ShadowVerified
  events), plus the `verified` badge at ≥ 3. A badge authorizes nothing.

## Shadow execution: prove the preview before trusting it (as built)

`intent execute --shadow` runs the **full gauntlet** (token, policy,
approval — nothing is skipped) with one difference: destructive effects are
**redirected**:

- `cap.write`/`cap.append` targets move under `scratch/shadow/` (same
  relative path); `cap.read`/`cap.list` pass through (harmless);
  `cap.http` is refused in shadow mode (`fetch cannot run in shadow mode`).
- All shadow events carry `"shadow": true`; the **fold skips them** — a
  shadow run never advances the intent's status (it stays `Approved`), and
  replay still agrees with the store (tested). The world model skips them
  too (spec 12): redirected runs are provenance, not capability facts.
- The run folds a `ShadowVerified` event comparing the dry-run's
  `effects_preview` against the shadow run's actual effects (paths
  normalized: `scratch/shadow/x` ≡ `scratch/x`):
  `{match: bool, predicted: [...], actual: [...]}`.
- The one-shot token is consumed by the shadow run — the real execution
  needs a fresh approval. Every execution costs one human decision.
- A form with ≥ 3 matching `ShadowVerified` events shows the `verified`
  badge in `forms list`. **A badge authorizes nothing.**

## The evolution chain, audited

Every step is an event on the canonical logs; the chain reads:

```
FrictionDetected → FormSaved → FormRun → ShadowVerified* → (human decides)
```

- **No auto-promotion.** Spec 04's "auto-promote" is explicitly replaced by
  the human checkpoint; the closest thing is the verified badge.
- **No LLM patch generation.** Slot inference is deterministic diffing;
  spec 04's LLM step stays deferred indefinitely.
- Nothing executes that did not go through simulate → approve → execute.

## Scope boundary (explicitly still deferred)

A/B validation, statistical promotion, P2P sharing of forms, and any
form of autonomous self-modification. If those ever come, they get their
own named spec and their own human gate.

## Contract (as built)

Fixture sections in `tests/contract/v0_kernel.json`: `form_template_cases`
(IR → template/slot or the fetch refusal), `shadow_diff_cases` (predicted
vs actual → match, with redirect normalization), and a fold case proving
shadow events leave the status untouched. TS implements template building
+ the diff; shadow execution itself is CLI-side. The slot-aware friction
key is covered by store/CLI unit tests.
