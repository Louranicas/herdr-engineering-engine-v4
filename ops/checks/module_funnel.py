#!/usr/bin/env python3
"""HEE v4 module-funnel integrity check (planning material; HOLD, V4-0).

Run from the repo root:  python3 ops/checks/module_funnel.py [--control]
Exit: 0 PASS, 1 FAIL (including a run that measured zero cards, ids, paths, cites or pins), 2 usage.

Checks (each prints its counts with their denominators):
  cards      every modules/<crate>/<module>/MODULE.md, measured_over=<cards>
  sections   the 11 numbered sections, present and in order
  lines      max / mean (rounded) / cards over 165 lines
  manifest   MODULES.toml [[module]] card set == discovered card set; migrated_paths exist
  readme     README module table: card set and phases == manifest
  v3map      every v3 module row of the module audit (MA) appears in the README coverage table
  ids        AP/EX/A/D/V4/D-U/J/E/H cited in cards and manifest resolve to a DEFINITION row
             parsed from the defining file. Fenced blocks and inline-code spans holding regex
             metacharacters are excluded, so pull-command fragments are not cites. Zero ids
             cited fails as ids_unmeasured.
  paths      ~/..., /var/..., and repo paths under docs/ plan/ gates/ migrated/ modules/ ops/
             exist; paths in the V4-11 hee4 runtime namespace are design targets (not checked).
             Zero paths checked fails as paths_unmeasured.
  v3refs     no v3 working-dir / v3-evidence / v3-worktree path in the cards, README or manifest
  keylines   every KEY:n, KEY:n-m, KEY:n,m cite in the CITE WORLD resolves through LEGEND to an
             existing, non-blank line. The cite world is modules/**/*.md (every key; unknown keys
             fail by name) plus docs/*.md (legend keys only). One scanner (`scan`) reads it:
             fenced blocks are excluded, inline code is NOT. CITE_WORLD_EXCLUSIONS names every
             file class left out, with its reason; a legend cite in any other repo .md fails as
             cite_outside_world. Zero cites fails as keyline_unmeasured.
  pins       (cite_pins.py) each cited KEY's file still has the sha256 its cites were pinned
             against, and the snapshot still has that sha256; a changed file is pin_stale until
             `cite_pins.py repin KEY`; a cite repin could not map is written KEY:?<spec> and fails
             as keyline_unmapped (file:line). A pin run that measured no key fails as
             pins_unmeasured.

--control enumerates, with `ast`, every fail("<name>", ...) site in check() and every
diag("<name>", ...) site in cite_pins.status(), and plants at least one fault per site, each in
its own temp copy of the repo with its own pins dir. A case counts only if its own diagnostic
name AND the detail text only that site produces appear in the planted run and NOT in the
unplanted copy. It prints clauses=covered/sites and refuses a site with no plant, a plant with no
site, and a name raised from two sites. Quiet cases (fenced cites, regex inline code, docs
non-legend keys) must produce no diagnostic. The real tree is hashed before and after.
"""
from __future__ import annotations

import ast
import hashlib
import json
import os
import re
import shutil
import sys
import tempfile
import tomllib
from collections.abc import Callable
from pathlib import Path

HOME = Path.home()
EV = Path(os.environ.get("HEE4_EVIDENCE", HOME / "hee4-evidence"))
REF = EV / "reference" / "v3-evidence-b5367bc"

# Source-key legend: KEY -> file. Repo-relative paths are resolved against the repo root.
LEGEND: dict[str, str] = {
    "UM": str(EV / "design/ULTRAMAP.md"),
    "AT": str(EV / "design/DEPLOYMENT_ATLAS.md"),
    "DP": str(EV / "design/DECISION_POINTS-b5367bc.md"),
    "JQ": str(EV / "design/JEV_QUESTION_RESPONSES.md"),
    "PL": str(EV / "learnings/PROCESS-LEARNINGS.md"),
    "ER": str(EV / "reference/ERRATA-v3-evidence-b5367bc.md"),
    "MA": str(REF / "module-audit-b5367bc.md"),
    "AR": str(REF / "architecture-review-b5367bc.md"),
    "CMAP": str(REF / "cluster-map-b5367bc.md"),
    "IM": str(REF / "interface-map-b5367bc.md"),
    "JM": str(REF / "jev-decision-map-b5367bc.md"),
    "RT": str(REF / "route-to-v010-b5367bc.md"),
    "RR": str(REF / "review-range-7ed3728-dfb32d5.md"),
    "V1": str(EV / "verification/V1-repo.md"),
    "V2": str(EV / "verification/V2-design.md"),
    "V3": str(EV / "verification/V3-evidence.md"),
    "V4": str(EV / "verification/V4-records.md"),
    "V5": str(EV / "verification/V5-module-cards.md"),
    "DEC": "plan/DECISIONS.md",
    "CH": "CHARTER.md",
    "REQ": "gates/REQUIREMENTS.md",
    "MIG": "migrated/v3-b5367bc/MIGRATION.md",
    "EXX": "docs/EXEMPLARS.md",
    "APX": "docs/ANTIPATTERNS.md",
}

# Id family -> (defining file, definition-row regex capturing the number).
ID_DEFS: dict[str, tuple[str, str]] = {
    "AP": ("docs/ANTIPATTERNS.md", r"^\| \*\*AP-(\d+)\*\* \|"),
    "EX": ("docs/EXEMPLARS.md", r"^\| EX-(\d+) \|"),
    "A": ("docs/EXEMPLARS.md", r"^### A-(\d+) "),
    "D": ("docs/DRIFT_AND_OVERENGINEERING.md", r"^### D-(\d+) "),
    "V4": ("plan/DECISIONS.md", r"^- \*\*V4-(\d+)\b"),
    "DU": ("plan/DECISIONS.md", r"^\| D-U(\d+) \|"),
    "J": (str(REF / "jev-decision-map-b5367bc.md"), r"^\| J(\d+) \|"),
    "E": (str(EV / "reference/ERRATA-v3-evidence-b5367bc.md"), r"^\| E(\d+) \|"),
    "H": (str(EV / "design/DEPLOYMENT_ATLAS.md"), r"^\| H-(\d+)\b"),
}
# Cite regexes. Order matters only for display; each family is disjoint by construction.
ID_CITES: dict[str, str] = {
    "AP": r"\bAP-(\d+)\b",
    "EX": r"\bEX-(\d+)\b",
    "A": r"(?<![\w-])A-(\d+)\b",
    "D": r"(?<![\w-])D-(\d+)\b",
    "V4": r"(?<![\w-])V4-(\d+)\b",
    "DU": r"(?<![\w-])D-U(\d+)\b",
    "J": r"(?<![\w-])J(\d{1,2})\b",
    "E": r"(?<![\w-])E(\d{1,2})\b",
    "H": r"(?<![\w-])H-(\d+)\b",
}
FMT = {"AP": "AP-{:02d}", "EX": "EX-{:02d}", "A": "A-{}", "D": "D-{:02d}", "V4": "V4-{}",
       "DU": "D-U{}", "J": "J{}", "E": "E{}", "H": "H-{}"}

