# Agent roster: operations
One runner for every roster agent: `run-agent.sh <agent> light|deep|selfcheck`.
- **Code measures** (`<agent>/measure.sh`); the agent reads the measurements and curates. Every measurement section prints `UNMEASURED (<reason>)` when its source is absent (an unmounted STORAGE, a missing vault, script or evidence dir) or it looked at nothing (F138); a 0 from an absent source is never printed.
- **The exit code is the agent's typed verdict**, read from the report's LAST non-empty line only (trailing CR stripped), which must match `^<agent without hee4-> verdict=(PASS_WITH_GAPS|PASS|FAIL|BLOCKED|STOP)( |$)` (BLOCKED and STOP added 2026-10-05, V4-84, so the runner, `.claude/agents/PROTOCOL.md` §5 and `ops/firstmate/schema/001_init.sql`'s `exits` CHECK agree; `selfcheck.py` case `vocabulary` measures the three homes):

| Exit | Meaning |
|---|---|
| 0 | PASS, and the run was clean: `claude_rc=0`, `is_error=False`, cost measured |
| 10 | PASS_WITH_GAPS, or a PASS degraded (`degraded_by=` in the log) by an unclean run (`claude_rc`≠0, `is_error`≠False, cost UNMEASURED) **or by a measurement floor** (`degraded_by=measured_*`): `v3_refs>0` (`measured_v3_refs=N`), `funnel: verdict=FAIL` (`measured_funnel_fail`), or any measurement line containing UNMEASURED (`measured_unmeasured_lines=N`). Code measures, the agent curates: a red floor outranks the agent's PASS |
| 20 | FAIL; or BLOCKED (the line names what blocks — an H-row, a grant, a missing input; `blocked=` in the log) |
| 30 | STOP (the andon: raised only by a watcher or refuter with a MEASURED reason, `andon=` in the log; the unit halts until the captain clears it in `firstmate.db`); or REFUSED / UNMEASURED: no report, empty report, or a last line not in the typed form (`verdict_line=REFUSED <reason>`); also any run claude ended with `subtype=error_max_budget_usd` (cut off mid-run, whatever the report says) |
| 40 | SKIPPED: another run of the same agent held the lock; one line in `skipped.log` and in that run's log |
| 2 / 3 | usage / setup |

- Every run log carries `exit=N measure_rc=M` (`M` is measure.sh's own exit status; `NA` on a skip), then one `db_record=ok|failed rc=N run_rc=R measurement_rc=M` line: after the exit line the runner records the run and its measurements in the ops DB (`ops/db/hee4db record run --log`, `record measurement --file`; hee4db's output in `db-<stamp>-<mode>.txt`). A DB failure is logged and **never changes the exit code** (stub battery: 36/36 cases give the same exit with hee4db working and with it replaced by `/bin/false`, 2026-10-01).
- Single-instance per agent (flock on fd 9, closed for the measure and claude children); budget-capped per mode (`<agent>/modes.conf`).
- `dontAsk` permissions from `<agent>/settings.json`, least privilege (V4-26): Read only the v4 repo, `$HEE4_EVIDENCE` and the v4 vault (Read denied on `~/.claude/**` and the v3 homes); Edit only the agent's own vault files and its report dir; the repo is denied; no hooks (`disableAllHooks`, V4-21). Bash: only the exact spellings the agent's `settings.json` allows.
- Logs, measurements and reports: `$HEE4_EVIDENCE/roster/<agent>/`.
- The runner's precondition is `ops/roster/<agent>/{measure.sh,modes.conf,settings.json}` plus `.claude/agents/<agent>.md`; it exits 2 otherwise. Today no roster agent has that directory (`ls ops/roster` → `README.md run-agent.sh selfcheck.py`), so the runner has no live subject: it stays because `ops/db/hee4db` parses `run-agent.sh <agent> <mode>` schedule lines, `hee4-watch-fence` greps its permission mode and `runbooks/roster-selfcheck.toml` names it.

## Roster
The rows below are generated from `.claude/agents/ROSTER.md` (first-column backticked `hee4-*` tokens, in order; `ops/roster/selfcheck.py` case `roster_rows_have_files` keeps them equal to the agent files). Spawn a roster agent with the `Agent` tool or `/hee4-roster`; the brief comes first (PROTOCOL §2).

| Agent | Definition | Section in ROSTER.md |
|---|---|---|
| `hee4-contracts-architect` | `.claude/agents/hee4-contracts-architect.md` | Facet specialists |
| `hee4-store-recovery` | `.claude/agents/hee4-store-recovery.md` | Facet specialists |
| `hee4-app-runtime` | `.claude/agents/hee4-app-runtime.md` | Facet specialists |
| `hee4-isolation` | `.claude/agents/hee4-isolation.md` | Facet specialists |
| `hee4-verdict` | `.claude/agents/hee4-verdict.md` | Facet specialists |
| `hee4-control-socket` | `.claude/agents/hee4-control-socket.md` | Facet specialists |
| `hee4-worker-route` | `.claude/agents/hee4-worker-route.md` | Facet specialists |
| `hee4-receipts-chain` | `.claude/agents/hee4-receipts-chain.md` | Facet specialists |
| `hee4-gate` | `.claude/agents/hee4-gate.md` | Facet specialists |
| `hee4-outer-loop` | `.claude/agents/hee4-outer-loop.md` | Facet specialists |
| `hee4-craft-curator` | `.claude/agents/hee4-craft-curator.md` | Facet specialists |
| `hee4-floor-display` | `.claude/agents/hee4-floor-display.md` | Facet specialists |
| `hee4-watch-drift` | `.claude/agents/hee4-watch-drift.md` | Watchers |
| `hee4-watch-contradiction` | `.claude/agents/hee4-watch-contradiction.md` | Watchers |
| `hee4-watch-evidence` | `.claude/agents/hee4-watch-evidence.md` | Watchers |
| `hee4-watch-recovery` | `.claude/agents/hee4-watch-recovery.md` | Watchers |
| `hee4-watch-fence` | `.claude/agents/hee4-watch-fence.md` | Watchers |
| `hee4-watch-budget` | `.claude/agents/hee4-watch-budget.md` | Watchers |
| `hee4-coordinator` | `.claude/agents/hee4-coordinator.md` | Collaboration roles |
| `hee4-refuter` | `.claude/agents/hee4-refuter.md` | Collaboration roles |
| `hee4-scribe` | `.claude/agents/hee4-scribe.md` | Collaboration roles |

**Adding an agent:** `.claude/agents/<name>.md` with the H2s the selfcheck requires (`Facet and rung`, `Law`, `Draws from`, `Reads`, `Writes`, `Refuses`, `Report shape`) and its typed verdict token, a row in `ROSTER.md`, then `just roster-selfcheck` (exit 0). For the paid runner, add `ops/roster/<name>/{measure.sh,modes.conf,settings.json}` and a selfcheck run that exits 0 or 10.

## Schedule
This host runs no cron and no roster timer: roster work runs through the `Agent` tool under a brief (V4-93).
The sibling slice `ops-db-schedule` owns timers and rewrites this section if it lands.
Until then nothing here runs unattended and nothing here spends.

## Self-check
- `just roster-selfcheck` ($0): `python3 ops/roster/selfcheck.py` reads files only (no process, no model, no network) and prints one `case=` line per structural case, then `roster-selfcheck verdict=PASS|FAIL cases=k/n`; `--control` plants one fault per case over a temp copy and prints `roster-selfcheck control cases=k/k verdict=PASS`. `just verify` runs it as a step.
- `just roster-selfcheck AGENT` (paid, about $0.25 per run): `run-agent.sh AGENT selfcheck`, kept for an agent that has `ops/roster/AGENT/`.
