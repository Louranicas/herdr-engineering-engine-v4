#!/usr/bin/env python3
"""Roster self-check: the structural drift a roster can show, measured from files alone. Zero spend: stdlib only,
no process is spawned, no model is called, no network. Same output contract as ops/checks/*.py: one line per case
(`case=<name> verdict=PASS|FAIL detail=...`), then `roster-selfcheck verdict=PASS|FAIL cases=k/n`, exit 0/1.

Seams: --root DIR (the repo; default: two levels above this file), FM_HOME and HEE4_EVIDENCE from the environment
with hee4.env's defaults, --control (each case run over a temp copy with one planted fault; the case must FAIL
there and PASS on the clean tree; prints `roster-selfcheck control cases=k/k verdict=PASS|FAIL`).
"""
from __future__ import annotations

import argparse
import hashlib
import os
import re
import shutil
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

AGENTS_DIR = Path(".claude/agents")
ROSTER_MD = AGENTS_DIR / "ROSTER.md"
PROTOCOL_MD = AGENTS_DIR / "PROTOCOL.md"
RUNNER = Path("ops/roster/run-agent.sh")
SCHEMA_001 = Path("ops/firstmate/schema/001_init.sql")
STANDING = Path("agents/standing-orders.md")
STALE_DOCS = (AGENTS_DIR / "hee4-coordinator.md", Path("plan/FIRSTMATE-ORCHESTRATION-2026-10-05.md"), Path("ops/roster/README.md"))
STALE = ("accepts only PASS", "rejects BLOCKED", "still rejects", "toolbox run", "fedora-toolbox", "crond is active")
REQUIRED_H2 = ("## Facet and rung", "## Draws from", "## Reads", "## Writes", "## Refuses", "## Report shape")
NOT_AGENT_FILES = {"PROTOCOL.md", "ROSTER.md"}
EXEMPT_H2 = "## Not in this roster, on purpose"
STANDING_ORDER_COUNT = 8            # PROTOCOL §2 / agents/standing-orders.md: eight numbered orders, Luke's
ROW_RE = re.compile(r"^\| `(hee4-[a-z-]+)`")          # first column only: `hee4-contracts` in column 2 does not count
VOCAB_RE = re.compile(r"verdict=\(([A-Z_|]+)\)")
HEADER_VOCAB_RE = re.compile(r"verdict=([A-Z_|]+)")
EXITS_IN_RE = re.compile(r"verdict IN \(([^)]*)\)")


@dataclass(frozen=True)
class World:
    root: Path
    fm_home: Path
    evidence: Path

    def read(self, rel: Path) -> str:
        return (self.root / rel).read_text(encoding="utf-8", errors="replace")


@dataclass(frozen=True)
class Result:
    name: str
    passed: bool
    detail: str

    def line(self, prefix: str = "") -> str:
        return f"{prefix}case={self.name} verdict={'PASS' if self.passed else 'FAIL'} detail={self.detail}"


def roster_rows(w: World) -> list[str]:
    return [m.group(1) for ln in w.read(ROSTER_MD).splitlines() if (m := ROW_RE.match(ln))]


def frontmatter_name(text: str) -> str | None:
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return None
    for ln in lines[1:]:
        if ln.strip() == "---":
            return None
        if ln.startswith("name:"):
            return ln.split(":", 1)[1].strip()
    return None


def agent_files(w: World) -> list[Path]:
    return sorted(p for p in (w.root / AGENTS_DIR).glob("*.md") if p.name not in NOT_AGENT_FILES)


def section(text: str, h2: str) -> str:
    """The body of one H2 section (heading matched whole, or by prefix such as `## 5`): to the next H2 or the end."""
    out, inside = [], False
    for ln in text.splitlines():
        if ln.startswith("## "):
            inside = ln.strip() == h2 or ln.startswith(h2 + " ")
            continue
        if inside:
            out.append(ln)
    return "\n".join(out)


def alternation(text: str, rx: re.Pattern[str]) -> list[str]:
    """The first `A|B|C` alternation the pattern finds (a lone `verdict=FAIL` in prose is not one)."""
    alts = [m for m in rx.findall(text) if "|" in m]
    return alts[0].split("|") if alts else []