SECTIONS = [f"## {n} · " for n in range(1, 12)]
MAX_LINES = 165
# v3 homes, assembled so this file does not itself carry the literal strings the curator's
# independence check (ops/roster/hee4-curator/measure.sh) searches for.
V3_PATTERNS = ["herdr-engineering-engine" + "-v3", "hee3" + "-evidence", "hee3" + "-worktrees"]
V3_LABELS = ["v3-working-dir", "v3-evidence", "v3-worktrees"]  # printed instead of the literals
REPO_DIRS = ("docs", "plan", "gates", "migrated", "modules", "ops")
PATH_RE = re.compile(
    r"(?:(?<![\w./~$-])(?:~/|/var/|/run/|/home/|\$HOME/)|(?<![\w./~$-])(?:" + "|".join(REPO_DIRS) + r")/)"
    r"[^\s`'\"|()<>\[\]]*"
)
SPEC = r"\d+(?:-\d+)?(?:,\d+(?:-\d+)?)*"
KEY_RE = re.compile(r"(?<![\w/.-])([A-Z][A-Z0-9]{0,4}):(" + SPEC + r")(?![\w])")
# A cite repin could not map: KEY:?<original spec>. Same key shape, so one scanner reads both.
UNPINNED_RE = re.compile(r"(?<![\w/.-])([A-Z][A-Z0-9]{0,4}):\?(" + SPEC + r")(?![\w])")
REGEX_META = set("[]()|*+?\\^$")

# The cite world: (directory, glob, legend_only). Every other repo .md must carry no legend cite,
# except the classes below; each exclusion carries its reason.
CITE_WORLD: tuple[tuple[str, str, bool], ...] = (("modules", "**/*.md", False), ("docs", "*.md", True))
CITE_WORLD_EXCLUSIONS: dict[str, str] = {
    "docs/*.md non-legend keys":
        "CM, FF, MM, SA, JU, AH, CA and the other non-legend keys in docs/ name v3-vault files outside "
        "the v4 fence; LEGEND has no path for them (printed as docs_nonlegend_cites=N)",
    "LEGEND source files outside modules/ and docs/":
        "the ~/hee4-evidence design, learnings, verification and reference files and the repo's plan/, "
        "gates/, CHARTER.md and MIGRATION.md are sources, not citers; their cites into each other are a "
        "known unchecked class (printed as unchecked_cites=N)",
}


def fence_mask(lines: list[str]) -> list[bool]:
    """True for every line that is a fence marker or inside a fenced block."""
    mask, inside = [], False
    for line in lines:
        if line.lstrip().startswith("```"):
            inside = not inside
            mask.append(True)
            continue
        mask.append(inside)
    return mask


def strip_fences(text: str) -> str:
    """Blank out fenced blocks (keeping line count)."""
    lines = text.split("\n")
    return "\n".join("" if f else ln for ln, f in zip(lines, fence_mask(lines)))


def scan(text: str, rx: re.Pattern[str] = KEY_RE) -> list[tuple[int, re.Match[str]]]:
    """THE cite scanner: (1-based line, match) for every rx match outside fenced blocks.
    Inline code is scanned. Keyline resolution, cited keys, repin and the unmapped scan all use it."""
    lines = text.split("\n")
    return [(i, m) for i, (ln, f) in enumerate(zip(lines, fence_mask(lines)), 1) if not f
            for m in rx.finditer(ln)]


def rewrite(text: str, fn: Callable[[re.Match[str]], str]) -> str:
    """Apply fn to every KEY_RE cite that `scan` would see, and to nothing else."""
    lines = text.split("\n")
    return "\n".join(ln if f else KEY_RE.sub(fn, ln) for ln, f in zip(lines, fence_mask(lines)))


def endpoints(spec: str) -> list[int]:
    """`58` -> [58]; `58-60` -> [58, 60]; `5,8-9` -> [5, 8, 9]."""
    return [int(x) for part in spec.split(",") for x in part.split("-")]


def cite_world(root: Path) -> list[tuple[Path, bool]]:
    """(file, legend_only) for every file of the cite world, in a stable order."""
    out: list[tuple[Path, bool]] = []
    for d, pattern, legend_only in CITE_WORLD:
        out.extend((p, legend_only) for p in sorted((root / d).glob(pattern)) if p.is_file())
    return out


def world_cites(root: Path, legend: dict[str, str]) -> tuple[list[tuple[Path, int, str, str]], int]:
    """In-scope cites (file, line, key, spec) and the count of excluded docs non-legend cites."""
    cites: list[tuple[Path, int, str, str]] = []
    excluded = 0
    for f, legend_only in cite_world(root):
        for li, m in scan(f.read_text(encoding="utf-8")):
            if legend_only and m.group(1) not in legend:
                excluded += 1
                continue
            cites.append((f, li, m.group(1), m.group(2)))
    return cites, excluded


