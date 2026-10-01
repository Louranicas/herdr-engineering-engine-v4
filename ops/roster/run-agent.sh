#!/usr/bin/env bash
# HEE v4 agent-roster runner — ONE door for every roster agent (AP-01).
# Usage: run-agent.sh <agent> light|deep|selfcheck
# Code measures (the agent's measure.sh), the agent curates, the exit code is the agent's typed verdict:
#   0  PASS — the report's last line is a typed PASS AND the run was clean (claude_rc=0, is_error=False, cost measured)
#   10 PASS_WITH_GAPS — a typed PASS_WITH_GAPS, or a typed PASS (or PASS_WITH_GAPS) degraded (`degraded_by=` in the log)
#      by an unclean run (claude_rc≠0, is_error≠False, cost UNMEASURED) or by a measurement floor (`degraded_by=measured_*`):
#      v3_refs>0 (measured_v3_refs=N), `funnel: verdict=FAIL` (measured_funnel_fail), or any line of measure.sh's output
#      containing UNMEASURED (measured_unmeasured_lines=N). Code measures, the agent curates: a red floor outranks its PASS.
#   20 FAIL — a typed FAIL
#   30 REFUSED (UNMEASURED) — no report, or its last non-empty line is not `<agent-without-hee4-> verdict=PASS|PASS_WITH_GAPS|FAIL`
#      followed by a space or end of line; also any run claude ended with subtype error_max_budget_usd (cut off mid-run)
#   40 SKIPPED — another run of this agent holds the lock (written to skipped.log and to this run's log)
#   2 usage · 3 setup
# Every log that reaches the claude step carries `exit=N measure_rc=M` (M = measure.sh's own exit status).
# After that line the run is recorded in the ops DB (CN-04): `hee4db record run --log` and `record measurement --file`,
# then ONE line `db_record=ok|failed rc=N run_rc=R measurement_rc=M` (rc 0 or 10 from hee4db counts as ok; their
# output goes to db-<stamp>-<mode>.txt). A DB failure never changes the exit code; the daily `ops/db/daily.sh`
# backfills any run whose record failed (`--from-logs`, idempotent by log path and digest).
# v4 only: reads no v3 path (V4-9).
set -uo pipefail
AGENT="${1:-}"; MODE="${2:-light}"
V4=/var/home/Louranicas/herdr-engineering-engine-v4
LOGDIR=/var/home/Louranicas/hee4-evidence/roster/$AGENT
CLAUDE=/var/home/Louranicas/.local/bin/claude
ROSTER=$V4/ops/roster
DIR=$ROSTER/$AGENT
[[ "$AGENT" =~ ^[a-z0-9-]+$ ]] && [ -f "$DIR/modes.conf" ] && [ -x "$DIR/measure.sh" ] && [ -f "$DIR/settings.json" ] && [ -f "$V4/.claude/agents/$AGENT.md" ] || { echo "usage: $0 <agent> light|deep|selfcheck (agent is [a-z0-9-]+ and needs ops/roster/<agent>/{modes.conf,measure.sh,settings.json} and .claude/agents/<agent>.md)" >&2; exit 2; }
NAME=${AGENT#hee4-}
mkdir -p "$LOGDIR" || exit 3
STAMP=$(date -u +%Y%m%dT%H%M%SZ)
LOG="$LOGDIR/run-$STAMP-$MODE.log"; MEAS="$LOGDIR/measure-$STAMP-$MODE.txt"; REPORT="$LOGDIR/report-$STAMP-$MODE.md"
MRC=NA
HEE4DB=/var/home/Louranicas/herdr-engineering-engine-v4/ops/db/hee4db
# The DB step runs in this shell's error-tolerant mode (no set -e) and its status is only ever logged.
db_record() {
  local out="$LOGDIR/db-$STAMP-$MODE.txt" rrc mrc=skipped_no_measure_file worst=0
  "$HEE4DB" record run --log "$LOG" > "$out" 2>&1 </dev/null 9>&-; rrc=$?
  if [ -s "$MEAS" ]; then "$HEE4DB" record measurement --file "$MEAS" >> "$out" 2>&1 </dev/null 9>&-; mrc=$?; fi
  case "$rrc" in 0|10) ;; *) worst=$rrc ;; esac
  case "$mrc" in 0|10|skipped_no_measure_file) ;; *) [ "$worst" != 0 ] || worst=$mrc ;; esac
  if [ "$worst" = 0 ]; then echo "db_record=ok rc=0 run_rc=$rrc measurement_rc=$mrc" >> "$LOG"
  else echo "db_record=failed rc=$worst run_rc=$rrc measurement_rc=$mrc" >> "$LOG"; fi
}
finish() { echo "exit=$1 measure_rc=$MRC" >> "$LOG"; db_record; exit "$1"; }
exec 9>"$LOGDIR/.lock" || exit 3
if ! flock -n 9; then
  msg="$AGENT: lock held; skipped $STAMP $MODE exit=40"
  echo "$msg" >> "$LOGDIR/skipped.log"
  echo "$msg" >> "$LOG"
  finish 40