def exits_vocab(sql: str) -> list[str]:
    i = sql.find("CREATE TABLE IF NOT EXISTS exits")
    block = sql[i:] if i >= 0 else ""
    m = EXITS_IN_RE.search(block)
    return [t.strip().strip("'") for t in m.group(1).split(",")] if m else []


def header_comment(text: str) -> str:
    """The runner's leading comment block (every `#` line before the first non-comment line)."""
    lines = []
    for ln in text.splitlines():
        if ln.startswith("#"):
            lines.append(ln)
        else:
            break
    return "\n".join(lines)


# ---- cases ---------------------------------------------------------------------------------------------------------

def case_roster_rows_have_files(w: World) -> Result:
    rows = roster_rows(w)
    bad = []
    for name in rows:
        p = w.root / AGENTS_DIR / f"{name}.md"
        if not p.is_file():
            bad.append(f"{name}:no_file")
        elif frontmatter_name(p.read_text(encoding="utf-8", errors="replace")) != name:
            bad.append(f"{name}:frontmatter_name_differs")
    return Result("roster_rows_have_files", not bad and bool(rows), f"rows={len(rows)} files={len(rows) - len(bad)}/{len(rows)} bad={','.join(bad) or 'none'}")


def case_files_not_in_roster(w: World) -> Result:
    rows = set(roster_rows(w))
    others = [p.stem for p in agent_files(w) if p.stem not in rows]
    exempt_text = section(w.read(ROSTER_MD), EXEMPT_H2)
    exempt = [n for n in others if f"`{n}`" in exempt_text]
    missing = [n for n in others if n not in exempt]
    return Result("files_not_in_roster", not missing,
                  f"not_in_roster={','.join(others) or 'none'} exempt={len(exempt)} unexplained={','.join(missing) or 'none'}")


def case_sections(w: World) -> Result:
    rows = roster_rows(w)
    bad = []
    for name in rows:
        p = w.root / AGENTS_DIR / f"{name}.md"
        if not p.is_file():
            bad.append(f"{name}:no_file")
            continue
        h2s = [ln.rstrip() for ln in p.read_text(encoding="utf-8", errors="replace").splitlines() if ln.startswith("## ")]
        lacking = [h for h in REQUIRED_H2 if h not in h2s]
        if not any(h.startswith("## Law") for h in h2s):
            lacking.append("## Law")
        if lacking:
            bad.append(f"{name}:{'|'.join(lacking)}")
    return Result("sections", not bad and bool(rows), f"files={len(rows) - len(bad)}/{len(rows)} required={len(REQUIRED_H2) + 1} bad={';'.join(bad) or 'none'}")


def case_verdict_token(w: World) -> Result:
    rows = roster_rows(w)
    bad = []
    for name in rows:
        p = w.root / AGENTS_DIR / f"{name}.md"
        token = f"`{name.removeprefix('hee4-')} verdict="
        if not p.is_file() or token not in p.read_text(encoding="utf-8", errors="replace"):
            bad.append(name)
    return Result("verdict_token", not bad and bool(rows), f"files={len(rows) - len(bad)}/{len(rows)} bad={','.join(bad) or 'none'}")


def vocab_homes(w: World) -> dict[str, list[str]]:
    runner = [ln for ln in w.read(RUNNER).splitlines() if ln.startswith("re=")]
    return {
        "protocol": alternation(section(w.read(PROTOCOL_MD), "## 5"), VOCAB_RE),
        "runner": alternation(" ".join(runner), VOCAB_RE),
        "schema": exits_vocab(w.read(SCHEMA_001)),
    }


def case_vocabulary(w: World) -> Result:
    homes = vocab_homes(w)
    sets = {k: set(v) for k, v in homes.items()}
    ref = sets["protocol"]
    agree = [k for k, s in sets.items() if s == ref and s]
    detail = f"vocab={','.join(homes['protocol']) or 'none'} homes={len(agree)}/{len(sets)}"
    if len(agree) != len(sets):
        detail += " " + " ".join(f"{k}={','.join(sorted(s)) or 'none'}" for k, s in sets.items())
    return Result("vocabulary", len(agree) == len(sets), detail)