def outside_world(root: Path, legend: dict[str, str]) -> tuple[list[tuple[Path, int, str]], int]:
    """Legend cites in repo .md files outside the world and its exclusions (violations), and the
    count of legend cites inside the excluded legend source files (unchecked_cites)."""
    world = {f.resolve() for f, _ in cite_world(root)}
    sources = {resolve(root, v).resolve() for v in legend.values()} - world
    unchecked = 0
    for s in sorted(sources):
        text = read_text(s)
        if text is not None:
            unchecked += sum(1 for _, m in scan(text) if m.group(1) in legend)
    bad: list[tuple[Path, int, str]] = []
    for p in sorted(root.rglob("*.md")):
        rp = p.resolve()
        if ".git" in p.relative_to(root).parts or rp in world or rp in sources:
            continue
        text = read_text(p)
        for li, m in scan(text or ""):
            if m.group(1) in legend:
                bad.append((p, li, m.group(0)))
    return bad, unchecked


def strip_regex_code(text: str) -> str:
    """Remove inline-code spans that contain regex metacharacters (pull-command fragments)."""
    def repl(m: re.Match[str]) -> str:
        return "" if REGEX_META & set(m.group(1)) else m.group(0)
    return re.sub(r"`([^`\n]+)`", repl, text)


def expand_braces(p: str) -> list[str]:
    m = re.search(r"\{([^{}]*)\}", p)
    if not m:
        return [p]
    out: list[str] = []
    for alt in m.group(1).split(","):
        out.extend(expand_braces(p[: m.start()] + alt + p[m.end():]))
    return out


def is_runtime_target(p: str) -> bool:
    """V4-11: paths in the hee4 runtime namespace are design targets under the HOLD."""
    runtime_roots = ("~/.local/", "~/.config/", "~/.cache/", "/run/", "$HOME/.local/",
                     "/var/mnt/STORAGE-10TB/hee4-")
    return p.startswith(runtime_roots) and "hee4" in p


def resolve(root: Path, p: str) -> Path:
    if p.startswith("~/"):
        return HOME / p[2:]
    if p.startswith("$HOME/"):
        return HOME / p[6:]
    if p.startswith("/"):
        return Path(p)
    return root / p


def read_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None


def read_lines(path: Path) -> list[str] | None:
    t = read_text(path)
    return None if t is None else t.split("\n")


