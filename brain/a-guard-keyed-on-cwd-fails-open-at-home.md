# A guard keyed on cwd fails open when sessions run from home

**Seen 2026-10-05 (V4-102, H-10a).** Two user-level hooks that sent prompt text to Jev guarded
HEE v4 by refusing sessions whose cwd was an engine path, and by refusing prompts that named the
engine. The v4 captain session runs with cwd=`~` and writes into the repo by absolute path, so
the cwd test never matched. Most prompts (task notifications, agent hand-backs, cross-session
messages) carried v4 shas, paths and findings without the engine's name. 43 sends in two hours.

**Rule.** An egress guard fails closed: it sends only for a session that has explicitly opted in
(a marker the user made), never because nothing matched a deny pattern. Deny lists over cwd or
text are a second line, not the door. Any hook that can send data off the machine is full
rigour in habitat-door, whatever it does with the reply.

**For this repo.** "Nothing to Jev" is a standing order for agents. It cannot fence a user-level
hook that fires in every session. Check `~/.claude/settings.json` hooks at session start when
egress matters; the project settings cannot override user-level hooks. See [[meta-goal-rungs]].
