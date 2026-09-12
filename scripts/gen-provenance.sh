#!/usr/bin/env bash
# Phase 22 — emit an unsigned SLSA Level 1 provenance statement (in-toto
# statement, slsaProvenance v0.2) for every tarball in the release dir.
#
# Honest by construction: this is provenance, not an attestation. It is
# NOT sigstore-signed; SLSA Levels 2-4 and signing stay deferred.
#
# Usage:
#   scripts/gen-provenance.sh [--repo OWNER/REPO] [--release-dir DIR] [VERSION]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

REPO="${REPO:-aquinoc627-lab/imperium}"
REL_DIR="$ROOT/dist"
BUILDER_ID="${BUILDER_ID:-local}"
BUILD_TYPE="${BUILD_TYPE:-https://github.com/$REPO/.github/workflows/release.yaml@v1}"
ENTRY_POINT="${ENTRY_POINT:-scripts/make-release-artifacts.sh}"
BUILD_INVOCATION_ID="${GITHUB_RUN_ID:-local}"
COMMIT="${GITHUB_SHA:-$(git rev-parse HEAD 2>/dev/null || echo unknown)}"

while [ $# -gt 0 ]; do
  case "$1" in
    --repo) REPO="$2"; shift 2 ;;
    --release-dir) REL_DIR="$2"; shift 2 ;;
    *) VERSION="${1#v}"; shift ;;
  esac
done

[ -n "${VERSION:-}" ] || { echo "usage: $0 VERSION" >&2; exit 1; }
[ -f "$REL_DIR/SHA256SUMS" ] || { echo "missing $REL_DIR/SHA256SUMS" >&2; exit 1; }

subjects=""
while read -r digest name; do
  [ -z "$digest" ] && continue
  subjects="$subjects$(cat <<EOF

      {
        "name": "$name",
        "digest": {"sha256": "$digest"}
      },
EOF
)"
done < "$REL_DIR/SHA256SUMS"
subjects="${subjects%,}"

cat <<EOF
{
  "_type": "https://in-toto.io/Statement/v0.1",
  "subject": [$subjects
  ],
  "predicateType": "https://slsa.dev/provenance/v0.2",
  "predicate": {
    "builder": {"id": "$BUILDER_ID"},
    "buildType": "$BUILD_TYPE",
    "invocation": {
      "configSource": {
        "uri": "https://github.com/$REPO",
        "digest": {"sha1": "$COMMIT"},
        "entryPoint": "$ENTRY_POINT"
      },
      "metadata": {"buildInvocationId": "$BUILD_INVOCATION_ID"}
    },
    "materials": [
      {
        "uri": "https://github.com/$REPO",
        "digest": {"sha1": "$COMMIT"}
      }
    ]
  }
}
EOF
