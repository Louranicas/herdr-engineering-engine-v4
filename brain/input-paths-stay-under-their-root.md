# A path read from input is checked under its declared root before open

**Seen 2026-10-05.** In `hee4db`, the `Unit=` value from a `.timer` file was joined to the
directory without any check. `Unit=../outside.txt` or `Unit=/etc/hostname` then made hee4db
open, hash and parse a file outside the declared source. In wave 11 (h5-core-ledger),
`restore` in `crates/hee4-core/src/backup.rs:536` got the right shape. It refuses any manifest
key other than `ledger.sqlite3` or `objects/<name>.brief` before it reads a file (:558-562). It
refuses a symlinked `objects/` dir (:580-588). Each open goes through `open_regular` (:219),
which refuses a symlink and a file that changed identity between the check and the open.

**Rule.** A path that comes from input (a manifest key, a config value, an argv word) is
resolved and checked under its declared root before the first open. Accept only the shapes the
writer could have produced. Refuse `..`, an absolute path and a symlink by name: a file
outside the declared root is never opened. A path traversal check after the open protects
nothing. Best: a rooted-path type whose only constructor does the check, so the open cannot
take an unchecked path.

- Failure family: `sandbox_escape` (n=7). MEASURED:
  `workflow-curator q "SELECT n FROM failure_modes WHERE family='sandbox_escape'"`.
- Example key: `wf_640da3dc-5a5/journal.jsonl#key=v2:cbcc0072…&origin=refuter&idx=1`.
- Rung: impossible when a rooted-path type exists, otherwise refused at the boundary. restore
  is at refused. Its fixer recorded one open gap: the check on the `objects/` dir and the later
  per-file opens are separate steps, which leaves an intermediate-dir TOCTOU window
  (h5-core-ledger fix idx 1; closing it needs an `openat` walk).
- Review lens: [hee4-review-lenses](../.claude/skills/hee4-review-lenses/SKILL.md) Lens 3
  (boundaries).

Related: [[a-door-has-one-constructor]], [[meta-goal-rungs]].
