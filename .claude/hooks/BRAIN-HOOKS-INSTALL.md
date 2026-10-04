# Brain hooks: install by hand (settings.json is not edited by the port)

> **Installed 2026-10-04 on Luke's authorisation**, together with a third SessionStart hook, `context-doctor.sh` (present|MISSING per home and tool, exit 0 always) and deny rules for this machine's v3 copies. This note stays as the record of what was added and how to redo it on another host.

`.claude/settings.json` carries host-specific permission rules, so the brainmaxxing port did not touch
it. To turn the brain hooks on, add these two entries. Both scripts are advisory (exit 0 always,
≤ 2 s, no network, no Jev) and read the brain from `$HEE4_BRAIN` (default `$CLAUDE_PROJECT_DIR/brain`).
Their proofs are already in place: `tests/test_brain-inject.py`, `tests/test_brain-auto-index.py`
(`run_all.py` picks them up the moment the hooks are registered).

## 1. SessionStart → `brain-inject.sh` (append to `hooks.SessionStart`)
```json
{
  "matcher": "startup|resume|clear|compact",
  "hooks": [
    { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/brain-inject.sh\"", "timeout": 2 }
  ]
}
```
(brainmaxxing used `startup|resume`; `clear|compact` is added so the index is re-injected after compaction,
matching `hee4-session-start.sh`.)

## 2. PostToolUse Edit|Write → `brain-auto-index.sh` (append to `hooks.PostToolUse`)
```json
{
  "matcher": "Edit|Write",
  "hooks": [
    { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/brain-auto-index.sh\"", "timeout": 2 }
  ]
}
```
Claude Code matchers match the **tool name**, not a path (brainmaxxing's `"matcher": "brain/"` never fired).
The script itself reads `tool_input.file_path` from the hook JSON and does nothing unless that path is
inside the brain, so this runs beside `hee4-regen-nudge.sh` without interfering.

## One-line merge (from the repo root; review the diff, then `just verify`)
```sh
jq '.hooks.SessionStart += [{"matcher":"startup|resume|clear|compact","hooks":[{"type":"command","command":"bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/brain-inject.sh\"","timeout":2}]}] | .hooks.PostToolUse += [{"matcher":"Edit|Write","hooks":[{"type":"command","command":"bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/brain-auto-index.sh\"","timeout":2}]}]' .claude/settings.json > .claude/settings.json.new && mv .claude/settings.json.new .claude/settings.json && python3 .claude/hooks/tests/run_all.py
```
Roster agents (`ops/roster/run-agent.sh`) run with `disableAllHooks: true`, so neither hook runs there.
