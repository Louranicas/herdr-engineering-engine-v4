"""hee4-v3-guard: warns on a write verb over a v3 path, quiet on reads and on v4 paths.

The v3 roots come from .claude/settings.json deny rules (the old home read as $HOME) plus this machine's
local v3 copies (HEE4_V3_ROOTS); a fake settings file proves the first (a root nobody hard-coded fires),
and the real roots must equal the Edit deny rules counted independently.
"""
from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _util import BUDGET_S, REPO, Cases, context, run_hook  # noqa: E402

HOOK = "hee4-v3-guard.sh"
V3 = "/var/home/herdr-engineering-engine-v3"


def main() -> int:
    c = Cases(HOOK)
    fire = {
        "rm -rf in v3": f"rm -rf {V3}/target",
        "redirect into ~/hee3-evidence": "echo hi > ~/hee3-evidence/note.md",
        "sed -i on host-view v3": f"sed -i 's/a/b/' /run/host{V3}/README.md",
        "cd v3 then cargo": f"cd {V3} && cargo build",
        "cp into hee3-worktrees": "cp /tmp/a $HOME/.cache/hee3-worktrees/x/a",
        "git -C v3 commit": f"git -C {V3} commit -m x",
        "chmod the deployment": "chmod -R u+w $HOME/.local/lib/herdr-engineering-engine-v3",
        "rm in a local v3 copy": "rm -rf /mnt/storage-10tb/hee3-backup/target",
        "redirect into the local v3 vault": "echo x >> '/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v3.vault/a.md'",
        "mv inside the second copy": "mv /mnt/storage-10tb/HEE-v3-SECOND-COPY/a /mnt/storage-10tb/HEE-v3-SECOND-COPY/b",
        "touch in the t3 backups": "touch /mnt/storage-10tb/herdr-engineering-engine-v3-t3-backups/x",
        "tee after a pipe": f"echo x | tee -a {V3}/notes.md",
        "mv inside hee3-implementation": "mv ~/.cache/hee3-implementation/a ~/.cache/hee3-implementation/b",
    }
    for name, cmd in fire.items():
        rc, out, err, dt = run_hook(HOOK, {"tool_name": "Bash", "tool_input": {"command": cmd}})
        ctx = context(out) if rc == 0 else None
        c.check(name, "fire", rc == 0 and ctx is not None and "FROZEN v3 path" in ctx and dt < BUDGET_S,
                f"rc={rc} out={out[:160]!r} t={dt:.2f}s")
    quiet = {
        "cat a v3 file": f"cat {V3}/README.md",
        "grep v3 evidence": "grep -rn foo ~/hee3-evidence",
        "cp FROM v3": f"cp {V3}/a /tmp/b",
        "git log in v3": f"git -C {V3} log --oneline -3",
        "ls worktrees": "ls ~/.cache/hee3-worktrees",
        "v4 reference copy": "rm ~/hee4-evidence/reference/v3-evidence-b5367bc/scratch.tmp",
        "v4 repo write": "touch $HOME/herdr-engineering-engine-v4/x",
        "v4 repo write (storage spelling)": "touch /mnt/storage-10tb/herdr-engineering-engine-v4/x",
        "cat a local v3 copy": "cat /mnt/storage-10tb/hee3-backup/README.md",
        "just restore-v3-modes": "just restore-v3-modes",
        "no command": "",
    }
    for name, cmd in quiet.items():
        rc, out, err, dt = run_hook(HOOK, {"tool_name": "Bash", "tool_input": {"command": cmd}})
        c.check(name, "quiet", rc == 0 and out.strip() == "" and dt < BUDGET_S, f"rc={rc} out={out[:160]!r}")
    rc, out, err, dt = run_hook(HOOK, "{broken")
    c.check("malformed JSON", "quiet", rc == 0 and out.strip() == "", f"rc={rc} out={out[:80]!r}")

    # Roots come from settings: count the Edit deny rules independently and compare.
    sys.path.insert(0, str(REPO / ".claude/hooks/lib"))
    import v3_guard
    deny = json.loads((REPO / ".claude/settings.json").read_text())["permissions"]["deny"]
    n_edit = sum(1 for r in deny if r.startswith("Edit(//") and r.endswith("/**)"))
    roots = v3_guard.roots_from_settings(REPO / ".claude/settings.json")
    c.check("roots == Edit deny rules", "setup", len(roots) == n_edit and n_edit >= 6, f"roots={len(roots)} deny_edit={n_edit}")
    c.check("old home read as $HOME", "setup", not any(r.startswith(v3_guard.OLD_HOME) for r in roots)
            and any(r.startswith(str(Path.home())) for r in roots), f"roots={roots}")
    # The local copies come from HEE4_V3_ROOTS (its default), not from the settings file: with settings that omit them
    # the default still fires, and a HEE4_V3_ROOTS that omits them is quiet.
    with tempfile.TemporaryDirectory() as td:
        s = Path(td) / "settings.json"
        s.write_text(json.dumps({"permissions": {"deny": ["Edit(//tmp/fake-v3-root/**)"]}}))
        cmd = {"tool_input": {"command": "rm -rf /mnt/storage-10tb/hee3-backup/x"}}
        rc, out, err, dt = run_hook(HOOK, cmd, env={"HEE4_HOOK_SETTINGS": str(s)})
        c.check("local copy fires from the HEE4_V3_ROOTS default", "fire", context(out) is not None, f"out={out[:120]!r}")
        rc, out, err, dt = run_hook(HOOK, cmd, env={"HEE4_HOOK_SETTINGS": str(s), "HEE4_V3_ROOTS": td + "/fake-v3"})
        c.check("local copy quiet when HEE4_V3_ROOTS omits it", "quiet", out.strip() == "", f"out={out[:120]!r}")
    with tempfile.TemporaryDirectory() as td:
        s = Path(td) / "settings.json"
        s.write_text(json.dumps({"permissions": {"deny": ["Edit(//tmp/fake-v3-root/**)"]}}))
        rc, out, err, dt = run_hook(HOOK, {"tool_input": {"command": "rm /tmp/fake-v3-root/a"}},
                                    env={"HEE4_HOOK_SETTINGS": str(s)})
        c.check("fake deny root fires", "fire", context(out) is not None, f"out={out[:120]!r}")
        rc, out, err, dt = run_hook(HOOK, {"tool_input": {"command": f"rm -rf {V3}/target"}},
                                    env={"HEE4_HOOK_SETTINGS": str(s)})
        c.check("real v3 root quiet when settings omit it", "quiet", out.strip() == "", f"out={out[:120]!r}")
    return c.finish()


if __name__ == "__main__":
    sys.exit(main())
