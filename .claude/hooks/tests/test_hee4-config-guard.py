"""hee4-config-guard: fires on a gate's own config (main tree and any worktree), quiet on code and docs.

The door rows are the hook's own (lib/config_guard.py DOORS); this test pins that each named door
fires with its own path in the context, that MultiEdit's per-edit paths are read, and that a path
outside any HEE v4 tree stays quiet.
"""
from __future__ import annotations

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _util import BUDGET_S, REPO, Cases, context, run_hook  # noqa: E402

HOOK = "hee4-config-guard.sh"


def edit(path: Path, tool: str = "Edit") -> dict:
    return {"tool_name": tool, "tool_input": {"file_path": str(path)}}


def main() -> int:
    c = Cases(HOOK)
    fire = ["tools/lint-floor.toml", "Cargo.toml", "crates/hee4-core/Cargo.toml", "deny.toml", "gate.toml", "layers.toml",
            ".cargo/config.toml", "tools/gate", "tools/lint-ratchet", "tools/advisories", "tools/tests/test_gate.py",
            "ops/checks/module_funnel.py", ".claude/settings.json", ".claude/hooks/lib/v3_guard.py"]
    for rel in fire:
        rc, out, _, dt = run_hook(HOOK, edit(REPO / rel))
        ctx = context(out) or ""
        c.check(rel, "fire", rc == 0 and f"`{rel}`:" in ctx and dt < BUDGET_S, f"rc={rc} ctx={ctx[:120]!r} t={dt:.2f}s")
    multi = {"tool_name": "MultiEdit", "tool_input": {"file_path": str(REPO / "README.md"),
             "edits": [{"file_path": str(REPO / "gate.toml")}]}}
    rc, out, _, dt = run_hook(HOOK, multi)
    c.check("MultiEdit reads per-edit paths", "fire", "`gate.toml`:" in (context(out) or ""), out[:200])
    with tempfile.TemporaryDirectory() as d:
        wt = Path(d)
        (wt / "crates").mkdir()
        (wt / "gate.toml").write_text("")
        rc, out, _, dt = run_hook(HOOK, edit(wt / "gate.toml", "Write"))
        c.check("a worktree's own gate.toml", "fire", "`gate.toml`:" in (context(out) or ""), out[:200])
    quiet = {
        "crate source": edit(REPO / "crates/hee4-core/src/lib.rs"),
        "README": edit(REPO / "README.md"),
        "a card": edit(REPO / "modules/hee4-contracts/contracts/MODULE.md"),
        "outside any tree": edit(Path("/tmp/gate.toml")),
        "Cargo.toml under a non-crate dir": edit(REPO / "migrated/x/Cargo.toml"),
        "no file_path": {"tool_name": "Edit", "tool_input": {}},
        "garbage stdin": "not json",
    }
    for name, payload in quiet.items():
        rc, out, _, dt = run_hook(HOOK, payload)
        c.check(name, "quiet", rc == 0 and out.strip() == "" and dt < BUDGET_S, f"rc={rc} out={out[:120]!r}")
    return c.finish()


if __name__ == "__main__":
    sys.exit(main())
