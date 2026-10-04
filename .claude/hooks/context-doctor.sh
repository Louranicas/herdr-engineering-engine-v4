#!/usr/bin/env bash
# context-doctor: can a fresh agent reach everything the repo tells it to read or run?
# Prints one line per item, present|MISSING, and ends with a verdict line. Never fails the session
# (exit 0 always): its job is to say what is missing before the agent discovers it by failing.
# Doctor-first (gates/features/README.md): orientation is a feature; a result against a missing home is not context.
set -u
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
[ -f "$ROOT/hee4.env" ] && . "$ROOT/hee4.env"
EV="${HEE4_EVIDENCE:-$HOME/hee4-evidence}"
VAULT="${HEE4_VAULT:-/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault}"
HAND="${HEE4_HANDOFFS:-$HOME/handoffs}"
BRAIN="${HEE4_BRAIN:-$ROOT/brain}"

ok=0; miss=0; out=""
item() { # label  test-expression...
  local label=$1; shift
  if "$@" >/dev/null 2>&1; then ok=$((ok+1)); out+="  present  $label"$'\n'
  else miss=$((miss+1)); out+="  MISSING  $label"$'\n'; fi
}
item "START.md (one-hop entry)"              test -f "$ROOT/START.md"
item "evidence home $EV"                     test -d "$EV/design"
item "ULTRAMAP / ATLAS"                      test -f "$EV/design/DEPLOYMENT_ATLAS.md"
item "restart pointer $HAND/HEE4_RESTART.md" test -f "$HAND/HEE4_RESTART.md"
item "vault $VAULT"                          test -d "$VAULT/16 System Maps"
item "feature map gates/features"            test -f "$ROOT/gates/features/README.md"
item "brain has notes beyond its README"     bash -c "ls '$BRAIN'/*.md 2>/dev/null | grep -v -e README.md -e index.md | grep -q ."
item "just on PATH"                          command -v just
item "hee4db on PATH"                        command -v hee4db
item "habitat-runbook on PATH"               command -v habitat-runbook
item "pstack router skill"                   test -f "$ROOT/.claude/skills/poteto-mode/SKILL.md"
item "cargo (for the walking skeleton)"      command -v cargo
item "local model endpoint (ollama :11434)"  bash -c "command -v curl >/dev/null && curl -s -m 1 http://127.0.0.1:11434/api/tags"

echo "context-doctor"
printf '%s' "$out"
v=PASS; [ "$miss" -eq 0 ] || v=GAPS
echo "context-doctor verdict=$v present=$ok missing=$miss  (MISSING lines are what to fix or route around before trusting any pointer)"
exit 0
