# Tests and VERIFY run in a temp home; the live unit is the captain's

**Seen 2026-10-05.** In c5-check-deployed-d1, a brief's VERIFY line restarted the live unit
through D8's default `--drive-cmd --allow-restart`. That went against the standing rule, and it
happened once during the run. In c5-drive-reasons-crash, `is_live` was derived from
`XDG_RUNTIME_DIR`. With another runtime dir, the live socket counted as disposable, and the
crash leg printed PASS where it should have printed UNMEASURED.

**Rule.** A test, a plant and a VERIFY line each run against a home they made themselves:
`FM_HOME`, the ledger, the socket and the work dir all come from a temp dir (`mktemp -d`, or
`tempfile.TemporaryDirectory` as in `ops/firstmate/tests/control.py:147-153,228`). A test
never touches the live `hee4.service`, the live `firstmate.db` or the live ledger. The live act (init, deploy, drill, restart) is the
captain's, and the slice reports it as UNMEASURED. A tool that cannot tell its temp home from
the live unit has a bug: it decides by an explicit root it was given, never by a `$HOME` or
cwd default.

- Failure family: `live_unit_touched` (n=44). MEASURED:
  `workflow-curator q "SELECT n FROM failure_modes WHERE family='live_unit_touched'"`. Many of
  the 44 rows are disclaimers ("the live drill is the captain's act"), not incidents, so
  the n is an upper bound.
- Example key: `wf_77b1f483-7e8/journal.jsonl#key=v2:f2230376…&origin=critic:weak_acceptance&idx=7`.
- Rung: checked, by a test's own temp home and review. Candidate for rung 2: tools refuse the
  live home while under test.
- Brief law: LAW 10 in [hee4-brief](../.claude/skills/hee4-brief/SKILL.md) ("an explicit root,
  never a $HOME default").

Related: [[a-guard-keyed-on-cwd-fails-open-at-home]], [[tmp-is-ram-keep-builds-in-cache]]
(build output in a temp home still goes under `~/.cache`), [[meta-goal-rungs]].
