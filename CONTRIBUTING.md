# Contributing to IMPERIUM

Thank you for contributing to IMPERIUM — The Self-Synthesizing Intent Runtime.

## Code of Conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating, you agree to uphold this code.

Read [AGENTS.md](AGENTS.md) and [specs/v0-slice.md](specs/v0-slice.md) before adding surface area.

## Development Setup

### Using Nix (Recommended)

```bash
nix develop
```

### Manual Setup

```bash
rustup toolchain install stable
cargo install cargo-nextest

uv venv
uv pip install -e "./python[dev]"

cd frontend && pnpm install
```

## Workflow

### 1. Create an Issue

Before writing code, create an issue describing the problem, proposed solution, and any breaking changes.

### 2. Branch

```bash
git checkout -b feature/your-feature-name
```

### 3. Write Code

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
just test-all
# or
just test-rust
just test-python
just test-frontend
```

Contract fixtures: `tests/contract/intent_ir/`.

### 5. Commit

Use conventional commits:

```
feat(intent): add counterfactual query support
fix(simulation): handle empty world model gracefully
docs(readme): update quick start guide
```

### 6. Push and PR

```bash
git push origin feature/your-feature-name
```

Open a PR against `master`. Ensure CI passes.

## Coding Standards

- **No external network calls in tests**
- **Deterministic tests**
- **Structured logging** — `tracing` (Rust), `structlog` (Python)
- Public APIs need doc comments

### Rust
- MSRV: 1.78
- `anyhow::Result` for application code, `thiserror` for libraries

### Python
- Python 3.12+
- Type hints required
- `pydantic` for validation

## Testing Requirements

| Test Type | Location | When |
|-----------|----------|------|
| Unit | crate/package tests | Every PR |
| Contract | `tests/contract/` | Every PR |
| Integration | `tests/integration/` | When the v0 loop exists |

Chaos, fuzz, and nightly suites are **targets**, not present yet.

## Performance budgets (targets, not gates)

v0 (when implemented):

| Metric | Target |
|--------|--------|
| Rules compile | <100ms |
| Simulate (≤500 rollouts) | <2s |
| Execute echo | <1s |

Larger budgets in older drafts (10k rollouts, voice, vault) apply only after those features exist.

## Security

Do not assume Sigstore, SLSA, or a security@ mailbox exist yet. Report issues via GitHub.

---

**IMPERIUM** — We build the future by simulating it first.
