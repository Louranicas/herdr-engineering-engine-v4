"""context_pressure: say when the context window crosses a band (PostToolUse, advisory).

Assimilated from ECC suggest-compact (2026-10-05), keeping only its primary signal: the latest
assistant `usage` record in the session transcript (input + cache creation + cache read tokens),
not a tool-call count. Bands are fractions of the window; each band is said once per session.
Window: $HEE4_CONTEXT_WINDOW (default 200000); a usage above it proves a larger window, so 1000000
is assumed from then on. Reads only the transcript's last 512 KiB.
"""
from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hee4_paths import emit  # noqa: E402

BANDS = (0.6, 0.75, 0.9)
TAIL = 512 * 1024


def latest_tokens(transcript: Path) -> int | None:
    with transcript.open("rb") as f:
        f.seek(0, os.SEEK_END)
        f.seek(max(0, f.tell() - TAIL))
        tail = f.read().decode("utf-8", errors="replace").splitlines()
    for line in reversed(tail):
        if '"usage"' not in line:
            continue
        try:
            msg = json.loads(line).get("message") or {}
        except ValueError:
            continue
        u = msg.get("usage") if isinstance(msg, dict) else None
        if isinstance(u, dict) and "input_tokens" in u:
            return sum(int(u.get(k) or 0) for k in
                       ("input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"))
    return None


def band(tokens: int, window: int) -> tuple[int, int]:
    """(window used, highest band index crossed or -1)."""
    if tokens > window:
        window = max(window, 1_000_000)
    crossed = -1
    for i, b in enumerate(BANDS):
        if tokens >= b * window:
            crossed = i
    return window, crossed


def advance(session: str, idx: int) -> bool:
    """Record band idx for the session; True only when it is higher than the one already said."""
    base = Path(os.environ.get("XDG_RUNTIME_DIR") or "/tmp") / "hee4-context-pressure"
    base.mkdir(mode=0o700, parents=True, exist_ok=True)
    p = base / hashlib.sha256(session.encode()).hexdigest()[:24]
    try:
        said = int(p.read_text().strip())
    except (OSError, ValueError):
        said = -1
    if idx <= said:
        return False
    p.write_text(str(idx))
    return True


def main() -> int:
    try:
        data = json.load(sys.stdin)
        tp, session = data.get("transcript_path"), str(data.get("session_id") or "")
        if not tp or not session:
            return 0
        tokens = latest_tokens(Path(tp))
        if tokens is None:
            return 0
        window, idx = band(tokens, int(os.environ.get("HEE4_CONTEXT_WINDOW") or 200_000))
        if idx < 0 or not advance(session, idx):
            return 0
        pct = round(100 * tokens / window)
        emit("PostToolUse", f"HEE4 context-pressure: {pct}% of the window",
             f"HEE4 context-pressure (advisory): context is {tokens} tokens, {pct}% of a {window}-token "
             "window (MEASURED from the transcript's last usage record). Finish the current verifiable "
             "unit, then write the handoff (`handoff` skill) or compact at that boundary; route bulk "
             "reads and fan-out to subagents (`principle-guard-the-context-window`).")
    except Exception:  # advisory: fail open
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
