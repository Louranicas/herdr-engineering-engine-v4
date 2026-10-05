#!/usr/bin/env bash
# hee4-config-guard.sh — PreToolUse(Edit|Write|MultiEdit): name the gate a file feeds before it is edited (ECC config-protection; the refusal itself is tools/lint-ratchet).
# Advisory (warn, never block); exits 0 on any internal error; bounded by `timeout`; no network.
# Policy: lib/config_guard.py. Assimilated from ECC (everything-claude-code) 2026-10-05.
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd) || exit 0
PYTHONDONTWRITEBYTECODE=1 timeout -k 0.2 1.5 python3 "$here/lib/config_guard.py" 2>/dev/null
exit 0
