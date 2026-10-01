# HEE v4 planning repo: ops recipes (HOLD, V4-0: ops tooling only, no engine code).
#
# Every recipe calls an EXISTING door and invents no check. Each prints the verdict lines of the
# doors it ran and fails as ONE unit. Every recipe has its procedure in runbooks/<name>.toml
# (precondition, step, verify); validate them with
#   habitat-runbook preview <name> --directory runbooks
# just 1.57 semantics: arguments are POSITIONAL (`just highway store`, `just repin AT`);
# `just name=value` is a variable override, not an argument. Run from anywhere: recipes cd here.

set shell := ["bash", "-uc"]
set positional-arguments := true
set dotenv-load := false

# List the recipes
default:
    @just --justfile "{{justfile()}}" --list --unsorted

# Every check in order, all of them, then ONE verdict: `verify verdict=PASS|FAIL steps=N/M`
verify:
    #!/usr/bin/env bash
    set -uo pipefail
    stamp=$(date -u +%Y%m%dT%H%M%SZ)
    logs="$HOME/.cache/hee4-just/verify-$stamp"; mkdir -p "$logs"
    # The tree this verdict is about, read from .git files (no git process is spawned).
    head="unmeasured"
    if [ -f .git/HEAD ]; then
      ref=$(sed -n 's/^ref: //p' .git/HEAD)
      if [ -z "$ref" ]; then head=$(cut -c1-12 .git/HEAD)
      elif [ -f ".git/$ref" ]; then head=$(cut -c1-12 ".git/$ref")
      elif [ -f .git/packed-refs ] && grep -q " $ref\$" .git/packed-refs; then head=$(awk -v r="$ref" '$2==r{print substr($1,1,12)}' .git/packed-refs)
      else head="unborn:${ref#refs/heads/}"; fi
    fi
    names=(cite_pins funnel funnel_control hee4db_check db_control jev_entry_control trace render)
    cmds=(
      "python3 ops/checks/cite_pins.py status"
      "python3 ops/checks/module_funnel.py"
      "python3 ops/checks/module_funnel.py --control"
      "hee4db check"
      "python3 ops/db/tests/control.py"
      "python3 ops/db/tests/jev_entry_control.py"
      "python3 ops/checks/funnel_trace.py"
      "node ops/checks/render/render.cjs"
    )
    ok=0; total=${#names[@]}; summary=""; failed=""
    for i in "${!names[@]}"; do
      n=${names[$i]}; log="$logs/$((i+1))-$n.log"
      bash -c "${cmds[$i]}" > "$log" 2>&1 < /dev/null; rc=$?
      echo "── step $((i+1))/$total $n rc=$rc :: ${cmds[$i]}"
      grep -v '^{' "$log" | grep -E 'verdict=|^pins |refused' | tail -n 2 | sed 's/^/   /'
      if [ "$rc" -eq 0 ]; then ok=$((ok+1)); else failed="$failed $n"; echo "   log=$log"; fi
      summary="$summary $n=$rc"
    done
    v=PASS; [ "$ok" -eq "$total" ] || v=FAIL
    echo "verify verdict=$v steps=$ok/$total head=$head dirty=unmeasured logs=$logs${failed:+ failed=${failed# }} rc:$summary"
    [ "$v" = PASS ]

# After a register edit: ingest, regenerate ATLAS §9 / ULTRAMAP §7 / readiness note, repin AT+UM, ingest, check
regen:
    #!/usr/bin/env bash
    set -uo pipefail
    names=(ingest regen_blocks repin_AT repin_UM ingest_again hee4db_check funnel stale_readback blocks_readback)
    cmds=(
      "hee4db ingest"
      "python3 ops/checks/regen.py"
      "python3 ops/checks/cite_pins.py repin AT"
      "python3 ops/checks/cite_pins.py repin UM"
      "hee4db ingest"
      "hee4db check"
      "python3 ops/checks/module_funnel.py"
      "hee4db stale"
      "python3 ops/checks/regen.py --check"
    )
    total=${#names[@]}; ok=0
    # A sequence: each step's input is the previous step's output, so it stops at the first red step
    # and names it (the steps after it were not reached, never "passed").
    for i in "${!names[@]}"; do
      n=${names[$i]}
      out=$(bash -c "${cmds[$i]}" 2>&1 < /dev/null); rc=$?
      echo "── step $((i+1))/$total $n rc=$rc :: ${cmds[$i]}"
      printf '%s\n' "$out" | grep -v '^{' | grep -E 'verdict=|^regen |^repin |^pinned ' | tail -n 4 | sed 's/^/   /'
      if [ "$rc" -ne 0 ]; then
        printf '%s\n' "$out" | tail -n 15 | sed 's/^/   | /'
        echo "regen verdict=FAIL steps=$ok/$total failed_at=$n not_reached=$((total-ok-1))"; exit 1
      fi
      ok=$((ok+1))
    done
    echo "regen verdict=PASS steps=$ok/$total"

# Map every KEY cite to the edited file and repin it (KEY from ops/checks/module_funnel.py LEGEND), then the funnel
repin KEY:
    #!/usr/bin/env bash
    set -uo pipefail
    python3 ops/checks/cite_pins.py repin "$1" < /dev/null; r1=$?
    # the funnel's OWN rc (PIPESTATUS), never tail's: a piped $? would read a failing funnel as PASS
    python3 ops/checks/module_funnel.py < /dev/null | tail -n 2; r2=${PIPESTATUS[0]}
    # a pinned file may also be a DB source (DECISIONS, ATLAS, ...): re-ingest so `hee4db check` stale stays green
    hee4db ingest < /dev/null > /dev/null 2>&1; r3=$?
    v=PASS; [ "$r1" -eq 0 ] && [ "$r2" -eq 0 ] && [ "$r3" -eq 0 ] || v=FAIL
    echo "repin verdict=$v key=$1 repin_rc=$r1 funnel_rc=$r2 ingest_rc=$r3"
    [ "$v" = PASS ]

# End-to-end funnel trace over all modules (ops/checks/funnel_trace.py)
trace:
    python3 ops/checks/funnel_trace.py < /dev/null

# Render every mermaid block in the v4 vault; `just render plant` runs the negative control (must FAIL, exit 1)
render MODE="":
    #!/usr/bin/env bash
    set -uo pipefail
    case "$1" in
      "") exec node ops/checks/render/render.cjs ;;
      plant) exec node ops/checks/render/render.cjs --plant ;;
      *) echo "usage: just render [plant]  (got: $1)" >&2; exit 2 ;;
    esac

