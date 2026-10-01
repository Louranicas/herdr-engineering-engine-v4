"""Shared test plumbing: run a hook script with JSON on stdin, under a budget, and record cases."""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

HOOKS = Path(__file__).resolve().parents[1]
REPO = HOOKS.parents[1]
BUDGET_S = 2.0          # every hook is bounded at <= 2 s (the brief); asserted per call


class Cases:
    def __init__(self, hook: str) -> None:
        self.hook, self.results = hook, []          # (name, kind, ok, detail)

    def check(self, name: str, kind: str, ok: bool, detail: str = "") -> None:
        self.results.append((name, kind, bool(ok), detail))

    def finish(self) -> int:
        bad = [r for r in self.results if not r[2]]
        for name, kind, ok, detail in self.results:
            if not ok:
                print(f"  FAIL {kind} {name}: {detail}")
        fire = sum(1 for r in self.results if r[1] == "fire" and r[2])
        quiet = sum(1 for r in self.results if r[1] == "quiet" and r[2])
        n = len(self.results)
        v = "PASS" if not bad and fire > 0 and quiet > 0 else "FAIL"
        print(f"{self.hook} verdict={v} cases={n - len(bad)}/{n} fire={fire} quiet={quiet}")
        return 0 if v == "PASS" else 1


def run_hook(script: str, payload, env: dict | None = None) -> tuple[int, str, str, float]:
    data = payload if isinstance(payload, str) else json.dumps(payload)
    e = dict(os.environ)
    e.update(env or {})
    t0 = time.monotonic()
    p = subprocess.run(["bash", str(HOOKS / script)], input=data, capture_output=True, text=True,
                       env=e, timeout=BUDGET_S * 3, stdin=None)
    return p.returncode, p.stdout, p.stderr, time.monotonic() - t0


def context(stdout: str) -> str | None:
    if not stdout.strip():
        return None
    return json.loads(stdout)["hookSpecificOutput"]["additionalContext"]


if __name__ == "__main__":
    sys.exit("helper module; run run_all.py")
