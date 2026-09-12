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

# --- Phase 8: read / append / list / sensitive deny / grants ---

read_line="$("$BIN" intent compile --input "Read file notes.txt")"
read_id="$(printf '%s\n' "$read_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$read_id" | grep -q "read scratch/notes.txt"
# Low risk: executes without explicit approve (auto-approve, audited).
"$BIN" intent execute --intent-id "$read_id" | grep -q hello-from-v0

append_line="$("$BIN" intent compile --input "Append file log.txt with contents tail")"
append_id="$(printf '%s\n' "$append_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$append_id" | grep -q "append scratch/log.txt (4 bytes)"
"$BIN" intent approve --intent-id "$append_id" >/dev/null
"$BIN" intent execute --intent-id "$append_id" >/dev/null
grep -q tail "$IMPERIUM_HOME/scratch/log.txt"

dir_line="$("$BIN" intent compile --input "Write file notes_dir/x.txt with contents d")"
dir_id="$(printf '%s\n' "$dir_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$dir_id" >/dev/null
"$BIN" intent approve --intent-id "$dir_id" >/dev/null
"$BIN" intent execute --intent-id "$dir_id" >/dev/null
list_line="$("$BIN" intent compile --input "List files under notes_dir")"
list_id="$(printf '%s\n' "$list_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$list_id" | grep -q "list scratch/notes_dir"
"$BIN" intent execute --intent-id "$list_id" | grep -q x.txt

# Sensitive path: dry-run denies, approval is refused.
printf 'X=1' > "$IMPERIUM_HOME/scratch/.env"
sens_line="$("$BIN" intent compile --input "Read file .env")"
sens_id="$(printf '%s\n' "$sens_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$sens_id" | grep -q "DENIED cap.read scratch/.env: sensitive path denied"
if "$BIN" intent approve --intent-id "$sens_id" >/dev/null 2>&1; then
  echo "sensitive read should not be approvable" >&2
  exit 1
fi

# Grants fail-closed: widening beyond built-in defaults is rejected.
cp "$IMPERIUM_HOME/grants.json" "$IMPERIUM_HOME/grants.json.bak"
node -e '
  const fs = require("fs");
  const p = process.argv[1];
  const g = JSON.parse(fs.readFileSync(p, "utf8"));
  g.grants["cap.read"].fs.push("outside");
  fs.writeFileSync(p, JSON.stringify(g, null, 2));
' "$IMPERIUM_HOME/grants.json"
if "$BIN" intent approve --intent-id "$read_id" >/dev/null 2>&1; then
  echo "grants exceeding defaults should be rejected" >&2
  exit 1
fi
mv "$IMPERIUM_HOME/grants.json.bak" "$IMPERIUM_HOME/grants.json"

# --- Phase 9: the semantic firewall ---

printf 'deny read matching scratch/notes*\n' > "$IMPERIUM_HOME/policy.imp"
"$BIN" policy lint | grep -q "^ok ("
"$BIN" policy explain --verb read --path scratch/notes.txt | grep -q "DENY rule 0"
pol_line="$("$BIN" intent compile --input "Read file notes.txt")"
pol_id="$(printf '%s\n' "$pol_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$pol_id" | grep -q "policy: deny read matching scratch/notes\*"
if "$BIN" intent approve --intent-id "$pol_id" >/dev/null 2>&1; then
  echo "policy-denied read should not be approvable" >&2
  exit 1
fi

# A path outside the deny glob is allowed and its decision is audited.
pol_ok_line="$("$BIN" intent compile --input "Read file log.txt")"
pol_ok_id="$(printf '%s\n' "$pol_ok_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$pol_ok_id" >/dev/null
"$BIN" intent execute --intent-id "$pol_ok_id" | grep -q tail
"$BIN" intent replay --intent-id "$pol_ok_id" >/dev/null
grep -q PolicyEvaluated "$IMPERIUM_HOME/intents/$pol_ok_id.json"
rm "$IMPERIUM_HOME/policy.imp"

# --- Phase 10: the ledger ---

# Friction: the third semantically identical compile emits FrictionDetected.
"$BIN" intent compile --input "Echo this message: ping" >/dev/null
"$BIN" intent compile --input "say ping" --propose >/dev/null
frict_line="$("$BIN" intent compile --input "Echo this message: ping")"
frict_id="$(printf '%s\n' "$frict_line" | awk '{print $1}')"
grep -q FrictionDetected "$IMPERIUM_HOME/intents/$frict_id.json"
grep -q "reusable form" "$IMPERIUM_HOME/intents/$frict_id.json"

# Full trace.
show_out="$("$BIN" show --intent-id "$write_id")"
printf '%s\n' "$show_out" | grep -q "^events:"

