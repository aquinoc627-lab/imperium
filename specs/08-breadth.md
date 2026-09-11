# Phase 8 — Capability breadth (normative)

Implemented canonically in `imperium-core::v0` + `imperium-cli`, mirrored by
the `web/v0` TS reference (incl. the WASM guest). Shared fixtures in
`tests/contract/v0_kernel.json` cover every case below.

## Canonical intent forms

```
Echo this message: <text>
Write file <path> with contents <text>
Read file <path>
Append file <path> with contents <text>
List files under <path>
```

`<path>` always resolves under `scratch/` (same rules as `cap.write`).

## Capabilities

| Name | Effect | Host semantics | Built-in grant |
|------|--------|----------------|----------------|
| `cap.echo` | return text | none | none |
| `cap.write` | persist file | create/truncate | `fs: ["scratch"]` |
| `cap.read` | read file | fail if missing | `fs: ["scratch"]` |
| `cap.append` | append to file | create if missing, never truncates | `fs: ["scratch"]` |
| `cap.list` | list a directory | entries as text | `fs: ["scratch"]` |

## Sensitive-path firewall (host layer, hard-coded)

Patterns: `*.key`, `*secret*`, `.env`, `.env.*` — matched per path segment,
case-insensitive. Enforced for **all** fs capabilities at preview and
execution; denial reason is `sensitive path denied`. Policy (Phase 9) may
deny more; it can never weaken this.

## Risk by verb and approval policy

| Capability | Risk |
|------------|------|
| `cap.echo` | 0.0 |
| `cap.list` | 0.1 |
| `cap.read` | 0.2 |
| `cap.append` | 0.4 |
| `cap.write` | 0.5 |
| unknown | 1.0 |

- `requires_approval = risk >= APPROVAL_THRESHOLD (0.4)` — compiler-derived.
- `execute` of a low-risk simulated intent **auto-approves**: it issues a
  one-shot token and emits `IntentApproved {auto: true, risk}` +
  `TokenIssued {auto: true}`. High-risk verbs still require explicit
  `intent approve`.
- Approval is refused only for failed dry-runs (`success_probability < 1`).
  Risk no longer blocks approval — approval *is* the human gate for risk.

## Grants file — three-layer subset check

`.imperium/grants.json` (written by `init`) declares per-capability
permissions:

```json
{ "grants": { "cap.write": { "fs": ["scratch"], "net": [], "env": [] } } }
```

Layering enforced fail-closed at load:

```
token permissions ⊆ declared (grants.json) ⊆ built-in defaults
```

- Unknown capabilities in the file → error.
- Any grant wider than its built-in default → error.
- Tokens are issued from the *declared* grant; execution re-verifies against
  the same file. Narrowing the file narrows live tokens immediately.

## WASM guest

`guest.wat` gains host imports `read`, `append`, `list` (and matching
`run_*` exports). `scripts/assemble-guest.mjs` encodes the binary directly —
it must first reproduce the historical echo+write module byte-for-byte
before emitting (validated at build time). Every host import enforces
rights, granted prefixes, and the sensitive-path firewall; each denial path
has a test (`wasm-host.test.ts`).