# P0 Jev entry read-back: the door refuses every v4 line, and no sender sent an engine row since the fix (H-8, H-19)
jev-entry SINCE="":
    #!/usr/bin/env bash
    # V4-74: JP0 battery through the real door + sent_engine_rows over all four senders since the door and its
    # loggers were installed (or since SINCE, an ISO time with a UTC offset). Control: ops/db/tests/jev_entry_control.py
    if [ -n "$1" ]; then exec hee4db jev-entry --table --since "$1"; else exec hee4db jev-entry --table; fi

# Every planning hop for one module, as bounded JSON
highway MODULE:
    hee4db highway "$1" < /dev/null

# One roster agent's selfcheck (spends about $0.25 of API per run)
roster-selfcheck AGENT:
    bash ops/roster/run-agent.sh "$1" selfcheck < /dev/null

# Host corpus backup rehearsal + restore drill, then remove ONLY the drill's own left-over scratch dir; `just drill dry` stops after the rehearsal
drill MODE="":
    #!/usr/bin/env bash
    set -uo pipefail
    case "$1" in ""|dry) ;; *) echo "usage: just drill [dry]  (got: $1)" >&2; exit 2 ;; esac
    if [ -f /run/.containerenv ]; then H=(flatpak-spawn --host); where=host_via_flatpak_spawn; else H=(); where=host; fi
    BIN="$HOME/.local/bin"
    listdirs() { "${H[@]}" bash -c 'shopt -s nullglob; for d in /tmp/corpus-drill-*; do [ -d "$d" ] && printf "%s\n" "$d"; done' < /dev/null | sort; }
    before=$(listdirs)
    echo "── step 1 backup rehearsal ($where) :: habitat-corpus-backup --dry-run"
    out=$("${H[@]}" "$BIN/habitat-corpus-backup" --dry-run < /dev/null 2>&1); r1=$?
    printf '%s\n' "$out" | grep -E 'verdict=' | tail -n 2 | sed 's/^/   /'
    # Known defect in the habitat tool (measured 2026-10-01, not fixed here): rsync -X + --link-dest +
    # --dry-run cannot open the not-yet-created destination dirs to copy xattrs, so every source exits
    # 23 and the rehearsal exits 1 while printing verdict=DRY-RUN. The rehearsal is therefore judged
    # on its own verdict line (would_transfer>0) with every rsync failure being code 23; any other
    # rsync code, a missing verdict line, or would_transfer=0 fails it.
    wt=$(printf '%s\n' "$out" | sed -n 's/.*verdict=DRY-RUN would_transfer=\([0-9][0-9]*\).*/\1/p' | tail -n 1)
    f23=$(printf '%s\n' "$out" | grep -c 'rsync FAILED (23)' || true)
    fother=$(printf '%s\n' "$out" | grep 'rsync FAILED (' | grep -vc 'rsync FAILED (23)' || true)
    reh=PASS
    if [ -z "$wt" ] || [ "$wt" -eq 0 ] || [ "$fother" -ne 0 ]; then reh=FAIL; fi
    if [ "$r1" -ne 0 ] && [ "$f23" -eq 0 ]; then reh=FAIL; fi
    echo "   rehearsal=$reh rc=$r1 would_transfer=${wt:-none} rsync23_dryrun_xattr=$f23 rsync_other_failures=$fother"
    if [ "$1" = dry ]; then
      echo "   pre-existing /tmp/corpus-drill-* dirs (not touched): $(printf '%s\n' "$before" | grep -c . || true)"
      echo "drill verdict=$reh mode=dry rehearsal=$reh drill=not_run cleanup=not_run"; [ "$reh" = PASS ]; exit
    fi
    [ "$reh" = PASS ] || { echo "drill verdict=FAIL rehearsal=FAIL drill=not_run"; exit 1; }
    echo "── step 2 restore drill ($where) :: habitat-corpus-drill"
    out=$("${H[@]}" "$BIN/habitat-corpus-drill" < /dev/null 2>&1); r2=$?
    printf '%s\n' "$out" | grep -E 'verdict=|corrupt=|control=' | tail -n 4 | sed 's/^/   /'
    # The drill's own `rm -rf` trap cannot remove a restored read-only directory. Remove ONLY the
    # dirs that appeared during this drill, each by its exact path, after making it writable.
    after=$(listdirs)
    new=$(comm -13 <(printf '%s\n' "$before" | grep . || true) <(printf '%s\n' "$after" | grep . || true))
    cleaned=0; cfail=0; found=0
    while IFS= read -r d; do
      [ -n "$d" ] || continue; found=$((found+1))
      if [[ ! "$d" =~ ^/tmp/corpus-drill-[A-Za-z0-9]{6}$ ]]; then echo "   refuse_cleanup unexpected_name=$d"; cfail=$((cfail+1)); continue; fi
      "${H[@]}" chmod -R u+w -- "$d" < /dev/null && "${H[@]}" rm -rf -- "$d" < /dev/null
      if "${H[@]}" test -e "$d" < /dev/null; then echo "   cleanup_failed $d (still present)"; cfail=$((cfail+1)); else echo "   cleaned $d"; cleaned=$((cleaned+1)); fi
    done <<< "$new"
    echo "── step 3 cleanup: left_behind=$found cleaned=$cleaned failed=$cfail (pre-existing dirs untouched)"
    v=PASS; [ "$r2" -eq 0 ] && [ "$cfail" -eq 0 ] || v=FAIL
    echo "drill verdict=$v rehearsal_rc=$r1 drill_rc=$r2 left_behind=$found cleaned=$cleaned cleanup_failed=$cfail"
    [ "$v" = PASS ]

