#!/usr/bin/env python3
"""check_pointers — every pointer in the hee4-lessons skill resolves to its home.

World: every backtick span in SKILL.md and reference/*.md. A span that matches a pointer grammar
below is a pointer and must resolve; any other span (a command, a placeholder) is counted as
`not_pointer` and ignored. Ids are resolved by PARSING their defining files, never by a list here:
  AP / EX / A / D / V4 / H    ops/checks/module_funnel.py ID_DEFS (the funnel's one door)
  Ln / Kn                     ~/hee4-evidence/learnings/PROCESS-LEARNINGS.md table rows
  S-n                         ~/hee4-evidence/learnings/FINAL-LESSONS-REVIEW.md table rows
  MM#n                        diary Reflections/Mistakes I Made.md  `## n ` / `## n -` headings
  Pn                          diary Principles/00 - Principles.md   `## Pn ·` headings
  Fn                          ~/CLAUDE.md (the always-loaded standard) as a whole word
  spell:<Name>                diary Spellbook: `**<Name>` appears
  memory:<name>               ~/.claude/projects/-var-home-Louranicas/memory/<name>.md exists
  <path>[#Heading]            the file exists; the heading text equals a Markdown heading in it
  obsidian://open?vault=V&file=F   <vaults>/V/F.md exists
Entry lines (`- trigger → lesson → home`) must carry ≥ 1 pointer in the home and a lesson of
≤ 15 words. Coverage: every AP/EX/A/D/L id defined in its home appears somewhere in the skill.
`--control` plants one bad pointer per grammar plus a long lesson in a temp copy and requires each
to be reported by name (a check that cannot fail measured nothing).
Last line: `pointers resolved=N/M … verdict=PASS|FAIL`.
"""
from __future__ import annotations

import importlib.machinery
import importlib.util
import os
import re
import shutil
import sys
import tempfile
import urllib.parse
from pathlib import Path

SKILL = Path(__file__).resolve().parents[1]
REPO = SKILL.parents[2]
HOME = Path.home()
EV = HOME / "hee4-evidence"
VAULTS = Path(os.environ.get("HEE4_VAULTS") or "/mnt/storage-10tb/fedora-obsidian-vaults")  # Omarchy; Fedora was /var/mnt/STORAGE-10TB
DIARY = VAULTS / "my-diary.vault"
MEMORY = Path(os.environ.get("HEE4_MEMORY_DIR") or HOME / ".claude/projects/-home-louranicas/memory")  # Omarchy; Fedora was -var-home-Louranicas
PL = EV / "learnings/PROCESS-LEARNINGS.md"
FLR = EV / "learnings/FINAL-LESSONS-REVIEW.md"
MISTAKES = DIARY / "Reflections/Mistakes I Made.md"
PRINCIPLES = DIARY / "Principles/00 - Principles.md"
SPELLBOOK = DIARY / "Reflections/The Spellbook and the Ember.md"
STANDARD = Path(os.environ.get("HEE4_STANDARD_MD") or HOME / "CLAUDE.md")  # absent on Omarchy (not migrated): UNMEASURED until it returns
MAX_LESSON_WORDS = 15


def _funnel():
    loader = importlib.machinery.SourceFileLoader("hee4_mf", str(REPO / "ops/checks/module_funnel.py"))
    spec = importlib.util.spec_from_loader("hee4_mf", loader)
    assert spec is not None
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


def _ids(path: Path, rx: str) -> set[str]:
    r = re.compile(rx)
    out = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        m = r.match(line)
        if m:
            out.add(str(int(m.group(1))))
    return out


def _headings(path: Path) -> set[str]:
    return {re.sub(r"^#+\s*", "", l).strip() for l in path.read_text(encoding="utf-8").splitlines()
            if re.match(r"^#{1,6}\s", l)}


class World:
    def __init__(self) -> None:
        mf = _funnel()
        self.idsets: dict[str, set[str]] = {}
        for fam, (f, rx) in mf.ID_DEFS.items():
            p = Path(f) if Path(f).is_absolute() else REPO / f
            self.idsets[fam] = _ids(p, rx)
        self.idsets["L"] = _ids(PL, r"^\| \*{0,2}L(\d+)\*{0,2} \|")
        self.idsets["K"] = _ids(PL, r"^\| \*{0,2}K(\d+)\*{0,2} \|")
        self.idsets["S"] = _ids(FLR, r"^\| S-(\d+) \|")
        self.idsets["MM"] = _ids(MISTAKES, r"^## (\d+)\b")
        self.idsets["P"] = _ids(PRINCIPLES, r"^## P(\d+) ·")
        self.standard = STANDARD.read_text(encoding="utf-8")
        self.spellbook = SPELLBOOK.read_text(encoding="utf-8")
        self.spells_world = set(re.findall(r"^\| \*\*(.+?)\*\* \|", self.spellbook, re.M))


