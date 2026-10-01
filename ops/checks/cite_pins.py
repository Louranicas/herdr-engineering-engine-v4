#!/usr/bin/env python3
"""Pins for KEY:line cites in the module funnel (planning material; HOLD, V4-0).

A `KEY:n` cite is a claim about one version of the file the KEY names. When that file is edited,
lines move and every cite below the edit silently points somewhere else (2026-10-01: an ATLAS edit
shifted 502 `AT:` cites; the non-blank check saw 115 of them). A pin records, per KEY, the sha256
of the file the cites were written against, a snapshot copy, and the sorted multiset of every cite
endpoint for KEY across the cite world at pin time, so drift is loud and repair is mechanical.

The cite world and its scanner are module_funnel's (`cite_world`, `scan`, `rewrite`): modules/**/*.md
plus docs/*.md (legend keys only), fenced blocks excluded, inline code included. Every command
here reads and rewrites exactly that world.

Run from the v4 repo root:
  python3 ops/checks/cite_pins.py status            exit 0 all pinned and fresh, 1 otherwise
  python3 ops/checks/cite_pins.py pin KEY [--from SNAPSHOT] [--force-pin]
        record SNAPSHOT (default: the current file) as the version KEY's cites are written against,
        with the current cite multiset. Refused when a snapshot exists and the line map from it to
        SNAPSHOT is not the identity on every cited line (that is drift: use repin). --force-pin
        overrides, printing a warning that names the moved count. Use pin after adding or
        re-pointing cites by hand while the source is unchanged.
  python3 ops/checks/cite_pins.py repin KEY
        map every KEY cite from the pinned snapshot to the current file (equal lines exactly;
        a changed table row only to the row with the same first cell; a changed prose line to its
        most similar prose line, ratio >= 0.6), rewrite the cites, then pin. Refused when the
        current cites differ from the stored multiset (someone edited them by hand). A cite with
        an unmapped endpoint is rewritten `KEY:?<original spec>` and fails as keyline_unmapped.
  python3 ops/checks/cite_pins.py --control
        fixtures in a temp world; prints cases=N/N.

Exit: 0 ok, 1 stale/unpinned/unmapped/refused, 2 usage.
"""
from __future__ import annotations

import contextlib
import difflib
import hashlib
import io
import json
import shutil
import sys
import tempfile
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from module_funnel import (KEY_RE, LEGEND, UNPINNED_RE, cite_world, endpoints,  # noqa: E402
                           resolve, rewrite, scan, tree_digest)  # one legend, one scanner, one door

PINS_REL = Path("ops/checks/pins")
SIMILAR = 0.6  # an amended line (text appended) is still that line


class PinsUnreadable(Exception):
    pass


