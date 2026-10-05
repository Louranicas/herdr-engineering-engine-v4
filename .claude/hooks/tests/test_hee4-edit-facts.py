"""hee4-edit-facts: first touch of a crate per session carries its card(s) and importers; quiet after.

Expected importers are read here from crates/*/Cargo.toml independently of the hook; the card for an
exact module comes from MODULES.toml. A fake tree proves the facts are derived, not hard-coded.
"""
from __future__ import annotations

import sys
import tempfile
import tomllib
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _util import BUDGET_S, REPO, Cases, context, run_hook  # noqa: E402

HOOK = "hee4-edit-facts.sh"


def importers_of(root: Path, crate: str) -> list[str]:
    return sorted(m.parent.name for m in (root / "crates").glob("*/Cargo.toml")
                  if m.parent.name != crate and crate in tomllib.loads(m.read_text()).get("dependencies", {}))


def main() -> int:
    c = Cases(HOOK)
    run = tempfile.mkdtemp(prefix="ef-run-")
    env = {"XDG_RUNTIME_DIR": run}
    sid = str(uuid.uuid4())
    pay = lambda p, tool="Edit", s=sid: {"session_id": s, "tool_name": tool, "tool_input": {"file_path": str(p)}}

    want = importers_of(REPO, "hee4-contracts")
    rc, out, _, dt = run_hook(HOOK, pay(REPO / "crates/hee4-contracts/src/lib.rs"), env)
    ctx = context(out) or ""
    c.check("first touch: every importer named", "fire",
            rc == 0 and bool(want) and all(f"`{w}`" in ctx for w in want) and dt < BUDGET_S, f"want={want} ctx={ctx[:300]!r}")
    rc, out, _, dt = run_hook(HOOK, pay(REPO / "crates/hee4-contracts/src/brief.rs"), env)
    c.check("second touch of the same crate is quiet", "quiet", rc == 0 and out.strip() == "", out[:120])

    mods = tomllib.loads((REPO / "modules/MODULES.toml").read_text())["module"]
    core = [m for m in mods if m["crate"] == "hee4-core" and (REPO / "crates/hee4-core/src" / m["name"]).exists()]
    if core:
        m = core[0]
        rc, out, _, dt = run_hook(HOOK, pay(REPO / f"crates/hee4-core/src/{m['name']}/mod.rs"), env)
        c.check("src/<module>/ names that module's card", "fire", f"card `{m['card']}`" in (context(out) or ""), out[:300])

    rc, out, _, dt = run_hook(HOOK, pay(REPO / "crates/hee4-app/src/zz_new.rs", "Write", str(uuid.uuid4())), env)
    c.check("a new file owes its owner", "fire", "is new: name the module" in (context(out) or "") and "no other crate" in (context(out) or ""), out[:300])

    with tempfile.TemporaryDirectory() as d:
        t = Path(d)
        for name, deps in {"k-a": {}, "k-b": {"k-a": {"path": "../k-a"}}}.items():
            (t / "crates" / name / "src").mkdir(parents=True)
            body = '[package]\nname = "%s"\n[dependencies]\n' % name + "".join(f'{k} = {{ path = "{v["path"]}" }}\n' for k, v in deps.items())
            (t / "crates" / name / "Cargo.toml").write_text(body)
        (t / "gate.toml").write_text("")
        (t / "modules").mkdir()
        (t / "modules/MODULES.toml").write_text('[[module]]\nname = "zed"\ncrate = "k-a"\ncard = "modules/k-a/zed/MODULE.md"\nap = ["AP-99"]\n')
        rc, out, _, dt = run_hook(HOOK, pay(t / "crates/k-a/src/zed.rs", "Edit", str(uuid.uuid4())), env)
        ctx = context(out) or ""
        c.check("fake tree: importer and card derived", "fire",
                "`k-b`" in ctx and "card `modules/k-a/zed/MODULE.md`" in ctx and "AP-99" in ctx, ctx[:300])

    quiet = {
        "not a crate path": pay(REPO / "README.md", s=str(uuid.uuid4())),
        "outside any tree": pay(Path("/tmp/crates/x/src/lib.rs"), s=str(uuid.uuid4())),
        "garbage stdin": "{",
    }
    for name, payload in quiet.items():
        rc, out, _, dt = run_hook(HOOK, payload, env)
        c.check(name, "quiet", rc == 0 and out.strip() == "" and dt < BUDGET_S, f"rc={rc} out={out[:120]!r}")
    return c.finish()


if __name__ == "__main__":
    sys.exit(main())
