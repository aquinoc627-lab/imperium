# Phase 17 — Policy Impact & Coverage (normative)

Two read-only analysis surfaces over the policy + ledger. Neither modifies
state; both are pure report functions (`policy_impact_report`,
`policy_coverage_report`) with unit tests.

## `imperium policy impact --rule <RULE>`

Answers: *if this rule were added, which existing intents would flip?*

Accepted rule forms (the `.imp` grammar):

| Form | Simulation semantics |
|---|---|
| `deny <verb> matching <pattern>` | flips any scanned intent whose decision is not already `deny` **and** whose natural-language source contains the pattern (substring, case-insensitive) |
| `require approval <verb>` | flips scanned intents currently at `allow` to `require_approval`; existing `deny`/`require_approval` decisions never count as changed |
| `allow ...` | **rejected, fail-closed** — first-match ordering makes allow-rules position-dependent; `policy explain` shows the live chain instead |

Documented simplifications (they bound the tool's honesty):

- **Relevance = the intent's nl source mentions the verb.** The ledger's
  search index stores name/source/output text, not resolved capabilities.
- **Path proxy.** Decisions are re-evaluated with the nl line as the path;
  policy globs therefore match against the full nl line (e.g. `*b.txt*`).
- **Fetch is out of scope** — the ledger does not store resolved URLs.

Unknown verbs are rejected. Exit codes: `0` when no decision would change,
`1` when at least one would (CI-friendly).

## `imperium policy coverage`

A census, not a simulation: total intents in the ledger, total policy rules,
rules grouped by canonical form, and intents grouped by their opening word.
No decisions are re-evaluated; with no `policy.imp` it reports the built-in
defaults honestly.

## Tests

`main.rs::policy_tool_tests`: rule parsing (both forms, reject allow/short
input/unknown verb), deny flip + non-match, require-approval flipping only
allows (policy-backed deny stays), and the coverage census.