fi
line=$(grep -E "^$MODE\|" "$DIR/modes.conf" | head -1) || true
[ -n "$line" ] || { echo "unknown mode $MODE for $AGENT" >&2; exit 2; }
BUDGET=$(printf '%s' "$line" | cut -d'|' -f2); TASK=$(printf '%s' "$line" | cut -d'|' -f3-)
"$DIR/measure.sh" "$STAMP" > "$MEAS" 2>&1 9>&- </dev/null; MRC=$?
cd "$V4" || finish 3
{
  echo "$AGENT mode=$MODE start=$STAMP budget_usd=$BUDGET measurements=$MEAS measure_rc=$MRC"
  # JSON output (R16): the single result object carries total_cost_usd, is_error and subtype. It is appended to the log
  # as printed, then parsed by python3; the VERDICT still comes from the REPORT file (below), never from this JSON.
  # 9>&-: the claude child (and anything it leaves behind) must not inherit the lock.
  out=$("$CLAUDE" -p "$AGENT $TASK Measurements file: $MEAS. Write your report to $REPORT. Its LAST non-empty line must start at column 0 with the exact token '$NAME verdict=' followed by PASS, PASS_WITH_GAPS or FAIL, then a space and your verdict fields; nothing before the token, no fence or quote around it, nothing after that line. The runner refuses any other form as UNMEASURED (exit 30)." \
    --agent "$AGENT" --model claude-sonnet-5-5 \
    --settings "$DIR/settings.json" --permission-mode dontAsk \
    --max-budget-usd "$BUDGET" --no-session-persistence --output-format json </dev/null 9>&-); crc=$?
  printf '%s\n' "$out"
  echo "$AGENT claude_rc=$crc end=$(date -u +%Y%m%dT%H%M%SZ)"
} >> "$LOG" 2>&1
# One parse of the JSON, three fields; any failure reads UNMEASURED for all three.
meta=$(printf '%s' "$out" | python3 -c '
import json, sys
try:
    d = json.loads(sys.stdin.read())
    c = d.get("total_cost_usd"); e = d.get("is_error"); s = d.get("subtype")
    print("cost_usd=%s is_error=%s subtype=%s" % ("UNMEASURED" if c is None else c, "UNMEASURED" if e is None else e, "UNMEASURED" if s is None else s))
except Exception:
    print("cost_usd=UNMEASURED is_error=UNMEASURED subtype=UNMEASURED")
' 2>/dev/null) || meta="cost_usd=UNMEASURED is_error=UNMEASURED subtype=UNMEASURED"
echo "$meta" >> "$LOG"
cost=UNMEASURED; iserr=UNMEASURED; subtype=UNMEASURED
for kv in $meta; do case "$kv" in cost_usd=*) cost=${kv#*=} ;; is_error=*) iserr=${kv#*=} ;; subtype=*) subtype=${kv#*=} ;; esac; done

# Verdict: the report's LAST non-empty line only (trailing CR stripped), matched whole against the typed form.
# Acquisition bound: only the last 64 KiB of the report is read.
last=""; reason=""
if [ ! -f "$REPORT" ]; then reason=no_report
else
  while IFS= read -r l || [ -n "$l" ]; do l=${l%$'\r'}; [[ "$l" =~ [^[:space:]] ]] && last=$l; done < <(tail -c 65536 "$REPORT")
  [ -n "$last" ] || reason=empty_report
fi
re="^${NAME} verdict=(PASS_WITH_GAPS|PASS|FAIL)( |\$)"
verdict=""
if [ -z "$reason" ]; then
  if [[ "$last" =~ $re ]]; then verdict=${BASH_REMATCH[1]}; else reason=last_line_not_typed_verdict; fi
fi
if [ -n "$verdict" ]; then echo "verdict_line=$last" >> "$LOG"
else
  echo "verdict_line=REFUSED $reason" >> "$LOG"
  [ -n "$last" ] && printf 'refused_last_line=%q\n' "${last:0:200}" >> "$LOG"
fi

# A run claude cut off at its budget did not finish its steps: whatever the report says, it is not a verdict.
if [ "$subtype" = error_max_budget_usd ]; then echo "degraded_by=subtype=error_max_budget_usd" >> "$LOG"; finish 30; fi
deg=""
[ "$crc" = 0 ] || deg="${deg:+$deg,}claude_rc=$crc"
[ "$iserr" = False ] || deg="${deg:+$deg,}is_error=$iserr"
[ "$cost" != UNMEASURED ] || deg="${deg:+$deg,}cost=UNMEASURED"
# Code measures, the agent curates: a red or unmeasured MEASUREMENT outranks the agent's PASS.
# (2026-10-01: the curator wrote PASS over its own `v3_refs=11`.) Floors: v3_refs>0, `funnel: verdict=FAIL`, any UNMEASURED line.
if [ -r "$MEAS" ]; then
  v3=$(grep -o -E 'v3_refs=[0-9]+' "$MEAS" | tail -1); v3=${v3#v3_refs=}
  [ -z "$v3" ] || [ "$v3" = 0 ] || deg="${deg:+$deg,}measured_v3_refs=$v3"
  fv=$(grep -o -E '^funnel: verdict=[A-Z_]+' "$MEAS" | tail -1); fv=${fv#funnel: verdict=}
  [ "$fv" != FAIL ] || deg="${deg:+$deg,}measured_funnel_fail"
  um=$(grep -c 'UNMEASURED' "$MEAS")
  [ "$um" = 0 ] || deg="${deg:+$deg,}measured_unmeasured_lines=$um"
fi
case "$verdict" in
  PASS|PASS_WITH_GAPS)
    if [ -n "$deg" ]; then echo "degraded_by=$deg" >> "$LOG"; finish 10; fi
    [ "$verdict" = PASS ] && finish 0
    finish 10 ;;
  FAIL) finish 20 ;;
  *) finish 30 ;;
esac
