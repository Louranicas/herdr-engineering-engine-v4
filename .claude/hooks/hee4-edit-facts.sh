#!/usr/bin/env bash
# hee4-edit-facts.sh — PreToolUse(Edit|Write|MultiEdit): first touch of a crate per session gets its module card(s) and importing crates (ECC gateguard, facts supplied not demanded).
# Advisory (warn, never block); exits 0 on any internal error; bounded by `timeout`; no network.
# Policy: lib/edit_facts.py. Assimilated from ECC (everything-claude-code) 2026-10-05.
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd) || exit 0
PYTHONDONTWRITEBYTECODE=1 timeout -k 0.2 1.5 python3 "$here/lib/edit_facts.py" 2>/dev/null
exit 0
