# Phase 9 — The Semantic Firewall (normative)

Implemented canonically in `imperium-core::policy::imp` + `imperium-cli`,
mirrored by `web/v0/src/policy.ts`. Shared fixtures in
`tests/contract/v0_kernel.json` (`policy_cases`, `policy_parse_errors`,
`policy_lint_cases`, `builtin_content_cases`, policy simulate cases) pin the
contract across both implementations.

## The `.imp` language

One rule per line in `.imperium/policy.imp`; `#` comments; blank lines ignored.

```
deny <verb> matching <glob>      # glob over the full resolved path, * = any
deny containing <text>           # substring deny on the action text
require approval <verb>          # verb always needs explicit approval
allow <verb> under <prefix>      # explicit allow
```

- Verbs: `read`, `write`, `append`, `list` (anything else is a parse error).
- Evaluation is **first-match-wins in file order**; no match falls through to
  the grants layer.
- Globs match the full resolved path, `*` matches any sequence, case-insensitive.
- Content denies apply to: the echo text; path + contents for write/append;
  the path for read/list. At proposal time they apply to the whole input.
- **Load is fail-closed**: any parse issue → the policy refuses to load and
  every gate refuses (a broken firewall does not become an open one).

## Layering (each layer can only deny more, never less)

1. **Built-in host firewall** — sensitive paths (`*.key`, `*secret*`,
   `.env`, `.env.*`), per-segment, hard-coded at the host.
2. **Built-in content ceiling** — the former proposer BANNED list
   (`rm `, `sudo`, `curl`, … `delete from`), substring + case-insensitive,
   proposal-time only. Semantics unified across TS and Rust (plain substring;
   the old TS word-boundary regex is gone — see the
   `substring_not_word_boundary` fixture).
3. **User policy** — this file; extends the built-in ceiling at proposal time
   and denies/.requires-approval at dry-run + execution.
4. **Grants** — `grants.json` narrow-only permissions (Phase 8).

## Auditable decisions

Every fs action at execution emits a `PolicyEvaluated` event:

```json
{"verb": "read", "path": "scratch/notes.txt", "decision": "deny",
 "rule": 0, "reason": "policy: deny read matching scratch/notes*"}
```

Allowed actions emit the event too — the ledger records *why* something was
permitted, not just denied. When policy forces approval, `IntentApproved`
carries `"policy_approval": true`.

## Enforcement points

- **Dry-run (simulate)**: policy denies surface as `DENIED` previews with
  `policy: <rule>` reasons; denied dry-runs cannot be approved.
- **Proposal (compile --propose)**: user content denies + built-ins.
- **Execution**: re-checked independently of the dry-run (tightening a policy
  blocks an already-approved intent), with the audit event.
- **Approval policy**: `require approval <verb>` blocks auto-approve of
  low-risk verbs; explicit `intent approve` marks `policy_approval`.

## CLI

- `imperium policy lint` — parse + contradictions report:
  parse errors (error), duplicates, allows shadowed by earlier denies,
  denies already enforced by the host firewall (warnings).
- `imperium policy explain --verb <v> --path <p>` — the decision chain:
  policy rule (with index) → host firewall → grants → verdict.

## Lint issue messages (stable, fixture-pinned)

- `unrecognized rule` / `unknown verb: <v>` (parse errors)
- `duplicate of rule on line N`
- `unreachable: deny on line N covers this allow`
- `already enforced by the built-in host firewall`
