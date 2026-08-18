# Contributing to IMPERIUM

Thank you for contributing to IMPERIUM — The Self-Synthesizing Intent Runtime.

## Code of Conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating, you agree to uphold this code.

## Development Setup

### Using Nix (Recommended)

```bash
# Enter hermetic dev shell with all tools
nix develop

# Or use direnv for automatic shell loading
echo "use flake" > .envrc
direnv allow
```

### Manual Setup

```bash
# Rust
rustup toolchain install stable
cargo install cargo-nextest

# Python
uv venv
uv pip install -e "python[dev,ml,sim,synth]"

# Frontend
cd frontend && pnpm install
```

## Workflow

### 1. Create an Issue

Before writing code, create an issue describing:
- The problem or feature
- Proposed solution
- Any breaking changes

### 2. Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

### 3. Write Code

Follow the style guides for each language:

#### Rust
```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace
```

#### Python
```bash
cd python
uv run ruff format .
uv run ruff check --fix .
uv run mypy .
uv run pytest -xvs
```

#### TypeScript
```bash
cd frontend
pnpm format
pnpm lint
pnpm typecheck
pnpm test
```

### 4. Test

```bash
# Run all tests
just test-all

# Or individually
just test-rust
just test-python
just test-frontend
```

### 5. Benchmark (if performance-sensitive)

```bash
just bench-rust
just bench-python
```

### 6. Commit

Use conventional commits:

```
feat(intent): add counterfactual query support
fix(simulation): handle empty world model gracefully
docs(readme): update quick start guide
refactor(crypto): use ZeroizeOnDrop for keys
```

### 7. Push and PR

```bash
git push origin feature/your-feature-name
```

Open a PR against `main`. Ensure CI passes.

## Project Structure

```
imperium/
├── crates/           # Rust workspace
│   ├── imperium-core/      # Core types
│   ├── imperium-runtime/   # WASM host
│   ├── imperium-store/     # Event store
│   ├── imperium-sync/      # P2P sync
│   ├── imperium-policy/    # OPA engine
│   ├── imperium-cli/       # CLI
│   ├── imperium-daemon/    # Daemon
│   ├── imperium-voice/     # Voice bridge
│   ├── imperium-crypto/    # Crypto
│   └── imperium-ffi/       # FFI bindings
├── python/           # Python workspace
│   ├── imperium_intent/    # NL → IR
│   ├── imperium_simulation/# Monte Carlo
│   ├── imperium_synthesis/ # API → WASM
│   ├── imperium_evolution/ # Self-improvement
│   ├── imperium_router/    # Model routing
│   ├── imperium_memory/    # Vault engine
│   ├── imperium_voice/     # STT/TTS
│   ├── imperium_mcp/       # MCP tools
│   └── imperium_api/       # FastAPI server
├── frontend/         # TypeScript workspace
│   ├── packages/     # Shared packages
│   └── workbench/    # Main app
├── specs/            # Living specifications
├── benchmarks/       # Performance tests
├── tests/            # Integration tests
├── docker/           # Container images
├── installer/        # Platform installers
└── docs/             # Documentation
```

## Coding Standards

### General
- **No external network calls in tests** — use mocks, fixtures, local servers
- **Deterministic tests** — no flaky tests, fixed seeds for randomness
- **Structured logging** — use `tracing` (Rust), `structlog` (Python)
- **Error handling** — use `thiserror` (Rust), custom exceptions (Python), `Result` types
- **Documentation** — public APIs must have doc comments

### Rust
- Minimum supported Rust version (MSRV): 1.78
- Use `#[non_exhaustive]` for enums that may grow
- Prefer `anyhow::Result` for application code, `thiserror` for libraries
- `ZeroizeOnDrop` for all secrets
- `postcard` for binary serialization, `serde_json` for human-readable

### Python
- Python 3.12+ only
- Type hints required (`strict = true` in mypy)
- `pydantic` for validation and serialization
- `asyncio` for all I/O
- `pytest-asyncio` for async tests

### TypeScript
- Strict mode enabled
- No `any` — use `unknown` or proper types
- Functional components with hooks
- TanStack Query for server state
- Radix UI + Tailwind for components

## Testing Requirements

| Test Type | Location | When |
|-----------|----------|------|
| Unit | `tests/` in each crate/package | Every PR |
| Integration | `tests/integration/` | Every PR |
| Contract | `tests/contract/` | Every PR |
| Chaos | `tests/chaos/` | Nightly |
| Fuzz | `tests/fuzz/` | Nightly |
| Benchmark | `benchmarks/` | On performance changes |

## Performance Budgets

| Metric | Target |
|--------|--------|
| Cold start (binary → ready) | <3s |
| Intent compile (NL → IR) | <500ms |
| Simulation (10k rollouts) | <10s |
| Counterfactual query | <200ms |
| Capability synthesis (new API) | <30s |
| Voice wake word → response | <800ms |
| Vault search (10k notes) | <100ms |
| Binary size | <50MB |
| RAM baseline (idle) | <2GB |
| RAM full suite | <14GB |

## Security

- Report vulnerabilities to `security@imperium.dev` (PGP key in repo)
- All dependencies scanned with `cargo audit`, `pip-audit`, `pnpm audit`
- Sigstore signing for all releases
- SBOM generated for every build

## Release Process

1. Version bump in `Cargo.toml`, `pyproject.toml`, `package.json`
2. `CHANGELOG.md` updated
3. Tag: `git tag -s v0.x.y -m "v0.x.y"`
4. CI builds, signs, attests, publishes
5. GitHub Release created with artifacts

## Questions?

- Discussions: GitHub Discussions
- Chat: Discord (link in repo)
- Security: `security@imperium.dev`

---

**IMPERIUM** — We build the future by simulating it first.