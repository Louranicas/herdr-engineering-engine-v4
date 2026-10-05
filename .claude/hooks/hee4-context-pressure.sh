#!/usr/bin/env bash
# hee4-context-pressure.sh — PostToolUse: say once per band (60/75/90%) when the transcript's last usage record crosses the context window (ECC suggest-compact).
# Advisory (warn, never block); exits 0 on any internal error; bounded by `timeout`; no network.
# Policy: lib/context_pressure.py. Assimilated from ECC (everything-claude-code) 2026-10-05.
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd) || exit 0
PYTHONDONTWRITEBYTECODE=1 timeout -k 0.2 1.5 python3 "$here/lib/context_pressure.py" 2>/dev/null
exit 0