def resolve(tok: str, w: World, base: Path) -> tuple[bool | None, str]:
    """(True ok / False broken / None not a pointer, kind)."""
    m = re.fullmatch(r"(AP|EX|D|V4|H)-(\d+)", tok) or re.fullmatch(r"(A)-(\d+)", tok)
    if m:
        return str(int(m.group(2))) in w.idsets[m.group(1)], m.group(1)
    m = re.fullmatch(r"([LKP])(\d+)", tok)
    if m:
        return str(int(m.group(2))) in w.idsets[m.group(1)], m.group(1)
    m = re.fullmatch(r"S-(\d+)", tok)
    if m:
        return str(int(m.group(1))) in w.idsets["S"], "S"
    m = re.fullmatch(r"MM#(\d+)", tok)
    if m:
        return str(int(m.group(1))) in w.idsets["MM"], "MM"
    m = re.fullmatch(r"F(\d+)", tok)
    if m:
        return re.search(rf"\bF{m.group(1)}\b", w.standard) is not None, "F"
    m = re.fullmatch(r"spell:(.+)", tok)
    if m:
        return f"**{m.group(1)}" in w.spellbook, "spell"
    m = re.fullmatch(r"memory:([\w.-]+)", tok)
    if m:
        return (MEMORY / f"{m.group(1)}.md").is_file(), "memory"
    if tok.startswith("obsidian://open?"):
        q = urllib.parse.parse_qs(urllib.parse.urlsplit(tok).query)
        v, f = q.get("vault", [""])[0], q.get("file", [""])[0].split("#")[0]
        return bool(v and f) and (VAULTS / v / f"{f}.md").is_file(), "obsidian"
    if tok.startswith(("~/", "/")) or re.fullmatch(r"(docs|reference|plan|gates|modules)/[^\s]+\.(md|toml)(#.*)?", tok):
        path, _, anchor = tok.partition("#")
        p = Path(path.replace("~/", f"{HOME}/", 1)) if path.startswith("~/") else Path(path)
        if not p.is_absolute():
            p = (base / p) if (base / p).exists() else REPO / p
        if not p.exists() or "<" in path:
            return (None, "placeholder") if "<" in path else (False, "path")
        return (anchor.strip() in _headings(p) if anchor else True), "path"
    return None, "not_pointer"


def entry_problems(line: str, w: World, base: Path) -> list[str]:
    spans = re.findall(r"`([^`]+)`", line)
    masked = re.sub(r"`[^`]+`", "\x00", line)
    parts = masked[2:].split(" → ")
    if len(parts) < 3:
        return []
    lesson_words = len(re.sub("\x00", "x", parts[-2]).split())
    home_spans = parts[-1].count("\x00")
    home_toks = spans[len(spans) - home_spans:] if home_spans else []
    probs = []
    if lesson_words > MAX_LESSON_WORDS:
        probs.append(f"lesson_too_long words={lesson_words}>{MAX_LESSON_WORDS}")
    if not any(resolve(t, w, base)[0] is not None for t in home_toks):
        probs.append("home_has_no_pointer")
    return probs


