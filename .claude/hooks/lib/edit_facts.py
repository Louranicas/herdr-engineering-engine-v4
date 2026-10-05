"""edit_facts: the facts an edit to a crate owes, supplied by the hook (PreToolUse Edit|Write, advisory).

Assimilated from ECC gateguard (2026-10-05). Gateguard denies the first edit until the agent lists
the importers and the affected API itself; that leaves the facts to the agent's search. Here the hook
computes them from the two homes that own them and hands them over on the first touch of each crate
per session:
  - the module card(s): modules/MODULES.toml `[[module]]` rows whose `crate` is this crate (the file's
    `src/<name>` or `src/<name>.rs` picks one module when a row has that name);
  - the crates that import this one: `crates/*/Cargo.toml` dependency keys;
  - a new file (Write to a path that does not exist) owes "which module owns it".
State: one marker per (session, tree, crate) under $XDG_RUNTIME_DIR (or /tmp)/hee4-edit-facts/.
"""
from __future__ import annotations

import hashlib
import json
import os
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hee4_paths import edited_paths, emit, tree_of  # noqa: E402


def facts(root: Path, rel: str, new_file: bool) -> tuple[str, str] | None:
    """(crate, text) for a path under crates/<crate>/, else None. Pure over the tree's files."""
    parts = rel.split("/")
    if len(parts) < 3 or parts[0] != "crates":
        return None
    crate = parts[1]
    try:
        mods = tomllib.loads((root / "modules/MODULES.toml").read_text(encoding="utf-8")).get("module", [])
    except (OSError, tomllib.TOMLDecodeError):
        mods = []
    mine = [m for m in mods if m.get("crate") == crate]
    stem = parts[3].removesuffix(".rs") if len(parts) > 3 and parts[2] == "src" else None
    exact = [m for m in mine if stem and m.get("name") == stem]
    importers = []
    for man in sorted((root / "crates").glob("*/Cargo.toml")):
        if man.parent.name == crate:
            continue
        try:
            deps = tomllib.loads(man.read_text(encoding="utf-8")).get("dependencies", {})
        except (OSError, tomllib.TOMLDecodeError):
            continue
        if crate in deps:
            importers.append(man.parent.name)
    lines = [f"HEE4 edit-facts (advisory, first touch of `{crate}` this session):"]
    if exact:
        m = exact[0]
        lines.append(f"module `{m['name']}` card `{m.get('card', '?')}` (AP {', '.join(m.get('ap', [])) or 'none'};"
                     f" EX {', '.join(m.get('ex', [])) or 'none'}); `hee4db highway {m['name']}` for every layer.")
    elif mine:
        names = ", ".join(sorted(m["name"] for m in mine))
        lines.append(f"modules of this crate (cards under modules/{crate}/<m>/MODULE.md): {names}.")
    else:
        lines.append(f"no MODULES.toml row names crate `{crate}`.")
    lines.append("imported by: " + (", ".join(f"`{c}`" for c in importers) or "no other crate")
                 + (" — a public-API change here is a change to each of them." if importers else "."))
    if (root / "crates" / crate / "FLOW.md").is_file():
        lines.append(f"`crates/{crate}/FLOW.md` maps this crate's flows and wire codes.")
    if new_file:
        lines.append(f"`{rel}` is new: name the module that owns it, or the card that should.")
    return crate, " ".join(lines)


def first_touch(session: str, root: Path, crate: str) -> bool:
    base = Path(os.environ.get("XDG_RUNTIME_DIR") or "/tmp") / "hee4-edit-facts"
    key = hashlib.sha256(f"{session}\0{root}\0{crate}".encode()).hexdigest()[:24]
    try:
        base.mkdir(mode=0o700, parents=True, exist_ok=True)
        fd = os.open(base / key, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
        os.close(fd)
        return True
    except FileExistsError:
        return False


def main() -> int:
    try:
        data = json.load(sys.stdin)
        session = str(data.get("session_id") or "")
        out = []
        for path in edited_paths(data):
            t = tree_of(path)
            if not t:
                continue
            f = facts(t[0], t[1], data.get("tool_name") == "Write" and not os.path.exists(path))
            if f and (not session or first_touch(session, t[0], f[0])):
                out.append(f[1])
        if out:
            emit("PreToolUse", "HEE4 edit-facts: module card and importers attached", " ".join(out))
    except Exception:  # advisory: fail open
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