def sha(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def pins_dir_of(root: Path, pins_dir: Path | None) -> Path:
    return pins_dir if pins_dir is not None else root / PINS_REL


def load_pins(pd: Path) -> dict[str, dict]:
    """KEY -> {"sha256": str, "cites": list[int] | None}. A legacy bare-sha entry has cites None."""
    p = pd / "PINS.json"
    if not p.is_file():
        return {}
    try:
        raw = json.loads(p.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as e:
        raise PinsUnreadable(str(e)) from e
    if not isinstance(raw, dict):
        raise PinsUnreadable("top level is not an object")
    out: dict[str, dict] = {}
    for k, v in raw.items():
        if isinstance(v, str):
            out[k] = {"sha256": v, "cites": None}
        elif isinstance(v, dict) and isinstance(v.get("sha256"), str):
            out[k] = {"sha256": v["sha256"], "cites": v.get("cites")}
        else:
            raise PinsUnreadable(f"entry {k} has no sha256")
    return out


def save_pins(pd: Path, pins: dict[str, dict]) -> None:
    pd.mkdir(parents=True, exist_ok=True)
    (pd / "PINS.json").write_text(json.dumps(pins, indent=1, sort_keys=True) + "\n", encoding="utf-8")


def world_texts(root: Path) -> dict[Path, str]:
    return {f: f.read_text(encoding="utf-8") for f, _ in cite_world(root)}


def legend_only_of(root: Path) -> dict[Path, bool]:
    return dict(cite_world(root))


def key_cites(root: Path, key: str, texts: dict[Path, str] | None = None) -> list[int]:
    """Sorted multiset of every endpoint of every KEY cite in the world (the stored form)."""
    texts = world_texts(root) if texts is None else texts
    return sorted(n for t in texts.values() for _, m in scan(t) if m.group(1) == key
                  for n in endpoints(m.group(2)))


def cited_keys(root: Path, legend: dict[str, str]) -> set[str]:
    return {m.group(1) for t in world_texts(root).values() for _, m in scan(t) if m.group(1) in legend}


def marks(root: Path, texts: dict[Path, str] | None = None, key: str | None = None) -> list[tuple[str, int, str]]:
    """Every KEY:?<spec> mark in the world (inline code included), as (file, line, mark)."""
    texts = world_texts(root) if texts is None else texts
    return [(str(f.relative_to(root)), li, m.group(0)) for f, t in texts.items() for li, m in scan(t, UNPINNED_RE)
            if key is None or m.group(1) == key]


def status(root: Path, pins_dir: Path | None = None,
           legend: dict[str, str] | None = None) -> tuple[bool, list[str]]:
    legend = LEGEND if legend is None else legend
    pd = pins_dir_of(root, pins_dir)
    out: list[str] = []

    def diag(name: str, detail: str) -> None:
        out.append(f"  {name} {detail}")

    try:
        pins = load_pins(pd)
    except PinsUnreadable as e:
        diag("pins_unreadable", f"{pd / 'PINS.json'}: PINS.json could not be parsed: {e}")
        pins = {}
    keys = sorted(cited_keys(root, legend))
    fresh = stale = unpinned = mismatched = 0
    for k in keys:
        cur, snap = resolve(root, legend[k]), pd / f"{k}.snap"
        if k not in pins or not snap.is_file():
            unpinned += 1
            diag("pin_missing", f"{k}: no pin for {legend[k]}")
        elif sha(snap) != pins[k]["sha256"]:
            mismatched += 1
            diag("pin_snapshot_mismatch", f"{k}: snapshot sha256 {sha(snap)[:12]} != PINS.json "
                                          f"{pins[k]['sha256'][:12]}; the snapshot is not the pinned version")
        elif not cur.is_file() or sha(cur) != pins[k]["sha256"]:
            stale += 1
            diag("pin_stale", f"{k}: {legend[k]} changed since its cites were pinned; run `cite_pins.py repin {k}`")
        else:
            fresh += 1
    ms = marks(root)
    for f, li, mk in ms:
        diag("keyline_unmapped", f"{mk} at {f}:{li}: its line changed or was deleted; re-point by hand, "
                                 f"then `cite_pins.py pin KEY`")
    out.append(f"pins keys_cited={len(keys)} fresh={fresh}/{len(keys)} stale={stale} unpinned={unpinned} "
               f"snapshot_mismatch={mismatched} unmapped_cites={len(ms)}")
    return (len(out) == 1 and bool(keys)), out


def is_row(line: str) -> bool:
    return line.strip().startswith("|")


def first_cell(line: str) -> str:
    return line.strip().strip("|").split("|")[0].strip()


def ratio(a: str, b: str) -> float:
    return difflib.SequenceMatcher(None, a, b, autojunk=False).ratio()


def line_map_detail(old: list[str], new: list[str]) -> tuple[dict[int, int], set[int]]:
    """(map, similar). Equal lines map exactly. Inside a replaced block: a table row maps only to
    the ONE new row with the same stripped first cell, and only when similar; a prose line maps to
    its most similar prose line. `similar` holds the old lines mapped by similarity."""
    m: dict[int, int] = {}
    sim: set[int] = set()
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, old, new, autojunk=False).get_opcodes():
        if tag == "equal":
            for k in range(i2 - i1):
                m[i1 + k + 1] = j1 + k + 1
        elif tag == "replace":
            for i in range(i1, i2):
                if not old[i].strip():
                    continue
                if is_row(old[i]):
                    cell = first_cell(old[i])
                    cands = [j for j in range(j1, j2) if is_row(new[j]) and first_cell(new[j]) == cell]
                    if not cell or len(cands) != 1:
                        continue
                    best = cands[0]
                else:
                    cands = [j for j in range(j1, j2) if new[j].strip() and not is_row(new[j])]
                    if not cands:
                        continue
                    best = max(cands, key=lambda j: ratio(old[i], new[j]))
                if ratio(old[i], new[best]) >= SIMILAR:
                    m[i + 1] = best + 1
                    sim.add(i + 1)
    return m, sim


def line_map(old: list[str], new: list[str]) -> dict[int, int]:
    return line_map_detail(old, new)[0]


def write_pin(pd: Path, pins: dict[str, dict], key: str, src: Path, cites: list[int]) -> None:
    pd.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, pd / f"{key}.snap")
    pins[key] = {"sha256": sha(src), "cites": cites}
    save_pins(pd, pins)
    print(f"pinned {key} sha256={pins[key]['sha256']} cites={len(cites)} from={src}")


