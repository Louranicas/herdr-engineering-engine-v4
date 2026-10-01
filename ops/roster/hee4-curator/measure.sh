#!/usr/bin/env bash
# hee4-curator measurements (code, not the agent). Arg: UTC stamp. v4 paths only.
# F138: every section names what it looked at, and prints UNMEASURED (with the reason) when its source is absent or it
# looked at nothing; a count of 0 from an absent source is never printed.
set -uo pipefail
unset -f grep 2>/dev/null   # a shell function named grep (an interactive wrapper) must not stand in for GNU grep
STAMP="${1:-UNSET}"
V4=/var/home/Louranicas/herdr-engineering-engine-v4
EV=/var/home/Louranicas/hee4-evidence
STORAGE=/var/mnt/STORAGE-10TB
VAULT=$STORAGE/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault
BACKLINKS=/var/home/Louranicas/fedora-arena/scripts/audit/backlinks.py
EXCLFILE=$V4/ops/v3-independence-exclusions.txt
V3PAT='herdr-engineering-engine-v3|hee3-evidence|hee3-worktrees|\.cache/hee3|HEE3_[A-Za-z0-9_]+\.md'

# STORAGE is a nofail mount: an unmounted disk leaves an empty directory, which every count below would read as "0".
vault_ok=1; vault_why=""
if ! mountpoint -q "$STORAGE"; then vault_ok=0; vault_why="storage_not_mounted ($STORAGE)"
elif [ ! -d "$VAULT" ]; then vault_ok=0; vault_why="vault_absent ($VAULT)"; fi

echo "measured_at_utc=$STAMP"
if [ -d "$V4" ]; then
  n=$(find "$V4" -type f -not -path '*/.git/*' | wc -l)
  if [ "$n" -gt 0 ]; then echo "v4_files=$n"; else echo "v4_files=UNMEASURED (no files under $V4)"; fi
else echo "v4_files=UNMEASURED (absent $V4)"; fi
if [ -d "$V4/modules" ]; then echo "module_cards=$(find "$V4/modules" -name MODULE.md | wc -l)"
else echo "module_cards=UNMEASURED (absent $V4/modules)"; fi
manifest() { # <label> <dir>
  if [ ! -f "$2/MANIFEST.sha256" ]; then echo "$1=UNMEASURED (absent $2/MANIFEST.sha256)"
  elif (cd "$2" && sha256sum -c MANIFEST.sha256 --quiet) >/dev/null 2>&1; then echo "$1=OK"
  else echo "$1=FAIL"; fi
}
manifest migrated_manifest "$V4/migrated/v3-b5367bc"
# World vs manifest (CN-18): every file on disk is listed, or excused here by name with a reason.
#   MANIFEST.sha256 - the manifest itself; MIGRATION.md - the mutable map (hashing it would break on every edit)
mig="$V4/migrated/v3-b5367bc"
if [ -f "$mig/MANIFEST.sha256" ]; then
  extras=$(comm -13 <(awk '{print $2}' "$mig/MANIFEST.sha256" | sed 's|^\./||;s|^\*||' | sort) <(cd "$mig" && find . -type f | sed 's|^\./||' | sort))
  unexcused=$(printf '%s\n' "$extras" | /usr/bin/grep -vxE 'MANIFEST\.sha256|MIGRATION\.md' | /usr/bin/grep -c . )
  echo "migrated_world extras=$(printf '%s\n' "$extras" | /usr/bin/grep -c .) excused=2 unexcused=$unexcused$( [ "$unexcused" = 0 ] || printf ' UNMEASURED_OR_RED:%s' "$(printf '%s' "$extras" | /usr/bin/grep -vxE 'MANIFEST\.sha256|MIGRATION\.md' | head -3 | tr '\n' ',')")"
fi
manifest reference_manifest "$EV/reference/v3-evidence-b5367bc"

