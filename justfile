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
    names=(cite_pins funnel funnel_control hee4db_check db_control jev_entry_control trace render fm_db_control roster_selfcheck)
    cmds=(
      "python3 ops/checks/cite_pins.py status"
      "python3 ops/checks/module_funnel.py"
      "python3 ops/checks/module_funnel.py --control"
      "hee4db check"
      "python3 ops/db/tests/control.py"
      "python3 ops/db/tests/jev_entry_control.py"
      "python3 ops/checks/funnel_trace.py"
      "node ops/checks/render/render.cjs"
      "python3 ops/firstmate/tests/control.py"
      "python3 ops/roster/selfcheck.py"
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
    # ingest rc 10 is its own PASS_WITH_GAPS (a registry row it cannot read); the pins are still
    # fresh, so the recipe passes and names the gap rather than hiding a green pin behind it.
    hee4db ingest < /dev/null > /dev/null 2>&1; r3=$?
    v=PASS; [ "$r1" -eq 0 ] && [ "$r2" -eq 0 ] && { [ "$r3" -eq 0 ] || [ "$r3" -eq 10 ]; } || v=FAIL
    echo "repin verdict=$v key=$1 repin_rc=$r1 funnel_rc=$r2 ingest_rc=$r3$([ "$r3" -eq 10 ] && echo ' ingest=PASS_WITH_GAPS')"
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

# Roster self-check; no argument: the $0 structural self-check (ops/roster/selfcheck.py); with AGENT: the paid runner (about $0.25 per run)
roster-selfcheck AGENT="":
    @[ -z "$1" ] && exec python3 ops/roster/selfcheck.py || exec bash ops/roster/run-agent.sh "$1" selfcheck < /dev/null

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

# The derived tiered code gate on a `git archive` of HEAD: `just gate commit|stack|cut` (tools/gate, gate.toml)
gate tier:
    @tools/gate "{{tier}}"

# The kill -9 drill (`drill` is taken by the corpus backup drill): `just drill-kill9 [UNIT [SOCKET]]`
drill-kill9 unit="hee4.service" socket="":
    #!/usr/bin/env bash
    exec tools/drill --unit "$1" ${2:+--socket "$2"}

# Fresh-instance readiness: unit, socket perms, ledger, model, binary sha
doctor:
    @tools/doctor

# Build the release binary, install it, restart the unit, read health and the doctor. One verdict line.
deploy:
    #!/usr/bin/env bash
    set -uo pipefail
    export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cache/hee4-target}"
    cargo build -q -p hee4-app --release --offline || { echo "deploy verdict=FAIL step=build"; exit 1; }
    install -Dm755 "$CARGO_TARGET_DIR/release/hee4" "$HOME/.local/bin/hee4" || { echo "deploy verdict=FAIL step=install"; exit 1; }
    install -Dm644 systemd/hee4.service "$HOME/.config/systemd/user/hee4.service"
    systemctl --user daemon-reload && systemctl --user restart hee4.service || { echo "deploy verdict=FAIL step=restart"; exit 1; }
    # Serve start takes a backup before it listens (V4-99), so a fixed sleep raced it once and the
    # recipe printed PASS over "connection refused". Poll until ready, bounded; PASS only on a
    # measured ready=true recovery=complete at the installed head, and a doctor that passes.
    v=$(hee4 --version); h=""; t0=$SECONDS
    until [ $((SECONDS - t0)) -ge 60 ]; do
      h=$(hee4 health 2>&1 | head -1)
      case "$h" in *ready=true*recovery=complete*) break ;; esac
      sleep 1
    done
    head=$(git rev-parse --short=12 HEAD)
    case "$h" in
      *ready=true*recovery=complete*"head=$head"*) ;;
      *) echo "deploy verdict=FAIL step=health binary=\"$v\" health=\"$h\" waited_s=$((SECONDS - t0))"; exit 1 ;;
    esac
    d=$(hee4 doctor --repo . 2>&1 | tail -1)
    case "$d" in
      *"verdict=PASS"*) echo "deploy verdict=PASS binary=\"$v\" health=\"$h\" waited_s=$((SECONDS - t0))"; echo "$d" ;;
      *) echo "deploy verdict=FAIL step=doctor binary=\"$v\" health=\"$h\""; echo "$d"; exit 1 ;;
    esac

