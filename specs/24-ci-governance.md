# Phase 24 — CI governance integration (normative)

Status: done (2026-09-18)
Gate: `just v0` plus the governance-script checks below (locally runnable;
the composite action only orchestrates them)

## Goal

The semantic firewall (Phase 9) lives in `.imperium/policy.imp` and is
testable with `.imperium/policy.tests.json` (Phase 15) — but only locally.
Phase 24 turns it into a **review gate**: a composite GitHub Action that
downloads a checksum-verified `imperium` release binary (Phase 22), runs
`policy lint` + `policy test` against a consumer repo's committed policy
files, and fails the workflow when the firewall has lint errors or any
test entry flips. No new CLI verbs, no new MCP tools, no kernel changes.

## Normative behavior

### The gate script (`scripts/ci-governance.sh`)

Modes:

- `--from URL --policy-dir DIR` — the action path: download the release
  tarball and its `SHA256SUMS` (from the same base URL), verify the
  tarball digest **before extraction** (fail-closed on mismatch or
  missing entry), extract `bin/imperium`, then run the gate.
- `--binary PATH --policy-dir DIR` — run the gate against an existing
  binary (local tests).
- `--fetch-verify URL` — download + verify + extract only; prints the
  binary path.

The gate itself:

1. Requires `policy.tests.json` in `--policy-dir` — a governance job
   with nothing to test is a failure ("no policy.tests.json").
2. `policy.imp` is optional (absent = built-in defaults only).
3. Runs `imperium init` into a fresh `IMPERIUM_HOME`, copies the two
   files in, then `policy lint` and `policy test`.
4. Exit code: 0 only when lint is clean and every test entry PASSes.
   Any FAIL (or unloadable policy) exits 1 and the workflow goes red.

### The action (`action.yml`)

Composite action, inputs:

- `policy-dir` (default `.imperium`) — where the consumer repo keeps
  `policy.imp` + `policy.tests.json`.
- `version` (default `0.2.0`) — imperium release to gate with.

Platform triples: `Darwin/arm64 → aarch64-apple-darwin`,
`Darwin/x86_64 → x86_64-apple-darwin`, `Linux/x86_64 →
x86_64-unknown-linux-gnu`; anything else fails. The script is resolved
via `$GITHUB_ACTION_PATH`, so consumers never vendor it.

### Checks (`scripts/test-ci-governance.sh`)

1. A passing fixture (deny-glob that leaves the tested path allowed) →
   exit 0, output shows "1 passed".
2. A failing fixture (entry expects deny, policy allows) → exit 1.
3. Missing `policy.tests.json` → exit 1.
4. Full download path via `file://` URL: a real tarball (debug binary)
   + SHA256SUMS, `--from` fetch+verify+gate → exit 0; tampering the
   tarball after checksum generation → checksum-mismatch failure.

## Fail-closed cases

- Checksum mismatch / missing SHA256SUMS entry / missing `bin/imperium`
  in the tarball → the gate fails before any policy runs.
- Unsupported platform → immediate failure.
- Policy unloadable (parse errors) → `policy lint` reports errors and
  exits 1; a deny-only firewall can never be weakened by this phase.

## Scope boundary (explicitly still deferred)

- Windows runners, OIDC/sigstore verification of the downloaded binary
  (SLSA L2+; Phase 22 provenance is emitted, not signed), and any
  auto-fixing of policies. The action only observes and fails.