def pin(root: Path, key: str, src: Path | None, force: bool = False, pins_dir: Path | None = None,
        legend: dict[str, str] | None = None) -> int:
    legend = LEGEND if legend is None else legend
    pd = pins_dir_of(root, pins_dir)
    src = src or resolve(root, legend[key])
    if not src.is_file():
        print(f"pin refused: {key}: {src} is not a file", file=sys.stderr)
        return 1
    try:
        pins = load_pins(pd)
    except PinsUnreadable as e:
        print(f"pin refused: PINS.json could not be parsed: {e}", file=sys.stderr)
        return 1
    current = key_cites(root, key)
    snap = pd / f"{key}.snap"
    if snap.is_file():
        old = snap.read_text(encoding="utf-8").split("\n")
        mp = line_map(old, src.read_text(encoding="utf-8").split("\n"))
        cited = sorted(set(current) | set(pins.get(key, {}).get("cites") or []))
        in_snap = [n for n in cited if 1 <= n <= len(old)]
        moved = [n for n in in_snap if mp.get(n) != n]
        if moved and not force:
            print(f"pin refused: {key}: {len(moved)} of {len(in_snap)} cited lines are not the identity from the "
                  f"pinned snapshot to {src} (first: {moved[:5]}); that is drift: use repin {key}",
                  file=sys.stderr)
            return 1
        if force:
            print(f"WARNING force-pin {key}: moved={len(moved)} of cited={len(in_snap)} lines are not the identity "
                  f"from the pinned snapshot (outside_snapshot={len(cited) - len(in_snap)}); the current cites are "
                  f"now declared to be written against {src}")
    elif force:
        print(f"WARNING force-pin {key}: moved=0 (no prior snapshot); the current cites are declared to be "
              f"written against {src}")
    write_pin(pd, pins, key, src, current)
    return 0