# Install and enable the user timers (ops/db/daily.sh and tools/habitat-backup; never a roster agent). One verdict line.
install-timers:
    #!/usr/bin/env bash
    set -uo pipefail
    d="$HOME/.config/systemd/user"
    for u in hee4-daily.service hee4-daily.timer hee4-backup.service hee4-backup.timer; do
      install -Dm644 "systemd/$u" "$d/$u" || { echo "install-timers verdict=FAIL step=install unit=$u"; exit 1; }
    done
    systemctl --user daemon-reload || { echo "install-timers verdict=FAIL step=daemon-reload"; exit 1; }
    nexts=""
    for t in hee4-daily.timer hee4-backup.timer; do
      systemctl --user enable --now "$t" || { echo "install-timers verdict=FAIL step=enable timer=$t"; exit 1; }
      next=$(systemctl --user list-timers --no-legend "$t" | awk 'NR==1 {print $2 "T" $3}')
      [ -n "$next" ] || { echo "install-timers verdict=FAIL step=list-timers timer=$t"; exit 1; }
      nexts="$nexts $t=$next"
    done
    echo "install-timers verdict=PASS timers=hee4-daily.timer,hee4-backup.timer next=${nexts# }"

# Push main (and tags) to the local mirror that treehouse and Firstmate cut worktrees from. Never GitHub.
mirror:
    git push -q origin main --tags && echo "mirror verdict=PASS origin/main=$(git rev-parse --short=12 origin/main)"

