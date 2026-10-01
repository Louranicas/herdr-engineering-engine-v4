#!/usr/bin/env python3
"""Prove every project hook: it fires on a matching case and stays quiet otherwise.

The world is the hooks REGISTERED in .claude/settings.json, not a list kept here (an include list
cannot see an omission, F65/F147). Every registered script needs tests/test_<stem>.py; a missing
test is a FAIL by name. A registered script outside .claude/hooks/ is refused. Each test prints
`<hook> verdict=PASS|FAIL cases=k/n fire=a quiet=b` and passes only with a>0 and b>0.
Last line: `hooks proven=N/M verdict=PASS|FAIL` (M = registered hooks).
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

TESTS = Path(__file__).resolve().parent
HOOKS = TESTS.parent
SETTINGS = HOOKS.parent / "settings.json"


def registered() -> list[str]:
    data = json.loads(SETTINGS.read_text(encoding="utf-8"))
    out = []
    for event, groups in (data.get("hooks") or {}).items():
        for g in groups:
            for h in g.get("hooks") or []:
                m = re.search(r"\.claude/hooks/([A-Za-z0-9_.-]+\.sh)", h.get("command", ""))
                out.append(m.group(1) if m else f"UNRESOLVED:{event}:{h.get('command', '')}")
    return out


def main() -> int:
    hooks = registered()
    proven, lines = 0, []
    for h in hooks:
        if h.startswith("UNRESOLVED:"):
            lines.append(f"{h} verdict=FAIL reason=command_not_a_project_hook_script")
            continue
        if not (HOOKS / h).is_file():
            lines.append(f"{h} verdict=FAIL reason=script_missing")
            continue
        t = TESTS / f"test_{h[:-3]}.py"
        if not t.is_file():
            lines.append(f"{h} verdict=FAIL reason=no_test file={t.name}")
            continue
        try:
            p = subprocess.run([sys.executable, str(t)], capture_output=True, text=True, timeout=120, stdin=subprocess.DEVNULL)
        except subprocess.TimeoutExpired:
            lines.append(f"{h} verdict=FAIL reason=test_timeout budget=120s")
            continue
        out = (p.stdout + p.stderr).rstrip()
        print(out)
        last = [l for l in out.splitlines() if " verdict=" in l]
        ok = p.returncode == 0 and bool(last) and " verdict=PASS " in last[-1] + " "
        proven += ok
        if not ok:
            lines.append(f"{h} verdict=FAIL rc={p.returncode}")
    for l in lines:
        print(l)
    v = "PASS" if hooks and proven == len(hooks) else "FAIL"
    print(f"hooks proven={proven}/{len(hooks)} verdict={v}")
    return 0 if v == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