# v3 independence (V4-9; CN-05 widened 2026-10-01). Scans all THREE v4 homes, not just the repo, for v3 PATHS:
# the v3 working dir in any spelling (incl. /run/host/… and the v3 vault), v3 evidence, v3 worktrees, the v3 cache homes
# (~/.cache/hee3-*) and v3 handover/restart notes (HEE3_*.md). Bare v3 identifier spellings (`hee3.control`, `HEE3_TEST_*`,
# the `hee3` wrapper name, `MEM/hee3-*` memory names) are NOT paths: they are V4-11 rename items, not independence breaches.
# Exclusions file format: `<home>:<path or glob, relative to that home> | <reason>`, home in repo|evidence|vault; a pattern
# excludes a file it matches or any file under it. A row counts (and excludes) only with a known home and a non-empty reason.
# Per-home lines are indented and say `refs=`, so the runner's floor (`v3_refs=N`, last match) reads only the total line,
# which is printed last. A home that is absent, unmounted, empty or unreadable makes the total UNMEASURED, never a 0.
V3HOMES=(repo evidence vault)
declare -A V3DIR=([repo]="$V4" [evidence]="$EV" [vault]="$VAULT")
if [ ! -f "$EXCLFILE" ]; then echo "v3_refs=UNMEASURED (absent $EXCLFILE)"
else
  declare -A EXCL=(); exrows=0; exvalid=0
  while IFS= read -r row; do
    [[ "$row" =~ ^[[:space:]]*(#|$) ]] && continue
    exrows=$((exrows + 1))
    [[ "$row" == *"|"* ]] || continue
    lhs=${row%%|*}; reason=${row#*|}
    lhs=$(printf '%s' "$lhs" | sed -E 's/^[[:space:]]+|[[:space:]]+$//g'); reason=$(printf '%s' "$reason" | sed -E 's/^[[:space:]]+|[[:space:]]+$//g')
    h=${lhs%%:*}; pat=${lhs#*:}
    [ "$h" != "$lhs" ] && [ -n "$pat" ] && [ -n "$reason" ] && [ -n "${V3DIR[$h]+x}" ] || continue
    EXCL[$h]+="$pat"$'\n'; exvalid=$((exvalid + 1))
  done < "$EXCLFILE"
  total=0; tscanned=0; measured=0; why=""; hits=""
  for h in "${V3HOMES[@]}"; do
    d=${V3DIR[$h]}
    if [ "$h" = vault ] && [ $vault_ok = 0 ]; then echo "  v3_home=$h refs=UNMEASURED $vault_why"; why+=" $h:$vault_why"; continue; fi
    if [ ! -d "$d" ]; then echo "  v3_home=$h refs=UNMEASURED (absent $d)"; why+=" $h:absent"; continue; fi
    scanned=$(cd "$d" && find . -type f -not -path './.git/*' | wc -l)
    if [ "$scanned" -eq 0 ]; then echo "  v3_home=$h refs=UNMEASURED (0 files under $d)"; why+=" $h:empty"; continue; fi
    found=$(cd "$d" && grep -r -l -s -E "$V3PAT" --exclude-dir=.git . 2>/dev/null); grc=$?
    if [ "$grc" -gt 1 ]; then echo "  v3_home=$h refs=UNMEASURED (grep rc=$grc under $d: an unreadable file)"; why+=" $h:grep_rc=$grc"; continue; fi
    mapfile -t pats < <(printf '%s' "${EXCL[$h]:-}" | grep .)
    n=0; ex=0
    while IFS= read -r f; do
      [ -n "$f" ] || continue
      f=${f#./}; skip=0
      for e in "${pats[@]}"; do case "$f" in $e*) skip=1; break;; esac; done
      if [ $skip = 1 ]; then ex=$((ex + 1)); else n=$((n + 1)); hits+="  v3_ref $h:$f"$'\n'; fi
    done <<< "$found"
    echo "  v3_home=$h refs=$n files_scanned=$scanned files_excluded=$ex exclusion_rows=${#pats[@]}"
    total=$((total + n)); tscanned=$((tscanned + scanned)); measured=$((measured + 1))
  done
  printf '%s' "$hits"
  tail="excluded_with_reasons=$exvalid excluded_rows_without_reason=$((exrows - exvalid))"
  if [ $measured -eq ${#V3HOMES[@]} ]; then echo "v3_refs=$total homes=$measured/${#V3HOMES[@]} files_scanned=$tscanned $tail"
  else echo "v3_refs=UNMEASURED (homes=$measured/${#V3HOMES[@]}:$why) refs_in_measured_homes=$total files_scanned=$tscanned $tail"; fi
fi

# Module funnel (V7 F13: its first scheduled reader). Run from the repo root, as its docstring requires; no bytecode is
# written (a .pyc constant-folds the split v3 literal, V9 §2). Only its final verdict= line is kept.
if [ ! -f "$V4/ops/checks/module_funnel.py" ]; then echo "funnel: UNMEASURED (absent $V4/ops/checks/module_funnel.py)"
else
  fv=$(cd "$V4" && PYTHONDONTWRITEBYTECODE=1 python3 ops/checks/module_funnel.py 2>&1 </dev/null | grep -E '^verdict=' | tail -1)
  echo "funnel: ${fv:-UNMEASURED (module_funnel.py printed no verdict= line)}"
fi

if [ $vault_ok = 0 ]; then echo "backlinks: UNMEASURED $vault_why"
elif [ ! -f "$BACKLINKS" ]; then echo "backlinks: UNMEASURED (absent $BACKLINKS)"
else
  b=$(python3 "$BACKLINKS" "$VAULT" 2>&1 </dev/null | grep -E 'verdict=' | tail -1)
  echo "backlinks: ${b:-UNMEASURED (backlinks.py printed no verdict= line)}"
fi

echo "links:"
if [ $vault_ok = 0 ]; then echo "links=UNMEASURED $vault_why"
else
  ok=0; total=0
  while IFS= read -r u; do
    p=${u#file://}; p=$(printf '%b' "${p//%/\\x}"); total=$((total + 1))
    if [ -e "$p" ] || [ -e "/run/host$p" ]; then ok=$((ok + 1)); echo "  OK $p"; else echo "  MISSING $p"; fi
  done < <(grep -r -h -o -E 'file://[^) >"`]+' "$VAULT" --include='*.md' 2>/dev/null | sort -u)
  if [ "$total" -eq 0 ]; then echo "links=UNMEASURED (0 file:// links found in $VAULT)"; else echo "links=$ok/$total"; fi
fi
