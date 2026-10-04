"""context-doctor: one present|MISSING line per home and tool a fresh agent is told to use, then a verdict.

"Fire" means: an empty project root (no START.md, no homes) yields MISSING lines and verdict=GAPS, exit 0.
"Quiet" means: this repo, as re-homed, yields no MISSING line the repo controls (only the local model
endpoint may be absent on a given host), no stderr, exit 0; garbage stdin changes nothing.
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _util import REPO, Cases, run_hook  # noqa: E402

HOOK = "context-doctor.sh"
PAYLOAD = {"hook_event_name": "SessionStart", "source": "startup"}
VERDICT = re.compile(r"^context-doctor verdict=(PASS|GAPS) present=(\d+) missing=(\d+)", re.M)
DOCTOR_BUDGET_S = 5.0  # registered with timeout 5: the model probe is bounded at 1 s


def main() -> int:
    c = Cases(HOOK)
    with tempfile.TemporaryDirectory() as td:
        env = {"CLAUDE_PROJECT_DIR": td, "HEE4_EVIDENCE": f"{td}/no-evidence", "HEE4_HANDOFFS": f"{td}/no-handoffs",
               "HEE4_VAULT": f"{td}/no-vault", "HEE4_BRAIN": f"{td}/no-brain", "PATH": "/usr/bin:/bin"}
        rc, out, err, dt = run_hook(HOOK, PAYLOAD, env)
        m = VERDICT.search(out)
        missing = out.count("  MISSING  ")
        ok = rc == 0 and m is not None and m.group(1) == "GAPS" and int(m.group(3)) == missing >= 5 and dt < DOCTOR_BUDGET_S
        c.check("empty root: MISSING lines and GAPS", "fire", ok, f"rc={rc} dt={dt:.2f}s missing={missing} tail={out[-160:]!r}")
        c.check("empty root: no stderr, exit 0", "fire", rc == 0 and err == "", f"rc={rc} err={err!r}")

    rc, out, err, dt = run_hook(HOOK, PAYLOAD, {"CLAUDE_PROJECT_DIR": str(REPO)})
    m = VERDICT.search(out)
    missing_lines = [ln for ln in out.splitlines() if ln.startswith("  MISSING  ")]
    only_model = all("local model endpoint" in ln for ln in missing_lines)
    ok = rc == 0 and m is not None and err == "" and only_model and int(m.group(3)) == len(missing_lines) and dt < DOCTOR_BUDGET_S
    c.check("this repo: nothing the repo controls is MISSING", "quiet", ok,
            f"rc={rc} dt={dt:.2f}s err={err!r} missing={missing_lines!r}")
    rc2, out2, err2, _ = run_hook(HOOK, "not json at all", {"CLAUDE_PROJECT_DIR": str(REPO)})
    c.check("garbage stdin: same answer, exit 0", "quiet", rc2 == 0 and err2 == "" and VERDICT.search(out2) is not None,
            f"rc={rc2} err={err2!r}")
    return c.finish()


if __name__ == "__main__":
    sys.exit(main())
