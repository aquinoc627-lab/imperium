# Phase 18 — Secret Binding (normative)

The HMAC secret that signs every capability token must never live in a
shared-tracked plaintext file by default. Phase 17's audit found
`.imperium/token.secret` committed to git history (rotated + untracked as
remediation). This phase makes the storage posture explicit, OS-bindable,
and auditable.

## Scope

**Delivered:** OS keychain binding (macOS `security` CLI), file backend
(default, unchanged), migration, rotation, high-entropy generation,
`imperium secret` CLI verb.

**Explicitly out of scope:** TPM sealing (tpm2-tss / platform key sealing) —
a separate named spec. Keychain storage is OS-bound, *not* hardware-sealed;
the spec and CLI output must not claim otherwise.

## Backends

Resolution is explicit and fail-closed:

| `IMPERIUM_SECRET_BACKEND` | Behavior |
|---|---|
| unset / `file` | plaintext `token.secret` in the runtime home (legacy behavior) |
| `keychain` | macOS Keychain via `security`; file used only for one-time migration |
| any other value | **error** — no silent fallback |

The default remains `file` so tests, CI, and air-gapped use never touch the
user keychain. Opt-in is deliberate.

## Keychain mechanics (macOS)

Service `imperium`, account = canonicalized runtime-home path (multi-home
safe). No SDK — the existing `security` CLI, same hand-rolled philosophy as
`scripts/assemble-guest.mjs`:

- get: `security find-generic-password -s <svc> -a <acct> -w` (exit 44 = not found)
- put: `security add-generic-password -s <svc> -a <acct> -w <value> -U`
- delete: `security delete-generic-password -s <svc> -a <acct>`

Known limitation (documented, accepted): the secret transits argv briefly.
On non-darwin hosts the keychain backend errors with a clear message rather
than pretending to bind.

## Entropy

Fresh secrets are **two UUIDv4 `simple()` values concatenated** (64 hex
chars, ≈256 bits) — dependency-free, format-compatible (any string is a
valid HMAC key). Legacy single-UUID secrets keep working and are upgraded
on rotation.

## CLI verb (this phase justifies the new verb)

```
imperium secret status   # active backend, file presence, secret fingerprint
imperium secret bind     # migrate file secret → keychain, then delete the file
imperium secret rotate   # fresh high-entropy secret in the active backend
```

`bind` and `rotate` warn that every previously issued token stops
verifying. `status` reports the *real* posture — it must not print
"keychain" unless the keychain round-trip succeeded.

## Implementation contract

The signing path keeps a single chokepoint (`V0Home::secret`). Backends sit
behind an injectable `SecretStore` trait (`get`/`put`/`delete`); keychain
*command construction* is a pure function so the exact `security` argv is
unit-tested without ever executing it — same pattern as Phase 11's fake
HTTP transport. Tests always run on the `file`/memory backends.

## Tests

- file flow: generate-on-init persists, re-read stable
- entropy: fresh secrets are 64 hex chars
- keychain argv construction: exact commands, no execution
- resolution: unknown backend value fails closed; `keychain` on test hosts errors cleanly
- migration (`bind`): value preserved, file removed
- rotation: value replaced in the active backend
- `secret status` reflects backend + fingerprint
