# IMPERIUM

> Local. Sovereign. Absolute.

IMPERIUM compiles natural-language goals into an Intent IR, simulates, issues a capability token, and executes under those rights.

**The working product is [`web/v0`](web/v0) plus `imperium-cli`.** Most other crates and the mock workbench are still scaffolding. See [`specs/STATUS.md`](specs/STATUS.md).

## After clone

Node 22+ and Rust stable:

```bash
just v0
```

That runs the JS kernel tests, the Rust unit/integration tests, and a CLI smoke (echo + write to scratch + path-escape deny).

Without `just`:

```bash
cd web/v0 && node --experimental-strip-types --test src/*.test.ts
cargo test -p imperium-core --lib
cargo test -p imperium-cli
bash scripts/v0-smoke.sh
```

## Manual CLI

```bash
cargo run -p imperium-cli -- init
cargo run -p imperium-cli -- intent compile --input "Echo this message: ping"
cargo run -p imperium-cli -- intent simulate --intent-id <id>
cargo run -p imperium-cli -- intent approve --intent-id <id>
cargo run -p imperium-cli -- intent execute --intent-id <id>
cargo run -p imperium-cli -- intent replay --intent-id <id>
```

Canonical intents:

```
Echo this message: ping
Write file notes.txt with contents hello
```

`Write file ../secret with contents x` is rejected. Network is deny-all. Execute without approve, or after revoke, fails.

## Status

| Area | Status |
|------|--------|
| v0 loop (`web/v0` + `imperium-cli`) | **Working** — `just v0` |
| Intent IR types (Rust + Python) | Usable — shared schema + fixtures |
| Rules compiler / tokens / replay | **Working** in `web/v0` and `imperium-cli` |
| WASM guest | **Working** in `web/v0`; CLI uses the same host rules |
| Daemon / frontend workbench / unused crates | Scaffold or mock |
| Air-gap, SLSA, TPM, Sigstore | Targets, not implemented |

## Architecture

```
NL → propose? → rules IR → simulate → approve (token) → execute → fold events
```

See `specs/00-architecture.md`, `specs/01-intent-ir.md`, `specs/v0-slice.md`. Specs `02`–`04` are ahead of the code — do not implement them yet.

## License

Business Source License 1.1 (BSL-1.1) — converts to Apache-2.0 after 4 years.

See [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md). New work must keep `just v0` green and must not add unused crates.

---

**IMPERIUM** — Intent. Made Law.