def repin(root: Path, key: str, pins_dir: Path | None = None, legend: dict[str, str] | None = None) -> int:
    legend = LEGEND if legend is None else legend
    pd = pins_dir_of(root, pins_dir)
    snap = pd / f"{key}.snap"
    try:
        pins = load_pins(pd)
    except PinsUnreadable as e:
        print(f"repin refused: PINS.json could not be parsed: {e}", file=sys.stderr)
        return 1
    if not snap.is_file() or key not in pins:
        print(f"repin refused: {key} has no pinned snapshot", file=sys.stderr)
        return 1
    stored = pins[key]["cites"]
    if stored is None:
        print(f"repin refused: {key} has no stored cite multiset (legacy pin); check its cites against the "
              f"snapshot by hand, then `pin {key} --force-pin`", file=sys.stderr)
        return 1
    texts = world_texts(root)
    current = key_cites(root, key, texts)
    cs, cc = Counter(stored), Counter(current)
    if cs != cc:
        diff = sum((cc - cs).values()) + sum((cs - cc).values())
        print(f"repin refused: {key}: the current cites differ from the stored ones (diff={diff}: "
              f"added={sum((cc - cs).values())} removed={sum((cs - cc).values())}); they were edited by hand. "
              f"Check them; `pin {key}` if the source is unchanged, else `pin {key} --force-pin`", file=sys.stderr)
        return 1
    cur = resolve(root, legend[key])
    old = snap.read_text(encoding="utf-8").split("\n")
    new = cur.read_text(encoding="utf-8").split("\n")
    mp, sim = line_map_detail(old, new)
    moved = kept = similar = unmapped = 0
    pairs: set[tuple[int, int]] = set()

    def sub(m) -> str:  # re.Match[str]
        nonlocal moved, kept, similar, unmapped
        if m.group(1) != key:
            return m.group(0)
        spec = m.group(2)
        ends = endpoints(spec)
        if any(o not in mp for o in ends):
            unmapped += 1
            return f"{key}:?{spec}"  # the ORIGINAL spec, in snapshot coordinates
        for o in ends:
            if o in sim:
                similar += 1
                pairs.add((o, mp[o]))
            elif mp[o] == o:
                kept += 1
            else:
                moved += 1
        parts = []
        for part in spec.split(","):
            parts.append("-".join(str(mp[int(x)]) for x in part.split("-")))
        return f"{key}:{','.join(parts)}"

    new_texts = {f: rewrite(t, sub) for f, t in texts.items()}
    seen = len(marks(root, new_texts, key)) - len(marks(root, texts, key))
    if seen != unmapped:
        print(f"repin refused: {key}: wrote {unmapped} unmapped marks but the status scanner sees {seen}; "
              f"nothing written", file=sys.stderr)
        return 1
    files = 0
    for f, t in new_texts.items():
        if t != texts[f]:
            f.write_text(t, encoding="utf-8")
            files += 1
    write_pin(pd, pins, key, cur, key_cites(root, key, new_texts))
    for o, n in sorted(pairs):
        print(f"  similar {key}:{o}→{n}  old={old[o - 1].strip()[:70]!r}  new={new[n - 1].strip()[:70]!r}")
    print(f"repin {key} files_rewritten={files} endpoints moved={moved} unchanged={kept} similar={similar} "
          f"cites_unmapped={unmapped}")
    return 1 if unmapped else 0


# ---------------------------------------------------------------- control

def _run(fn, *a, **kw) -> tuple[int, str]:
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(buf):
        rc = fn(*a, **kw)
    return rc, buf.getvalue()