def check(root: Path, pins_dir: Path | None = None,
          legend: dict[str, str] | None = None) -> tuple[bool, list[str]]:
    legend = LEGEND if legend is None else legend
    modules = root / "modules"
    out: list[str] = []
    fails: list[str] = []

    def fail(name: str, detail: str) -> None:
        fails.append(name)
        out.append(f"  {name} {detail}")

    cards = sorted(modules.glob("*/*/MODULE.md"))
    ncards = len(cards)
    texts = {c: c.read_text(encoding="utf-8") for c in cards}
    rel = {c: str(c.relative_to(modules)) for c in cards}
    crates: dict[str, int] = {}
    for c in cards:
        crates[c.parent.parent.name] = crates.get(c.parent.parent.name, 0) + 1
    out.append(f"cards={ncards} " + " ".join(f"{k}={v}" for k, v in sorted(crates.items())))
    out.append(f"measured_over={ncards}")
    if ncards == 0:
        fail("zero_cards", "no card was measured under modules/*/*/MODULE.md; refusing")

    # sections
    ok = 0
    for c in cards:
        heads = [ln for ln in texts[c].split("\n") if ln.startswith("## ")]
        if len(heads) == 11 and all(h.startswith(s) for h, s in zip(heads, SECTIONS)):
            ok += 1
        else:
            fail("sections_bad", f"{rel[c]}: {len(heads)} '## ' headings, not the 11 numbered sections in order")
    out.append(f"sections ok={ok}/{ncards}")

    # lines
    counts = [len(texts[c].rstrip("\n").split("\n")) for c in cards]
    over = [(rel[c], n) for c, n in zip(cards, counts) if n > MAX_LINES]
    mean = round(sum(counts) / ncards) if ncards else 0
    out.append(f"lines max={max(counts, default=0)} mean={mean} total={sum(counts)} "
               f"over{MAX_LINES}={len(over)}/{ncards}")
    for o, n in over:
        fail("card_too_long", f"{o} has {n} lines > {MAX_LINES}")

    # manifest
    man_path = modules / "MODULES.toml"
    try:
        manifest = tomllib.loads(man_path.read_text(encoding="utf-8"))["module"]
    except (OSError, KeyError, tomllib.TOMLDecodeError) as e:
        fail("manifest_unreadable", f"{man_path.name} could not be parsed: {e}")
        manifest = []
    man_cards = {m.get("card", "").removeprefix("modules/") for m in manifest}
    card_set = set(rel.values())
    only_m, only_c = sorted(man_cards - card_set), sorted(card_set - man_cards)
    out.append(f"manifest entries={len(manifest)} cards={ncards} only_manifest={len(only_m)} "
               f"only_cards={len(only_c)}")
    for x in only_m:
        fail("manifest_only", f"{x} is in the manifest but has no card")
    for x in only_c:
        fail("card_not_in_manifest", f"{x} is a card with no manifest entry")
    mp_total = mp_ok = 0
    for m in manifest:
        for p in m.get("migrated_paths", []):
            mp_total += 1
            if (root / p).exists():
                mp_ok += 1
            else:
                fail("migrated_path_missing", f"{m.get('card')}: {p} does not exist")
    out.append(f"manifest migrated_paths={mp_total} exist={mp_ok}/{mp_total}")

    # README table vs manifest (card + phases)
    readme = (modules / "README.md").read_text(encoding="utf-8") if (modules / "README.md").exists() else ""
    rows = re.findall(r"^\|[^\n]*\]\(([^)]+/MODULE\.md)\)[^\n]*\| ([^|\n]*) \|$", readme, re.M)
    man_phase = {m.get("card", "").removeprefix("modules/"): m.get("phases", []) for m in manifest}
    mism = 0
    for card, phases in rows:
        got = [p.strip() for p in phases.split(",")]
        if man_phase.get(card) != got:
            mism += 1
            fail("readme_row_mismatch", f"{card}: README phases {got} vs manifest {man_phase.get(card)}")
    out.append(f"readme_table rows={len(rows)} manifest={len(manifest)} mismatched={mism}")
    if len(rows) != len(manifest):
        fail("readme_row_count", f"{len(rows)} README table rows vs {len(manifest)} manifest entries")

    # v3 module coverage: MA rows vs README coverage table
    ma = read_lines(Path(legend["MA"])) or []
    v3mods = [m.group(1) for ln in ma if (m := re.match(r"^\| \d+ \| ([\w]+) \|", ln))]
    mapped: set[str] = set()
    in_cov = False
    table: list[str] = []
    for ln in readme.split("\n"):
        if ln.startswith("## "):
            in_cov = ln.startswith("## v3 module → card coverage")
            continue
        if in_cov and ln.startswith("|"):
            table.append(ln)
    for ln in table[2:]:  # skip header and separator rows
        cells = [x.strip() for x in ln.strip().strip("|").split("|")]
        if len(cells) >= 2 and cells[1]:
            mapped.update(x.strip() for x in cells[0].split("·"))
    unplaced = [m for m in v3mods if m not in mapped]
    out.append(f"v3_modules={len(v3mods)} mapped={len(v3mods) - len(unplaced)} unplaced={len(unplaced)}")
    for u in unplaced:
        fail("v3_module_unplaced", f"{u} (an MA module row) is not in the README coverage table")
    if not v3mods:
        fail("v3_modules_unmeasured", f"no module rows parsed from MA ({legend['MA']})")

    # ids
    defined: dict[str, set[int]] = {}
    for fam, (f, rx) in ID_DEFS.items():
        lines = read_lines(resolve(root, f))
        if lines is None:
            fail("id_source_missing", f"{fam}: {f} is unreadable")
            lines = []
        defined[fam] = {int(m.group(1)) for ln in lines if (m := re.match(rx, ln))}
    cited: dict[str, set[int]] = {k: set() for k in ID_CITES}
    where: dict[tuple[str, int], str] = {}
    for c in cards:
        t = strip_regex_code(strip_fences(texts[c]))
        for fam, rx in ID_CITES.items():
            for m in re.finditer(rx, t):
                n = int(m.group(1))
                cited[fam].add(n)
                where.setdefault((fam, n), rel[c])
    man_keys = {"ap": "AP", "ex": "EX", "anti": "A", "d": "D", "h": "H", "errata": "E"}
    for m in manifest:
        for key, fam in list(man_keys.items()) + [("v4", None)]:
            for s in m.get(key, []):
                if fam is None:
                    mm = re.fullmatch(r"V4-(\d+)", s) or re.fullmatch(r"D-U(\d+)", s)
                    f2 = "V4" if s.startswith("V4-") else "DU"
                else:
                    mm, f2 = re.fullmatch(r"\D*(\d+)", s), fam
                if mm is None:
                    fail("manifest_id_malformed", f"{m.get('card')}: {key}={s} is not an id")
                    continue
                n = int(mm.group(1))
                cited[f2].add(n)
                where.setdefault((f2, n), "MODULES.toml " + str(m.get("card")))
    total = found = 0
    for fam in ID_CITES:
        for n in sorted(cited[fam]):
            total += 1
            if n in defined[fam]:
                found += 1
            else:
                fail("id_missing", f"{FMT[fam].format(n)} (cited in {where[(fam, n)]}; "
                                   f"not defined in {ID_DEFS[fam][0]})")
    out.append(f"ids cited={total} found={found}/{total} missing={total - found} "
               + " ".join(f"{('D-U' if k == 'DU' else k)}={len(v)}" for k, v in cited.items()))
    if total == 0:
        fail("ids_unmeasured", "no AP/EX/A/D/V4/D-U/J/E/H id was cited in the cards or the manifest")

    # paths
    distinct: dict[str, str] = {}
    for c in cards:
        for m in PATH_RE.finditer(texts[c]):
            p = re.sub(r"[.,;:]+$", "", m.group(0))
            p = re.sub(r":[\d,-]+.*$", "", p)
            if p.rstrip("/") in ("", "~", "/var", "/run", "/home") or p.rstrip("/") in REPO_DIRS:
                continue
            distinct.setdefault(p, rel[c])
    runtime = checked = exist = patterns = 0
    for p, c in sorted(distinct.items()):
        if any(v in p for v in V3_PATTERNS):
            continue  # counted by v3refs
        if is_runtime_target(p):
            runtime += 1
            continue
        if "*" in p or "…" in p or "$" in p.replace("$HOME/", ""):
            patterns += 1
            continue
        for q in expand_braces(p):
            checked += 1
            if resolve(root, q).exists():
                exist += 1
            else:
                fail("path_missing", f"{q} (in {c}) does not exist")
    out.append(f"paths distinct={len(distinct)} v4_runtime_targets={runtime} (V4-11 namespace; "
               f"not checked under the HOLD) skipped_patterns={patterns} checked={checked} "
               f"exist={exist}/{checked} missing={checked - exist}")
    if checked == 0:
        fail("paths_unmeasured", f"no path was checked ({len(distinct)} distinct found in the cards)")

    # v3 refs
    v3 = 0
    scanned = cards + [modules / "README.md", man_path]
    for f in scanned:
        if not f.exists():
            continue
        for i, ln in enumerate(f.read_text(encoding="utf-8").split("\n"), 1):
            for pat, label in zip(V3_PATTERNS, V3_LABELS):
                if pat in ln:
                    v3 += 1
                    fail("v3_path_ref", f"{f.relative_to(modules)}:{i} names a v3 home ({label})")
    out.append(f"v3_path_refs={v3} files_scanned={len(scanned)}")

    # keylines: one scanner over the cite world
    cache: dict[str, list[str] | None] = {}
    n_cites = n_ok = n_blank = n_oor = n_missing = 0
    unknown: dict[str, int] = {}
    cites, docs_nonlegend = world_cites(root, legend)
    for f, li, key, spec in cites:
        name = f"{f.relative_to(root)}:{li}"
        for part in spec.split(","):
            n_cites += 1
            cite = f"{key}:{part}"
            if key not in legend:
                unknown[key] = unknown.get(key, 0) + 1
                fail("keyline_unknown_key", f"{key} ({cite} at {name}) is not a LEGEND key")
                continue
            if key not in cache:
                cache[key] = read_lines(resolve(root, legend[key]))
            lines = cache[key]
            if lines is None:
                n_missing += 1
                fail("keyline_source_missing", f"{cite} at {name} -> {legend[key]} is unreadable")
                continue
            a, _, b = part.partition("-")
            lo, hi = int(a), int(b or a)
            nlines = len(lines) - (1 if lines and lines[-1] == "" else 0)
            if lo < 1 or hi > nlines or hi < lo:
                n_oor += 1
                fail("keyline_out_of_range", f"{cite} at {name} ({legend[key]} has {nlines} lines)")
            elif not lines[lo - 1].strip() or not lines[hi - 1].strip():
                n_blank += 1
                fail("keyline_blank", f"{cite} at {name} lands on a blank line")
            else:
                n_ok += 1
    world_files = cite_world(root)
    out.append(f"cite_world files={len(world_files)} modules={sum(1 for _, lo in world_files if not lo)} "
               f"docs={sum(1 for _, lo in world_files if lo)} docs_nonlegend_cites={docs_nonlegend} (excluded)")
    bad, unchecked = outside_world(root, legend)
    for p, li, c in bad:
        fail("cite_outside_world", f"{c} at {p.relative_to(root)}:{li} is outside the cite world "
                                   f"(add the file to CITE_WORLD or an exclusion with its reason)")
    out.append(f"cite_outside_world={len(bad)} unchecked_cites={unchecked} "
               f"(legend sources, excluded: {len(CITE_WORLD_EXCLUSIONS)} exclusion classes)")
    out.append(f"keyline cites={n_cites} resolved={n_ok}/{n_cites} blank={n_blank} "
               f"out_of_range={n_oor} source_missing={n_missing} unknown_keys={len(unknown)}"
               + (" (" + ",".join(f"{k}x{v}" for k, v in sorted(unknown.items())) + ")" if unknown else "")
               + f" legend_keys={len(legend)}")
    if n_cites == 0:
        fail("keyline_unmeasured", "no KEY:line cite was found in the cite world")

    # pins: a KEY:n cite is a claim about one version of its file (cite_pins.py)
    import cite_pins  # lazy: cite_pins imports this module's scanner and LEGEND (one door)
    pins_ok, pin_lines = cite_pins.status(root, pins_dir, legend)
    pin_diags = pin_lines[:-1]
    for pl in pin_diags:
        name, _, detail = pl.strip().partition(" ")
        fail(name, detail)
    out.append(pin_lines[-1])
    if not pins_ok and not pin_diags:
        fail("pins_unmeasured", "cite_pins.status returned false with no named diagnostic (no key measured)")

    failed_checks = sorted(set(fails))
    verdict = "PASS" if not fails else "FAIL"
    out.append(f"verdict={verdict} checks_failed={len(failed_checks)} diagnostics={len(fails)}"
               + (" (" + ",".join(failed_checks) + ")" if failed_checks else ""))
    return not fails, out


