"""hee4_paths: which HEE v4 tree a path belongs to, and the hook output shape. Shared by the edit hooks.

A tree root is the nearest ancestor holding both `gate.toml` and `crates/`, so a path in the main
checkout, a /mnt/storage-10tb/hee4-wt/* worktree or a ~/.treehouse slot resolves to its own tree.
"""
from __future__ import annotations

import json
import os
from pathlib import Path


def tree_of(path: str) -> tuple[Path, str] | None:
    """(root, rel) for a path inside an HEE v4 tree, else None. The file need not exist yet."""
    if not path:
        return None
    p = Path(os.path.abspath(path))
    for parent in p.parents:
        if (parent / "gate.toml").is_file() and (parent / "crates").is_dir():
            return parent, p.relative_to(parent).as_posix()
    return None


def edited_paths(data: dict) -> list[str]:
    """Every file an Edit/Write/MultiEdit payload touches."""
    ti = data.get("tool_input") or {}
    out = [ti.get("file_path") or ti.get("notebook_path") or ""]
    out += [e.get("file_path", "") for e in ti.get("edits") or [] if isinstance(e, dict)]
    return [p for p in dict.fromkeys(out) if p]


def emit(event: str, banner: str, context: str) -> None:
    print(json.dumps({"systemMessage": banner,
                      "hookSpecificOutput": {"hookEventName": event, "additionalContext": context}}))
