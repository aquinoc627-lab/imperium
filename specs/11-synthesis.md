# Phase 11 — Synthesis + scoped network (normative)

Implemented canonically in `imperium-core` (`v0.rs`, `policy::imp`,
`synth.rs`) + `imperium-cli`, mirrored by the `web/v0` TS reference. Shared
fixtures in `tests/contract/v0_kernel.json` pin compile errors, previews,
and policy decisions for every case below.

## `cap.http` — the first network capability

Canonical form (rules compiler only):

```
Fetch <https-url>
```

- **The model can never propose network access.** The proposer
  (rules/local/model) has no fetch form; the built-in content ceiling
  (`https:` is banned content) blocks loose phrasings too. Only the user
  typing the canonical form creates a fetch intent.
- **URL hardening at compile time** (fail-closed): https only, no explicit
  port, no userinfo, no IP literals, no localhost/`.local`/`.internal`
  hosts. Reasons (fixture-pinned): `https required`, `port not allowed`,
  `ip addresses denied`, `local address denied`, `invalid url`.
- **Risk 0.6** → `requires_approval` always true. Fetch is never
  auto-approved: explicit human approval at every use.
- **Preview gate (dry-run)**: fetch is **default-deny** — without an
  explicit `allow fetch` policy rule the preview is
  `DENIED cap.http <url>: no allow fetch rule matched`, and denied
  dry-runs cannot be approved.

## Network layering (each layer can only deny more, never less)

1. **URL hardening** (compile + execute, hard-coded).
2. **Policy allowlist** — `allow fetch to <host>` (glob over the hostname,
   so `*.example.com` works; `deny fetch matching <glob>` wins by file
   order). No policy ⇒ deny. Fall-through ⇒ deny.
3. **Grants** — `grants.json` declares `cap.http: net: [<hosts>]`. The
   built-in ceiling for net is `["*"]` (any explicitly declared host is
   declarable); `init` writes **closed** net (`[]`) — nothing is fetchable
   out of the box. Token ⊆ declared ⊆ ceiling, fail-closed.
4. **Token floor** — non-`cap.http` tokens can never carry net permissions
   (`network denied`).
5. **Execution re-checks** everything (URL checks, token host, policy
   decision) independently of the dry-run; every fetch records a
   `PolicyEvaluated` event with the host and the decision.

`ALLOW_CLOUD_ROUTING` remains a compile-time `false` constant — it gates
model-provider routing (none exists in the kernel); `cap.http` is
user-declared capability traffic, not model routing.

## Execution transport

`HttpTransport` is injectable; the CLI wires `ureq` (rustls) — tests inject
a fake and **never touch the network**. Non-2xx → `TaskFailed` with reason
`http status <code>`; 2xx → the body becomes the intent output.

## Capability synthesis (`imperium-core::synth`)

`synthesize(name, openapi_json)` compiles an OpenAPI 3 document into a
`CapabilityManifest`:

- `hosts`: from the spec's https `servers` (invalid/local entries skipped;
  a spec with no usable https host cannot be synthesized).
- `operations`: every declared method+path, ids from `operationId` or a
  deterministic fallback; sorted.
- `wit`: deterministic generated interface text (`package imperium:cap;`).
- `source_hash`: BLAKE3 of the spec text. Same input + name ⇒ same manifest.

**Property-tested** (seeded LCG, 25 generated specs, no network): hosts come
only from the spec's https servers; operations come only from the spec;
WIT names the interface and every operation id; manifests round-trip
through JSON.

## Registry (CLI)

```
imperium capabilities add --spec <openapi.json> --name <kebab-name>
imperium capabilities approve --name <name>
imperium capabilities list
```

- Registration records the manifest **unapproved**; duplicates are refused.
- `approve` is an explicit human act; it stamps `approved_at`.
- **A manifest grants nothing.** Execution stays gated by policy + grants +
  approval; the registry documents the API surface and its hosts.

## Upgrading note

Existing `.imperium` homes have grants.json without `cap.http`; delete it
and run `imperium init` (or add a `cap.http` entry with `net: []`) to get
the closed-network baseline.