# ---------------------------------------------------------------- control

def tree_digest(dirs: list[Path]) -> dict[str, str]:
    """path -> sha256 for every file under dirs (the real-tree guard)."""
    return {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
            for d in dirs for p in sorted(d.rglob("*")) if p.is_file() and "__pycache__" not in p.parts}


def changed_files(before: dict[str, str], after: dict[str, str]) -> list[str]:
    return sorted(k for k in before.keys() | after.keys() if before.get(k) != after.get(k))


def same_tree(before: dict[str, str], after: dict[str, str]) -> bool:
    return not changed_files(before, after)


def fail_sites(path: Path, func: str, caller: str) -> list[tuple[str, int]]:
    """Every caller("<name>", ...) site inside def func, as (name, line)."""
    tree = ast.parse(path.read_text(encoding="utf-8"))
    fn = next((n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name == func), None)
    if fn is None:
        return []
    return [(n.args[0].value, n.lineno) for n in ast.walk(fn)
            if isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and n.func.id == caller
            and n.args and isinstance(n.args[0], ast.Constant) and isinstance(n.args[0].value, str)]


def fired(planted: list[str], baseline: list[str], name: str, needles: list[str]) -> bool:
    """A case fires only if a `name` diagnostic carrying EVERY needle is in the planted run and
    no such diagnostic is in the unplanted run."""
    def has(ls: list[str]) -> bool:
        return any(ln.startswith(f"  {name} ") and all(n in ln for n in needles) for ln in ls)
    return has(planted) and not has(baseline)


def quiet(planted: list[str], baseline: list[str], needles: list[str]) -> bool:
    """A quiet case holds when no diagnostic absent from the baseline mentions a needle."""
    new = [ln for ln in planted if ln.startswith("  ") and ln not in baseline]
    return not any(n in ln for ln in new for n in needles)


class PlantError(Exception):
    pass


def edit(p: Path, old: str, new: str) -> None:
    t = p.read_text(encoding="utf-8")
    if t.count(old) < 1:
        raise PlantError(f"anchor {old[:40]!r} absent from {p.name}")
    p.write_text(t.replace(old, new, 1), encoding="utf-8")


CARD = "hee4-contracts/contracts/MODULE.md"


def plant_line(root: Path, line: str, rel: str = CARD) -> int:
    """Insert `line` before section 11 of a card; return its 1-based line number."""
    p = root / "modules" / rel
    t = p.read_text(encoding="utf-8")
    anchor = "\n## 11 · "
    if anchor not in t:
        raise PlantError(f"section-11 anchor absent from {rel}")
    li = t[: t.index(anchor)].count("\n") + 2  # the anchor's own newline precedes the line
    p.write_text(t.replace(anchor, "\n" + line + anchor, 1), encoding="utf-8")
    return li


def append(p: Path, s: str) -> int:
    t = p.read_text(encoding="utf-8") if p.exists() else ""
    t = t if t.endswith("\n") or not t else t + "\n"
    p.write_text(t + s + "\n", encoding="utf-8")
    return len(t.split("\n"))


