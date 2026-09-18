#!/usr/bin/env bash
# Phase 23 — machine-check the shipped MCP client configs:
#   1. every file parses as JSON
#   2. it points at `imperium mcp` (command + exact args) under the
#      host-appropriate key
#   3. no approval/secret verbs appear anywhere (the agent-facing surface
#      must stay approval-free — Phase 14 invariant)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIR="$ROOT/mcp/clients"

BANNED='approve|revoke|secret bind|secret rotate'
failures=0

check_file() {
  local file="$1" key="$2"
  if python3 - "$file" "$key" <<'PY'
import json, sys
file, key = sys.argv[1], sys.argv[2]
with open(file) as f:
    raw = f.read()
data = json.loads(raw)
servers = data.get(key)
assert servers is not None, f"{file}: missing key {key!r}"
imperium = servers.get("imperium")
assert imperium is not None, f"{file}: missing imperium server entry"
assert imperium.get("command") == "imperium", f"{file}: wrong command"
assert imperium.get("args") == ["mcp"], f"{file}: args must be exactly [mcp]"
print(f"{file}: ok ({key})")
PY
  then
    return 0
  else
    failures=$((failures + 1))
    return 1
  fi
}

for file in "$DIR"/*.json; do
  case "$(basename "$file")" in
    vscode.json) check_file "$file" servers || true ;;
    *) check_file "$file" mcpServers || true ;;
  esac
  if grep -Eiq "$BANNED" "$file"; then
    echo "$file: BANNED approval/secret verb present" >&2
    failures=$((failures + 1))
  fi
done

[ "$failures" -eq 0 ] || { echo "mcp client config checks failed" >&2; exit 1; }
echo "mcp client config checks ok"
