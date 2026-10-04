"""brain-auto-index: rebuilds brain/index.md only for an Edit|Write inside the brain whose file set drifted.

"Fire": a new brain note not yet in the index -> the index is rewritten and now links it (and a removed
note disappears). "Quiet": the same drift but the edited path is outside the brain -> untouched; no drift
-> untouched byte for byte; no brain / no index / garbage stdin -> exit 0, nothing written, no stderr.
"""
from __future__ import annotations

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _util import BUDGET_S, Cases, run_hook  # noqa: E402

HOOK = "brain-auto-index.sh"


def payload(path: str, tool: str = "Write") -> dict:
    return {"hook_event_name": "PostToolUse", "tool_name": tool, "tool_input": {"file_path": path, "content": "x"},
            "tool_response": {"filePath": path, "success": True}}


def world(td: str) -> Path:
    brain = Path(td) / "brain"
    (brain / "codebase").mkdir(parents=True)
    (brain / "README.md").write_text("# README\n")
    (brain / "codebase" / "hooks-are-advisory.md").write_text("# Hooks are advisory\n")
    (brain / "index.md").write_text("# Brain\n\n## Codebase\n- [[codebase/hooks-are-advisory]]\n\n## Other\n- [[README]]\n")
    return brain


def main() -> int:
    c = Cases(HOOK)
    env = lambda b: {"HEE4_BRAIN": str(b)}  # noqa: E731

    with tempfile.TemporaryDirectory() as td:          # fire: a new note gets linked
        b = world(td)
        new = b / "codebase" / "ops-db-is-derived.md"
        new.write_text("# Ops DB is derived\n")
        rc, out, err, dt = run_hook(HOOK, payload(str(new)), env(b))
        idx = (b / "index.md").read_text()
        ok = rc == 0 and not err and dt < BUDGET_S and "- [[codebase/ops-db-is-derived]]" in idx \
            and "- [[codebase/hooks-are-advisory]]" in idx and "- [[README]]" in idx and idx.startswith("# Brain\n")
        c.check("new note under codebase/ linked", "fire", ok, f"rc={rc} err={err!r} idx={idx!r}")
        c.check("fire writes nothing to stdout", "fire", out == "", f"out={out!r}")

    with tempfile.TemporaryDirectory() as td:          # fire: a deleted note is dropped (Edit payload)
        b = world(td)
        (b / "codebase" / "hooks-are-advisory.md").unlink()
        rc, out, err, dt = run_hook(HOOK, payload(str(b / "index.md"), "Edit"), env(b))
        idx = (b / "index.md").read_text()
        c.check("removed note dropped from index", "fire",
                rc == 0 and "hooks-are-advisory" not in idx and "- [[README]]" in idx, f"rc={rc} idx={idx!r}")

    with tempfile.TemporaryDirectory() as td:          # quiet: drift exists but the edit was outside the brain
        b = world(td)
        (b / "codebase" / "stray.md").write_text("# stray\n")
        before = (b / "index.md").read_text()
        rc, out, err, dt = run_hook(HOOK, payload(str(Path(td) / "plan" / "DECISIONS.md")), env(b))
        c.check("edit outside brain leaves index alone", "quiet",
                rc == 0 and out == "" and err == "" and (b / "index.md").read_text() == before, f"rc={rc} err={err!r}")

    with tempfile.TemporaryDirectory() as td:          # quiet: no drift -> byte-identical (hand grouping survives)
        b = world(td)
        (b / "index.md").write_text("# Brain\n\n## Hand grouped\n- [[README]]\n- [[codebase/hooks-are-advisory]]\n")
        before = (b / "index.md").read_text()
        rc, out, err, dt = run_hook(HOOK, payload(str(b / "README.md")), env(b))
        c.check("no drift: index untouched byte for byte", "quiet",
                rc == 0 and out == "" and err == "" and (b / "index.md").read_text() == before, f"rc={rc} err={err!r}")

    with tempfile.TemporaryDirectory() as td:          # quiet: no index.md, no brain, garbage stdin
        b = Path(td) / "brain"
        b.mkdir()
        (b / "note.md").write_text("# n\n")
        rc, out, err, dt = run_hook(HOOK, payload(str(b / "note.md")), env(b))
        c.check("brain without index.md", "quiet", rc == 0 and out == "" and err == "" and not (b / "index.md").exists(), f"rc={rc} err={err!r}")
        rc, out, err, dt = run_hook(HOOK, payload("/x/brain/a.md"), {"HEE4_BRAIN": str(Path(td) / "absent")})
        c.check("no brain dir", "quiet", rc == 0 and out == "" and err == "", f"rc={rc} err={err!r}")
        rc, out, err, dt = run_hook(HOOK, "{not json", env(b))
        c.check("garbage stdin", "quiet", rc == 0 and out == "" and err == "", f"rc={rc} err={err!r}")
    return c.finish()


if __name__ == "__main__":
    sys.exit(main())
