#!/usr/bin/env bash
# hee4-regen-nudge — PostToolUse(Edit|Write): name the follow-up an edited file owes
# (`just regen`, `just repin <KEY>`, `hee4db ingest`, `just verify`). Advisory; silent for other
# paths; exits 0 on any internal error; bounded by `timeout`. Policy: lib/regen_nudge.py.
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd) || exit 0
PYTHONDONTWRITEBYTECODE=1 timeout -k 0.2 1.8 python3 "$here/lib/regen_nudge.py" 2>/dev/null
exit 0
