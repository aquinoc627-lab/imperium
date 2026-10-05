#!/usr/bin/env bash
# End-to-end CLI smoke: echo + write + fail-closed escape.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v cargo >/dev/null; then
  echo "need cargo (Rust stable)" >&2
  exit 1
fi

cargo build -p imperium-cli -q
BIN="$ROOT/target/debug/imperium-cli"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
export IMPERIUM_HOME="$WORK/home"

"$BIN" init

# Phase 25: one-command first-run path
demo_out="$("$BIN" demo)"
printf '%s\n' "$demo_out" | grep -q "demo ok"
printf '%s\n' "$demo_out" | grep -q ping

echo_line="$("$BIN" intent compile --input "Echo this message: ping")"
echo_id="$(printf '%s\n' "$echo_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$echo_id"
"$BIN" intent approve --intent-id "$echo_id"
echo_out="$("$BIN" intent execute --intent-id "$echo_id")"
printf '%s\n' "$echo_out" | grep -q ping
"$BIN" intent replay --intent-id "$echo_id" >/dev/null

# Multibyte echo: must compile (no truncation panic) and keep every char.
mb_line="$("$BIN" intent compile --input "Echo this message: ああああああああああああああ")"
printf '%s\n' "$mb_line" | grep -q "Echo ああああああああああああああ"

write_line="$("$BIN" intent compile --input "Write file notes.txt with contents hello-from-v0")"
write_id="$(printf '%s\n' "$write_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$write_id" --json | grep -q effects_preview
sim_out="$("$BIN" intent simulate --intent-id "$write_id")"
printf '%s\n' "$sim_out" | grep -q "write scratch/notes.txt ([0-9]* bytes)"
"$BIN" intent approve --intent-id "$write_id" | grep -q "write scratch/notes.txt"
"$BIN" intent execute --intent-id "$write_id" >/dev/null
test -f "$IMPERIUM_HOME/scratch/notes.txt"
grep -q hello-from-v0 "$IMPERIUM_HOME/scratch/notes.txt"

if "$BIN" intent compile --input "Write file ../secret with contents x" >/dev/null 2>&1; then
  echo "path escape should have been denied" >&2
  exit 1
fi

echo "v0-smoke: ok"
