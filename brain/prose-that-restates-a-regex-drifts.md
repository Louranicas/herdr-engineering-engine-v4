# Prose that restates a regex drifts

**Seen 2026-10-05.** The header comment of `ops/roster/run-agent.sh` (line 11) said the runner
accepts `verdict=PASS|PASS_WITH_GAPS|FAIL`, while the regex at :92 also accepted `BLOCKED|STOP`
(V4-84). Coordinator and orchestration prose had repeated the older vocabulary. Commit 4c7a3f6
fixed the header and added the door: `ops/roster/selfcheck.py` case `runner_header` compares the
header with the regex. Its control plants the three-verdict header (`plant_header_drift`,
selfcheck.py:275-277) and requires a FAIL. The machine homes now agree: run-agent.sh:92,
`ops/firstmate/schema/001_init.sql:60` CHECK and PROTOCOL.md:30 (§5) all list five verdicts.
MEASURED with `python3 ops/roster/selfcheck.py --control` (`cases=10/10 verdict=PASS`) and
`grep -n -E "PASS\|PASS_WITH_GAPS" ops/roster/run-agent.sh ops/firstmate/schema/001_init.sql`.

**Rule.** A sentence that restates a pattern a machine reads is a second home for it, and the
second home drifts. Point at the one home (`file:line`) instead of copying the pattern. If the
prose must carry the pattern, add a check that compares the two and prove the check fails on
the past drift (the correct ladder, rung 2).

Related: [[contradictions-2026-10-04]] (two homes disagreeing),
[[pinned-lines-append-only]] (a cite to a line is the safe way to point).
