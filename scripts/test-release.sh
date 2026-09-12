#!/usr/bin/env bash
# Phase 22 — release-script checks against a fabricated fixture release dir.
# No cargo build, no network; deterministic. Run from the repo root.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FX="$(mktemp -d)"
trap 'rm -rf "$FX"' EXIT

VERSION="9.9.9"
for tgt in x86_64-unknown-linux-gnu aarch64-apple-darwin x86_64-apple-darwin; do
  tar -C "$FX" -czf "$FX/imperium-$VERSION-$tgt.tar.gz" --files-from /dev/null \
    || true
  printf '%064d' 0 > "$FX/digest"
done
(cd "$FX" && shasum -a 256 imperium-*.tar.gz > SHA256SUMS)

# Formula: deterministic, contains every checksum, valid ruby keywords.
FORMULA="$(bash scripts/gen-homebrew-formula.sh --release-dir "$FX" "$VERSION")"
printf '%s\n' "$FORMULA" | grep -q "class Imperium < Formula"
printf '%s\n' "$FORMULA" | grep -q "url .*releases/download/v$VERSION/"
while read -r digest name; do
  [ -z "$digest" ] && continue
  printf '%s\n' "$FORMULA" | grep -q "$digest" || {
    echo "formula missing checksum for $name" >&2; exit 1; }
done < "$FX/SHA256SUMS"

# Provenance: valid JSON, one subject per tarball, unsigned SLSA level.
PROV="$(bash scripts/gen-provenance.sh --release-dir "$FX" "$VERSION")"
printf '%s\n' "$PROV" | python3 -c '
import json, sys
s = json.load(sys.stdin)
assert s["_type"] == "https://in-toto.io/Statement/v0.1"
assert s["predicateType"] == "https://slsa.dev/provenance/v0.2"
assert len(s["subject"]) == 3
assert all("sha256" in x["digest"] for x in s["subject"])
print("provenance ok")
'

echo "release-script checks ok"
