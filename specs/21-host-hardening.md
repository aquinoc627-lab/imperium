# Phase 21 — Host-layer hardening (normative)

Status: done (2026-09-12)
Gate: `just v0` plus the property tests below (dev-dependency: `proptest`,
test-only, no runtime surface)

## Goal

The host layer already denies path escapes, sensitive paths, symlinks, and
oversized risk. Phase 21 makes those boundaries **mechanically exercised**:
property tests hammer the compiler, proposer, path resolver, URL checker,
policy parser, diff preview, and token verifier with adversarial inputs, and
input-size caps give the rejections a fixed ceiling. No new capabilities, no
new CLI verbs, no runtime dependencies.

## Folded-in bugfixes (shipped before this phase; normative here)

- Symlink escape denial at every host fs op (write/append/read/list):
  `symlink_escape_guard` in `imperium-cli` rejects any path whose existing
  components include a symlink. Not an anti-race guarantee — a local
  single-user tool — but a planted link can no longer tunnel effects out of
  `scratch/`. (unix regression tests)
- `token.secret` is written owner-only (0600) via the `FileStore` chokepoint.

## Normative behavior

### Input-size caps (compiler + proposer, both kernels)

- `MAX_NL_SOURCE_LEN = 64 * 1024` bytes. `compile_rules` and `local_propose`
  reject longer sources with exactly:
  `Natural language source is too long.` (same string both kernels; pinned
  in the shared `compile_errors` / `propose_errors` fixtures).
- The cap applies to the raw source **before** trimming; the empty check
  still applies after trimming.
- CLI `load_policy` rejects `policy.imp` files larger than 1 MiB
  (fail-closed, same wording as a parse failure path).

### Property invariants (proptest, deterministic, no network)

`crates/imperium-core/tests/properties.rs` — every case runs inside
`cargo test -p imperium-core` (the gated integration target):

1. `compile_rules` never panics on arbitrary Unicode strings (including
   NUL, controls, multibyte, near-cap lengths). On `Ok`, the IR passes
   `validate()`, `risk_score ∈ [0, 1]`, every task has ≥ 1 capability,
   and no path field contains `\0` or `..` as a component.
2. `local_propose` never panics on arbitrary strings.
3. `resolve_scratch_path` never panics; on `Ok` the result starts with
   `scratch/`, contains no `..` component, no NUL, no `\`.
4. `fetch_url_host` never panics; on `Ok` the host contains `.`, no `@`,
   no `:`, and is not an IPv4 literal.
5. `parse_rules` / `lint_policy` never panic on arbitrary text.
6. `diff_preview` never panics; output ≤ 21 lines.
7. Token: `issue_token` → `verify_token` is `Ok` under matching context;
   flipping any byte of the signature yields `InvalidSignature`; adding
   net permissions to a non-http token yields `NetworkDenied`.
8. `path_allowed` agrees with the reference prefix rule
   (`p == prefix || p.starts_with(prefix + "/")` after slash normalization).
9. `mc_gate` agrees with integer-exact 0.9 (`successes * 10_000 >= 9_000 * trials`).

## Fail-closed cases

- Oversized NL: compile and propose both return the exact error string
  (fixtures pin TS/Rust parity). Nothing is persisted.
- Oversized policy: `simulate`/`compile --propose` paths that load policy
  fail closed with the policy-size error; a policy can only deny more,
  never less, so a rejected policy is a rejected load.

## Scope boundary (explicitly still deferred)

- Anti-race filesystem guarantees (O_NOFOLLOW per-component open) — would
  need platform-specific host work; the string + symlink-metadata layers
  remain the confinement story.
- TPM sealing, SLSA/Sigstore, air-gap attestation — separate named specs.
- No changes to capability breadth, token format, or the IR schema.
