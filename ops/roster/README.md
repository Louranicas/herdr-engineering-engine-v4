# Agent roster: operations
One runner for every roster agent: `run-agent.sh <agent> light|deep|selfcheck`.
- **Code measures** (`<agent>/measure.sh`); the agent reads the measurements and curates. Every measurement section prints `UNMEASURED (<reason>)` when its source is absent (an unmounted STORAGE, a missing vault, script or evidence dir) or it looked at nothing (F138); a 0 from an absent source is never printed.
- **The exit code is the agent's typed verdict**, read from the report's LAST non-empty line only (trailing CR stripped), which must match `^<agent without hee4-> verdict=(PASS_WITH_GAPS|PASS|FAIL)( |$)`:

| Exit | Meaning |
|---|---|
| 0 | PASS, and the run was clean: `claude_rc=0`, `is_error=False`, cost measured |
| 10 | PASS_WITH_GAPS, or a PASS degraded (`degraded_by=` in the log) by an unclean run (`claude_rc`≠0, `is_error`≠False, cost UNMEASURED) **or by a measurement floor** (`degraded_by=measured_*`): `v3_refs>0` (`measured_v3_refs=N`), `funnel: verdict=FAIL` (`measured_funnel_fail`), or any measurement line containing UNMEASURED (`measured_unmeasured_lines=N`). Code measures, the agent curates: a red floor outranks the agent's PASS |
| 20 | FAIL |
| 30 | REFUSED / UNMEASURED: no report, empty report, or a last line not in the typed form (`verdict_line=REFUSED <reason>`); also any run claude ended with `subtype=error_max_budget_usd` (cut off mid-run, whatever the report says) |
| 40 | SKIPPED: another run of the same agent held the lock; one line in `skipped.log` and in that run's log |
| 2 / 3 | usage / setup |

- Every run log carries `exit=N measure_rc=M` (`M` is measure.sh's own exit status; `NA` on a skip), then one `db_record=ok|failed rc=N run_rc=R measurement_rc=M` line: after the exit line the runner records the run and its measurements in the ops DB (`ops/db/hee4db record run --log`, `record measurement --file`; hee4db's output in `db-<stamp>-<mode>.txt`). A DB failure is logged and **never changes the exit code** (stub battery: 36/36 cases give the same exit with hee4db working and with it replaced by `/bin/false`, 2026-10-01). A failed record is backfilled by the daily upkeep below.
- `hee4-curator`'s `v3_refs` scans all three v4 homes (repo, `~/hee4-evidence`, the v4 vault) for v3 **paths**, honouring `ops/v3-independence-exclusions.txt` (`<home>:<path> | <reason>`), and prints a per-home breakdown then `v3_refs=N homes=3/3 files_scanned=N …` (CN-05).
- Single-instance per agent (flock on fd 9, closed for the measure and claude children); budget-capped per mode (`<agent>/modes.conf`).
- `dontAsk` permissions from `<agent>/settings.json`, least privilege (V4-26): Read only the v4 repo, `~/hee4-evidence` and the v4 vault (Read denied on `~/.claude/**` and the v3 homes); Edit only the agent's own vault files and its report dir; the repo is denied; no hooks (`disableAllHooks`, V4-21). Bash: the curator may run exactly `mempalace mine <v4 vault>` with or without `--dry-run`, and `mempalace search`; the workflow curator runs no Bash.
- Logs, measurements and reports: `~/hee4-evidence/roster/<agent>/`.

| Agent | Definition | Subject |
|---|---|---|
| hee4-curator | `.claude/agents/hee4-curator.md` | corpus state vs measured tree |
| hee4-workflow-curator | `.claude/agents/hee4-workflow-curator.md` | workflows and loops: record, measure, propose |

**Adding an agent:** `.claude/agents/<name>.md`, then `ops/roster/<name>/{measure.sh,modes.conf,settings.json}`, a row here, a row in the vault's `80 Agents/Agent Roster`, and a selfcheck run that exits 0 or 10.

## Schedule (host crontab; crond is active)
```
XDG_RUNTIME_DIR=/run/user/1000
DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus
RUN="/usr/bin/toolbox run -c fedora-toolbox-44 /var/home/Louranicas/herdr-engineering-engine-v4/ops/roster/run-agent.sh"
17 */6 * * * $RUN hee4-curator light
40 2 * * *   $RUN hee4-curator deep
47 */8 * * * $RUN hee4-workflow-curator light
10 3 * * *   $RUN hee4-workflow-curator deep
30 2 * * *   /usr/bin/toolbox run -c fedora-toolbox-44 /var/home/Louranicas/herdr-engineering-engine-v4/ops/db/daily.sh
```
The 02:30 line (added 2026-10-01, CN-04) is not a roster agent: `ops/db/daily.sh` runs `hee4db ingest`, backfills runs and measurements (`--from-logs`, `--from-measure-files`), records yesterday's Jev egress **counts** (`record jev-daily`) and runs `hee4db check`, so curator deep at 02:40 reads a current DB. Its log is `~/hee4-evidence/db/daily/daily-<stamp>.log`, last line `daily verdict=… steps_ok=K/5`. No model, no spend.
The deep runs start at 02:40 and 03:10: after the evidence backup at 01:30, and not overlapping each other.

**9 runs a day:** curator light 4 (00:17, 06:17, 12:17, 18:17) + curator deep 1 + workflow-curator light 3 (00:47, 08:47, 16:47) + workflow-curator deep 1.
**Spend ceiling** (each run is capped by `--max-budget-usd` from `modes.conf`; a run that hits its cap exits 30): curator 4 × $1.00 + $4.00 = $8.00; workflow curator 3 × $1.00 + $3.00 = $6.00; **$14.00/day at most, about $420 per 30 days.** Measured spend is lower (selfchecks printed `cost_usd` ≈ 0.20–0.23, V4-24); read `cost_usd=` in the run logs, never this ceiling, for the real figure.