# The D10 cut record at a DEPLOYED HEAD: refuses unless `hee4 --version`'s head = HEAD (`just deploy` first), then mirror, gate cut, check-deployed --control then check-deployed, cold-clone, push-scan, layers, watch; writes $HEE4_CUT_ROOT/<sha12>/cut-check.json
cut-check:
    #!/usr/bin/env bash
    set -uo pipefail
    root="${HEE4_CUT_ROOT:-$HOME/.cache/hee4-cut}"
    sha=$(git rev-parse --verify -q 'HEAD^{commit}') || { echo "cut-check verdict=REFUSED reason=no_head"; exit 2; }
    s12=${sha:0:12}
    # every run at this sha voids the earlier record first, a refused one included: no refusal leaves a PASS for `just tag`
    rm -f "$root/$s12/cut-check.json" || { echo "cut-check verdict=FAIL reason=record_unremovable sha=$s12"; exit 1; }
    # the third field of `hee4 <VERSION> <head12> ...` (crates/hee4-app/src/main.rs), compared whole, never a substring
    bin=$(hee4 --version 2>/dev/null < /dev/null | awk '{print $3}')
    [[ "$bin" =~ ^[0-9a-f]{12}$ ]] || bin="UNMEASURED(hee4_--version)"
    if [ "$bin" != "$s12" ]; then
      echo "cut-check verdict=REFUSED reason=binary_head_mismatch binary=$bin head=$s12 hint='just deploy first'"; exit 2
    fi
    stamp=$(date -u +%Y%m%dT%H%M%SZ); logs="$root/$stamp-$s12"; mkdir -p "$logs" "$root/$s12"
    # an earlier run's record never outlives this one: RUNNING replaces it atomically before any door
    # runs, so a killed or failed run leaves nothing `just tag` admits (ATLAS:41)
    rec="$root/$s12/cut-check.json"
    printf '{"sha": "%s", "run": "%s", "verdict": "RUNNING"}\n' "$sha" "$stamp-$s12" > "$rec.tmp.$$" && mv -f "$rec.tmp.$$" "$rec" \
      || { echo "cut-check verdict=FAIL reason=record_unwritable record=$rec"; exit 1; }
    # the control runs before the aggregate's first real use (ATLAS §1, P7)
    names=(mirror gate_cut check_deployed_control check_deployed cold_clone push_scan layers watch)
    cmds=(
      "just --justfile '{{justfile()}}' --working-directory '$PWD' mirror"
      "tools/gate cut"
      "tools/check-deployed --control"
      "tools/check-deployed"
      "tools/cold-clone --sha $sha"
      "tools/push-scan"
      "tools/layers"
      "tools/watch"
    )
    total=${#names[@]}; ok=0; steps=""
    for i in "${!names[@]}"; do
      n=${names[$i]}; log="$logs/$n.log"
      bash -c "${cmds[$i]}" > "$log" 2>&1 < /dev/null; rc=$?
      echo "── step $((i+1))/$total $n rc=$rc :: ${cmds[$i]}"
      grep -E 'verdict=|^D8 ' "$log" | tail -n 2 | sed 's/^/   /'
      if [ "$rc" -eq 0 ]; then ok=$((ok+1)); else echo "   log=$log"; fi
      steps="$steps $n=$rc"
    done
    # Compose the D10 fields ONLY from the lines these doors just printed (ATLAS D10, the one home).
    python3 - "$logs" "$root/$s12/cut-check.json" "$sha" "$ok" "$total" $steps <<'PY'
    import json, os, sys, time
    logs, out, sha, ok, total, steps = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5]), sys.argv[6:]
    steps = [{"name": s.split("=")[0], "rc": int(s.split("=")[1])} for s in steps]
    def lines(n):
        try:
            return open(f"{logs}/{n}.log").read().splitlines()
        except OSError:
            return []
    def last(n, prefix):
        hit = [l for l in lines(n) if l.startswith(prefix)]
        return dict(t.split("=", 1) for t in hit[-1].split() if "=" in t) if hit else {}
    d8, dep = last("check_deployed", "D8 "), last("check_deployed", "deployed=")
    cold, push = last("cold_clone", "cold-clone "), last("push_scan", "push-scan range=")
    lay, watch = last("layers", "apparatus_ratio="), last("watch", "watch verdict=")
    f, missing, bad = {}, [], []
    if all(k in d8 for k in ("flows", "l2", "reasoned", "unexplained")):
        f["scoreboard"] = "scoreboard " + " ".join(f"{k}={d8[k]}" for k in ("flows", "l2", "reasoned", "unexplained"))
    else:
        missing.append("scoreboard")
    if "deployed" in dep:
        f["deployed"] = f"deployed={dep['deployed']}"
        n, _, m = dep["deployed"].partition("/")
        if not (n.isdigit() and n == m): bad.append("deployed")
    else:
        missing.append("deployed")
    if "steps" in cold and "matched" in cold:
        f["cold_clone"] = f"cold-clone steps={cold['steps']} matched={cold['matched']}"
        if cold["steps"] != cold["matched"]: bad.append("cold_clone")
    else:
        missing.append("cold_clone")
    if "hits" in push:
        f["push_scan"] = f"push-scan hits={push['hits']}"
        if push["hits"] != "0": bad.append("push_scan")
    else:
        missing.append("push_scan")
    if "apparatus_ratio" in lay:
        f["apparatus_ratio"] = f"apparatus_ratio={lay['apparatus_ratio']}"
    else:
        missing.append("apparatus_ratio")
    # tree= is the sha12 check-deployed printed; the record carries the full sha only when that prefix matches HEAD
    if dep.get("tree") and sha.startswith(dep["tree"]) and len(dep["tree"]) >= 12:
        f["tree"] = sha
    else:
        missing.append("tree")
    if "dirty" in dep:
        f["dirty"] = dep["dirty"]
        if dep["dirty"] != "0": bad.append("dirty")
    else:
        missing.append("dirty")
    if "watchers" not in watch: missing.append("watch")
    v = "PASS" if ok == total and not missing and not bad else "FAIL"
    rec = {"sha": sha, "run": os.path.basename(logs), "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
           "verdict": v, "steps": steps, "fields": f}
    tmp = f"{out}.tmp.{os.getpid()}"
    with open(tmp, "w") as fh:
        json.dump(rec, fh, indent=1)
    os.replace(tmp, out)
    failed = [s["name"] for s in steps if s["rc"] != 0]
    g = lambda k, pre: f[k][len(pre):] if k in f else "UNMEASURED(missing)"
    print(f"cut-check verdict={v}" + (f" missing_field={','.join(missing)}" if missing else "")
          + (f" bad_field={','.join(bad)}" if bad else "") + (f" failed={','.join(failed)}" if failed else "")
          + f" sha={sha[:12]} deployed={g('deployed', 'deployed=')} cold={g('cold_clone', 'cold-clone ')}"
          + f" push={g('push_scan', 'push-scan ')} apparatus_ratio={g('apparatus_ratio', 'apparatus_ratio=')}"
          + f" watch={watch.get('watchers', 'UNMEASURED(missing)')} tree={dep.get('tree', 'UNMEASURED(missing)')}"
          + f" dirty={f.get('dirty', 'UNMEASURED(missing)')} record={out} github_push=UNMEASURED(no credential: gh not logged in)")
    sys.exit(0 if v == "PASS" else 1)
    PY

