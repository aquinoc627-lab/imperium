# Phase 22 — Release & distribution (normative)

Status: done (2026-09-12)
Gate: `just v0` plus the release-script checks below (all scripts are
locally runnable; the workflow only orchestrates them per-OS)

## Goal

The CLI's only install path has been "clone + `cargo run`". Phase 22 ships
**versioned, checksummed release artifacts** for macOS (arm64 + x86_64)
and Linux x86_64, a Homebrew formula for the tap, and an **unsigned
SLSA Level 1 provenance statement** per release. No new crates; the only
new surface is `scripts/` and one tag-triggered workflow.

## Normative behavior

### Versioning

- Workspace member crates (`imperium-core`, `imperium-store`,
  `imperium-cli`) carry a real semver (`0.2.0`); no more `-dev` suffixes.
- A release is a pushed tag `vX.Y.Z` matching the crate version. The
  release workflow refuses mismatches (tag must equal the CLI's
  `CARGO_PKG_VERSION`).

### Artifacts (`scripts/make-release-artifacts.sh`)

For the current platform, with `VERSION` from the tag (defaults to the
crate version):

1. `cargo build -p imperium-cli --release` (gated: runs the Rust test
   targets first in `--check` mode via the `CHECK` flag).
2. Packages `imperium-<version>-<target>.tar.gz` containing the binary,
   `LICENSE`, and `README.md`.
3. Emits `SHA256SUMS` covering every tarball.
4. Naming: `<arch>-apple-darwin` (arm64/x86_64) and
   `x86_64-unknown-linux-gnu`.

### Homebrew (`scripts/gen-homebrew-formula.sh`)

Emits `imperium.rb` for the tap repo: version, URLs pointing at the
GitHub release tarballs per platform, and the exact `sha256` from the
release's `SHA256SUMS`. Deterministic output — testable against a local
fixture release directory.

### Provenance (`scripts/gen-provenance.sh`)

Emits an in-toto statement (`slsaProvenance` v0.2) per artifact subject:

- `builder.id`: GitHub-hosted runner (or `local` when run locally).
- `materials`: repo URI + git commit digest + entryPoint.
- `metadata.buildInvocationId`: GITHUB_RUN_ID when present.
- **Unsigned** — this is SLSA Level 1 (provenance exists, no sigstore
  signing). Levels 2–4 and sigstore signing stay deferred; the README
  says so in those exact words.

### Workflow (`.github/workflows/release.yaml`)

Trigger: push of `v*` tags. Matrix: `macos-14` (arm64), `macos-14`
cross-compiling `x86_64-apple-darwin` (the `macos-13` Intel runner pool
is deprecated upstream — the first release attempt waited 24h for a
runner and was cancelled; the fix cross-compiles with clang + SDKROOT,
verified locally and on the runner), `ubuntu-24.04` (x86_64). Each job
runs the artifact script, uploads tarballs + SHA256SUMS as release
assets via `gh release create` (the single authoritative job assembles
the final release and publishes the provenance statement).

### Local publish path (no Actions required)

An Actions **billing lock does not block GitHub Releases**. The identical
artifacts can be built and published from any machine:

```bash
bash scripts/make-release-artifacts.sh --out dist <version>
GH_PAT=<token> bash scripts/publish-release.sh <version>
```

`publish-release.sh` regenerates checksums + provenance, creates the
release for the existing tag via the REST API, and replaces
same-named assets (idempotent). Its `--dry-run` path is gate-tested
without network. This is the documented fallback whenever the tag
workflow cannot run.

## Fail-closed cases

- Tag/crate-version mismatch: workflow fails before any build.
- `just v0` red on the tag commit: the Linux job fails the release
  (the release workflow runs the gate first).
- Missing `gh` auth / release conflict: workflow fails; no partial
  release (all-or-nothing upload).

## Scope boundary (explicitly still deferred)

- Sigstore/cosign signing, SLSA Levels 2–4, npm/crates.io/brew-core
  publication, Windows builds, notarization of macOS binaries.
- The tap repo itself is owner-created; this phase ships the formula
  generator and instructions only.
