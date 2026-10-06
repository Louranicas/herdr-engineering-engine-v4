# A VERIFY line is evidence only if its own exit fails when the work is absent

**Seen 2026-10-05 (workflow curator rows).** VERIFY lines such as
`cargo test --workspace --offline 2>&1 | tail -5` printed a passing tail and exited 0 whatever the
tests did. `fm-db record brief` now refuses these lines in a recorded brief
(`verify_cannot_fail`, `ops/firstmate/fm-db:512`, which is the one home of the shape list). Two
kinds of line never pass that door: the lines a builder rewrites mid-slice, and the VERIFY lines
inside a workflow prompt.

**Rule.** A VERIFY line that cannot fail is not evidence. `| tail`, `| head`, `; echo rc=$?`,
`|| true` and `grep -v` all print output and still exit 0. These are examples; the detector holds
the full list. For any line that did not go through the brief door, take the verdict from the
command's own exit: `gate <cmd>` (`~/.local/bin/gate`), `set -o pipefail`, or `test $? -eq N`.
Do not copy the shapes into a brief or a prompt. Point at the detector instead
([[prose-that-restates-a-regex-drifts]]).

- Failure family: `verify_cannot_fail` (n=228), `pipe_exit_code` (n=31). MEASURED:
  `workflow-curator q "SELECT family, n FROM failure_modes WHERE family IN ('verify_cannot_fail','pipe_exit_code')"`
  (learn of 2026-10-05T23:38:29Z).
- Unchecked rewrites: 96 builder results rewrote 1090 VERIFY lines in all. MEASURED:
  `workflow-curator q "SELECT count(*), sum(verify_rewritten) FROM wf_results WHERE verify_rewritten > 0"`.
- Example key: `wf_44a4782d-940/journal.jsonl#key=v2:9ab6d68b…&origin=brief:verify&idx=0`.
- Rung: refused, for recorded briefs. Rewritten lines and prompt lines are still at rung 4
  (review).
- Catalogued as `L23`, `AP-29` and LAW 11. Do not restate them here; see
  [hee4-lessons](../.claude/skills/hee4-lessons/SKILL.md) and
  [hee4-brief](../.claude/skills/hee4-brief/SKILL.md).

Related: [[generated-briefs-must-pass-the-brief-door]], [[meta-goal-rungs]].
