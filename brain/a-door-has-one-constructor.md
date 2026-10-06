# A type that guards a door has private fields and one constructor

**Seen 2026-10-05 (V4-96).** Every field of `SpawnPlan` was `pub`. The refuter's attack crate
wrote a plan literal (`PermitId(0)`, an empty receipt, any program) from outside `hee4-host`
and called `spawn::start` with no `Permit`. `spawn::plan()` never ran. Commit 76cced0 made the
fields private and left `spawn::plan` as the only constructor. Commit b039284 added one
`compile_fail` doctest per field (`crates/hee4-host/src/spawn.rs:159-192`). The census in
`crates/hee4-host/tests/spawn_plan_door.rs` counts literals outside the door file.

**Rule.** A type whose value grants a permission has private fields and one constructor, and
that constructor checks the permission. The slice's ACCEPTANCE builds the literal from another
crate and shows that it does not compile. A door with pub fields, or a second public
constructor, is a bypass and is refused. Pin the error code
(`compile_fail,E0451`, `E0616`): a bare `compile_fail` also passes on a typo.

- Failure family: `door_bypass` (n=20). MEASURED:
  `workflow-curator q "SELECT n FROM failure_modes WHERE family='door_bypass'"`.
- Example key: `wf_640da3dc-5a5/journal.jsonl#key=v2:3dee50b9…&origin=refuter&idx=0`.
- Rung: impossible. The compiler refuses the literal, and the doctests hold each field's
  privacy.
- Review lens for the rest:
  [hee4-review-lenses](../.claude/skills/hee4-review-lenses/SKILL.md) Lens 2 (type design).

Related: [[meta-goal-rungs]], [[out-of-scope-is-a-report-line]] (the fixer that found this
reported it out of SCOPE).
