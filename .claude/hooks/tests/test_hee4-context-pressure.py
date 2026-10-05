"""hee4-context-pressure: says each band once per session from the transcript's LAST usage record.

Transcripts are synthetic JSONL; the newest usage record wins over an older, larger one.
"""
from __future__ import annotations

import json
import sys
import tempfile
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _util import BUDGET_S, Cases, context, run_hook  # noqa: E402

HOOK = "hee4-context-pressure.sh"


def transcript(d: Path, *totals: int) -> Path:
    p = d / f"{uuid.uuid4()}.jsonl"
    rows = [{"type": "user", "message": {"role": "user", "content": "x"}}]
    for t in totals:
        rows.append({"type": "assistant", "message": {"role": "assistant", "usage": {
            "input_tokens": 10, "cache_creation_input_tokens": 0, "cache_read_input_tokens": t - 10, "output_tokens": 5}}})
    p.write_text("\n".join(json.dumps(r) for r in rows) + "\n")
    return p


def main() -> int:
    c = Cases(HOOK)
    d = Path(tempfile.mkdtemp(prefix="cp-"))
    env = {"XDG_RUNTIME_DIR": str(d), "HEE4_CONTEXT_WINDOW": "200000"}
    pay = lambda tp, s: {"session_id": s, "transcript_path": str(tp), "tool_name": "Read"}

    s = str(uuid.uuid4())
    rc, out, _, dt = run_hook(HOOK, pay(transcript(d, 125_000), s), env)
    c.check("60% band fires with the measured count", "fire",
            rc == 0 and "context is 125000 tokens, 62% of a 200000-token" in (context(out) or "") and dt < BUDGET_S, out[:200])
    rc, out, _, _ = run_hook(HOOK, pay(transcript(d, 130_000), s), env)
    c.check("same band again is quiet", "quiet", out.strip() == "", out[:120])
    rc, out, _, _ = run_hook(HOOK, pay(transcript(d, 185_000), s), env)
    c.check("90% band fires (skipping 75%)", "fire", "92% of a 200000" in (context(out) or ""), out[:200])
    s2 = str(uuid.uuid4())
    rc, out, _, _ = run_hook(HOOK, pay(transcript(d, 650_000), s2), env)
    c.check("above the window implies the 1M window", "fire", "65% of a 1000000-token" in (context(out) or ""), out[:200])

    quiet = {
        "under the first band": pay(transcript(d, 50_000), str(uuid.uuid4())),
        "newest record wins (older was large)": pay(transcript(d, 190_000, 40_000), str(uuid.uuid4())),
        "no usage records": pay(transcript(d), str(uuid.uuid4())),
        "missing transcript": pay(d / "nope.jsonl", str(uuid.uuid4())),
        "no session id": {"transcript_path": str(transcript(d, 190_000))},
        "garbage stdin": "x",
    }
    for name, payload in quiet.items():
        rc, out, _, dt = run_hook(HOOK, payload, env)
        c.check(name, "quiet", rc == 0 and out.strip() == "" and dt < BUDGET_S, f"rc={rc} out={out[:120]!r}")
    return c.finish()


if __name__ == "__main__":
    sys.exit(main())
