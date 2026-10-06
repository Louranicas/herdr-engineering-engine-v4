# Meta goal: hard to write bad code (Luke, 2026-10-04)

Every door has a rung; the highest rung wins:
1. Impossible — types and architecture (the wrong state cannot be represented). Cost: zero.
2. Refused at admission — no RESTATEMENT, no `input_sha256`, zero files, stale binary, two sites for one rule. Cost: seconds.
3. Caught by a check — build, tests, plants, deep-diff-forge, feature-map drive. Cost: minutes.
4. Caught by review — interrogate, swarm, a human. Cost: hours, if anyone looks.
5. Caught in production — crash drill, receipts, recovery. Cost: the tail.

Consequences: the `correct` ladder is the growth law (a recurring mistake moves up a rung; the new door must fail on the real past instance); review is the floor, not the gate, so tier-1 stays advisory in `decide`. Metric worth printing at every cut: how many mistake classes sit at each rung.

Related: [[stack-thesis]], [[verification-spine]] · [hee4-review-lenses](../.claude/skills/hee4-review-lenses/SKILL.md) · applied to habitat tooling: adopt-door (`~/.claude/skills/adopt-door/SKILL.md`)
