#!/usr/bin/env bash
# Phase 22 — emit a Homebrew formula for the owner's tap repo.
#
# Usage:
#   scripts/gen-homebrew-formula.sh [--repo OWNER/REPO] [--tap-owner OWNER] \
#       [--release-dir DIR] [--base-url URL] VERSION
#
# Reads SHA256SUMS from --release-dir (default: dist/) and prints
# imperium.rb. Deterministic for a fixed release directory.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

REPO="${REPO:-aquinoc627-lab/imperium}"
TAP_OWNER="${TAP_OWNER:-aquinoc627-lab}"
REL_DIR="$ROOT/dist"
BASE_URL=""
while [ $# -gt 0 ]; do
  case "$1" in
    --repo) REPO="$2"; shift 2 ;;
    --tap-owner) TAP_OWNER="$2"; shift 2 ;;
    --release-dir) REL_DIR="$2"; shift 2 ;;
    --base-url) BASE_URL="$2"; shift 2 ;;
    *) VERSION="$1"; shift ;;
  esac
done

[ -n "${VERSION:-}" ] || { echo "usage: $0 VERSION" >&2; exit 1; }
[ -f "$REL_DIR/SHA256SUMS" ] || { echo "missing $REL_DIR/SHA256SUMS" >&2; exit 1; }

SHA256SUMS="$REL_DIR/SHA256SUMS"
BASE_URL="${BASE_URL:-https://github.com/$REPO/releases/download/v$VERSION}"

sha() {
  awk -v f="imperium-$VERSION-$1.tar.gz" '$2 == f { print $1 }' "$SHA256SUMS" \
    | grep -E '^[0-9a-f]{64}$' || { echo "no checksum for $1" >&2; exit 1; }
}

# Fail closed before emitting anything: every platform must have a checksum.
for tgt in x86_64-unknown-linux-gnu aarch64-apple-darwin x86_64-apple-darwin; do
  sha "$tgt" >/dev/null
done

cat <<EOF
class Imperium < Formula
  desc "Local intent runtime: compile, simulate, approve, execute"
  homepage "https://github.com/$REPO"
  url "$BASE_URL/imperium-$VERSION-x86_64-unknown-linux-gnu.tar.gz"
  sha256 "$(sha x86_64-unknown-linux-gnu)"
  license "BSL-1.1"
  version "$VERSION"

  on_macos do
    if Hardware::CPU.arm?
      url "$BASE_URL/imperium-$VERSION-aarch64-apple-darwin.tar.gz"
      sha256 "$(sha aarch64-apple-darwin)"
    else
      url "$BASE_URL/imperium-$VERSION-x86_64-apple-darwin.tar.gz"
      sha256 "$(sha x86_64-apple-darwin)"
    end
  end

  def install
    bin.install "bin/imperium"
  end

  test do
    system "#{bin}/imperium", "--version"
  end
end
EOF