def check(skill: Path, w: World) -> tuple[list[str], dict]:
    files = [skill / "SKILL.md"] + sorted((skill / "reference").glob("*.md"))
    errors, ok, total, notp, entries = [], 0, 0, 0, 0
    text_all = ""
    for f in files:
        text = f.read_text(encoding="utf-8")
        text_all += text
        for n, line in enumerate(text.splitlines(), 1):
            for tok in re.findall(r"`([^`]+)`", line):
                res, kind = resolve(tok, w, skill)
                if res is None:
                    notp += 1
                    continue
                total += 1
                if res:
                    ok += 1
                else:
                    errors.append(f"unresolved kind={kind} token={tok!r} at {f.name}:{n}")
            if f.parent.name == "reference" and line.startswith("- ") and " → " in line:
                entries += 1
                for p in entry_problems(line, w, skill):
                    errors.append(f"{p} at {f.name}:{n}")
    cover = {}
    for fam in ("AP", "EX", "A", "D", "L"):
        want = w.idsets[fam]
        pat = {"AP": "AP-{}", "EX": "EX-{}", "A": "A-{}", "D": "D-{}", "L": "L{}"}[fam]
        spell = (lambda i: pat.format(i if fam in ("A", "L") else f"{int(i):02d}"))
        have = {i for i in want if re.search(r"`" + re.escape(spell(i)) + r"`", text_all)}
        cover[fam] = (len(have), len(want))
        for i in sorted(want - have, key=int):
            errors.append(f"uncovered id={spell(i)} (defined in its home, no pointer here)")
    spells_have = {s for s in w.spells_world if f"`spell:{s}`" in text_all}
    cover["spells"] = (len(spells_have), len(w.spells_world))
    for s in sorted(w.spells_world - spells_have):
        errors.append(f"uncovered spell={s!r}")
    stats = {"ok": ok, "total": total, "not_pointer": notp, "entries": entries, "cover": cover}
    return errors, stats


def line_for(stats: dict, errors: list[str]) -> str:
    cov = " ".join(f"{k}={a}/{b}" for k, (a, b) in stats["cover"].items())
    v = "PASS" if not errors and stats["total"] > 0 else "FAIL"
    return (f"pointers resolved={stats['ok']}/{stats['total']} entries={stats['entries']} "
            f"not_pointer={stats['not_pointer']} cover {cov} errors={len(errors)} verdict={v}")


def control(w: World) -> int:
    plants = {
        "AP": "`AP-99`", "EX": "`EX-99`", "A": "`A-99`", "D": "`D-99`", "L": "`L99`", "K": "`K99`",
        "S": "`S-99`", "MM": "`MM#999`", "P": "`P99`", "F": "`F99999`", "spell": "`spell:No Such Spell`",
        "memory": "`memory:no-such-memory`", "obsidian": "`obsidian://open?vault=my-diary.vault&file=No%20Such%20Note`",
        "path": "`~/hee4-evidence/learnings/PROCESS-LEARNINGS.md#No Such Heading`",
    }
    found = 0
    with tempfile.TemporaryDirectory() as td:
        base = Path(td) / "skill"
        shutil.copytree(SKILL, base, ignore=shutil.ignore_patterns("tests", "__pycache__"))
        clean_errors, _ = check(base, w)
        if clean_errors:
            print(f"control verdict=FAIL reason=unplanted_copy_not_clean errors={len(clean_errors)}")
            return 1
        ref = base / "reference/antipatterns.md"
        orig = ref.read_text(encoding="utf-8")
        for kind, tok in plants.items():
            ref.write_text(orig + f"\n- planted trigger → planted lesson → {tok}\n", encoding="utf-8")
            errs, _ = check(base, w)
            hit = any(f"kind={kind} " in e and tok.strip("`") in e for e in errs)
            found += hit
            if not hit:
                print(f"  control MISSED kind={kind} token={tok}")
        ref.write_text(orig + "\n- t → " + " ".join(["word"] * 16) + " → `AP-01`\n", encoding="utf-8")
        errs, _ = check(base, w)
        long_hit = any(e.startswith("lesson_too_long words=16") for e in errs)
        ref.write_text(orig.replace("→ `AP-01`\n", "→ `just verify`\n", 1), encoding="utf-8")
        errs, _ = check(base, w)
        nohome_hit = any(e.startswith("home_has_no_pointer") for e in errs)
    n = len(plants) + 2
    k = found + long_hit + nohome_hit
    v = "PASS" if k == n else "FAIL"
    print(f"control cases={k}/{n} verdict={v}")
    return 0 if v == "PASS" else 1


def main() -> int:
    try:
        w = World()
    except (OSError, AssertionError) as e:
        print(f"pointers resolved=0/0 verdict=UNMEASURED reason=world_unreadable detail={e}")
        return 30
    if "--control" in sys.argv[1:]:
        return control(w)
    errors, stats = check(SKILL, w)
    for e in errors:
        print("  " + e)
    print(line_for(stats, errors))
    return 0 if not errors and stats["total"] > 0 else 1


if __name__ == "__main__":
    sys.exit(main())
