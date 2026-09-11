# Phase 15 — The Last Mile (normative)

Two small but vital capabilities: a dry-run diff preview and a `policy test`
harness. Together they close the feedback loop between intent formulation and
human decision.

## 15a — Dry-run diff preview (presentational enrichment)

**Problem:** The `simulate` engine produces an `EffectPreview` with a list of
`Write` effects, but the approval screen shows only the raw preview text. Humans
need to see *what changes* relative to the current state.

**Solution:** Add a pure, testable `diff_preview` helper in `imperium-core` that
takes `(old_content: Option<String>, new_content: String) -> Vec<String>` where
each string is a concise diff line (`+ line` or `- line`). The CLI enrichment
step, after a dry-run, reads the actual file at the target path (if it exists)
and renders `diff_preview(current, proposed)` inline in the approval
confirmation.

**API:** `pub fn diff_preview(old: Option<&str>, new: &str) -> Vec<String>`

**Behaviour:**
- If `old` is `None` (new file): all lines of `new` are prefixed `+`.
- If `old` is `Some`: unified-ish diff — common leading/trailing lines collapsed,
  interior lines marked `+` (in new but not old) or `-` (in old but not new).
- Truncated to 20 lines; excess shown as `... N more lines`.
- Pure (no fs access) — testable with fixtures.

**CLI enrichment:** After `simulate --dry-run`, for each `Write` effect in the
preview, if the path is on disk, read it, call `diff_preview`, and embed the
result in the approval prompt as a `diff:` section. This is presentation-only;
the persisted `IntentSimulated` event stores the raw preview; the diff is
ephemeral audit assistance.

## 15b — `imperium policy test` harness

**Problem:** No way to validate a `.imperium/policy.imp` file against expected
behaviour as part of CI or local development. You can lint it, but "does this
policy do what I think?" requires a manual walk.

**Solution:** New CLI subcommand `imperium policy test` that runs a policy test
harness. A test file `.imperium/policy.tests.json` lists entries; the CLI
evaluates each against the current policy and prints pass/fail.

**Format:** `.imperium/policy.tests.json` is a JSON array:

```json
[
  {"verb": "read",  "path": "secrets.json",    "expect": "allow"},
  {"verb": "write", "path": "notes.txt",       "expect": "deny"},
  {"verb": "fetch", "path": "https://example.com", "expect": "allow"},
  {"verb": "write", "path": "system.conf", "expect": "require_approval"}
]
```

Each entry has:
- `verb` — one of `read`, `write`, `append`, `list`, `fetch`
- `path` — a path pattern (simple substring match against the resolved host /
  resource)
- `expect` — one of `allow`, `deny`, `require_approval`
- `text` — optional free-form description (shown in output)

**CLI behaviour:**
1. Load `.imperium/policy.imp` (or built-in defaults).
2. For each test entry, evaluate `policy.evaluate(&verb, &resolved_host, &path)`.
3. Compare the resulting `PolicyDecision` kind (`Allow`/`Deny`/`RequireApproval`)
   against `expect`.
4. Print one line per test: `PASS verb path` or `FAIL verb path (expected X,
   got Y)`.
5. Exit 0 if all pass, exit 1 if any fail.

**Pure & fixture-able:** The policy evaluator is pure; the test harness can be
tested by feeding it synthetic policy data and asserting the correct exit codes
and output lines.