def control_cases(real_root: Path, pk: str) -> list[tuple[str, str, Callable[[Path], tuple[list[str], dict[str, str] | None]]]]:
    """(site, label, plant) where plant(root) edits a temp copy and returns (needles, legend or None).
    pk is a repo-relative legend key that is cited and clean in the baseline (pin plants use it)."""
    um = read_lines(resolve(real_root, LEGEND["UM"])) or []
    blank = next((i for i in range(2, len(um)) if not um[i - 1].strip() and um[i - 2].strip()), 0)
    nb = blank - 1
    v3path = "~/" + V3_PATTERNS[1] + "/planted.md"
    man = "modules/MODULES.toml"

    def card(token: str, needles: list[str], rel: str = CARD):
        def f(r: Path):
            li = plant_line(r, "- PLANTED control line: " + token, rel)
            return [n.replace("@", f"modules/{rel}:{li}") for n in needles], None
        return f

    def legend_with(key: str, path: Path) -> dict[str, str]:
        d = dict(LEGEND)
        d[key] = str(path)
        return d

    def ma_extra(r: Path):
        p = r / "MA-planted.md"
        p.write_text((read_text(Path(LEGEND["MA"])) or "") + "| 99 | plantedmod | x |\n", encoding="utf-8")
        return ["plantedmod", "not in the README coverage table"], legend_with("MA", p)

    def ma_empty(r: Path):
        p = r / "MA-empty.md"
        p.write_text("no rows\n", encoding="utf-8")
        return ["no module rows parsed from MA"], legend_with("MA", p)

    def strip_world(rx: re.Pattern[str]):
        def f(r: Path):
            for p, _ in cite_world(r):
                p.write_text(rx.sub("", p.read_text(encoding="utf-8")), encoding="utf-8")
        return f

    def no_cites(needles: list[str]):
        def f(r: Path):
            strip_world(KEY_RE)(r)
            strip_world(UNPINNED_RE)(r)
            return needles, None
        return f

    def no_ids(r: Path):
        for c in (r / "modules").glob("*/*/MODULE.md"):
            t = c.read_text(encoding="utf-8")
            for rx in ID_CITES.values():
                t = re.sub(rx, "", t)
            c.write_text(t, encoding="utf-8")
        p = r / man
        p.write_text(re.sub(r"^(ap|ex|anti|d|h|errata|v4) = \[.*\]$", r"\1 = []",
                            p.read_text(encoding="utf-8"), flags=re.M), encoding="utf-8")
        return ["no AP/EX/A/D/V4/D-U/J/E/H id was cited"], None

    def no_paths(r: Path):
        for c in (r / "modules").glob("*/*/MODULE.md"):
            c.write_text(PATH_RE.sub("", c.read_text(encoding="utf-8")), encoding="utf-8")
        return ["no path was checked"], None

    def zero_cards(r: Path):
        for c in (r / "modules").glob("*/*/MODULE.md"):
            c.unlink()
        return ["no card was measured"], None

    def pins_json(r: Path) -> Path:
        return r / "ops/checks/pins/PINS.json"

    def drop_pin(r: Path):
        p = pins_json(r)
        d = json.loads(p.read_text(encoding="utf-8"))
        d.pop(pk)
        p.write_text(json.dumps(d), encoding="utf-8")
        return [f"{pk}: no pin for"], None

    def unmapped_inline(r: Path):
        li = plant_line(r, "- PLANTED control line: `UM:?3`")
        return ["UM:?3", f"modules/{CARD}:{li}"], None

    def unmapped_docs(r: Path):
        li = append(r / "docs/EXEMPLARS.md", "PLANTED UM:?4-6")
        return ["UM:?4-6", f"docs/EXEMPLARS.md:{li}"], None

    def docs_oor(r: Path):
        li = append(r / "docs/EXEMPLARS.md", "PLANTED UM:999999")
        return ["UM:999999", f"docs/EXEMPLARS.md:{li}", "lines)"], None

    def readme_unknown(r: Path):
        li = append(r / "modules/README.md", "PLANTED ZZQ:6")
        return ["ZZQ:6", f"modules/README.md:{li}", "is not a LEGEND key"], None

    def readme_v3(r: Path):
        li = append(r / "modules/README.md", f"PLANTED `{v3path}`")
        return [f"README.md:{li}", "names a v3 home", V3_LABELS[1]], None

    def manifest_v3(r: Path):
        edit(r / man, "[[module]]\n", f"[[module]]\nnote = \"{v3path}\"\n")
        return ["MODULES.toml:", "names a v3 home", V3_LABELS[1]], None

    cases: list[tuple[str, str, Callable[[Path], tuple[list[str], dict[str, str] | None]]]] = [
        ("zero_cards", "all cards removed", zero_cards),
        ("sections_bad", "heading 3 renamed",
         lambda r: (edit(r / "modules" / CARD, "\n## 3 · ", "\n## 3 - "),
                    ([CARD, "not the 11 numbered sections"], None))[1]),
        ("card_too_long", "200 lines appended",
         lambda r: (append(r / "modules" / CARD, "\n".join(["- filler"] * 200)),
                    ([CARD, f"lines > {MAX_LINES}"], None))[1]),
        ("manifest_unreadable", "garbage manifest",
         lambda r: ((r / man).write_text("[[module]\n", encoding="utf-8"),
                    (["MODULES.toml could not be parsed"], None))[1]),
        ("manifest_only", "ghost manifest entry",
         lambda r: (append(r / man, '[[module]]\ncard = "modules/hee4-zz/ghost/MODULE.md"'),
                    (["hee4-zz/ghost/MODULE.md", "has no card"], None))[1]),
        ("card_not_in_manifest", "card copied to a new dir",
         lambda r: ((r / "modules/hee4-zz/planted").mkdir(parents=True),
                    shutil.copyfile(r / "modules" / CARD, r / "modules/hee4-zz/planted/MODULE.md"),
                    (["hee4-zz/planted/MODULE.md", "no manifest entry"], None))[2]),
        ("migrated_path_missing", "manifest path that does not exist",
         lambda r: (edit(r / man, "migrated_paths = [", 'migrated_paths = ["migrated/PLANTED_NO_SUCH", '),
                    (["migrated/PLANTED_NO_SUCH does not exist"], None))[1]),
        ("readme_row_mismatch", "README phase changed",
         lambda r: (edit(r / "modules/README.md", "(hee4-contracts/contracts/MODULE.md) | contracts | PARTIAL | HARDEN | P0, P1, P5, P6 |",
                         "(hee4-contracts/contracts/MODULE.md) | contracts | PARTIAL | HARDEN | P0, P1, P5, P7 |"),
                    ([CARD, "README phases", "'P7'"], None))[1]),
        ("readme_row_count", "README row deleted",
         lambda r: (edit(r / "modules/README.md",
                         "| hee4-contracts | K0 | [state-enums](hee4-contracts/state-enums/MODULE.md) | recovery, store, contracts | PARTIAL | KEEP / REFACTOR | P0, P1 |\n", ""),
                    (["README table rows vs", "manifest entries"], None))[1]),
        ("v3_module_unplaced", "MA row with no coverage", ma_extra),
        ("v3_modules_unmeasured", "MA with no rows", ma_empty),
        ("id_source_missing", "D-family source deleted",
         lambda r: ((r / "docs/DRIFT_AND_OVERENGINEERING.md").unlink(),
                    (["D: docs/DRIFT_AND_OVERENGINEERING.md is unreadable"], None))[1]),
        ("manifest_id_malformed", "non-id in ap list",
         lambda r: (edit(r / man, "ap = [", 'ap = ["APX", '), (["ap=APX is not an id"], None))[1]),
        ("id_missing", "undefined AP id in a card", card("AP-99", ["AP-99", f"cited in {CARD}", "not defined in"])),
        ("id_missing", "undefined AP id in the manifest",
         lambda r: (edit(r / man, "ap = [", 'ap = ["AP-97", '),
                    (["AP-97", "cited in MODULES.toml", "not defined in"], None))[1]),
        ("ids_unmeasured", "every id removed", no_ids),
        ("path_missing", "missing docs path",
         card("`docs/NO_SUCH_FILE_PLANTED.md`", [f"docs/NO_SUCH_FILE_PLANTED.md (in {CARD}) does not exist"])),
        ("paths_unmeasured", "every path removed", no_paths),
        ("v3_path_ref", "v3 path in a card", card(f"`{v3path}`", [f"{CARD}:", "names a v3 home", V3_LABELS[1]])),
        ("v3_path_ref", "v3 path in the README", readme_v3),
        ("v3_path_ref", "v3 path in the manifest", manifest_v3),
        ("keyline_unknown_key", "unknown key in a card", card("ZZQ:5", ["ZZQ:5 at @", "is not a LEGEND key"])),
        ("keyline_unknown_key", "unknown key in the README", readme_unknown),
        ("keyline_source_missing", "REQ source deleted",
         lambda r: ((r / "gates/REQUIREMENTS.md").unlink(),
                    (["REQ:", "gates/REQUIREMENTS.md is unreadable"], None))[1]),
        ("keyline_out_of_range", "line past the end, in a card", card("UM:999999", ["UM:999999 at @", "lines)"])),
        ("keyline_out_of_range", "line past the end, in docs/", docs_oor),
        ("keyline_blank", "blank line", card(f"UM:{blank}", [f"UM:{blank} at @", "lands on a blank line"])),
        ("keyline_blank", "range whose high end is blank",
         card(f"UM:{nb}-{blank}", [f"UM:{nb}-{blank} at @", "lands on a blank line"])),
        ("keyline_blank", "second part of a multi-cite is blank",
         card(f"UM:{nb},{blank}", [f"UM:{blank} at @", "lands on a blank line"])),
        ("keyline_blank", "cite inside inline code is scanned",
         card(f"`UM:{blank}`", [f"UM:{blank} at @", "lands on a blank line"])),
        ("keyline_unmeasured", "every cite removed", no_cites(["no KEY:line cite was found"])),
        ("cite_outside_world", "legend cite in plan/PLANTED.md",
         lambda r: ((r / "plan/PLANTED.md").write_text("see UM:5\n", encoding="utf-8"),
                    (["UM:5 at plan/PLANTED.md:1", "is outside the cite world"], None))[1]),
        ("pins_unmeasured", "no key cited", no_cites(["returned false with no named diagnostic"])),
        ("pin_stale", f"{pk} source edited",
         lambda r: (append(r / LEGEND[pk], "planted line"), ([f"{pk}:", "changed since its cites were pinned"], None))[1]),
        ("pin_missing", f"{pk} pin dropped", drop_pin),
        ("pin_snapshot_mismatch", f"{pk} snapshot edited",
         lambda r: (append(r / f"ops/checks/pins/{pk}.snap", "planted"), ([f"{pk}:", "snapshot sha256"], None))[1]),
        ("pins_unreadable", "garbage PINS.json",
         lambda r: (pins_json(r).write_text("{", encoding="utf-8"), (["PINS.json could not be parsed"], None))[1]),
        ("keyline_unmapped", "KEY:?n inside inline code in a card", unmapped_inline),
        ("keyline_unmapped", "KEY:?n range in docs/", unmapped_docs),
    ]
    return cases