# Lay the annotated D10 tag NAME at HEAD from HEAD's newest PASS cut-check record, LOCALLY (never pushed); refuses unless `hee4 --version` is HEAD and the positional argument is `confirm`
tag NAME CONFIRM="":
    #!/usr/bin/env bash
    set -uo pipefail
    root="${HEE4_CUT_ROOT:-$HOME/.cache/hee4-cut}"
    sha=$(git rev-parse --verify -q 'HEAD^{commit}') || { echo "tag verdict=REFUSED reason=no_head name=$1"; exit 3; }
    s12=${sha:0:12}; rec="$root/$s12/cut-check.json"; msg=$(mktemp); trap 'rm -f "$msg"' EXIT
    refuse() { echo "tag verdict=REFUSED reason=$1 name=$NAME sha=$s12${2:+ $2}"; exit 3; }
    NAME="$1"
    # the binary that cut-check measured must still be the installed one (same compare as cut-check)
    bin=$(hee4 --version 2>/dev/null < /dev/null | awk '{print $3}')
    [[ "$bin" =~ ^[0-9a-f]{12}$ ]] || bin="UNMEASURED(hee4_--version)"
    [ "$bin" = "$s12" ] || refuse binary_head_mismatch "binary=$bin head=$s12 hint='just deploy first'"
    [ -f "$rec" ] || refuse no_cut_check_at_sha "record=$rec"
    # the six message lines, in the D10 order, from THIS sha's record only, and only from the newest run
    why=$(python3 - "$rec" "$sha" "$msg" "$root" <<'PY'
    import glob, json, os, sys
    rec, sha, msg, root = sys.argv[1:5]
    try:
        r = json.load(open(rec))
        f = r["fields"] if r.get("verdict") != "RUNNING" else None
    except (OSError, ValueError, KeyError, TypeError):
        print("cut_check_failed"); sys.exit(1)
    if r.get("sha") != sha:
        print("no_cut_check_at_sha"); sys.exit(1)
    if r.get("verdict") == "RUNNING":
        print("cut_check_incomplete"); sys.exit(1)
    if r.get("verdict") != "PASS":
        print("cut_check_failed"); sys.exit(1)
    runs = sorted(os.path.basename(d) for d in glob.glob(os.path.join(root, f"*-{sha[:12]}")) if os.path.isdir(d))
    if not runs or r.get("run") != runs[-1]:
        print("stale_cut_check"); sys.exit(1)
    try:
        body = [f["scoreboard"], f["deployed"], f["cold_clone"], f["push_scan"], f["apparatus_ratio"], f"tree={f['tree']} dirty={f['dirty']}"]
    except KeyError:
        print("cut_check_failed"); sys.exit(1)
    open(msg, "w").write("\n".join(body) + "\n")
    PY
    ) || refuse "$why" "record=$rec"
    dirty=$(git status --porcelain | wc -l)
    [ "$dirty" -eq 0 ] || refuse dirty "dirty=$dirty"
    git rev-parse -q --verify "refs/tags/$NAME" > /dev/null && refuse tag_exists
    want=$(wc -l < "$msg")
    if [ "${2:-}" != confirm ]; then
      echo "   the annotated tag $NAME at $s12 would carry:"; sed 's/^/     /' "$msg"
      echo "   to lay it (locally; never pushed): just tag $NAME confirm"
      echo "tag verdict=REFUSED reason=no_confirm name=$NAME sha=$s12 fields=$want"; exit 2
    fi
    git tag -a "$NAME" -F "$msg" || { echo "tag verdict=FAIL step=git_tag name=$NAME sha=$s12"; exit 1; }
    # read back: the tag object's message (after the header's blank line) and its target
    got=$(git cat-file -p "$NAME" | sed '1,/^$/d')
    same=$(diff <(printf '%s\n' "$got") "$msg" > /dev/null && echo "$want" || echo 0)
    target=$(git rev-parse "$NAME^{commit}")
    v=PASS; [ "$same" = "$want" ] && [ "$target" = "$sha" ] || v=FAIL
    printf '%s\n' "$got" | sed 's/^/   /'
    echo "tag verdict=$v name=$NAME sha=$s12 fields=$same/$want pushed=no"
    [ "$v" = PASS ]

# The six roster watchers' deterministic detectors (tools/watch); zero spend, no fix, writes nothing
watch:
    @tools/watch

# Build-cache prune (tools/prune): dry run by default; `just prune apply` removes the listed candidates
prune MODE="":
    #!/usr/bin/env bash
    set -uo pipefail
    case "$1" in
      "") exec tools/prune ;;
      apply) exec tools/prune --apply ;;
      *) echo "usage: just prune [apply]  (got: $1)" >&2; exit 2 ;;
    esac
