#!/usr/bin/env bash
# Phase 22 — build and package release artifacts for the current platform.
#
# Usage:
#   scripts/make-release-artifacts.sh [--check] [--out DIR] [VERSION]
#
#   --check   run the Rust test gate before building (CI release gate)
#   --out DIR write tarballs + SHA256SUMS under DIR (default: dist/)
#   VERSION   release version; defaults to the CLI crate version; the
#             GitHub workflow passes the tag (v0.2.0 -> 0.2.0)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CHECK=0
OUT="$ROOT/dist"
while [ $# -gt 0 ]; do
  case "$1" in
    --check) CHECK=1; shift ;;
    --out) OUT="$2"; shift 2 ;;
    *) VERSION="${1#v}"; shift ;;
  esac
done

if [ -z "${VERSION:-}" ]; then
  VERSION="$(cargo metadata --no-deps --format-version 1 2>/dev/null \
    | sed -n 's/.*"name":"imperium-cli","version":"\([^"]*\)".*/\1/p')"
fi
[ -n "$VERSION" ] || { echo "cannot determine version" >&2; exit 1; }

if [ "$CHECK" -eq 1 ]; then
  cargo test -p imperium-core -p imperium-store -p imperium-cli -q
fi

TARGET="$(rustc -vV | sed -n 's/^host: //p')"
[ -n "$TARGET" ] || { echo "cannot determine rustc host triple" >&2; exit 1; }

cargo build -p imperium-cli --release

BIN="target/release/imperium-cli"
[ -x "$BIN" ] || { echo "missing binary: $BIN" >&2; exit 1; }

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
PKG="$STAGE/imperium-$VERSION-$TARGET"
mkdir -p "$PKG/bin" "$OUT"
cp "$BIN" "$PKG/bin/imperium"
cp LICENSE README.md "$PKG/"
tar -C "$PKG" -czf "$OUT/imperium-$VERSION-$TARGET.tar.gz" .

# The SHA256SUMS file covers every tarball found in OUT (multi-OS releases
# upload their parts before the final checksum pass).
(cd "$OUT" && shasum -a 256 imperium-*.tar.gz > SHA256SUMS)

echo "packaged $OUT/imperium-$VERSION-$TARGET.tar.gz"