def quiet_cases() -> list[tuple[str, Callable[[Path], list[str]]]]:
    def fenced(r: Path) -> list[str]:
        plant_line(r, "```\nZZQ:7 UM:999998 UM:?8\n```")
        return ["ZZQ", "UM:999998", "UM:?8"]

    def regex_code(r: Path) -> list[str]:
        plant_line(r, "- PLANTED quiet: `grep 'AP-98[0-9]'`")
        return ["AP-98"]

    def docs_nonlegend(r: Path) -> list[str]:
        append(r / "docs/EXEMPLARS.md", "PLANTED ZZQ:9")
        return ["ZZQ"]

    def prose_doc(r: Path) -> list[str]:
        plant_line(r, "- PLANTED quiet: a cite repin could not map is written `KEY:?<n>`")
        return ["KEY:?"]

    return [("fenced cites are not cites", fenced), ("regex inline code is not an id", regex_code),
            ("docs non-legend key is excluded", docs_nonlegend), ("KEY:?<n> prose is not a mark", prose_doc)]


def copy_repo(src: Path, dst: Path) -> None:
    shutil.copytree(src, dst, ignore=shutil.ignore_patterns(".git", "__pycache__"))


def control(root: Path) -> bool:
    guarded = [root / d for d in ("modules", "docs", "plan", "ops/checks/pins")]
    before = tree_digest(guarded)
    here = Path(__file__).resolve().parent
    sites = fail_sites(here / "module_funnel.py", "check", "fail") + \
        fail_sites(here / "cite_pins.py", "status", "diag")
    names = [n for n, _ in sites]
    dup = sorted({n for n in names if names.count(n) > 1})
    # Self-tests of the scoring itself: a control whose scoring cannot fail is not a control.
    self_tests = [
        fired(["  x a b"], [], "x", ["a", "b"]) is True,
        fired(["  x a b"], ["  x a b"], "x", ["a", "b"]) is False,
        fired(["  x a"], [], "x", ["a", "b"]) is False,
        fired(["  y a b"], [], "x", ["a", "b"]) is False,
        quiet(["  x ZZQ"], [], ["ZZQ"]) is False,
        quiet(["  x ZZQ"], ["  x ZZQ"], ["ZZQ"]) is True,
        same_tree({"f": "a"}, {"f": "b"}) is False,
    ]
    st_ok = sum(self_tests)
    covered: set[str] = set()
    n_case_ok = 0
    q_ok = 0
    errors: list[str] = []
    with tempfile.TemporaryDirectory(prefix="module-funnel-control-") as td:
        base = Path(td) / "base"
        copy_repo(root, base)
        ok_clean, clean_lines = check(base)
        # the pin plants need a key that is cited and has no diagnostic in the baseline
        cited = {m.group(1) for f, _ in cite_world(base) for _, m in scan(f.read_text(encoding="utf-8"))}
        pk = next((k for k in sorted(LEGEND) if not LEGEND[k].startswith("/") and k in cited
                   and not any(f"{k}:" in ln for ln in clean_lines if ln.startswith("  "))), "")
        if not pk:
            print("control verdict=FAIL reason=no_clean_repo_legend_key_for_pin_plants")
            return False
        cases = control_cases(root, pk)
        planted_names = {s for s, _, _ in cases}
        no_plant = sorted(set(names) - planted_names)
        no_site = sorted(planted_names - set(names))
        for i, (site, label, plant) in enumerate(cases):
            t = Path(td) / f"case{i}"
            copy_repo(base, t)
            try:
                needles, legend = plant(t)
            except (PlantError, OSError, ValueError, KeyError) as e:
                errors.append(f"{site}/{label}: {e}")
                print(f"control case={site} plant={label!r} fired=NO plant_error={e}")
                continue
            ok_planted, lines = check(t, legend=legend)
            hit = fired(lines, clean_lines, site, needles) and not ok_planted
            n_case_ok += hit
            if hit:
                covered.add(site)
            print(f"control case={site} plant={label!r} needles={needles} fired={'yes' if hit else 'NO'}")
            shutil.rmtree(t)
        qcases = quiet_cases()
        for i, (label, plant) in enumerate(qcases):
            t = Path(td) / f"quiet{i}"
            copy_repo(base, t)
            needles = plant(t)
            _, lines = check(t)
            held = quiet(lines, clean_lines, needles)
            q_ok += held
            print(f"control quiet={label!r} needles={needles} held={'yes' if held else 'NO'}")
            shutil.rmtree(t)
        # count case: unchecked_cites is a printed number, so a cite planted in an excluded legend
        # source must move it by exactly one
        t = Path(td) / "count"
        copy_repo(base, t)
        append(t / LEGEND["CH"], "PLANTED see UM:5")
        _, lines = check(t)

        def unchecked(ls: list[str]) -> int:
            return next((int(m.group(1)) for ln in ls if (m := re.search(r"unchecked_cites=(\d+)", ln))), -1)
        count_ok = unchecked(lines) == unchecked(clean_lines) + 1 and unchecked(clean_lines) >= 0
        print(f"control count=unchecked_cites baseline={unchecked(clean_lines)} planted={unchecked(lines)} "
              f"moved_by_one={'yes' if count_ok else 'NO'}")
    after = tree_digest(guarded)
    unchanged = same_tree(before, after)
    for c in changed_files(before, after):
        print(f"control real_tree_changed {c} (the control writes only temp copies: another writer, or a defect)")
    for n in no_plant:
        print(f"control refused: site {n} has no plant")
    for n in no_site:
        print(f"control refused: plant {n} names no fail/diag site")
    for n in dup:
        print(f"control refused: {n} is raised from {names.count(n)} sites; one case cannot cover both")
    good = (not no_plant and not no_site and not dup and not errors and len(covered) == len(set(names))
            and n_case_ok == len(cases) and q_ok == len(qcases) and st_ok == len(self_tests)
            and count_ok and unchanged and bool(sites))
    print(f"control sites={len(sites)} clauses={len(covered)}/{len(set(names))} cases={n_case_ok}/{len(cases)} "
          f"quiet={q_ok}/{len(qcases)} counts={int(count_ok)}/1 self_tests={st_ok}/{len(self_tests)} "
          f"baseline_copy={'PASS' if ok_clean else 'FAIL'} real_tree_unchanged={'yes' if unchanged else 'NO'} "
          f"verdict={'PASS' if good else 'FAIL'}")
    return good


def main(argv: list[str]) -> int:
    args = argv[1:]
    if args not in ([], ["--control"]):
        print("usage: python3 ops/checks/module_funnel.py [--control]  (run from the v4 repo root)",
              file=sys.stderr)
        return 2
    root = Path.cwd()
    if not (root / "modules" / "MODULES.toml").is_file() or not (root / "plan" / "DECISIONS.md").is_file():
        print(f"usage: run from the v4 repo root (no modules/MODULES.toml under {root})", file=sys.stderr)
        return 2
    if args == ["--control"]:
        return 0 if control(root) else 1
    ok, lines = check(root)
    print("\n".join(lines))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
