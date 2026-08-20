# IMPERIUM

> Local. Sovereign. Absolute.

IMPERIUM compiles natural-language goals into an Intent IR, simulates, issues a capability token, and executes under those rights.

**The working product is [`web/v0`](web/v0).** Most Rust crates and the mock workbench are still scaffolding. See [`specs/STATUS.md`](specs/STATUS.md).

## Run the v0 slice

Node 22+ (kernel tests):

```bash
cd web/v0
npm test
```

Rust CLI (same loop, local `.imperium/` store):

```bash
cargo test -p imperium-core --lib
cargo run -p imperium-cli -- init
cargo run -p imperium-cli -- intent compile --input "Echo this message: ping"
cargo run -p imperium-cli -- intent simulate --intent-id <id>
cargo run -p imperium-cli -- intent approve --intent-id <id>
cargo run -p imperium-cli -- intent execute --intent-id <id>
cargo run -p imperium-cli -- intent replay --intent-id <id>
```

`Write file ../secret with contents x` is rejected. Network is deny-all. Execute without approve, or after revoke, fails.

Canonical intents:

```
Echo this message: ping
Write file notes.txt with contents hello
```

`Write file ../secret with contents x` is rejected. Network is deny-all.

## Status

| Area | Status |
|------|--------|
| v0 loop (`web/v0` + `imperium-cli`) | **Working** — phases 1–6 |
| Intent IR types (Rust + Python) | Usable — shared schema + fixtures |
| Rules compiler / tokens / WASM / replay | **Working in `web/v0`**, not yet in Rust |
| CLI / daemon / frontend workbench | Scaffold or mock |
| Air-gap, SLSA, TPM, Sigstore | Targets, not implemented |

## Architecture

```
NL → propose? → rules IR → simulate → approve (token) → WASM execute → fold events
```

See `specs/00-architecture.md`, `specs/01-intent-ir.md`, `specs/v0-slice.md`. Specs `02`–`04` are ahead of the code — do not implement them yet.

## License

Business Source License 1.1 (BSL-1.1) — converts to Apache-2.0 after 4 years.

See [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md). New work must keep `web/v0` tests green and must not add unused crates.

---

**IMPERIUM** — Intent. Made Law.