# Ledger stats + rebuildability: destroy the projection, rebuild, search.
stats_out="$("$BIN" ledger stats)"
printf '%s\n' "$stats_out" | grep -q "^intents: "
rebuilt_out="$("$BIN" ledger rebuild)"
printf '%s\n' "$rebuilt_out" | grep -q "^rebuilt [0-9]* intents"
rm -f "$IMPERIUM_HOME/ledger.db"
rebuilt_out="$("$BIN" ledger rebuild)"
printf '%s\n' "$rebuilt_out" | grep -q "^rebuilt [0-9]* intents"
search_out="$("$BIN" search hello)"
printf '%s\n' "$search_out" | grep -q "$write_id"
search_out="$("$BIN" search ping)"
printf '%s\n' "$search_out" | grep -q "$frict_id"

# --- Phase 11: synthesis + scoped network ---

# Register + approve a capability manifest from the sample OpenAPI spec.
cap_out="$("$BIN" capabilities add --spec tests/contract/openapi/sample.json --name example_api)"
printf '%s\n' "$cap_out" | grep -q "hosts=api.example.com"
printf '%s\n' "$cap_out" | grep -q "unapproved"
"$BIN" capabilities approve --name example_api >/dev/null
cap_list="$("$BIN" capabilities list)"
printf '%s\n' "$cap_list" | grep -q "example_api	approved"
printf '%s\n' "$cap_list" | grep -q "2 GET"

# Fetch compiles and previews, but network is default-deny without policy.
fetch_line="$("$BIN" intent compile --input "Fetch https://api.example.com/v1/things")"
fetch_id="$(printf '%s\n' "$fetch_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$fetch_id" | grep -q "no allow fetch rule matched"
if "$BIN" intent approve --intent-id "$fetch_id" >/dev/null 2>&1; then
  echo "fetch without policy allowlist should not be approvable" >&2
  exit 1
fi

# With an explicit host allowlist the preview is permitted (no live network
# in smoke; execution is transport-injected and covered by unit tests).
printf 'allow fetch to api.example.com\n' > "$IMPERIUM_HOME/policy.imp"
lint_out="$("$BIN" policy lint)"
printf '%s\n' "$lint_out" | grep -q "^ok ("
"$BIN" intent simulate --intent-id "$fetch_id" | grep -q "fetch https://api.example.com/v1/things"
rm "$IMPERIUM_HOME/policy.imp"

# --- Phase 12: probabilistic dry-run ---

mc_line="$("$BIN" intent compile --input "Write file mc.txt with contents x")"
mc_id="$(printf '%s\n' "$mc_line" | awk '{print $1}')"
mc_out="$("$BIN" intent simulate --intent-id "$mc_id" --trials 500)"
printf '%s\n' "$mc_out" | grep -q "p_success="
printf '%s\n' "$mc_out" | grep -q "P(cap.write)"
world_out="$("$BIN" world)"
printf '%s\n' "$world_out" | grep -q "cap.write"

# --- Phase 13: the evolution loop ---

# Friction now fires on intents that differ only in content.
"$BIN" intent compile --input "Echo this message: red" >/dev/null
"$BIN" intent compile --input "Echo this message: green" >/dev/null
slot_line="$("$BIN" intent compile --input "Echo this message: blue")"
slot_id="$(printf '%s\n' "$slot_line" | awk '{print $1}')"
grep -q FrictionDetected "$IMPERIUM_HOME/intents/$slot_id.json"

# Save a form, run it through the gauntlet.
"$BIN" forms save --intent-id "$echo_id" --name echo_form | grep -q "{{slot}}"
form_line="$("$BIN" forms run --name echo_form --slot from-the-form)"
form_id="$(printf '%s\n' "$form_line" | awk 'NR==1 {print $1}')"
printf '%s\n' "$form_line" | grep -q from-the-form
"$BIN" intent approve --intent-id "$form_id" >/dev/null
"$BIN" intent execute --intent-id "$form_id" | grep -q from-the-form

# Shadow: promise verified against the redirected reality; status unchanged.
sh_line="$("$BIN" intent compile --input "Write file shadowed.txt with contents probe")"
sh_id="$(printf '%s\n' "$sh_line" | awk '{print $1}')"
"$BIN" intent simulate --intent-id "$sh_id" >/dev/null
"$BIN" intent approve --intent-id "$sh_id" >/dev/null
"$BIN" intent execute --shadow --intent-id "$sh_id" | grep -q "scratch/shadow/shadowed.txt"
test -f "$IMPERIUM_HOME/scratch/shadow/shadowed.txt"
test ! -f "$IMPERIUM_HOME/scratch/shadowed.txt"
grep -q '"match": true' "$IMPERIUM_HOME/intents/$sh_id.json"

forms_out="$("$BIN" forms list)"
printf '%s\n' "$forms_out" | grep -q "echo_form"

echo "v0 smoke ok  echo=$echo_id  write=$write_id"
