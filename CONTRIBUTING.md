# Contributing to IMPERIUM

Read [AGENTS.md](AGENTS.md) and [specs/STATUS.md](specs/STATUS.md) before adding surface area.

After clone, the only required check is:

```bash
just v0
```

That is the product gate: TypeScript reference tests, Rust tests for `imperium-core` / `imperium-store` / `imperium-cli`, and `scripts/v0-smoke.sh`.

Do not add crates, UI pages, or spec `02`–`04` work until that stays green.

## Setup

Node 22+ and Rust stable (MSRV 1.78+):

```bash
just v0
```

Nix is optional (`nix develop`). Python is only needed if you touch `schemas/intent_ir.schema.json` or `python/` IR models. The `frontend/` tree is leftover scaffolding — do not develop against it. The browser slice is `web/workbench` plus the kernel in `web/v0`.

## Workflow

1. Branch from `master`.
2. Change only the working product unless a named spec says otherwise.
3. Keep `just v0` green.
4. Clippy the members:

```bash
cargo clippy -p imperium-core -p imperium-store -p imperium-cli --all-targets -- -D warnings
```

5. Open a PR against `master`.

## Coding standards

- No external network calls in tests. `cap.http` tests inject a fake transport.
- Deterministic tests. Monte Carlo uses the pinned LCG in the shared fixtures.
- No README or STATUS claim without a test.
- No silent no-op CLI verbs.
- Never commit `.imperium/` (runtime home, HMAC secret, grants, schedules).

## What not to do

- Do not implement voice, P2P, TPM, SLSA, Sigstore, OPA, or model-provider routing without a new named spec.
- Do not add an MCP `approve` tool.
- Do not treat `specs/02`–`04` as the next ticket.

## Security

Do not assume Sigstore, SLSA, or a security@ mailbox exist yet. Report issues via GitHub. The HMAC token secret is per-machine; if it is ever committed, rotate it and treat the old value as burned.

---

**IMPERIUM** — Intent. Made Law.