def control(real_root: Path) -> bool:
    guarded = [real_root / d for d in ("modules", "docs", "plan", "ops/checks/pins")]
    before = tree_digest(guarded)
    base = [f"para {i:02d}: the quick brown fox number {i:02d} jumps" for i in range(1, 31)]
    base[11] = "| R1 | alpha beta gamma delta |"
    base[12] = "| R2 | epsilon zeta eta theta |"
    base[14] = "prose line fifteen that will be amended slightly"
    lg = {"SRC": "src/S.md"}
    results: list[tuple[str, bool, str]] = []

    def world(td: Path, card: str, doc: str = "SRC:3\n") -> tuple[Path, Path, Path]:
        r = td / "w"
        (r / "modules/c/m").mkdir(parents=True)
        (r / "docs").mkdir()
        (r / "src").mkdir()
        (r / "modules/MODULES.toml").write_text("", encoding="utf-8")
        (r / "modules/c/m/MODULE.md").write_text(card, encoding="utf-8")
        (r / "docs/D.md").write_text(doc, encoding="utf-8")
        (r / "src/S.md").write_text("\n".join(base) + "\n", encoding="utf-8")
        return r, r / "modules/c/m/MODULE.md", r / "pins"

    def src_set(r: Path, lines: list[str]) -> None:
        (r / "src/S.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    shift = ["new a", "new b", "new c"] + base
    card0 = "x SRC:5 and SRC:10-11 and `SRC:7` and SRC:2,8\n```\nSRC:9\n```\n"
    card3 = "x SRC:8 and SRC:13-14 and `SRC:10` and SRC:5,11\n```\nSRC:9\n```\n"

    with tempfile.TemporaryDirectory(prefix="cite-pins-control-") as t:
        # 1 · 3-line shift maps every cite; inline code scanned; fenced cite untouched; docs repinned
        r, card, pd = world(Path(t) / "1", card0)
        _run(pin, r, "SRC", None, pins_dir=pd, legend=lg)
        src_set(r, shift)
        rc, o = _run(repin, r, "SRC", pins_dir=pd, legend=lg)
        ok = rc == 0 and card.read_text() == card3 and (r / "docs/D.md").read_text() == "SRC:6\n" \
            and "moved=7 unchanged=0 similar=0 cites_unmapped=0" in o
        results.append(("shift3_maps_all+inline_code+fence_untouched+docs", ok, o.strip().splitlines()[-1]))
        # 5 · double repin is idempotent
        snap_card, snap_pins = card.read_text(), (pd / "PINS.json").read_text()
        rc2, o2 = _run(repin, r, "SRC", pins_dir=pd, legend=lg)
        ok = rc2 == 0 and card.read_text() == snap_card and (pd / "PINS.json").read_text() == snap_pins \
            and "files_rewritten=0" in o2
        results.append(("double_repin_idempotent", ok, o2.strip().splitlines()[-1]))

        # 2 · table row whose first cell changed -> ?; a rewritten prose line -> ?;
        #     same-cell amended row and amended prose -> similar
        r, card, pd = world(Path(t) / "2", "a SRC:12 b SRC:13 c SRC:15 d SRC:20 e SRC:17\n", "")
        _run(pin, r, "SRC", None, pins_dir=pd, legend=lg)
        nl = list(base)
        nl[11] = "| R9 | alpha beta gamma delta |"
        nl[12] = "| R2 | epsilon zeta eta theta iota |"
        nl[14] = "prose line fifteen that will be amended slightly, now amended"
        nl[16] = "an unrelated sentence on storage engines"
        src_set(r, nl)
        rc, o = _run(repin, r, "SRC", pins_dir=pd, legend=lg)
        _, st = status(r, pd, lg)
        ok = rc == 1 and card.read_text() == "a SRC:?12 b SRC:13 c SRC:15 d SRC:20 e SRC:?17\n" \
            and "unchanged=1 similar=2 cites_unmapped=2" in o and "similar SRC:13→13" in o \
            and "similar SRC:15→15" in o and "'| R2 | epsilon zeta eta theta |'" in o \
            and "unmapped_cites=2" in st[-1] and any("SRC:?12 at modules/c/m/MODULE.md:1" in s for s in st)
        results.append(("row_first_cell_changed_is_?+rewritten_prose_is_?+similar_not_unchanged", ok, o.strip().splitlines()[-1]))

        # 3 · deletion -> ?; a range with an unmapped endpoint keeps its ORIGINAL spec; counts agree
        r, card, pd = world(Path(t) / "3", "a SRC:20 b SRC:19-20 c SRC:25 d `SRC:4,20`\n", "")
        _run(pin, r, "SRC", None, pins_dir=pd, legend=lg)
        src_set(r, base[:19] + base[20:])
        rc, o = _run(repin, r, "SRC", pins_dir=pd, legend=lg)
        _, st = status(r, pd, lg)
        ok = rc == 1 and card.read_text() == "a SRC:?20 b SRC:?19-20 c SRC:24 d `SRC:?4,20`\n" \
            and "cites_unmapped=3" in o and "unmapped_cites=3" in st[-1]
        results.append(("deletion_is_?+original_spec+count_equals_status", ok, o.strip().splitlines()[-1]))

        # 6 · pin over drift is refused; --force-pin names the moved count
        r, card, pd = world(Path(t) / "6", card0)
        _run(pin, r, "SRC", None, pins_dir=pd, legend=lg)
        before_pins = (pd / "PINS.json").read_text()
        src_set(r, shift)
        rc, o = _run(pin, r, "SRC", None, pins_dir=pd, legend=lg)
        ok1 = rc == 1 and "use repin SRC" in o and (pd / "PINS.json").read_text() == before_pins
        rcf, of = _run(pin, r, "SRC", None, True, pins_dir=pd, legend=lg)
        ok = ok1 and rcf == 0 and "WARNING force-pin SRC: moved=7 of cited=7" in of
        results.append(("pin_over_drift_refused+force_names_moved", ok, (o + of).strip().splitlines()[0]))

        # 7 · a hand-fixed shift followed by repin is refused (no double shift)
        r, card, pd = world(Path(t) / "7", card0)
        _run(pin, r, "SRC", None, pins_dir=pd, legend=lg)
        src_set(r, shift)
        card.write_text(card3)
        rc, o = _run(repin, r, "SRC", pins_dir=pd, legend=lg)
        ok = rc == 1 and "differ from the stored ones (diff=" in o and card.read_text() == card3
        results.append(("hand_shift_then_repin_refused", ok, o.strip().splitlines()[0]))

        # 8 · pin while the source is unchanged (new cite added by hand) is allowed and records it
        r, card, pd = world(Path(t) / "8", card0)
        _run(pin, r, "SRC", None, pins_dir=pd, legend=lg)
        card.write_text(card0 + "more SRC:21\n")
        rc, o = _run(pin, r, "SRC", None, pins_dir=pd, legend=lg)
        stored = json.loads((pd / "PINS.json").read_text())["SRC"]["cites"]
        ok = rc == 0 and stored == sorted([5, 10, 11, 7, 2, 8, 21, 3])
        results.append(("pin_unchanged_source_records_new_cites", ok, o.strip().splitlines()[-1]))

    for name, ok, detail in results:
        print(f"cite_pins control case={name} ok={'yes' if ok else 'NO'} | {detail}")
    unchanged = tree_digest(guarded) == before
    n_ok = sum(ok for _, ok, _ in results)
    good = n_ok == len(results) and unchanged
    print(f"cite_pins control cases={n_ok}/{len(results)} real_tree_unchanged={'yes' if unchanged else 'NO'} "
          f"verdict={'PASS' if good else 'FAIL'}")
    return good


def main(argv: list[str]) -> int:
    root = Path.cwd()
    if not (root / "modules" / "MODULES.toml").is_file():
        print("usage: run from the v4 repo root", file=sys.stderr)
        return 2
    a = argv[1:]
    if a == ["status"]:
        ok, out = status(root)
        print("\n".join(out))
        if not ok and len(out) == 1:
            print("  pins_unmeasured: no legend cite in the cite world; refusing")
        return 0 if ok else 1
    if a == ["--control"]:
        return 0 if control(root) else 1
    if len(a) >= 2 and a[0] == "pin" and a[1] in LEGEND:
        rest, force, src = a[2:], False, None
        if "--force-pin" in rest:
            force = True
            rest.remove("--force-pin")
        if len(rest) == 2 and rest[0] == "--from":
            src = Path(rest[1])
        elif rest:
            print(__doc__, file=sys.stderr)
            return 2
        return pin(root, a[1], src, force)
    if len(a) == 2 and a[0] == "repin" and a[1] in LEGEND:
        return repin(root, a[1])
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
