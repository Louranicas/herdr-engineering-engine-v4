#!/usr/bin/env bash
# hee4-ops.db daily upkeep (CN-04): keeps the DB current with no human in the loop. User timer hee4-daily.timer,
# 02:30 daily (systemd/hee4-daily.{timer,service}, installed by `just install-timers`; SuccessExitStatus=10).
# Steps, in order, each logged `step=<name> rc=N`:
#   ingest                      rebuild the derived tables from their file homes (+ FTS sidecar)
#   runs                        record run --from-logs: backfills any run whose own db_record step failed (idempotent)
#   measurements                record measurement --from-measure-files (idempotent on name, measured_at, source)
#   jev-daily                   record jev-daily --day <yesterday, local>: COUNTS ONLY, no text (the day is complete)
#   check                       the aggregate integrity check, after the ingest
# Last line: `daily verdict=PASS|PASS_WITH_GAPS|FAIL exit=N steps_ok=K/5 worst_step=<name> log=<path>`; the aggregate
# refuses as one unit (an rc outside 0/10 in any step is FAIL; any 10 is PASS_WITH_GAPS). Exit codes as hee4db's.
# Writes only ~/hee4-evidence/db (the DB, its sidecar, daily/*.log). Reads no v3 path (V4-9).
set -uo pipefail
HEE4DB=${HEE4_ROOT:-/mnt/storage-10tb/herdr-engineering-engine-v4}/ops/db/hee4db
OUT=${HEE4_EVIDENCE:-/mnt/storage-10tb/hee4-evidence}/db/daily
mkdir -p "$OUT" || exit 3
STAMP=$(date -u +%Y%m%dT%H%M%SZ)
LOG="$OUT/daily-$STAMP.log"
YESTERDAY=$(date -d yesterday +%Y-%m-%d) || { echo "daily verdict=FAIL exit=3 reason=date_failed" >> "$LOG"; exit 3; }
ok=0; worst=0; worst_step=none
step() { # <name> <hee4db args…>
  local name=$1 rc; shift
  echo "## step=$name argv=hee4db $*" >> "$LOG"
  "$HEE4DB" "$@" >> "$LOG" 2>&1 </dev/null; rc=$?
  local why=""
  if [ "$name" = jev-daily ] && [ "$rc" = 10 ]; then # the gap, by name: which senders the verb could not measure
    local senders; senders=$(grep -o 'senders_fully_measured=[0-9]*/[0-9]*' "$LOG" | tail -n 1)
    case "$senders" in *=0/*) why=" reason=no_sender_installed $senders" ;; *) why=" reason=senders_partial $senders" ;; esac
  fi
  echo "step=$name rc=$rc$why" >> "$LOG"
  case "$rc" in
    0) ok=$((ok + 1)) ;;
    10) ok=$((ok + 1)); if [ "$worst" = 0 ]; then worst=10; worst_step=$name; fi ;;
    *) if [ "$worst" = 0 ] || [ "$worst" = 10 ]; then worst=$rc; worst_step=$name; fi ;;
  esac
}
echo "hee4db daily start=$STAMP jev_day=$YESTERDAY" >> "$LOG"
step ingest ingest
step runs record run --from-logs
step measurements record measurement --from-measure-files
step jev-daily record jev-daily --day "$YESTERDAY"
step check check
case "$worst" in 0) v=PASS ;; 10) v=PASS_WITH_GAPS ;; *) v=FAIL ;; esac
echo "daily verdict=$v exit=$worst steps_ok=$ok/5 worst_step=$worst_step log=$LOG" | tee -a "$LOG"
exit "$worst"
