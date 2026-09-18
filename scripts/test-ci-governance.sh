#!/usr/bin/env bash
# Phase 24 — governance-gate checks, fully local (file:// download path,
# debug binary, no network). Run from the repo root.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo build -p imperium-cli -q
BIN="$ROOT/target/debug/imperium-cli"
[ -x "$BIN" ] || { echo "missing $BIN" >&2; exit 1; }

FX="$(mktemp -d)"
trap 'rm -rf "$FX"' EXIT

# --- passing fixture: deny-glob leaves the tested path allowed ------------
mkdir -p "$FX/good"
printf 'deny read matching *.key\n' > "$FX/good/policy.imp"
printf '[{"verb":"read","path":"notes.txt","expect":"allow"}]\n' \
  > "$FX/good/policy.tests.json"
out="$(bash scripts/ci-governance.sh --binary "$BIN" --policy-dir "$FX/good")"
printf '%s\n' "$out" | grep -q "1 passed, 0 failed"

# --- failing fixture: entry expects deny, policy allows -------------------
mkdir -p "$FX/bad"
printf 'deny read matching *.key\n' > "$FX/bad/policy.imp"
printf '[{"verb":"read","path":"notes.txt","expect":"deny"}]\n' \
  > "$FX/bad/policy.tests.json"
if bash scripts/ci-governance.sh --binary "$BIN" --policy-dir "$FX/bad" \
    >/dev/null 2>&1; then
  echo "expected the failing fixture to exit 1" >&2
  exit 1
fi

# --- missing policy.tests.json must fail the gate -------------------------
mkdir -p "$FX/empty"
printf 'deny read matching *.key\n' > "$FX/empty/policy.imp"
if bash scripts/ci-governance.sh --binary "$BIN" --policy-dir "$FX/empty" \
    >/dev/null 2>&1; then
  echo "expected a missing policy.tests.json to fail" >&2
  exit 1
fi

# --- full download path via file:// (fetch + verify + gate) ---------------
TGT="$(rustc -vV | sed -n 's/^host: //p')"
STAGE="$FX/stage"
mkdir -p "$STAGE/bin"
cp "$BIN" "$STAGE/bin/imperium"
cp LICENSE README.md "$STAGE/"
tar -C "$STAGE" -czf "$FX/imperium-0.2.0-$TGT.tar.gz" .
(cd "$FX" && shasum -a 256 "imperium-0.2.0-$TGT.tar.gz" > SHA256SUMS)
out="$(bash scripts/ci-governance.sh \
  --from "file://$FX/imperium-0.2.0-$TGT.tar.gz" \
  --policy-dir "$FX/good")"
printf '%s\n' "$out" | grep -q "1 passed, 0 failed"

# --- checksum tamper must fail before any policy runs ---------------------
printf 'x' >> "$FX/imperium-0.2.0-$TGT.tar.gz"
if bash scripts/ci-governance.sh \
    --from "file://$FX/imperium-0.2.0-$TGT.tar.gz" \
    --policy-dir "$FX/good" >/dev/null 2>&1; then
  echo "expected a checksum mismatch to fail" >&2
  exit 1
fi

echo "ci-governance checks ok"
