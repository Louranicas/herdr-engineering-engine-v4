"""v3_guard — warn (never block) when a Bash command names a frozen HEE-v3 path with a write verb.

Defence in depth only: the v3 trees are already chmod read-only (V4-68) and the project
settings deny Edit on them. The v3 roots are NOT listed here: they are read from this project's
`.claude/settings.json` `permissions.deny` `Edit(//<root>/**)` rules, the one home of that list.

Policy is pure (`findings`), so the tests choose commands instead of arranging the world (F95).
A segment (split on ; && || | newlines) is flagged when, after ~/$HOME expansion:
  W1 its verb is in WRITE_ANY and any argument is under a v3 root (rm, mv, chmod, tee, sed -i, ...);
  W2 its verb is in WRITE_DEST and its LAST argument is under a v3 root (cp, rsync, install, ln);
  W3 a redirection target (> >> &>) is under a v3 root;
  W4 `git` with a v3 argument (or after `cd <v3>`) runs a mutating subcommand;
  W5 a build verb (cargo, just, make, hee3-*) runs with a v3 argument or after `cd <v3>`.
Reads (cat, grep, ls, git log, cp FROM v3) stay quiet.
"""
from __future__ import annotations

import json
import os
import re
import shlex
import sys
from pathlib import Path

WRITE_ANY = {"rm", "rmdir", "mv", "chmod", "chown", "chgrp", "touch", "mkdir", "truncate", "tee",
             "shred", "unlink", "setfacl", "chattr", "patch", "dd"}
INPLACE = {"sed": ("-i", "--in-place"), "perl": ("-i", "-pi")}
WRITE_DEST = {"cp", "rsync", "install", "ln"}
GIT_WRITE = {"add", "commit", "checkout", "switch", "restore", "reset", "merge", "rebase", "pull",
             "push", "stash", "clean", "rm", "mv", "tag", "apply", "am", "worktree", "cherry-pick",
             "revert", "init", "gc", "prune", "branch", "update-ref", "config"}
BUILD = {"cargo", "just", "make", "cargo-mutants"}
SPLIT = re.compile(r"\s*(?:&&|\|\||;|\||\n)\s*")
REDIR = re.compile(r"(?:^|\s)(?:\d?>>?|&>>?)\s*(\S+)")


def roots_from_settings(settings: Path) -> list[str]:
    data = json.loads(settings.read_text(encoding="utf-8"))
    out = []
    for rule in (data.get("permissions") or {}).get("deny") or []:
        m = re.fullmatch(r"Edit\(/(/.+?)/\*\*\)", rule)
        if m:
            out.append(m.group(1).rstrip("/"))
    return out


def _expand(tok: str, home: str) -> str:
    for pre in ("~/", "$HOME/", "${HOME}/"):
        if tok.startswith(pre):
            return home + "/" + tok[len(pre):]
    return tok


def _under(tok: str, roots: list[str]) -> str | None:
    t = tok.strip("'\"").rstrip("/")
    for r in roots:
        if t == r or t.startswith(r + "/"):
            return r
    return None


def findings(command: str, roots: list[str], home: str) -> list[str]:
    hits: list[str] = []
    in_v3 = None                                  # set by `cd <v3 root>` for later segments
    for seg in SPLIT.split(command):
        if not seg.strip():
            continue
        for m in REDIR.finditer(seg):
            r = _under(_expand(m.group(1), home), roots)
            if r:
                hits.append(f"W3 redirect into {r}: {seg.strip()[:120]}")
        try:
            toks = [_expand(t, home) for t in shlex.split(seg, comments=True)]
        except ValueError:
            toks = [_expand(t, home) for t in seg.split()]
        toks = [t for t in toks if not re.match(r"^\d?>>?|&>", t)]
        while toks and re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", toks[0]):   # VAR=x cmd
            toks = toks[1:]
        while toks and toks[0] in ("sudo", "env", "nice", "timeout", "command", "exec", "gate"):
            toks = toks[1:]
            while toks and (toks[0].startswith("-") or re.fullmatch(r"[\d.]+[smh]?", toks[0])):
                toks = toks[1:]
        if not toks:
            continue
        verb, args = os.path.basename(toks[0]), toks[1:]
        v3args = [r for r in (_under(a, roots) for a in args) if r]
        if verb in ("cd", "pushd"):
            in_v3 = _under(args[0], roots) if args else None
            continue
        if verb in WRITE_ANY and v3args:
            hits.append(f"W1 {verb} on {v3args[0]}")
        elif verb in INPLACE and any(a.startswith(INPLACE[verb]) for a in args) and v3args:
            hits.append(f"W1 {verb} in-place on {v3args[0]}")
        elif verb in WRITE_DEST:
            nonopt = [a for a in args if not a.startswith("-")]
            r = _under(nonopt[-1], roots) if nonopt else None
            if r:
                hits.append(f"W2 {verb} into {r}")
        elif verb == "git":
            target = v3args[0] if v3args else in_v3
            sub = next((a for i, a in enumerate(args) if not a.startswith("-")
                        and not (i > 0 and args[i - 1] in ("-C", "-c", "--git-dir", "--work-tree"))), None)
            if target and sub in GIT_WRITE:
                hits.append(f"W4 git {sub} in {target}")
        elif verb in BUILD or verb.startswith("hee3-"):
            target = v3args[0] if v3args else in_v3
            if target:
                hits.append(f"W5 {verb} in {target}")
    return hits


def main() -> int:
    try:
        data = json.load(sys.stdin)
        cmd = (data.get("tool_input") or {}).get("command") or ""
        if not cmd or ("v3" not in cmd and "hee3" not in cmd):    # cheap prefilter
            return 0
        here = Path(__file__).resolve().parent
        settings = Path(os.environ.get("HEE4_HOOK_SETTINGS") or here.parents[1] / "settings.json")
        roots = roots_from_settings(settings)
        hits = findings(cmd, roots, os.environ.get("HOME", ""))
        if hits:
            msg = ("HEE4 v3-guard (advisory): this command names a FROZEN v3 path with a write verb: "
                   + "; ".join(hits[:3]) + ". v3 is read-only reference (CLAUDE.md fence, V4-9; trees "
                   "chmod read-only V4-68). Read v3 through ~/hee4-evidence/reference/v3-evidence-b5367bc/.")
            print(json.dumps({"systemMessage": "⚠ HEE4 v3-guard: write verb on a frozen v3 path",
                              "hookSpecificOutput": {"hookEventName": "PreToolUse",
                                                     "additionalContext": msg}}))
    except Exception:  # advisory: fail open
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
