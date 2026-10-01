#!/usr/bin/env python3
"""Regenerate the three generated blocks of the HEE v4 planning corpus from hee4db (HOLD, V4-0: ops tooling).

The register (vault `15 Module Design/00 - Module Design Index`) is the one home; these blocks are its
generated projections, and `hee4db check` fails when a block and the register disagree
(`alignment_atlas`, `alignment_um`, `readiness_note`). This helper only moves generator output into
place; it decides nothing. Run it after `hee4db ingest` (the generators read the DB); `just regen` runs
the whole sequence.

  block       target                                              generator
  atlas_s9    ATLAS: the table under `## 9 · Phase entry`          hee4db highway --phase-table --md
  ultramap_s7 ULTRAMAP: between the hee4db:um-amendments markers  hee4db highway --um-amendments --md
  readiness   vault `00 Hub/Module Readiness 2026-10-01.md`       hee4db readiness --md   (whole file)

Usage (repo root):  python3 ops/checks/regen.py [--check]
  --check  write nothing; exit 1 if any block would change (the idempotence read-back).
Exit: 0 ok · 1 --check found a block that would change · 3 refused (generator failed, anchor missing
or ambiguous, generator output malformed). A file is rewritten only when its bytes change, atomically.
"""
from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from module_funnel import LEGEND, resolve  # noqa: E402  one legend: AT and UM resolve where the cites do

REPO = Path(__file__).resolve().parents[2]
HEE4DB = REPO / "ops/db/hee4db"
VAULT = Path(os.environ.get(
    "HEE4DB_VAULT", "/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault"))
ATLAS_HEADING = "## 9 · Phase entry"
UM_BEGIN, UM_END = "<!-- hee4db:um-amendments:begin -->", "<!-- hee4db:um-amendments:end -->"
READINESS_NOTE = VAULT / "00 Hub" / "Module Readiness 2026-10-01.md"
GEN_TIMEOUT_S = 120
GEN_BYTES_CAP = 1 << 20


class Refused(Exception):
    pass


def generate(*args: str) -> str:
    p = subprocess.run([str(HEE4DB), *args], stdin=subprocess.DEVNULL, capture_output=True,
                       timeout=GEN_TIMEOUT_S, check=False)
    verdict = (p.stderr.decode("utf-8", "replace").strip().splitlines() or ["(no stderr)"])[-1]
    if p.returncode != 0:
        raise Refused(f"generator_failed argv=hee4db {' '.join(args)} rc={p.returncode} last_stderr={verdict!r}")
    if len(p.stdout) > GEN_BYTES_CAP:
        raise Refused(f"generator_output_cap argv=hee4db {' '.join(args)} bytes>{GEN_BYTES_CAP}")
    return p.stdout.decode("utf-8")


def replace_atlas_table(text: str, table: str) -> str:
    rows = table.strip("\n").split("\n")
    if not rows or rows == [""] or not all(r.startswith("|") for r in rows):
        raise Refused(f"atlas_table_malformed rows={len(rows)} (generator must print only table rows)")
    lines = text.split("\n")
    heads = [i for i, ln in enumerate(lines) if ln.startswith(ATLAS_HEADING)]
    if len(heads) != 1:
        raise Refused(f"atlas_heading count={len(heads)} want=1 heading={ATLAS_HEADING!r}")
    start = None
    for i in range(heads[0] + 1, len(lines)):
        if lines[i].startswith("## "):
            break
        if lines[i].startswith("|"):
            start = i
            break
    if start is None:
        raise Refused("atlas_table_absent under its heading")
    end = start
    while end < len(lines) and lines[end].startswith("|"):
        end += 1
    return "\n".join(lines[:start] + rows + lines[end:])


def replace_um_block(text: str, body: str) -> str:
    nb, ne = text.count(UM_BEGIN), text.count(UM_END)
    if nb != 1 or ne != 1:
        raise Refused(f"um_markers begin={nb} end={ne} want=1/1")
    b, e = text.index(UM_BEGIN), text.index(UM_END)
    if e < b:
        raise Refused("um_markers end precedes begin")
    inner = body.strip("\n")
    return text[: b + len(UM_BEGIN)] + "\n" + (inner + "\n" if inner else "") + text[e:]


def write_if_changed(path: Path, new: str, check: bool) -> bool:
    old = path.read_text(encoding="utf-8") if path.is_file() else None
    if old == new:
        return False
    if not check:
        fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.", suffix=".tmp")
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                f.write(new)
                f.flush()
                os.fsync(f.fileno())
            os.replace(tmp, path)
        except BaseException:
            Path(tmp).unlink(missing_ok=True)
            raise
        if path.read_text(encoding="utf-8") != new:  # read back
            raise Refused(f"readback_mismatch {path}")
    return True


def main(argv: list[str]) -> int:
    check = argv == ["--check"]
    if argv and not check:
        print(__doc__, file=sys.stderr)
        return 2
    atlas, um = resolve(REPO, LEGEND["AT"]), resolve(REPO, LEGEND["UM"])
    jobs = [
        ("atlas_s9", atlas, lambda t: replace_atlas_table(t, generate("highway", "--phase-table", "--md"))),
        ("ultramap_s7", um, lambda t: replace_um_block(t, generate("highway", "--um-amendments", "--md"))),
        ("readiness", READINESS_NOTE, lambda _t: generate("readiness", "--md")),
    ]
    changed = 0
    try:
        for name, path, fn in jobs:
            if not path.parent.is_dir():
                raise Refused(f"target_dir_absent {path.parent}")
            cur = path.read_text(encoding="utf-8") if path.is_file() else ""
            did = write_if_changed(path, fn(cur), check)
            changed += did
            print(f"regen block={name} changed={'would' if did and check else 'yes' if did else 'no'} target={path}")
    except (Refused, subprocess.TimeoutExpired, OSError, UnicodeDecodeError) as e:
        print(f"regen-blocks verdict=REFUSED reason={e}")
        return 3
    mode = "check" if check else "write"
    if check and changed:
        print(f"regen-blocks verdict=FAIL mode=check blocks=3/3 would_change={changed}")
        return 1
    print(f"regen-blocks verdict=PASS mode={mode} blocks=3/3 changed={0 if check else changed}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
