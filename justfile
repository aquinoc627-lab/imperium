# IMPERIUM Task Runner

default: help

help:
	@just --list

# One-shot after clone. Requires Node 22+ and Rust stable.
v0: test-v0-js test-v0-rust
	bash scripts/v0-smoke.sh

# --- v0 slice (working product) ---

test-v0-js:
	cd web/v0 && node --experimental-strip-types --test src/*.test.ts

test-v0-rust:
	cargo test -p imperium-core --lib
	cargo test -p imperium-cli

test-v0: test-v0-js test-v0-rust
	bash scripts/v0-smoke.sh

# --- Build ---
build-rust:
	cargo build -p imperium-core -p imperium-cli

build-python:
	cd python && uv pip compile pyproject.toml -o requirements.txt && uv pip install -r requirements.txt

build-frontend:
	cd frontend && pnpm install && pnpm build

# --- Other tests (scaffolding; expect gaps) ---
test-rust:
	cargo test -p imperium-core --lib
	cargo test -p imperium-cli

test-python:
	cd python && uv run pytest -xvs

test-frontend:
	cd frontend && pnpm test

# --- Lint/Format ---
fmt-rust:
	cargo fmt -p imperium-core -p imperium-cli

fmt-python:
	cd python && uv run ruff format . && uv run ruff check --fix .

fmt-frontend:
	cd frontend && pnpm format

lint-rust:
	cargo clippy -p imperium-core -p imperium-cli --all-targets -- -D warnings

lint-python:
	cd python && uv run ruff check . && uv run mypy .

lint-frontend:
	cd frontend && pnpm lint

# --- Run ---
run-cli:
	cargo run -p imperium-cli --

# --- Dev ---
dev-shell:
	nix develop

# --- Clean ---
clean:
	cargo clean
	rm -rf python/.venv frontend/node_modules frontend/dist .imperium
	rm -f requirements.txt