def case_runner_header(w: World) -> Result:
    text = w.read(RUNNER)
    header = alternation(header_comment(text), HEADER_VOCAB_RE)
    regex = alternation(" ".join(ln for ln in text.splitlines() if ln.startswith("re=")), VOCAB_RE)
    ok = bool(header) and set(header) == set(regex)
    return Result("runner_header", ok, f"header={'|'.join(header) or 'none'} regex={'|'.join(regex) or 'none'}")


def case_standing_orders(w: World) -> Result:
    p = w.root / STANDING
    if not p.is_file():
        return Result("standing_orders", False, f"absent={p}")
    numbered = [int(m.group(1)) for ln in p.read_text(encoding="utf-8", errors="replace").splitlines() if (m := re.match(r"^([1-8])\. ", ln))]
    ok = numbered == list(range(1, STANDING_ORDER_COUNT + 1))
    return Result("standing_orders", ok, f"numbered={len(numbered)}/{STANDING_ORDER_COUNT} in_order={numbered == sorted(numbered)}")


def case_brief_include(w: World) -> Result:
    inc = w.fm_home / "config" / "brief-include.md"
    src = w.root / STANDING
    if not inc.is_file():
        return Result("brief_include", False, f"absent={inc} (byte copy of {STANDING} expected)")
    a, b = hashlib.sha256(inc.read_bytes()).hexdigest(), hashlib.sha256(src.read_bytes()).hexdigest() if src.is_file() else "absent"
    return Result("brief_include", a == b, f"path={inc} sha_equal={a == b} sha256={a[:12]}")


def case_protocol6_dirs(w: World) -> Result:
    dirs = {name: (w.evidence / name).is_dir() for name in ("reviews", "roster")}
    return Result("protocol6_dirs", all(dirs.values()), " ".join(f"{w.evidence / k}={'present' if v else 'ABSENT'}" for k, v in dirs.items()))


def case_stale_sentences(w: World) -> Result:
    hits = []
    for rel in STALE_DOCS:
        p = w.root / rel
        if not p.is_file():
            hits.append(f"{rel}:absent")
            continue
        for n, ln in enumerate(p.read_text(encoding="utf-8", errors="replace").splitlines(), start=1):
            for s in STALE:
                if s in ln:
                    hits.append(f"{rel}:{n}:{s!r}")
    return Result("stale_sentences", not hits, f"files={len(STALE_DOCS)} patterns={len(STALE)} hits={';'.join(hits) or 'none'}")


CASES = (case_roster_rows_have_files, case_files_not_in_roster, case_sections, case_verdict_token, case_vocabulary,
         case_runner_header, case_standing_orders, case_brief_include, case_protocol6_dirs, case_stale_sentences)


# ---- control: one planted fault per case ----------------------------------------------------------------------------

def edit(w: World, rel: Path, old: str, new: str, every: bool = False) -> None:
    p = w.root / rel
    text = p.read_text(encoding="utf-8")
    if old not in text:
        raise RuntimeError(f"control plant: {old!r} not in {rel}")
    p.write_text(text.replace(old, new, -1 if every else 1), encoding="utf-8")


def plant_roster_row_without_file(w: World) -> str:
    (w.root / AGENTS_DIR / f"{roster_rows(w)[-1]}.md").unlink()
    return "roster_row_without_file"


def plant_unexplained_file(w: World) -> str:
    rows = set(roster_rows(w))
    name = next(p.stem for p in agent_files(w) if p.stem not in rows)
    edit(w, ROSTER_MD, f"`{name}`", "`retired`")
    return f"exemption_removed={name}"


def plant_missing_refuses(w: World) -> str:
    edit(w, AGENTS_DIR / f"{roster_rows(w)[0]}.md", "## Refuses", "## Declines")
    return "refuses_h2_renamed"


def plant_no_verdict_token(w: World) -> str:
    name = roster_rows(w)[0]
    edit(w, AGENTS_DIR / f"{name}.md", f"`{name.removeprefix('hee4-')} verdict=", f"`{name.removeprefix('hee4-')} verdict ", every=True)
    return "verdict_token_broken"


def plant_runner_without_stop(w: World) -> str:
    edit(w, RUNNER, "re=\"^${NAME} verdict=(PASS_WITH_GAPS|PASS|FAIL|BLOCKED|STOP)", "re=\"^${NAME} verdict=(PASS_WITH_GAPS|PASS|FAIL|BLOCKED)")
    return "runner_regex_without_STOP"


