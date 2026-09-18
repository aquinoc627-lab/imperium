#!/usr/bin/env bash
# Phase 22/24 portability — sanity-check .gitlab-ci.yml locally:
#   1. parses as YAML
#   2. defines the expected jobs and stages
#   3. the release job is tag-gated
# No network, no gitlab-runner.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FILE="$ROOT/.gitlab-ci.yml"

parse_yaml() {
  if command -v ruby >/dev/null 2>&1; then
    ruby -ryaml -e 'YAML.safe_load(File.read(ARGV[0]), aliases: true); puts "yaml ok"' "$FILE"
  elif command -v python3 >/dev/null 2>&1; then
    python3 - "$FILE" <<'PY'
import sys
try:
    import yaml  # type: ignore
except ImportError:
    sys.exit(77)  # skip: no yaml parser
yaml.safe_load(open(sys.argv[1]))
print("yaml ok")
PY
  else
    echo "no ruby/python3 yaml parser available" >&2
    exit 77
  fi
}

out="$(parse_yaml)" || {
  if [ $? -eq 77 ]; then
    echo "skipped: no yaml parser"; exit 0
  fi
  echo ".gitlab-ci.yml does not parse" >&2; exit 1
}
printf '%s\n' "$out" | grep -q "yaml ok"

grep -q "js-kernel:" "$FILE"
grep -q "rust-gate:" "$FILE"
grep -q "release-linux:" "$FILE"
grep -q "stages:" "$FILE"
# The release job must be tag-gated, not branch-gated.
grep -q 'CI_COMMIT_TAG =~ /^v\[0-9\]' "$FILE"

# Every embedded shell block must be valid bash.
if command -v ruby >/dev/null 2>&1; then
  ruby -ryaml -e '
    YAML.safe_load(File.read(ARGV[0]), aliases: true)
      .select { |k, _| k != "stages" && k != "variables" && k != "cache" }
      .each do |name, job|
        Array(job["script"]).each do |s|
          IO.popen(["bash", "-n"], "r+") { |io| io.write(s); io.close_write }
          abort "bash -n failed for #{name}" unless $?.success?
        end
      end
  ' "$FILE"
fi

echo "gitlab-ci checks ok"
