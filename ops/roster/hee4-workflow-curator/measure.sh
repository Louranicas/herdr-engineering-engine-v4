#!/usr/bin/env bash
# hee4-workflow-curator measurements (code, not the agent). Arg: UTC stamp. v4 + habitat process paths only; no v3 path.
# F138: every section names what it looked at, and prints UNMEASURED (with the reason) when its source is absent or it
# looked at nothing; a count of 0 from an absent source is never printed.
set -uo pipefail
shopt -s nullglob
STAMP="${1:-UNSET}"
V4=${HEE4_ROOT:-/mnt/storage-10tb/herdr-engineering-engine-v4}
EV=${HEE4_EVIDENCE:-/mnt/storage-10tb/hee4-evidence}
ARENA=$HOME/fedora-arena
STORAGE=${HEE4_STORAGE:-/mnt/storage-10tb}
VAULT=${HEE4_VAULT:-$STORAGE/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault}
PL=$EV/learnings/PROCESS-LEARNINGS.md

# STORAGE is a nofail mount: an unmounted disk leaves an empty directory, which every count below would read as "0".
vault_ok=1; vault_why=""
if ! mountpoint -q "$STORAGE"; then vault_ok=0; vault_why="storage_not_mounted ($STORAGE)"
elif [ ! -d "$VAULT" ]; then vault_ok=0; vault_why="vault_absent ($VAULT)"; fi

echo "measured_at_utc=$STAMP"

if [ ! -d "$ARENA" ]; then echo "friction: UNMEASURED (absent $ARENA)"
elif ! command -v just >/dev/null 2>&1; then echo "friction: UNMEASURED (just not on PATH)"
else
  fr=$(cd "$ARENA" && timeout 120 just friction 2>/dev/null </dev/null | grep -E 'retry_rate|commands=' | tail -1)
  echo "friction: ${fr:-UNMEASURED (just friction printed no commands=/retry_rate line)}"
fi

