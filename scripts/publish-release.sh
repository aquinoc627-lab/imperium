#!/usr/bin/env bash
# Phase 22 fallback — publish release artifacts from a local machine.
# GitHub Releases are NOT blocked by an Actions billing lock; this script
# uploads dist/*.tar.gz + SHA256SUMS + provenance.json through the REST
# API instead of a workflow.
#
# Usage:
#   GH_PAT=<token> scripts/publish-release.sh [--repo OWNER/REPO] [--dry-run] VERSION
#
# Requires: curl, jq-free (pure sed/awk), a PAT with `repo` scope, and
# the v<VERSION> tag already pushed. Idempotent: existing assets with the
# same name are replaced via the upload API (it refuses duplicates, so we
# delete-then-upload).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

REPO="${REPO:-aquinoc627-lab/imperium}"
DRY=0
DIST="dist"
while [ $# -gt 0 ]; do
  case "$1" in
    --repo) REPO="$2"; shift 2 ;;
    --dist) DIST="$2"; shift 2 ;;
    --dry-run) DRY=1; shift ;;
    *) VERSION="${1#v}"; shift ;;
  esac
done

[ -n "${VERSION:-}" ] || { echo "usage: $0 VERSION" >&2; exit 1; }
[ -n "${GH_PAT:-}" ] && [ "$DRY" -eq 0 ] || {
  if [ "$DRY" -eq 1 ]; then GH_PAT="dry-run"; else
    echo "set GH_PAT (GitHub personal access token with repo scope)" >&2
    exit 1
  fi
}

[ -f "$DIST/SHA256SUMS" ] || { echo "no $DIST/SHA256SUMS; run make-release-artifacts first" >&2; exit 1; }
(cd "$DIST" && shasum -a 256 imperium-*.tar.gz > SHA256SUMS)
bash scripts/gen-provenance.sh --repo "$REPO" --release-dir "$DIST" "$VERSION" > "$DIST/provenance.json"

API="https://api.github.com/repos/$REPO"
AUTH="Authorization: Bearer $GH_PAT"

# Release object: reuse the tag's release or create one.
release_json=""
if [ "$DRY" -eq 0 ]; then
  release_json="$(curl -fsSL -H "$AUTH" "$API/releases/tags/v$VERSION" 2>/dev/null || true)"
fi
if [ -z "$release_json" ]; then
  echo "creating release v$VERSION"
  if [ "$DRY" -eq 0 ]; then
    body_file="$(mktemp)"
    printf '{"tag_name":"v%s","name":"v%s","body":"IMPERIUM v%s (published locally - see specs/22-release.md)"}' \
      "$VERSION" "$VERSION" "$VERSION" > "$body_file"
    release_json="$(curl -fsSL -X POST -H "$AUTH" "$API/releases" --data-binary @"$body_file")"
    rm -f "$body_file"
  fi
fi
release_id="$(printf '%s' "$release_json" | sed -n 's/.*"id": *\([0-9]*\).*/\1/p' | head -1)"
[ -n "$release_id" ] || [ "$DRY" -eq 1 ] || { echo "could not determine release id" >&2; exit 1; }

upload() {
  local name="$1" ctype="$2"
  if [ "$DRY" -eq 1 ]; then
    echo "dry-run: upload $DIST/$name -> release $release_id"
    return 0
  fi
  # Replace any existing asset with the same name.
  local assets
  assets="$(curl -fsSL -H "$AUTH" "$API/releases/$release_id/assets" \
    | sed -n "s/.*\"id\": *\([0-9]*\),.*\"name\": *\"$name\".*/\1/p" || true)"
  for id in $assets; do
    curl -fsSL -X DELETE -H "$AUTH" "$API/releases/assets/$id" >/dev/null
  done
  curl -fsSL -X POST -H "$AUTH" -H "Content-Type: $ctype" \
    --data-binary "@$DIST/$name" \
    "$API/releases/$release_id/assets?name=$name" >/dev/null
  echo "uploaded $name"
}

for tarball in "$DIST"/imperium-*.tar.gz; do
  upload "$(basename "$tarball")" "application/gzip"
done
upload "SHA256SUMS" "text/plain"
upload "provenance.json" "application/json"

count=$(ls "$DIST"/imperium-*.tar.gz | wc -l | tr -d " ")
echo "release v$VERSION published with $count tarball(s)"
