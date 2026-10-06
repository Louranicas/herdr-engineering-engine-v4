# Out of SCOPE is a REPORT line, never a hunk

**Seen 2026-10-05/06 (U-harden-05 wave 11).** Three of six slices changed files that were not in
their SCOPE. h5-gate-timers-ddf changed three (Cargo.lock, `crates/hee4-evidence/Cargo.toml`,
a fixture stub), h5-core-ledger changed one (a test expectation in `store/attempts.rs`), and
h5-roster-attempts changed one (a deletion in `dispatcher.rs`). Each was disclosed. Each still
needed the coordinator to widen SCOPE or reassign the commit before merge.
MEASURED: `python3 -c` over the `summary[].violations` lists in
`/mnt/storage-10tb/hee4-evidence/roster/U-harden-05/wave11-results.json` (5 files in 3 slices).
The captain's rulings on these sit in the Firstmate exits; UNMEASURED here.

**Rule.** A builder or fixer that finds an out of scope gap writes one REPORT line: the gap,
its `file:line`, and the slice that owns it. Then it stops. The gap goes into the next wave's
brief, not into this diff. If the fix cannot land without the file (for example, a lint forbids
the code it would remove), the builder reports that and waits for the coordinator to widen
SCOPE before it writes the hunk.

- Failure family: `scope_violation` (n=142). MEASURED:
  `workflow-curator q "SELECT n FROM failure_modes WHERE family='scope_violation'"`. A REGEXP
  recount of `wf_gaps` up to the learn time also gives 142, and the count was 153 on
  2026-10-06.
- Example key: `wf_640da3dc-5a5/journal.jsonl#key=v2:a2ae1597…&origin=fix&idx=0`. This is the
  right shape: the fixer named `SpawnPlan` pub fields, wrote "Belongs to the S6 contracts
  slice", and left the hunk out.
- Rung: checked today, by the verifier's `git diff --stat main...HEAD` against SCOPE. Proposed
  to become refused: a merge check that compares the diff with the SCOPE recorded with the
  brief.

Related: [[generated-briefs-must-pass-the-brief-door]], [[meta-goal-rungs]],
[[a-door-has-one-constructor]].