def plant_header_drift(w: World) -> str:
    edit(w, RUNNER, "verdict=PASS|PASS_WITH_GAPS|FAIL|BLOCKED|STOP", "verdict=PASS|PASS_WITH_GAPS|FAIL")
    return "header_comment_three_verdicts"


def plant_seven_orders(w: World) -> str:
    p = w.root / STANDING
    lines = p.read_text(encoding="utf-8").splitlines()
    p.write_text("\n".join(ln for ln in lines if not ln.startswith("8. ")) + "\n", encoding="utf-8")
    return "order_8_removed"


def plant_brief_include_one_byte(w: World) -> str:
    with (w.fm_home / "config" / "brief-include.md").open("ab") as f:
        f.write(b"\n")
    return "brief_include_plus_one_byte"


def plant_no_reviews_dir(w: World) -> str:
    shutil.rmtree(w.evidence / "reviews")
    return "reviews_dir_removed"


def plant_stale_sentence(w: World) -> str:
    edit(w, STALE_DOCS[0], "## Reads", "The roster runner regex rejects BLOCKED and STOP.\n\n## Reads")
    return "rejects_BLOCKED_reinserted"


PLANTS = (plant_roster_row_without_file, plant_unexplained_file, plant_missing_refuses, plant_no_verdict_token,
          plant_runner_without_stop, plant_header_drift, plant_seven_orders, plant_brief_include_one_byte,
          plant_no_reviews_dir, plant_stale_sentence)
COPIED = (AGENTS_DIR, RUNNER, SCHEMA_001, STANDING, *STALE_DOCS)


def temp_world(src: World, base: Path, name: str) -> World:
    root = base / name
    for rel in COPIED:
        s, d = src.root / rel, root / rel
        if s.is_dir():
            shutil.copytree(s, d)
        elif s.is_file():
            d.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(s, d)
    fm_home, evidence = base / f"{name}-fm", base / f"{name}-evidence"
    (fm_home / "config").mkdir(parents=True)
    shutil.copyfile(root / STANDING, fm_home / "config" / "brief-include.md")   # the control's clean world is self-contained
    for sub in ("reviews", "roster"):
        (evidence / sub).mkdir(parents=True)
    return World(root, fm_home, evidence)


def control(src: World) -> int:
    ok = 0
    with tempfile.TemporaryDirectory(prefix="roster-selfcheck-") as tmp:
        base = Path(tmp)
        for case, plant in zip(CASES, PLANTS, strict=True):
            w = temp_world(src, base, case.__name__)
            clean = case(w)
            try:
                planted = plant(w)
            except (RuntimeError, OSError, StopIteration) as e:
                print(f"control case={clean.name} verdict=FAIL detail=plant_failed:{e}")
                continue
            faulty = case(w)
            passed = clean.passed and not faulty.passed
            ok += passed
            print(f"control case={clean.name} planted={planted} clean={'PASS' if clean.passed else 'FAIL'} "
                  f"faulty={'FAIL' if not faulty.passed else 'PASS'} verdict={'PASS' if passed else 'FAIL'}")
    print(f"roster-selfcheck control cases={ok}/{len(CASES)} verdict={'PASS' if ok == len(CASES) else 'FAIL'}")
    return 0 if ok == len(CASES) else 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent.parent)
    ap.add_argument("--control", action="store_true")
    a = ap.parse_args()
    w = World(a.root.resolve(),
              Path(os.environ.get("FM_HOME") or Path.home() / "firstmate"),
              Path(os.environ.get("HEE4_EVIDENCE") or "/mnt/storage-10tb/hee4-evidence"))
    if not (w.root / ROSTER_MD).is_file():
        print(f"roster-selfcheck verdict=FAIL cases=0/{len(CASES)} detail=no_roster_at={w.root / ROSTER_MD}")
        return 1
    if a.control:
        return control(w)
    results = [c(w) for c in CASES]
    for r in results:
        print(r.line())
    k = sum(r.passed for r in results)
    print(f"roster-selfcheck verdict={'PASS' if k == len(results) else 'FAIL'} cases={k}/{len(results)}")
    return 0 if k == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