# Runs reconcile per agent: runs (run-*.log) = verdicts (a typed verdict_line) + unverdicted (REFUSED, NONE, skipped, cut off).
echo "roster_runs:"
agents=("$EV"/roster/*/)
if [ ! -d "$EV/roster" ]; then echo "  UNMEASURED (absent $EV/roster)"
elif [ ${#agents[@]} -eq 0 ]; then echo "  UNMEASURED (no agent directories under $EV/roster)"
else
  for d in "${agents[@]}"; do a=$(basename "$d"); logs=("$d"run-*.log)
    s="0 (no skipped.log)"; [ -f "$d"skipped.log ] && s=$(wc -l < "$d"skipped.log)
    if [ ${#logs[@]} -eq 0 ]; then echo "  agent=$a runs=UNMEASURED (no run logs) skipped_log_lines=$s"; continue; fi
    m=0; types=""; unv=""; exits=""; deg=0
    for l in "${logs[@]}"; do
      vl=$(grep -m1 '^verdict_line=' "$l"); ex=$(grep -m1 -o -E '^exit=[0-9]+' "$l")
      [ -n "$ex" ] && exits+="${ex#exit=} "
      grep -q '^degraded_by=' "$l" && deg=$((deg + 1))
      if [[ "$vl" =~ ^verdict_line=[a-z0-9-]+\ verdict=(PASS_WITH_GAPS|PASS|FAIL)(\ |$) ]]; then m=$((m + 1)); types+="${BASH_REMATCH[1]} "
      elif [[ "$vl" =~ ^verdict_line=REFUSED\ ([a-z_]+) ]]; then unv+="REFUSED_${BASH_REMATCH[1]} "
      elif [ "$vl" = verdict_line=NONE ]; then unv+="NONE "
      elif [ -n "$vl" ]; then unv+="untyped_line "
      else unv+="no_verdict_line "; fi
    done
    tally() { local t; t=$(printf '%s' "$1" | tr ' ' '\n' | grep . | sort | uniq -c | awk '{printf "%s:%s ", $2, $1}'); printf '%s' "${t:-none}"; }
    echo "  agent=$a runs=${#logs[@]} verdicts=$m unverdicted=$(( ${#logs[@]} - m )) skipped_log_lines=$s degraded=$deg"
    echo "    verdicts: $(tally "$types")"
    echo "    unverdicted: $(tally "$unv")"
    echo "    exits (logs carrying exit=): $(tally "$exits")"
  done
fi

# A verification report's verdict is its TYPED verdict line, never its first PASS token. The convention in
# ~/hee4-evidence/verification: the first line that, after leading markdown (# * ` > space), reads `Verdict: X`,
# `verdict=X` or `<ID> verdict=X` with X in PASS|PASS_WITH_GAPS|FAIL. Anything else is UNTYPED.
vr=("$EV"/verification/*.md)
if [ ! -d "$EV/verification" ]; then echo "verification_reports=UNMEASURED (absent $EV/verification)"
elif [ ${#vr[@]} -eq 0 ]; then echo "verification_reports=UNMEASURED (no .md under $EV/verification)"
else
  echo "verification_reports=${#vr[@]}"
  python3 - "${vr[@]}" <<'PY'
import os, re, sys
TYPED = re.compile(r"^(?:[A-Z][A-Za-z0-9-]* )?[Vv]erdict(?::\s*|=)(PASS_WITH_GAPS|PASS|FAIL)(?![A-Za-z0-9_])")
for path in sys.argv[1:]:
    verdict = "UNTYPED"
    with open(path, encoding="utf-8", errors="replace") as fh:
        for n, line in enumerate(fh, 1):
            if n > 5000:
                break
            m = TYPED.match(line[:4096].lstrip("#*`> \t"))
            if m:
                verdict = "%s (line %d)" % (m.group(1), n)
                break
    print("  %s: %s" % (os.path.basename(path), verdict))
PY
fi

if [ -f "$V4/plan/DECISIONS.md" ]; then echo "decisions_v4=$(grep -c -E '^- \*\*V4-[0-9]+' "$V4/plan/DECISIONS.md")"
else echo "decisions_v4=UNMEASURED (absent $V4/plan/DECISIONS.md)"; fi
if [ -f "$PL" ]; then echo "process_learnings: loops=$(grep -c -E '^\| L[0-9]+ ' "$PL") keeps=$(grep -c -E '^\| K[0-9]+ ' "$PL")"
else echo "process_learnings: UNMEASURED (absent $PL)"; fi

echo "workflow_runs_last10:"
wn=0
while read -r w; do
  j="$w/journal.jsonl"; [ -f "$j" ] || continue; wn=$((wn + 1))
  echo "  $(basename "$w") agents=$(grep -c '"type":"result"' "$j") mtime=$(date -u -r "$j" +%Y-%m-%dT%H:%MZ)"
done < <(ls -dt "$HOME"/.claude/projects/*/*/subagents/workflows/wf_* 2>/dev/null | head -10)
[ $wn -gt 0 ] || echo "  UNMEASURED (no workflow journal found under ~/.claude/projects/*/*/subagents/workflows/)"

if [ $vault_ok = 0 ]; then
  echo "vault_70_workflows: UNMEASURED $vault_why"
  echo "loop_register_rows=UNMEASURED $vault_why"
else
  vw=$(ls "$VAULT/70 Workflows" 2>/dev/null | tr '\n' ';')
  echo "vault_70_workflows: ${vw:-UNMEASURED (empty or absent $VAULT/70 Workflows)}"
  # grep -c prints its own 0 (and exits 1) on no match; a `|| echo 0` there printed a second, stray line.
  if [ -f "$VAULT/70 Workflows/Loop Register.md" ]; then echo "loop_register_rows=$(grep -c -E '^\| L[0-9]+ ' "$VAULT/70 Workflows/Loop Register.md")"
  else echo "loop_register_rows=UNMEASURED (absent $VAULT/70 Workflows/Loop Register.md)"; fi
fi
# The L-ids the register must carry (R17): the agent fills cells for these and authors no rows of its own.
if [ -f "$PL" ]; then
  ids=$(grep -o -E '^\| L[0-9]+ ' "$PL" | tr -d '| ' | paste -sd, -)
  echo "learning_ids=${ids:-NONE} count=$(printf '%s' "$ids" | tr ',' '\n' | grep -c .)"
else echo "learning_ids=UNMEASURED (absent $PL)"; fi
