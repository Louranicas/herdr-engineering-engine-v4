"""regen_nudge — which follow-up an edited file owes (PostToolUse Edit|Write, advisory).

Policy is pure (`follow_ups`), I/O is thin (`load_world`, `main`), so every branch is reachable by
choosing an argument (~/CLAUDE.md F95). No world list lives here: the legend comes from
ops/checks/module_funnel.py LEGEND, the ingested files from ops/db/hee4db WORLD_FILES /
WORLD_GLOBS / EXCLUDED, and the design-conflict register path from hee4db's design_index kind.
Two doors keeping one rule is a promise (AP-01); this file only reads the one door.

Rules (each a follow-up; a path may owe several; order is the order to run them):
  R1 design-conflict register (hee4db world kind `design_index`)  -> `just regen`
  R2 a LEGEND key file                                           -> `just repin <KEY>`
  R3 a hee4db world file (not EXCLUDED) or a card
     modules/<crate>/<module>/MODULE.md, or modules/MODULES.toml -> `hee4db ingest`
     (kind `jev_fit_map` owes `hee4db ingest` only, per the brief)
  R4 any of R1..R3 except a lone jev_fit_map                     -> then `just verify`
  R1 subsumes R3's ingest and the AT/UM repins (`just regen` runs them itself).
"""
from __future__ import annotations

import importlib.machinery
import importlib.util
import json
import os
import sys
from dataclasses import dataclass
from fnmatch import fnmatch
from pathlib import Path


@dataclass(frozen=True)
class World:
    repo: Path
    roots: dict[str, Path]                       # hee4db ROOTS: repo / evidence / vault
    legend: dict[str, Path]                      # KEY -> absolute file
    files: tuple[tuple[str, str, str | None], ...]   # (root, rel, kind)
    globs: tuple[tuple[str, str, str | None], ...]
    excluded: frozenset[tuple[str, str]]


def _real(p: Path) -> str:
    return os.path.realpath(p)


def _rel_under(path: str, root: Path) -> str | None:
    r = _real(root)
    return os.path.relpath(path, r) if path == r or path.startswith(r + os.sep) else None


def _glob_hit(rel: str, pattern: str) -> bool:
    # A `*` must not cross a directory: same parent, name matches.
    return os.path.dirname(rel) == os.path.dirname(pattern) and fnmatch(os.path.basename(rel), os.path.basename(pattern))


def world_kind(path: str, w: World) -> tuple[bool, str | None]:
    """(is the file a hee4db ingest source, its kind)."""
    for root, rel_pat, kind in w.files:
        rel = _rel_under(path, w.roots[root])
        if rel == rel_pat and (root, rel) not in w.excluded:
            return True, kind
    for root, pat, kind in w.globs:
        rel = _rel_under(path, w.roots[root])
        if rel is not None and _glob_hit(rel, pat) and (root, rel) not in w.excluded:
            return True, kind
    return False, None


def is_card(path: str, w: World) -> bool:
    rel = _rel_under(path, w.repo)
    if rel is None:
        return False
    parts = Path(rel).parts
    return (len(parts) == 4 and parts[0] == "modules" and parts[3] == "MODULE.md") or rel == "modules/MODULES.toml"


def follow_ups(path: str, w: World) -> list[str]:
    path = _real(Path(path))
    ingest, kind = world_kind(path, w)
    keys = sorted(k for k, f in w.legend.items() if _real(f) == path)
    steps: list[str] = []
    if kind == "design_index":
        steps.append("just regen")
        keys = [k for k in keys if k not in ("AT", "UM")]
        ingest = False
    steps += [f"just repin {k}" for k in keys]
    if ingest or is_card(path, w):
        steps.append("hee4db ingest")
    if steps and not (kind == "jev_fit_map" and steps == ["hee4db ingest"]):
        steps.append("just verify")
    return steps


def _load(name: str, path: Path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    if spec is None:
        raise ImportError(name)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


def load_world(repo: Path) -> World:
    mf = _load("hee4_module_funnel", repo / "ops/checks/module_funnel.py")
    db = _load("hee4_hee4db", repo / "ops/db/hee4db")
    # The repo root is the one this hook resolved, not hee4db's default (HEE4_ROOT or the main
    # checkout): from a worktree the two differ and the worktree's world files go unseen.
    roots = {**{k: Path(v) for k, v in db.ROOTS.items()}, "repo": repo}
    legend = {k: (Path(v) if os.path.isabs(v) else repo / v) for k, v in mf.LEGEND.items()}
    return World(repo=repo, roots=roots, legend=legend, files=tuple(db.WORLD_FILES),
                 globs=tuple(db.WORLD_GLOBS), excluded=frozenset(db.EXCLUDED))


def render(path: str, steps: list[str]) -> str:
    return (f"HEE4 follow-up owed for {path}: run " + " → ".join(f"`{s}`" for s in steps)
            + " (derived files and cite pins read this file; .claude/hooks/lib/regen_nudge.py).")


def main() -> int:
    try:
        data = json.load(sys.stdin)
        path = (data.get("tool_input") or {}).get("file_path") or ""
        if not path:
            return 0
        repo = Path(os.environ.get("HEE4_HOOK_REPO") or os.environ.get("CLAUDE_PROJECT_DIR")
                    or Path(__file__).resolve().parents[3])
        steps = follow_ups(path, load_world(repo))
        if steps:
            print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse",
                                                     "additionalContext": render(path, steps)}}))
    except Exception:  # advisory hook: fail open, never break an edit
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
