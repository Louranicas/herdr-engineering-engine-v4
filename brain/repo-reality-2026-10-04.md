# What each repo actually is (measured 2026-10-04)

- **noodle** (`~/noodle`): Go outer loop, v0.1.5, 843 tests, dormant since 2026-03-19; agents run with `bypassPermissions`; git-worktree isolation only; no cron. Take the shape (orders.json, control.ndjson, mode dial, reconcile), not the runtime.
- **pstack** (`~/cursor-plugins/pstack`, ported to `.claude/skills/pstack/`): v0.15.9, 23 playbooks, 24 principles, 50 skills, 2 agents. Prototype = "speed over polish" (the transcript had it backwards).
- **brainmaxxing** (`~/brainmaxxing`): Claude Code native hooks; ported as `brain/` + `brain-inject.sh` + `brain-auto-index.sh`.
- **verification-skill-example**: feature map = four H2s per feature; doctor first.
- **deep-diff-forge** (`~/deep-diff-forge`): Rust, 983 tests after sealing; Rust-only tree-sitter on a side path; risk = path-rule weights; learning loop wired to nothing; "L9" is a constant.
- **loom-lattice-habitat** (`~/loom-lattice-habitat`): 155k lines, 8,100 tests, built in 6 days, dormant; daemon state in memory (lost on restart); no loom advances past Seated outside a stub test; SendPermit has no live actuator; no Turso/SQLite/Jev/Omarchy anywhere; Zellij substrate, herdr clean-room.
- **HEE v4**: 0 lines of engine code; ~25k lines of planning/tooling; now re-homed to this machine.

Not found in any repo: the herdr "under 50 columns" bug, Turso per-loom isolation, "Jev scores it", "indirect prompt", "README-driven development".

Related: [[stack-thesis]].
