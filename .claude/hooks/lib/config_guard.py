"""config_guard: name the gate a file feeds before it is edited (PreToolUse Edit|Write, advisory).

Assimilated from ECC config-protection (2026-10-05). ECC refuses the edit; here the refusal lives
one rung up, in the gate (`tools/lint-ratchet`, the commit tier's `lints` step), so it holds for any
editor. This hook only says, before the edit, which door the file is part of, so a change that
weakens a door is made on purpose and not to get a red step green.

`DOORS` is ordered; the first match wins. Each row: (glob over the tree-relative path, what it is).
A `*` stays inside one path segment; `**` crosses segments.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hee4_paths import edited_paths, emit, tree_of  # noqa: E402

DOORS: tuple[tuple[str, str], ...] = (
    ("tools/lint-floor.toml", "the lint floor: lowering a row is the only way to weaken a lint (tools/lint-ratchet)"),
    ("Cargo.toml", "workspace lints and `publish = false` (checked by `tools/lint-ratchet` and the `deps` step)"),
    ("crates/*/Cargo.toml", "a crate must keep `[lints] workspace = true` (lint-ratchet R2)"),
    ("deny.toml", "the dependency policy of the `deps` step (cargo-deny: licenses, bans, sources)"),
    ("gate.toml", "the gate, declared once; the commit tier keeps the floor's steps (lint-ratchet R6)"),
    ("layers.toml", "the layer classification read by `tools/layers` (apparatus_ratio)"),
    (".cargo/**", "rustflags here can cap or allow lints (lint-ratchet R5)"),
    ("rust-toolchain*", "the toolchain pin every gate step builds with"),
    ("tools/gate", "the gate runner"),
    ("tools/lint-ratchet", "the lint ratchet (commit tier `lints`)"),
    ("tools/tests/**", "the tests that prove the gate tools; a weaker test is a weaker gate"),
    ("ops/checks/**", "a check run by `just verify` / `just regen`"),
    (".claude/settings.json", "the project permissions (v3 deny rules) and the hook registry"),
    (".claude/hooks/**", "a project hook or its proof (`.claude/hooks/tests/run_all.py`)"),
)


def _rx(glob: str) -> re.Pattern[str]:
    out, i = "", 0
    while i < len(glob):
        if glob.startswith("**", i):
            out, i = out + ".*", i + 2
        elif glob[i] == "*":
            out, i = out + "[^/]*", i + 1
        else:
            out, i = out + re.escape(glob[i]), i + 1
    return re.compile(out + r"\Z")


RULES = tuple((_rx(g), g, why) for g, why in DOORS)


def door(rel: str) -> tuple[str, str] | None:
    for rx, glob, why in RULES:
        if rx.match(rel):
            return glob, why
    return None


def main() -> int:
    try:
        data = json.load(sys.stdin)
        hits = []
        for path in edited_paths(data):
            t = tree_of(path)
            d = t and door(t[1])
            if d:
                hits.append(f"`{t[1]}`: {d[1]}")
        if hits:
            emit("PreToolUse", "HEE4 config-guard: editing a gate's own config",
                 "HEE4 config-guard (advisory): " + "; ".join(hits[:4]) + ". Fix the code, not the door: "
                 "if this edit weakens a lint, a step, a budget or a check, say so in the change and name "
                 "why; the gate (`just gate commit`) re-checks the floor either way.")
    except Exception:  # advisory: fail open
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
