#!/usr/bin/env bash
# hee4-v3-guard — PreToolUse(Bash): WARN (never block) when a command names a frozen v3 path with
# a write verb. Defence in depth behind the read-only v3 trees (V4-68) and the Edit deny rules.
# Exits 0 always; bounded by `timeout`; no network. Policy: lib/v3_guard.py.
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd) || exit 0
PYTHONDONTWRITEBYTECODE=1 timeout -k 0.2 1.5 python3 "$here/lib/v3_guard.py" 2>/dev/null
exit 0
