"""hee4-session-start: the banner carries the measured lines, whole; degrades to UNMEASURED; never hangs.

Two fake worlds that differ in every field (F124: a renderer frozen to one example passes one
fixture, not two). "Quiet" for a SessionStart banner means: in a broken world it still exits 0,
writes nothing to stderr, and claims nothing it did not read (every measured line says UNMEASURED).
"""
from __future__ import annotations

import os
import stat
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _util import BUDGET_S, REPO, Cases, run_hook  # noqa: E402

HOOK = "hee4-session-start.sh"


def fake_db(d: Path, status: str, ready: str, check: str, sleep: int = 0, rc: int = 0, silent: bool = False) -> Path:
    p = d / "hee4db"
    body = f"""#!/usr/bin/env bash
sleep {sleep}
[ {int(silent)} = 1 ] && exit {rc}
case "$1" in
  readiness) echo "noise line" >&2; echo "hee4db readiness {ready}" >&2 ;;
  check) echo "hee4db check {check}" >&2 ;;
  get) echo '{{"row": {{"id": "H-5", "status": "{status}"}}}}' ;;
esac
exit {rc}
"""
    p.write_text(body)
    p.chmod(p.stat().st_mode | stat.S_IXUSR)
    return p


def lines(out: str) -> list[str]:
    return out.rstrip("\n").split("\n") if out.strip() else []


def main() -> int:
    c = Cases(HOOK)
    fixtures = [
        ("A", "open", "verdict=PASS exit=0 measured=53/53 ready_to_build=40", "verdict=PASS exit=0 measured=15/15",
         ["HEE4_HANDOVER_20261001_0900.md", "HEE4_HANDOVER_20261002_1200.md"], True),
        ("B", "closed", "verdict=FAIL exit=20 measured=7/9 ready_to_build=3", "verdict=UNMEASURED exit=30 measured=0/15",
         ["HEE4_HANDOVER_20991231_2359.md"], False),
    ]
    for tag, status, ready, check, handovers, restart in fixtures:
        with tempfile.TemporaryDirectory() as td:
            t = Path(td)
            db = fake_db(t, status, ready, check)
            hd = t / "handoffs"
            hd.mkdir()
            for i, h in enumerate(handovers):
                (hd / h).write_text("x")
                os.utime(hd / h, (1_000_000 + i, 1_000_000 + i))   # newest = last listed
            if restart:
                (hd / "HEE4_RESTART.md").write_text("x")
            rc, out, err, dt = run_hook(HOOK, {"hook_event_name": "SessionStart", "source": "startup"},
                                        env={"HEE4_HOOK_HEE4DB": str(db), "HEE4_HOOK_HANDOFFS": str(hd), "HEE4_HOOK_REPO": str(REPO)})
            L = lines(out)
            hold = ('HEE v4 session · HOLD open: H-5 "start coding" not given; no engine code (V4-0).' if status == "open"
                    else f"HEE v4 session · H-5 status={status} (read from hee4db; confirm with Luke's words before coding).")
            want = {
                0: hold,
                2: f"restart: {hd}/HEE4_RESTART.md" + ("" if restart else " (ABSENT)"),
                3: f"newest handover: {hd}/{handovers[-1]}",
                4: "first: just verify   (one verdict over every check)",
                5: f"readiness: hee4db readiness {ready}",
                6: f"check: hee4db check {check}",
            }
            bad = {i: (L[i] if i < len(L) else None, w) for i, w in want.items() if i >= len(L) or L[i] != w}
            fence_ok = len(L) > 1 and L[1].startswith('fence: habitat-scope set --session "$HABITAT_SCOPE_SESSION" --charter "HEE v4"')
            c.check(f"fixture {tag} whole lines", "fire", rc == 0 and not bad and fence_ok and len(L) <= 12 and dt < BUDGET_S,
                    f"rc={rc} n={len(L)} bad={bad} fence_ok={fence_ok} t={dt:.2f}s")

    # DB unreachable: exits 3 with no output -> UNMEASURED everywhere, no stderr, rc 0.
    with tempfile.TemporaryDirectory() as td:
        t = Path(td)
        db = fake_db(t, "", "", "", rc=3, silent=True)
        rc, out, err, dt = run_hook(HOOK, "{}", env={"HEE4_HOOK_HEE4DB": str(db), "HEE4_HOOK_HANDOFFS": str(t / "none")})
        L = lines(out)
        ok = (rc == 0 and err == "" and len(L) <= 12 and L[0].startswith("HEE v4 session · HOLD UNMEASURED")
              and L[5].startswith("readiness: UNMEASURED") and L[6].startswith("check: UNMEASURED")
              and L[3].startswith("newest handover: UNMEASURED") and L[2].endswith("(ABSENT)"))
        c.check("db unreachable + no handoffs", "quiet", ok, f"rc={rc} err={err!r} lines={L}")

    # DB hangs: every call bounded; the whole hook stays under the 2 s budget.
    with tempfile.TemporaryDirectory() as td:
        t = Path(td)
        db = fake_db(t, "open", "verdict=PASS", "verdict=PASS", sleep=10)
        rc, out, err, dt = run_hook(HOOK, "{}", env={"HEE4_HOOK_HEE4DB": str(db), "HEE4_HOOK_HANDOFFS": str(t)})
        L = lines(out)
        ok = rc == 0 and dt < BUDGET_S and L[5].startswith("readiness: UNMEASURED") and L[0].startswith("HEE v4 session · HOLD UNMEASURED")
        c.check("db hangs 10 s", "quiet", ok, f"rc={rc} t={dt:.2f}s budget={BUDGET_S}s lines={L[:1]}")

    # Missing binary and garbage stdin.
    rc, out, err, dt = run_hook(HOOK, "garbage", env={"HEE4_HOOK_HEE4DB": "/nonexistent/hee4db"})
    L = lines(out)
    c.check("missing hee4db binary", "quiet", rc == 0 and err == "" and "UNMEASURED (hee4db not executable" in (L[5] if len(L) > 5 else ""),
            f"rc={rc} lines={L}")
    return c.finish()


if __name__ == "__main__":
    sys.exit(main())
