# IMPERIUM Task Runner

default: help

help:
	@just --list

# --- Build ---
build-rust:
	cargo build --workspace --release

build-python:
	cd python && uv pip compile pyproject.toml -o requirements.txt && uv pip install -r requirements.txt

build-frontend:
	cd frontend && pnpm install && pnpm build

build-all: build-rust build-python build-frontend

# v0 slice (working product)
test-v0-js:
	cd web/v0 && node --experimental-strip-types --test src/*.test.ts

test-v0-rust:
	cargo test -p imperium-core --lib
	cargo test -p imperium-cli

test-v0: test-v0-js test-v0-rust

test-rust:
	cargo nextest run --workspace

test-python:
	cd python && uv run pytest -xvs

test-frontend:
	cd frontend && pnpm test

test-all: test-rust test-python test-frontend

# --- Lint/Format ---
fmt-rust:
	cargo fmt --all

fmt-python:
	cd python && uv run ruff format . && uv run ruff check --fix .

fmt-frontend:
	cd frontend && pnpm format

fmt-all: fmt-rust fmt-python fmt-frontend

lint-rust:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

lint-python:
	cd python && uv run ruff check . && uv run mypy .

lint-frontend:
	cd frontend && pnpm lint

lint-all: lint-rust lint-python lint-frontend

# --- Check (CI gate) ---
check: fmt-all lint-all test-all

# --- Run ---
run-daemon:
	cargo run --bin imperium-daemon -- --foreground

run-cli:
	cargo run --bin imperium-cli --

# --- Dev ---
dev-shell:
	nix develop

update-deps:
	cargo update
	cd python && uv pip compile --upgrade pyproject.toml -o requirements.txt
	cd frontend && pnpm update

# --- Benchmarks ---
bench-rust:
	cargo bench --workspace

bench-python:
	cd python && uv run pytest --benchmark-only

# --- Clean ---
clean:
	cargo clean
	rm -rf python/.venv frontend/node_modules frontend/dist
	rm -f requirements.txt

.PHONY: help build-rust build-python build-frontend build-all test-rust test-python test-frontend test-all fmt-rust fmt-python fmt-frontend fmt-all lint-rust lint-python lint-frontend lint-all check run-daemon run-cli dev-shell update-deps bench-rust bench-python clean