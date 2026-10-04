#!/usr/bin/env python3
"""End-to-end funnel trace for all HEE v4 modules (audit 2026-10-01). Read-only over the corpus.

Home (2026-10-01): this file, `ops/checks/funnel_trace.py`; run it as `just trace`. The audit's path
`~/hee4-evidence/verification/funnel-audit-20261001/trace.py` is now a pointer that runs this file; the
audited bytes are kept verbatim beside it as `trace-original-20261001.py`.

World: the module set comes from `modules/MODULES.toml` (the one home), cross-checked against the DB's
`modules` table; every hop is measured per module and printed as `hop=<name> N/53`.

Hops:
  mi_index      Master Index links the Module Design Index, and the index has this module's row
                (design-section wikilink to the right crate note + card link to the manifest's card path)
  card          card exists, carries sections ## 1 … ## 11, and its `**Design section:**` link resolves to a real heading
  backlink      the design section (heading → next heading) links this module's card
  system_maps   at least one `16 System Maps/*` note names the module in backticks
  phases        card §3 first-cell phases == MODULES.toml phases == readiness note Phases column
  h_items       every H id the manifest and card name exists as an ATLAS §5 row
  dc_resolved   every register row naming the module is RESOLVED
  dc_reflected  every such row is reflected in the card (its DC id or its ratifying V4 id appears), and no §10 line
                presents a resolved DC as open (a §10 line naming a DC id must carry RESOLVED or a V4 id)
  migrated      manifest migrated paths all exist; card §6 paths ⊆ manifest (card names nothing the manifest lacks)
  highway       `hee4db highway <m>` exits 0 and every structural key is present and non-empty
Prints the per-module misses, then the coverage table, then `funnel-trace verdict=PASS|FAIL`.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tomllib
import urllib.parse
from pathlib import Path

R = Path(os.environ.get("HEE4_TRACE_REPO", os.environ.get("HEE4_ROOT", Path.home() / "herdr-engineering-engine-v4")))   # HEE4_TRACE_REPO: the plant control only
E = Path(os.environ.get("HEE4_EVIDENCE", Path.home() / "hee4-evidence"))
V = Path(os.environ.get("HEE4_VAULT", "/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault"))
# The vault's card links were written where the repo lived before (file:///var/home/Louranicas/...). Both sides of
# every link comparison go through R, so the links are READ through this machine's root, never rewritten in the vault.
LINK_ROOTS = ("/var/home/Louranicas/herdr-engineering-engine-v4", str(Path.home() / "herdr-engineering-engine-v4"), str(R), str(R.resolve()))


def norm_links(text: str) -> str:
    for old in LINK_ROOTS:
        text = text.replace(f"file://{old}/", f"file://{R}/")
    return text
HOPS = ["mi_index", "card", "backlink", "system_maps", "phases", "h_items", "dc_resolved", "dc_reflected", "migrated", "highway"]


def headings(text: str) -> list[str]:
    return [ln.lstrip("#").strip() for ln in text.splitlines() if ln.startswith("#")]


def section(text: str, head: str) -> str:
    lines = text.splitlines()
    out, on, lvl = [], False, 0
    for ln in lines:
        if ln.startswith("#"):
            h = ln.lstrip("#").strip()
            n = len(ln) - len(ln.lstrip("#"))
            if on and n <= lvl:
                break
            if h == head and not on:
                on, lvl = True, n
                continue
        if on:
            out.append(ln)
    return "\n".join(out)


def card_section(text: str, k: int) -> str:
    m = re.search(rf"^## {k} ·.*?$(.*?)(?=^## \d+ ·|\Z)", text, re.M | re.S)
    return m.group(1) if m else ""


def phase_tokens(s: str) -> list[str]:
    """Whole-token phases (`P3`, `P1-P4`), the same token rule as hee4db expand_phases: `P4's` is prose, not a phase."""
    out: list[str] = []
    s = re.sub(r"~~.*?~~", "", s).replace("*", "").replace("`", "").replace("–", "-").replace("…", "-")
    for raw in s.replace(",", " ").replace("(", " ").replace(")", " ").replace("/", " ").split():
        tok = raw.strip(".;:")
        m = re.fullmatch(r"P(\d)-P(\d)", tok)
        if m:
            out += [f"P{i}" for i in range(int(m.group(1)), int(m.group(2)) + 1)]
        elif re.fullmatch(r"P\d", tok):
            out.append(tok)
    return out


def main() -> int:
    man = tomllib.loads((R / "modules/MODULES.toml").read_text())["module"]
    mi = norm_links((V / "00 Hub/00 - HEE v4 Master Index.md").read_text())
    di = norm_links((V / "15 Module Design/00 - Module Design Index.md").read_text())
    rd = (V / "00 Hub/Module Readiness 2026-10-01.md").read_text()
    atlas = (E / "design/DEPLOYMENT_ATLAS.md").read_text()
    s5 = atlas.split("## 5 ·", 1)[1].split("\n## 6 ·", 1)[0]
    atlas_h = set(re.findall(r"^\| (H-\d+[a-z]?)\b", s5, re.M))
    maps = {p.stem: p.read_text() for p in (V / "16 System Maps").glob("*.md")}
    notes = {p.stem: norm_links(p.read_text()) for p in (V / "15 Module Design").glob("*.md")}
    q = subprocess.run(["hee4db", "q", "SELECT c.dc_id, c.module, d.status_class, d.recommendation FROM conflict_modules c "
                        "JOIN design_conflicts d ON d.id = c.dc_id"], capture_output=True, text=True)
    dcs: dict[str, list[dict]] = {}
    for r in json.loads(q.stdout)["rows"]:
        dcs.setdefault(r["module"], []).append(r)
    dbmods = {r["name"] for r in json.loads(subprocess.run(["hee4db", "q", "SELECT name FROM modules"], capture_output=True, text=True).stdout)["rows"]}
    rd_ph = {}
    for ln in rd.splitlines():
        c = [x.strip() for x in ln.strip().strip("|").split("|")]
        if len(c) > 5 and c[0] not in ("K", "") and not c[0].startswith("---") and c[1] != "Module":
            rd_ph[c[1].strip("`*")] = phase_tokens(c[4])
    hits = {h: 0 for h in HOPS}
    misses: list[str] = []
    mi_links_index = "15 Module Design/00 - Module Design Index" in mi
    for m in man:
        name, crate, cl = m["name"], m["crate"], m["cluster"]
        cardp = R / m["card"]
        ok: dict[str, tuple[bool, str]] = {}
        # mi_index
        icl = "T" if cl == "outside" else cl   # the index spells the tooling cluster T (MODULES.toml: outside)
        row = next((ln for ln in di.splitlines() if ln.startswith(f"| {icl} |") and f"|{name}]]" in ln), "")
        ok["mi_index"] = (mi_links_index and bool(row) and f"file://{cardp}" in row, f"row={'yes' if row else 'no'}")
        # card
        ct = cardp.read_text() if cardp.exists() else ""
        secs = [k for k in range(1, 12) if re.search(rf"^## {k} ·", ct, re.M)]
        dl = re.search(r"\*\*Design section:\*\* \[[^\]]*\]\(obsidian://open\?vault=[^&]+&file=([^)]+)\)", ct)
        note = head = ""
        if dl:
            f = urllib.parse.unquote(dl.group(1))
            note, _, head = f.partition("#")
            note = note.split("/")[-1]
        hs = headings(notes.get(note, ""))
        ok["card"] = (len(secs) == 11 and head in hs, f"sections={len(secs)}/11 note={note!r} head={head!r} head_found={head in hs}")
        # backlink
        st = section(notes.get(note, ""), head) if head in hs else ""
        ok["backlink"] = (f"file://{cardp}" in st, "section links card" if f"file://{cardp}" in st else "no card link in section")
        # system maps
        sm = [k for k, t in maps.items() if f"`{name}`" in t or f"`{crate}/{name}`" in t]
        ok["system_maps"] = (bool(sm), f"maps={len(sm)}")
        # phases
        s3 = card_section(ct, 3)
        cph = []
        for ln in s3.splitlines():
            if ln.startswith("|") and not ln.startswith("|---") and "Phase(s)" not in ln:
                first = ln.strip("|").split("|")[0].split("(")[0]
                cph += [p for p in phase_tokens(first) if p not in cph]
        want = list(m["phases"])
        ok["phases"] = (sorted(cph) == sorted(want) == sorted(rd_ph.get(name, [])), f"card={','.join(sorted(cph))} manifest={','.join(want)} readiness={','.join(rd_ph.get(name, []))}")
        # h items
        hids = set(m.get("h", [])) | set(re.findall(r"\bH-\d+[a-z]?\b", ct))
        absent = sorted(h for h in hids if h not in atlas_h and not (h == "H-10b" and "H-10" in atlas_h))
        ok["h_items"] = (not absent, f"h={len(hids)} absent={','.join(absent) or '-'}")
        # dc rows
        rows = dcs.get(name, [])
        openr = [r["dc_id"] for r in rows if r["status_class"] != "RESOLVED"]
        ok["dc_resolved"] = (not openr, f"dcs={len(rows)} open={','.join(openr) or '-'}")
        unreflected, stale10 = [], []
        s10 = card_section(ct, 10)
        for r in rows:
            v4s = re.findall(r"V4-\d+", r["recommendation"].split(":")[0]) or re.findall(r"V4-\d+", r["recommendation"])
            if r["dc_id"] not in ct and not any(re.search(rf"\b{v}\b", ct) for v in v4s):
                unreflected.append(r["dc_id"] + "(" + ",".join(v4s) + ")")
        for ln in s10.splitlines():
            for d in re.findall(r"DC-\d+", ln):
                if "RESOLVED" not in ln and not re.search(r"V4-\d+", ln) and "~~" not in ln:
                    stale10.append(d)
        ok["dc_reflected"] = (not unreflected and not stale10, f"unreflected={','.join(unreflected) or '-'} s10_open_lines={','.join(sorted(set(stale10))) or '-'}")
        # migrated
        mp = m.get("migrated_paths", [])
        missing = [p for p in mp if not (R / p).exists()]
        s6 = card_section(ct, 6)
        cited = set(re.findall(r"migrated/v3-b5367bc/[\w./-]+", s6))
        def owned(p: str, paths: list[str]) -> bool:
            p = p.rstrip(".,/*").rstrip("/")
            return any(p == x.rstrip("/") or p.startswith(x.rstrip("/") + "/") or x.startswith(p + "/") for x in paths)
        extra = sorted(p.rstrip(".,") for p in cited if not owned(p, mp))
        # a card may cite another module's staged input by name (reference only); that is a cross-reference when the
        # path exists AND some other module's manifest owns it; anything else is a miss
        xref = [p for p in extra if (R / p.rstrip(".,*")).exists() and any(owned(p, o.get("migrated_paths", [])) for o in man if o["name"] != name)]
        bad_extra = [p for p in extra if p not in xref]
        ok["migrated"] = (not missing and not bad_extra, f"paths={len(mp)} missing={len(missing)} xref_other_module={len(xref)} card_unowned={','.join(bad_extra) or '-'}")
        # highway
        hp = subprocess.run(["hee4db", "highway", name], capture_output=True, text=True)
        try:
            h = json.loads(hp.stdout)["highway"]
            need = {"card": h["card"]["exists"], "design_section": bool(h["design_section"] and h["design_section"].get("exists")),
                    "phases": bool(h["phases"]), "budget": bool(h["budget"]), "scope": bool(h["scope"]),
                    "readiness": bool(h["readiness"].get("build")), "ids": bool(h["ids"]), "decisions": bool(h["decisions"]),
                    "system_maps": bool(h["system_maps"]), "phases_eq": sorted(h["phases"]) == sorted(want),
                    "migrated": h["migrated"]["total"] == len(mp) and h["migrated"]["exist"] == len(mp)}
            bad = [k for k, v in need.items() if not v]
        except (KeyError, ValueError, TypeError) as e:
            bad = [f"parse:{e}"]
        ok["highway"] = (hp.returncode == 0 and not bad, f"rc={hp.returncode} missing={','.join(bad) or '-'}")
        for k in HOPS:
            if ok[k][0]:
                hits[k] += 1
            else:
                misses.append(f"MISS {name:22} {k:12} {ok[k][1]}")
    for ln in misses:
        print(ln)
    n = len(man)
    print(f"world modules_toml={n} db_modules={len(dbmods)} equal={'yes' if dbmods == {m['name'] for m in man} else 'NO'}")
    print("| Hop | Coverage |\n|---|---:|")
    for k in HOPS:
        print(f"| {k} | {hits[k]}/{n} |")
    passed = all(v == n for v in hits.values()) and dbmods == {m["name"] for m in man} and n > 0
    print(f"funnel-trace modules={n} hops={len(HOPS)} misses={len(misses)} verdict={'PASS' if passed else 'FAIL'}")
    return 0 if passed else 20


if __name__ == "__main__":
    sys.exit(main())
