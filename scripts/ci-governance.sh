#!/usr/bin/env bash
# Phase 24 — the semantic firewall as a CI gate.
#
# Modes:
#   scripts/ci-governance.sh --from URL --policy-dir DIR
#       Download the release tarball at URL, verify it against the
#       SHA256SUMS next to it, extract bin/imperium, run the gate.
#   scripts/ci-governance.sh --binary PATH --policy-dir DIR
#       Run the gate with an existing imperium binary.
#   scripts/ci-governance.sh --fetch-verify URL
#       Download + verify + extract only; print the binary path.
#
# The gate requires policy.tests.json (a governance job with nothing to
# test is a failure); policy.imp is optional (built-in defaults only).
# Exit 0 only when policy lint is clean and every test entry passes.
set -euo pipefail

MODE=""
FROM=""
BINARY=""
POLICY_DIR=""
while [ $# -gt 0 ]; do
  case "$1" in
    --from) MODE=from; FROM="$2"; shift 2 ;;
    --binary) MODE=binary; BINARY="$2"; shift 2 ;;
    --policy-dir) POLICY_DIR="$2"; shift 2 ;;
    --fetch-verify) MODE=fetch; FROM="$2"; shift 2 ;;
    *) shift ;;
  esac
done

fetch_verify() {
  local url="$1"
  local work
  work="$(mktemp -d)"
  local name
  name="$(basename "$url")"
  local base="${url%"$name"}"
  curl -fsSL "$url" -o "$work/$name"
  curl -fsSL "${base}SHA256SUMS" -o "$work/SHA256SUMS"
  local expected
  expected="$(awk -v f="$name" '$2 == f { print $1 }' "$work/SHA256SUMS")"
  if [ -z "$expected" ]; then
    echo "no SHA256SUMS entry for $name" >&2
    exit 1
  fi
  local actual
  actual="$(shasum -a 256 "$work/$name" | awk '{ print $1 }')"
  if [ "$expected" != "$actual" ]; then
    echo "checksum mismatch for $name: expected $expected got $actual" >&2
    exit 1
  fi
  tar -xzf "$work/$name" -C "$work"
  if [ ! -x "$work/bin/imperium" ]; then
    echo "tarball has no executable bin/imperium" >&2
    exit 1
  fi
  echo "$work/bin/imperium"
}

run_gate() {
  local bin="$1" dir="$2"
  if [ ! -f "$dir/policy.tests.json" ]; then
    echo "governance gate: no policy.tests.json in $dir" >&2
    exit 1
  fi
  local home
  home="$(mktemp -d)"
  IMPERIUM_HOME="$home" "$bin" init >/dev/null
  if [ -f "$dir/policy.imp" ]; then
    cp "$dir/policy.imp" "$home/policy.imp"
  fi
  cp "$dir/policy.tests.json" "$home/policy.tests.json"
  IMPERIUM_HOME="$home" "$bin" policy lint
  IMPERIUM_HOME="$home" "$bin" policy test
}

case "$MODE" in
  from)
    [ -n "${POLICY_DIR:-}" ] || { echo "usage: --from URL --policy-dir DIR" >&2; exit 1; }
    bin="$(fetch_verify "$FROM")"
    run_gate "$bin" "$POLICY_DIR"
    ;;
  binary)
    [ -n "${BINARY:-}" ] && [ -n "${POLICY_DIR:-}" ] || {
      echo "usage: --binary PATH --policy-dir DIR" >&2; exit 1; }
    run_gate "$BINARY" "$POLICY_DIR"
    ;;
  fetch)
    [ -n "${FROM:-}" ] || { echo "usage: --fetch-verify URL" >&2; exit 1; }
    fetch_verify "$FROM"
    ;;
  *)
    echo "usage: $0 (--from URL|--binary PATH) --policy-dir DIR | --fetch-verify URL" >&2
    exit 1
    ;;
esac
