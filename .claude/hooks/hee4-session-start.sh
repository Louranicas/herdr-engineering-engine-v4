#!/usr/bin/env bash
# hee4-session-start — SessionStart banner for HEE v4 (advisory; never blocks, never fails).
#
# Prints at most 12 lines of plain text. Claude Code adds SessionStart plain-text stdout to the
# session's context (hooks docs, "SessionStart Stdout Behavior"). Every external read is bounded
# by `timeout`; anything unreachable prints UNMEASURED, never a guess. No network, no Jev.
#
# Seams (so the tests can choose the world instead of arranging it, ~/CLAUDE.md F95):
#   HEE4_HOOK_HEE4DB    the hee4db binary            (default: <repo>/ops/db/hee4db)
#   HEE4_HOOK_HANDOFFS  the handoffs directory       (default: ~/handoffs)
#   HEE4_HOOK_REPO      the repo root                (default: $CLAUDE_PROJECT_DIR, else this file's ../..)
#   HEE4_HOOK_TMO       per-call timeout, seconds    (default: 0.4; three calls stay under 2 s)
set -u
trap 'exit 0' ERR
cat > /dev/null 2>&1 || true   # drain the hook JSON on stdin; the banner does not depend on it

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd) || exit 0
repo=${HEE4_HOOK_REPO:-${CLAUDE_PROJECT_DIR:-$(cd "$here/../.." 2>/dev/null && pwd)}}
db=${HEE4_HOOK_HEE4DB:-$repo/ops/db/hee4db}
handoffs=${HEE4_HOOK_HANDOFFS:-$HOME/handoffs}
tmo=${HEE4_HOOK_TMO:-0.4}

# The last `verdict=` line a hee4db verb writes to stderr (hee4db's contract: the last stderr line
# is the typed verdict line). Empty output, a timeout or a missing binary -> UNMEASURED.
verdict_of() {
  local out
  [ -x "$db" ] || { echo "UNMEASURED (hee4db not executable: $db)"; return; }
  out=$(timeout -k 0.2 "$tmo" "$db" "$@" 2>&1 >/dev/null < /dev/null | grep 'verdict=' | tail -n 1)
  [ -n "$out" ] && echo "${out:0:200}" || echo "UNMEASURED (no verdict line within ${tmo}s)"
}

# H-5 ("start coding") status, read from the ops DB row, never from this file.
h5="UNMEASURED"
if [ -x "$db" ]; then
  s=$(timeout -k 0.2 "$tmo" "$db" get held H-5 2>/dev/null < /dev/null | jq -r '.row.status // empty' 2>/dev/null)
  [ -n "$s" ] && h5=$s
fi

# The fence command, quoted from the repo CLAUDE.md (one home; not restated here).
fence=$(grep -m1 -o 'habitat-scope set[^`]*' "$repo/CLAUDE.md" 2>/dev/null)
[ -n "$fence" ] || fence="UNMEASURED (no habitat-scope line in $repo/CLAUDE.md)"

restart="$handoffs/HEE4_RESTART.md"
[ -f "$restart" ] && rs="$restart" || rs="$restart (ABSENT)"

newest=""
if [ -d "$handoffs" ]; then
  # Both nullglob traps (HEE4_HANDOVER_20261001_0900 "Traps"): without nullglob an empty glob is the
  # literal pattern; with it, a bare `ls` on zero matches lists the cwd. So: array, count, then ls.
  shopt -s nullglob; hs=("$handoffs"/HEE4_HANDOVER_*); shopt -u nullglob
  if [ "${#hs[@]}" -gt 0 ]; then newest=$(ls -1t -- "${hs[@]}" 2>/dev/null < /dev/null | head -n 1); fi
fi
[ -n "$newest" ] || newest="UNMEASURED (no HEE4_HANDOVER_* in $handoffs)"

case "$h5" in
  open) hold="HOLD open: H-5 \"start coding\" not given; no engine code (V4-0)." ;;
  UNMEASURED) hold="HOLD UNMEASURED: H-5 unreadable; treat as open (V4-0)." ;;
  *) hold="H-5 status=$h5 (read from hee4db; confirm with Luke's words before coding)." ;;
esac

cat <<EOF
HEE v4 session · $hold
fence: $fence
restart: $rs
newest handover: $newest
first: just verify   (one verdict over every check)
readiness: $(verdict_of readiness)
check: $(verdict_of check)
commands: /verify /restart /highway /module /regen /hee4-status /lessons · skills: hee4-module-slice hee4-brief hee4-lessons
EOF
exit 0
