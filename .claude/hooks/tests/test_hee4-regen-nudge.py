"""hee4-regen-nudge: fires with the EXACT follow-up list on derived-from files, quiet elsewhere.

Expected lists come from reading ops/checks/module_funnel.py LEGEND and ops/db/hee4db WORLD_FILES /
WORLD_GLOBS / EXCLUDED (2026-10-01), not from regen_nudge.py. Whole-list equality, not `contains`
(F124). A fake legend proves the KEY is derived, not hard-coded.
"""
from __future__ import annotations

import os
import re
import sys
import tempfile
from dataclasses import replace
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from _util import BUDGET_S, REPO, Cases, context, run_hook  # noqa: E402
import regen_nudge  # noqa: E402

HOOK = "hee4-regen-nudge.sh"
HOME = Path.home()
EV = Path(os.environ.get("HEE4_EVIDENCE", HOME / "hee4-evidence"))
VAULT = Path(os.environ.get("HEE4_VAULT", "/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault"))


def steps_of(stdout: str) -> list[str] | None:
    ctx = context(stdout)
    if ctx is None:
        return None
    return re.findall(r"`([^`]+)`", ctx.split(": run ", 1)[1].split(" (derived", 1)[0])


def main() -> int:
    c = Cases(HOOK)
    cards = sorted(REPO.glob("modules/*/*/MODULE.md"))
    c.check("a card exists to test with", "setup", bool(cards), "no modules/*/*/MODULE.md")
    fire = {
        "design-conflict register": (VAULT / "15 Module Design/00 - Module Design Index.md", ["just regen", "just verify"]),
        "ATLAS (AT + ingested)": (EV / "design/DEPLOYMENT_ATLAS.md", ["just repin AT", "hee4db ingest", "just verify"]),
        "ULTRAMAP (UM, excluded from ingest)": (EV / "design/ULTRAMAP.md", ["just repin UM", "just verify"]),
        "JQ (excluded from ingest)": (EV / "design/JEV_QUESTION_RESPONSES.md", ["just repin JQ", "just verify"]),
        "DECISIONS (DEC + ingested)": (REPO / "plan/DECISIONS.md", ["just repin DEC", "hee4db ingest", "just verify"]),
        "REQUIREMENTS (REQ only)": (REPO / "gates/REQUIREMENTS.md", ["just repin REQ", "just verify"]),
        "ANTIPATTERNS (APX + docs glob)": (REPO / "docs/ANTIPATTERNS.md", ["just repin APX", "hee4db ingest", "just verify"]),
        "MODULES.toml": (REPO / "modules/MODULES.toml", ["hee4db ingest", "just verify"]),
        "a module card": (cards[0] if cards else REPO / "missing", ["hee4db ingest", "just verify"]),
        "a 16 System Maps note": (VAULT / "16 System Maps/Anti-Bloat Budget.md", ["hee4db ingest", "just verify"]),
        "Jev Fit Map": (VAULT / "50 Jev/Jev Fit Map.md", ["hee4db ingest"]),
    }
    for name, (path, want) in fire.items():
        rc, out, err, dt = run_hook(HOOK, {"tool_name": "Edit", "tool_input": {"file_path": str(path)}})
        got = steps_of(out)
        c.check(name, "fire", rc == 0 and got == want and dt < BUDGET_S, f"rc={rc} got={got} want={want} t={dt:.2f}s")

    quiet = {
        "README.md": {"tool_input": {"file_path": str(REPO / "README.md")}},
        "docs/INDEX.md (EXCLUDED, no legend key)": {"tool_input": {"file_path": str(REPO / "docs/INDEX.md")}},
        "a .claude file": {"tool_input": {"file_path": str(REPO / ".claude/settings.json")}},
        "a vault note outside the ingested dirs": {"tool_input": {"file_path": str(VAULT / "00 Hub/x.md")}},
        "a nested file (glob must not cross a dir)": {"tool_input": {"file_path": str(VAULT / "15 Module Design/sub/x.md")}},
        "a tmp file": {"tool_input": {"file_path": "/tmp/hee4-nudge-test.md"}},
        "no file_path": {"tool_input": {}},
        "malformed JSON": "{not json",
    }
    for name, payload in quiet.items():
        rc, out, err, dt = run_hook(HOOK, payload)
        c.check(name, "quiet", rc == 0 and out.strip() == "" and dt < BUDGET_S, f"rc={rc} out={out[:120]!r} t={dt:.2f}s")

    # The repo root is this checkout's, from any cwd and whatever hee4db's own default says
    # (a worktree must not read the main checkout's world).
    decoy_env = {"HEE4_ROOT": "/nonexistent-hee4-root", "HEE4DB_REPO": "/nonexistent-hee4-root"}
    with tempfile.TemporaryDirectory() as td:
        here = os.getcwd()
        os.chdir(td)
        try:
            rc, out, err, dt = run_hook(HOOK, {"tool_name": "Edit", "tool_input": {"file_path": str(REPO / "plan/DECISIONS.md")}},
                                        decoy_env)
        finally:
            os.chdir(here)
        got = steps_of(out)
        want = ["just repin DEC", "hee4db ingest", "just verify"]
        c.check("DECISIONS from another cwd, decoy HEE4_ROOT", "fire", rc == 0 and got == want, f"rc={rc} got={got} want={want}")
    roots_repo = regen_nudge.load_world(REPO).roots["repo"]
    c.check("load_world roots.repo is the given repo", "fire", roots_repo == REPO, f"roots.repo={roots_repo} repo={REPO}")

    # The key is derived from LEGEND: a fake legend entry produces its own key.
    with tempfile.TemporaryDirectory() as td:
        f = Path(td) / "zz.md"
        f.write_text("x\n")
        w = regen_nudge.load_world(REPO)
        w2 = replace(w, legend={"ZZ": f})
        got = regen_nudge.follow_ups(str(f), w2)
        c.check("fake LEGEND key ZZ", "fire", got == ["just repin ZZ", "just verify"], f"got={got}")
        got0 = regen_nudge.follow_ups(str(f), w)
        c.check("same file, real LEGEND", "quiet", got0 == [], f"got={got0}")
    return c.finish()


if __name__ == "__main__":
    sys.exit(main())
