# IMPERIUM

> **The Self-Synthesizing Intent Runtime** (vision)
>
> Local. Sovereign. Absolute.

IMPERIUM is intended to compile natural-language goals into an executable Intent IR, simulate outcomes, and execute under capability constraints — locally.

**This repository is early scaffolding.** Most crates and UI surfaces are stubs. The only normative product right now is the [v0 slice](specs/v0-slice.md).

## Current status

| Area | Status |
|------|--------|
| Intent IR types (Rust + Python) | **Usable** — shared schema + contract fixtures |
| IR JSON schema | **Usable** — `schemas/intent_ir.schema.json` |
| Rules compiler (NL → IR) | Stub |
| LLM compiler | Stub (out of scope until Phase 3) |
| Simulation | Stub |
| Event store | Stub |
| Capability tokens / WASM host | Stub |
| CLI | Command surface exists; handlers are placeholders |
| Daemon / frontend / sync / voice / evolution | Scaffold only |
| Air-gap, SLSA, TPM, Sigstore | **Targets, not implemented** |

Contract tests live in `tests/contract/intent_ir/` (fixtures) and `python/tests/contract/`.

## Architecture (target)

```
NL → Intent IR → policy check → simulate → approve → execute → events
```

See `specs/00-architecture.md` and `specs/01-intent-ir.md`. Specs `02`–`04` are ahead of the code.

## Quick Start

### Prerequisites

- [Nix](https://nixos.org/download.html) with flakes enabled
- Or manually: Rust 1.78+, Python 3.12+, Node 20+, pnpm 9+

### Development

```bash
git clone https://github.com/aquinoc627-lab/imperium.git
cd imperium

nix develop   # optional hermetic shell

just check          # fmt + lint + test (CI gate; expect gaps)
just test-rust      # includes IR contract tests in imperium-core
just test-python    # includes schema/Pydantic contract tests
```

There is **no working `imperium init` / execute loop yet.** Phase 1 will add:
`compile → simulate → approve → execute → replay` for `Echo this message: <text>`.

## Workspaces

| Workspace | Language | Purpose |
|-----------|----------|---------|
| `crates/imperium-core` | Rust | Core types: Intent IR, Events, Capabilities, Policy, Crypto |
| `crates/imperium-runtime` | Rust | WASM host, capability manager, sandbox |
| `crates/imperium-store` | Rust | SQLite event store |
| `crates/imperium-sync` | Rust | P2P sync (not started) |
| `crates/imperium-policy` | Rust | Policy evaluation (not started) |
| `crates/imperium-cli` | Rust | CLI binary |
| `crates/imperium-daemon` | Rust | Background daemon |
| `crates/imperium-voice` | Rust | Voice bridge (not started) |
| `crates/imperium-crypto` | Rust | Signing / encryption |
| `crates/imperium-ffi` | Rust | C/FFI |
| `python/imperium_intent` | Python | Intent IR models + future compiler |
| `python/imperium_simulation` | Python | Simulation (stub) |
| `python/imperium_synthesis` | Python | Capability synthesizer (stub) |
| `python/imperium_evolution` | Python | Evolution loop (stub) |
| `frontend/workbench` | TypeScript | Workbench UI (mock data) |

## License

Business Source License 1.1 (BSL-1.1) — converts to Apache-2.0 after 4 years.

See [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md).

---

**IMPERIUM** — Intent. Made Law.