# Undo the v3 write guard (V4-68) from the recorded modes; refuses unless the positional argument is `confirm`
restore-v3-modes CONFIRM="":
    #!/usr/bin/env bash
    set -uo pipefail
    S="$HOME/hee4-evidence/ops-records/restore-v3-modes.sh"; T="$HOME/hee4-evidence/ops-records/v3-modes-20261001.tsv"
    [ -f "$S" ] && [ -s "$T" ] || { echo "restore-v3-modes verdict=REFUSED reason=script_or_tsv_absent script=$S tsv=$T"; exit 3; }
    rows=$(wc -l < "$T")
    if [ "$1" != confirm ]; then
      echo "restore-v3-modes verdict=REFUSED reason=no_confirm rows=$rows script=$S"
      echo "   this UNDOES the v3 write guard (V4-68): it chmods $rows v3 paths back to their pre-protection modes."
      echo "   to run it: just restore-v3-modes confirm"
      exit 2
    fi
    if [ -f /run/.containerenv ]; then H=(flatpak-spawn --host); else H=(); fi
    out=$("${H[@]}" bash "$S" < /dev/null 2>&1); rc=$?
    printf '%s\n' "$out" | tail -n 3 | sed 's/^/   /'
    line=$(printf '%s\n' "$out" | grep -E '^restored=[0-9]+/[0-9]+$' | tail -n 1)
    n=${line#restored=}; n=${n%/*}; m=${line#*/}
    v=PASS; [ "$rc" -eq 0 ] && [ -n "$line" ] && [ "$n" = "$m" ] && [ "$m" = "$rows" ] || v=FAIL
    echo "restore-v3-modes verdict=$v rc=$rc restored=${line#restored=} tsv_rows=$rows"
    [ "$v" = PASS ]
