"""brain-inject: prints the brain index at session start, whole; silent when there is no brain.

"Quiet" means: no brain directory, or a brain without index.md, prints nothing, exits 0, no stderr.
"Fire" means: the index text appears verbatim in stdout, under the 2 s budget.
"""
from __future__ import annotations

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _util import BUDGET_S, Cases, run_hook  # noqa: E402

HOOK = "brain-inject.sh"
PAYLOAD = {"hook_event_name": "SessionStart", "source": "startup"}


def main() -> int:
    c = Cases(HOOK)
    for tag, body in (("A", "# Brain\n\n## Codebase\n- [[codebase/hooks-are-advisory]]\n"),
                      ("B", "# Brain\n\n## Other\n- [[README]]\n- [[zeta-topic]]\n")):
        with tempfile.TemporaryDirectory() as td:
            brain = Path(td) / "brain"
            brain.mkdir()
            (brain / "index.md").write_text(body)
            rc, out, err, dt = run_hook(HOOK, PAYLOAD, {"HEE4_BRAIN": str(brain)})
            ok = rc == 0 and body in out and str(brain) in out and not err and dt < BUDGET_S
            c.check(f"index {tag} printed verbatim", "fire", ok, f"rc={rc} dt={dt:.2f}s err={err!r} out={out[:120]!r}")
    with tempfile.TemporaryDirectory() as td:
        rc, out, err, dt = run_hook(HOOK, PAYLOAD, {"HEE4_BRAIN": str(Path(td) / "absent")})
        c.check("no brain dir", "quiet", rc == 0 and out == "" and err == "", f"rc={rc} out={out!r} err={err!r}")
        brain = Path(td) / "brain"
        brain.mkdir()
        (brain / "note.md").write_text("# note\n")
        rc, out, err, dt = run_hook(HOOK, PAYLOAD, {"HEE4_BRAIN": str(brain)})
        c.check("brain without index.md", "quiet", rc == 0 and out == "" and err == "", f"rc={rc} out={out!r} err={err!r}")
    rc, out, err, dt = run_hook(HOOK, "not json at all", {"HEE4_BRAIN": "/nonexistent/brain"})
    c.check("garbage stdin, no brain", "quiet", rc == 0 and out == "" and err == "", f"rc={rc} out={out!r} err={err!r}")
    return c.finish()


if __name__ == "__main__":
    sys.exit(main())
