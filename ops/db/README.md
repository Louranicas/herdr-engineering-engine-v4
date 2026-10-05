# hee4-ops.db: the HEE v4 operations database

**Status (2026-10-01):** built and checked, planning/ops infrastructure only, under the HOLD (V4-0). Luke asked for it on 2026-10-01. This is not engine code: it models no engine table.

| What | Where |
|---|---|
| Database | `~/hee4-evidence/db/hee4-ops.db` (plain SQLite format, WAL, created by `tursodb` 0.7.2) |
| Search sidecar (derived) | `~/hee4-evidence/db/hee4-ops-search.db` (Tantivy FTS, needs `--experimental-index-method`) |
| CLI | `ops/db/hee4db` (Python stdlib plus the `tursodb` binary) |
| Schema | `ops/db/schema/NNN_*.sql` (001 init, 002 `roster_runs.exit_status`, 003 `design_conflicts` / `conflict_modules` / `module_budgets`, 004 `jev_fits` / `jev_fit_modules` (rev 2026-10-01 jev-fit), 005 the registry: `registry_sources` / `skills` / `agents` / `agent_modes` / `agent_schedules` / `reflexes` + view `agent_status` (rev 2026-10-01 registry, V4-70), 006 the schedule sources: `registry_sources.kind` gains `timers`, `agent_schedules.kind` gains `daily`, both by table rebuild with the view recreated verbatim (rev 2026-10-05 schedule, U-stack-04)), applied by `hee4db migrate` and recorded in `schema_migrations` with each file's sha256 |
| Control | `ops/db/tests/control.py` (fault clauses plus quiet cases; 2026-10-01: `clauses=23/23 quiet=8/8`; after the alignment pass `clauses=30/30 quiet=10/10`; after the jev-fit pass `clauses=39/39 quiet=12/12`); `--neuter` runs the rule neuters (`killed=21/21`, then `killed=28/28`, then `killed=37/37`). After the ratification closed every register row, the three open-row plants insert their own synthetic register rows (DC-96…DC-99) rather than anchor on live open rows; re-run 2026-10-01 (funnel audit, V4-67): `clauses=39/39 quiet=12/12`, `killed=37/37`; after the registry (V4-70): `clauses=51/51 quiet=18/18`, `killed=53/53`; (rev 2026-10-05 schedule, on Omarchy, where the 2026-10-04 run read `clauses=44/52 quiet=12/19 verdict=FAIL` and the 2026-10-05 run crashed on a missing `crontab` binary) the control builds its own schedule world (a synthetic crontab, a planted `hee4-planted` agent, `modes.conf`, a timers dir through `HEE4DB_TIMERS_DIR`, two unwired hook scripts, a handover note, the crates' `.rs` files) and never reads the host crontab or the host's systemd user directory: `clauses=55/55 quiet=22/22`, `killed=59/59` |
| Daily upkeep | `ops/db/daily.sh` (was the Fedora host crontab 02:30; this host has no cron, so the systemd user timer `systemd/hee4-daily.timer` lands in wave 2 of U-stack-04 and is read back by `hee4db check registry_agents` as `timers=` with `kind=daily`; see below) |
| Build receipt | `~/hee4-evidence/db/receipts/build-20261001.txt` |
| Vault note | `herdr-engineering-engine-v4.vault/90 Database/HEE v4 Ops Database.md` |

## Purpose, and what it is not

Agents need typed, bounded answers to questions like "what blocks P0", "modules in P3 with their AP ids" and "last 10 roster runs with cost", without re-reading 2,000 lines of Markdown each time. The database is that index, plus a home for operational facts that have no file.

**It is not the engine's ledger.** The engine's SQLite store is product. K1 `hee4-core::store` owns it (ULTRAMAP §5a), and it is built after "start coding". Nothing here models tasks, settlements, the outbox or any engine state. The two must never share a file, a schema or a writer.

## The one-home rule

| Kind | Tables | Home | How rows change |
|---|---|---|---|
| **Derived** | `sources`, `decisions`, `decision_mentions`, `held_items`, `held_item_phases`, `antipatterns`, `exemplars`, `drift_controls`, `modules`, `module_refs`, `loops`, `verifications`, `design_notes`, `design_conflicts`, `conflict_modules`, `module_budgets` | The file. The DB is a regenerable index, and every row carries `source_id` + `source_sha256` + `ingested_at` | Only `hee4db ingest`, which rebuilds them all in one transaction. Never edit one by hand. `hee4db stale` lists what changed |
| **Primary** | `roster_runs`, `measurements`, `jev_egress_daily`, `claims`, `ingest_receipts` | This DB (no file holds them) | Only the named `hee4db record …` verbs |
| **Imported** | `habitat_learnings`, `habitat_engine_features`, `habitat_composition_rules`, `habitat_doc_sources`, `habitat_migration_ledger` | `habitat-ops.db` stays their home. These are frozen copies with `source_db`, `source_table`, `source_rowid` and `source_row_sha256` | `hee4db migrate-habitat`. `hee4db stale` re-reads the source and reports changed or deleted rows |

| **Derived (registry)** (rev 2026-10-01, V4-70) | `registry_sources`, `skills`, `agents`, `agent_modes`, `agent_schedules`, `reflexes` (+ view `agent_status`) | `~/.claude/skills/*/SKILL.md`, `<repo>/.claude/skills/*/SKILL.md`, `<repo>/.claude/agents/*.md`, `~/.claude/agents/*.md`, `ops/roster/<agent>/modes.conf`, the host crontab, (rev 2026-10-05 schedule) the systemd user timer directory `~/.config/systemd/user/*.timer` + the paired `.service` (files only, never `systemctl`; `HEE4DB_TIMERS_DIR` for the control), `~/.claude/hooks/*.sh` + `~/.claude/settings.json` hooks, `<repo>/.claude/hooks/*.sh` + `<repo>/.claude/settings.json` hooks. All **read only** | `hee4db ingest`, in the same transaction as the planning tables. Each row carries `source_root` (`claude`/`repo`/`host`/`abs`) + `source_rel`, `source_sha256`, `ingested_at` |

A decision, an H-row, an AP or a module is **never** edited here. Edit the file, then run `hee4db ingest`.

### The ingest world

- **Fixed files:** `plan/DECISIONS.md`, `design/DEPLOYMENT_ATLAS.md` §5, `docs/ANTIPATTERNS.md`, `docs/EXEMPLARS.md`, `docs/DRIFT_AND_OVERENGINEERING.md`, `modules/MODULES.toml`, `learnings/PROCESS-LEARNINGS.md`, and (rev 2026-10-01 alignment) the vault's `15 Module Design/00 - Module Design Index.md` (design note + the DC register → `design_conflicts`, `conflict_modules`) and `16 System Maps/Anti-Bloat Budget.md` (design note + §2 → `module_budgets`). A register row naming a module that `MODULES.toml` lacks, or a closed row carrying a phase, refuses the ingest by name. (rev 2026-10-01 jev-fit) The vault's `50 Jev/Jev Fit Map.md` (design note + §3 tables 3a–3d → `jev_fits`, `jev_fit_modules`); a row with an unknown grade, an unknown module (or a module under the wrong cluster) or a duplicate id refuses the ingest by name.
- **Globs:** `docs/*.md`, `verification/V*.md`, `design/*.md`, and the vault's `15 Module Design/*.md` and `16 System Maps/*.md`.
- **Accounting:** every file a glob finds is either parsed or listed in `EXCLUDED` with a reason. An unexplained file refuses the ingest (`ingest_world_unexplained`).
- **Absent sources:** a declared source that is missing refuses the ingest (`ingest_source_absent`, exit 30). It is never ingested as zero rows.
- **Malformed sources:** a malformed source refuses the whole ingest and rolls back (`ingest_malformed source=…`). A `refused` receipt is then written in its own transaction.
- **New files:** `stale` also reports a file that is in the world but has not been ingested (`stale_world_new_file`).

**Parsing.** Structure is parsed, never regexed over the file:
- fences and headings are tracked;
- table cells are split with code spans and `\|` respected;
- columns are found by header name;
- `MODULES.toml` goes through `tomllib`;
- `~~struck~~` text is dropped before a value is read from a cell. H-6's old "P6" is struck, so it is not a phase.
- **Not parsed:** decision supersession. The register is prose. `decision_mentions` stores every sentence that names another decision id, flagged `has_supersede`. These sentences are candidates for an agent to read; the register decides.

### The registry (rev 2026-10-01, V4-70)

- **Skills:** one row per `SKILL.md`, frontmatter parsed (flat `key: value`, continuation lines folded); `parse_error` records an absent/unterminated frontmatter, a missing name or description, or a name that differs from its directory. `scope` is `user` or `project`; an absent project skills dir is a `registry_sources` row with `status=absent`, never zero rows. A directory without `SKILL.md` (e.g. `synced`) is listed, not ingested.
- **`v4_relevant`** is a rule in code (`SKILL_V4_RULE`, printed by `ingest` and `recipe skills`): a word of the name or description is `v4` or `corpus`, or starts with `hee4`; or the skill is in `SKILL_V4_CURATED` (claim-discipline, vault-mining, adversarial-verify, jev-ask, handoff, each with its reason); `SKILL_V4_EXCLUDED` (hee-v3-corpus: v3 is frozen) wins over both.
- **Agents:** frontmatter name, model, tools; `agent_modes` from `modes.conf` (`mode|budget_usd|prompt`, a malformed line refuses the ingest); `agent_schedules` from two schedule sources (rev 2026-10-05 schedule; the rule lives in `hee4db` beside `read_crontab`/`read_timers`): `crontab -l` (via `flatpak-spawn --host`; `HEE4DB_CRONTAB_FILE` for the control) and the systemd user timers (`TIMERS_DIR`, default `~/.config/systemd/user`; `HEE4DB_TIMERS_DIR` for the control): every top-level `*.timer`, sorted, no recursion, each `OnCalendar=` line (else `OnBootSec=`/`OnUnitActiveSec=`) one row with the line's value as `schedule`, the paired `.service`'s first `ExecStart=` as `command` (`unit_absent:<name>.service` when missing) and the `.timer` path, sha256 and line as provenance. **Kind rule:** a command running `run-agent.sh <agent> <mode>` is `kind=roster` (fewer arguments refuse the ingest), one running `ops/db/daily.sh` is `kind=daily`, anything else `kind=other`. A `.timer` with no `[Timer]` section or no schedule line refuses the ingest (`ingest_malformed source=<path> …`). **Status words** for a `registry_sources` row: `ok`, `absent` (a measured fact about the host: no `crontab` binary, no seam file, no timer directory; `ingest` prints `registry_crontab_absent` and exits 0) and `unreadable` (a source that exists and failed: `rc!=0`, a permission error; `registry_crontab_unmeasured`, `ingest` exits 10). An absent source is never read as zero jobs.
- **`agent_status`** (view): per agent, its definition (scope:model), modes with budgets, schedule, the last `roster_runs` row (stamp, mode, exit, verdict, cost), and the run count, spend and unmeasured-cost count over the last 24 h (UTC stamps).
- **Reflexes:** every `*.sh` in a hooks dir (wired or not) plus every wired command outside one, with events and `event:matcher` pairs from the settings `hooks` block. `just reflex list` was not used: it is a human table with free-text columns, not a machine-readable source. A wired path under `~/.claude` is read against `HEE4DB_CLAUDE_HOME` (identical unless the control sets it).
- **`jev_sender`** is a rule in code (`JEV_SENDER_RULE`): the name starts with `jev-`, or a word of the script (or inline command) is `jevpack*`, `jev_*` or `jev-<x>*` other than the script's own name; `jev_reason` says which. It is a recorded fact for H-8/H-19 accounting, not a judgement that the hook sends v4 text.
- **Stale:** `hee4db stale` and `check=stale` also report `stale_registry path=<root>:<rel> state=changed|absent|new`, a changed crontab, and (rev 2026-10-05 schedule) the timer directory as one unit: `stale_registry path=<TIMERS_DIR> state=changed` when the sha256 over every `.timer` + paired `.service` (sorted) differs from the ingested one, `state=absent` when the directory vanished.
- **Checks:** `registry_skills` (every `v4_relevant` file exists and parses; every curated name is a skill: `registry_skill_absent`, `registry_skill_unparsed`, `registry_skill_curated_unknown`); `registry_agents` (rev 2026-10-05 schedule: every roster job from any schedule source has a clean definition, a `modes.conf` and the mode it runs: `registry_agent_undefined`, `registry_agent_definition_unparsed`, `registry_agent_no_modes`, `registry_agent_mode_unknown`, each located as `source=<label> line=N`). Its three rules: (1) no readable schedule source (crontab and timers both absent/unreadable) is UNMEASURED `measured=0/0 registry_schedule_unmeasured crontab=<status> timers=<status>`, never PASS; (2) readable sources with no roster job is PASS over those sources, `measured=<n>/<n> roster_jobs=0 roster_dirs=<N> timers=<T> crontab=<status> sources=present` (F138/AP-29: a PASS never carries `measured=0/0`); (3) otherwise the per-job rules above. `check=world` (rev 2026-10-05 schedule): an empty registry table whose `registry_sources` kind was read with `status=ok` (`skills`←`skills_dir`, `agents`←`agents_dir`, `agent_modes`←`roster_dir`|`modes`, `agent_schedules`←`crontab`|`timers`, `reflexes`←`hooks_dir`|`settings`) is a measured zero, printed `measured_empty=<table>:<kind>=ok`; one whose sources are all absent or unreadable stays `unmeasured_empty_world`. On this host (2026-10-05): `check=registry_agents verdict=PASS measured=1/1 roster_jobs=0 roster_dirs=0 timers=2 crontab=absent sources=present` and `check=world verdict=PASS measured=24/24 … measured_empty=agent_modes:roster_dir=ok`.

## Verbs

Every verb prints one JSON document on stdout by default; add `--table` for humans. The last stderr line is the typed verdict: `hee4db <verb> verdict=… exit=… measured=N/M …`. Global flags (`--db`, `--table`, `--rows N`, `--bytes N`) go anywhere on the line.

| Verb | Example |
|---|---|
| `help` | `hee4db help` |
| `schema [table]` | `hee4db schema roster_runs` |
| `recipes` / `recipe <name> [args]` | `hee4db recipe blocks-phase P0` |
| `q '<SELECT>'` (`--param v`, `--engine turso`) | `hee4db q 'SELECT id, verdict FROM verifications'` |
| `get <entity> <id>` | `hee4db get module store` (entities: decision held ap ex drift module loop verification note run claim learning source) |
| `search <text>` | `hee4db search verdict authority` |
| `migrate` | idempotent; refuses if an applied migration file changed (`migration_drift`) |
| `ingest` | rebuilds the derived tables, prints per-source `rows= sha= new/changed/unchanged`, then rebuilds the FTS sidecar |
| `record run` | `--log <roster/<agent>/run-*.log>` records one run (what `run-agent.sh` calls after its exit line); `--from-logs` backfills `~/hee4-evidence/roster/*/run-*.log`; or `--agent --mode --stamp --verdict [--exit --cost-usd --degraded-by …]`. Both log forms are idempotent by log path and a digest that leaves out the runner's trailing `db_record=` line, and set `exit_status` (migration 002: `recorded`, `legacy_no_exit`, `missing`) |
| `record measurement` | `--file <roster/<agent>/measure-*.txt>` (one file, as the runner calls it); `--from-measure-files`; or `--name v3_refs --value 0 --denominator 204 [--source …]` |
| `record jev-daily [--day D]` | counts only (see below); a day before a sender's first log row is UNMEASURED, never zero (rev 2026-10-02 V4-74) |
| `jev-entry [--since ISO]` | (rev 2026-10-02 V4-74) the ATLAS P0 Jev entry read-back: the JP0 battery (every hee4 crate in `MODULES.toml`, the hee4 namespace, the three homes) through the real `jev-boundary` CLI with a v3 and a clean control, then `sent_engine_rows=E/S` over verifier, router, read gate and recall since the door and its loggers were installed. A wrong control outranks every other answer (UNMEASURED). Method limits are printed: heads only are replayed; read-gate rows match by path hash. Control: `ops/db/tests/jev_entry_control.py` (`just verify` step 6). Recipe: `just jev-entry [SINCE]` |
| `record claim` / `record verify` | a claim stays `open` until someone **other than the claimant** verifies it (`refused_self_verify`, plus a CHECK in the schema) |
| `migrate-habitat` | one-time copy with provenance; the world is every table in `habitat-ops.db`, each with a disposition |
| `check` | integrity on two engines, foreign keys, migrations, world, stale, orphans, sidecar, and (rev 2026-10-01 alignment) `alignment_atlas` (ATLAS §9 = the register's open rows by phase), `alignment_um` (ULTRAMAP §7 marked list = open rows citing "UM §"/"UM-P"), `funnel_links` (card → design heading, design section → its own card), `funnel_phases` (card §3 phase rows = `MODULES.toml`), `readiness_note` (vault note = `readiness --md`). The aggregate refuses as one unit; (rev 2026-10-01 jev-fit) `jev_grades` (the fit map's §0 self-check line `intersections=N A= B= C= X=` = the table counts), `jev_scope` (while ATLAS H-8 is open no A/B row with engine data is allowed now or ungated; an A row is allowed now without engine data; every X row gives a reason), `jev_gates` (every gate a B row cites is an ATLAS §5 id, aliases resolved from the §5 row text); (rev 2026-10-05) `held` (`held_closed_word_misplaced id=H-nn`: an ATLAS §5 row whose Item cell parses as open while another cell of the row says `**CLOSED` — the status word written where `parse_held` does not read it, so the DB kept H-18, H-27 and H-29 open) |
| `highway <module>` | one bounded JSON with every planning hop: crate, card, design section (heading verified), phases, scope, budget, readiness, open DCs, held items, AP/EX/D/A/V4 ids, migrated paths with existence, decisions, V-reports, system maps. Also `--phase Pn`, `--dc DC-nn`, `--phase-table [--md]` (ATLAS §9), `--um-amendments [--md]` (ULTRAMAP §7); (rev 2026-10-01 jev-fit) a `jev_fits` hop (the module's fits with grade, allowed_now, gates, phases; `all_crates`; `manifest_jev` from MODULES.toml), and `--jev [--grade A\|B\|C\|X]` (every fit, bounded at 60 rows). Recipes `jev-fits-now` (allowed_now = yes and grade A) and `jev-held` (grade B with gates) |
| `readiness [--md]` | per-module readiness; the rules live in this code only; `--md` prints the vault note `00 Hub/Module Readiness 2026-10-01.md` byte for byte |
| `stale` | changed or absent sources, new world files, drifted habitat rows |
| `mcp` | a read-only snapshot over MCP stdio, confined by `bwrap` (see below) |

**Exit codes** (the roster contract): 0 ok · 10 ok-with-gaps · 20 refused or failed check · 30 unmeasured · 2 usage · 3 setup.

### Read safety
- **Bounded at acquisition:** rows are fetched lazily and refused at cap+1 (`refused_row_cap rows>200 cap=200`). Bytes are counted per row (`refused_byte_cap … cap=262144`). The sqlite VM has a step budget (`refused_step_budget`, which catches a cartesian `count(*)`). Caps rise with `--rows` (max 5000) and `--bytes` (max 4 MiB). There is no unbounded path.
- **Read-only in three layers:** `mode=ro`, `PRAGMA query_only=1` (read back), and an authorizer that denies every non-read action (`refused_not_read_only`). Multiple statements are refused (`refused_multi_statement`). `--engine turso` first compiles the raw statement under the authorizer with `EXPLAIN`, then runs it in `tursodb --readonly` with a `LIMIT cap+1` wrapper and a bounded stdout read.
- **An empty world is UNMEASURED:** a query returning 0 rows from tables that are all empty exits 30 (`unmeasured_empty_world`). A query returning 0 rows from non-empty tables exits 0 and prints `world_rows=N`. `check` prints UNMEASURED rather than PASS for any check whose denominator is 0 (F138).

### Write safety: one door
- Every write verb goes through `Writer`:
  - a `flock` on `hee4-ops.db.writer.lock`;
  - Python `sqlite3` with `journal_mode=WAL`;
  - `synchronous=FULL` and `foreign_keys=ON`, each **read back**;
  - one `BEGIN IMMEDIATE … COMMIT`, with a rollback on any error, printing `WRITE_FAILED ROLLED_BACK`.
- Success is reported only after the COMMIT has returned, and after a **second-engine read-back**: `tursodb --readonly` counts must equal Python's (`readback … matched=N/N`, else `readback_mismatch`).

## `just` recipes that drive hee4db (repo `justfile`, rev 2026-10-01)

These recipes are doors over the verbs above. They invent no check. Each recipe has a habitat runbook in `runbooks/<name>.toml`.

| Recipe | hee4db verbs it runs | Read-back |
|---|---|---|
| `just verify` | `check` (step 4), plus `ops/db/tests/control.py` (step 5) and `ops/db/tests/jev_entry_control.py` (step 6) | one line, `verify verdict=PASS\|FAIL steps=N/M`; every step runs even after a failure |
| `just regen` | `ingest` → `highway --phase-table --md` (ATLAS §9) and `highway --um-amendments --md` (ULTRAMAP §7), each placed by `ops/checks/regen.py` → `readiness --md` (the vault note) → repin AT, UM → `ingest` → `check` → `stale` | `regen.py --check` must print `changed=0`, and `stale` must print `stale_sources=0`. A second run writes no byte: 2026-10-01, sha256 of 84 files identical across two runs |
| `just highway MODULE` | `highway MODULE` | the runbook re-runs it and requires `verdict=PASS` |

`ops/checks/regen.py` only moves generator output into place. A block's anchor must be unique: one `## 9 · Phase entry` heading, and one begin and one end `hee4db:um-amendments` marker. If an anchor is not unique, the script refuses (exit 3). It rewrites a file only when its bytes change, and does so atomically with a read-back.

## Recipes (each run once on 2026-10-01; JSON bytes on stdout)

| Recipe | Question | Rows | Bytes |
|---|---|---|---|
| `blocks-phase [P0]` | open held-for-Luke items that block a phase | 7 | 2,250 |
| `modules-in-phase [P3]` | modules in a phase, with their AP ids | 21 | 2,472 |
| `last-runs [10]` | last N roster runs: verdict, exit, cost, degraded_by | 10 | 1,873 |
| `trend [funnel]` | a measurement's time series (name or `name.*`) | 6 | 1,360 |
| `luke-held` | every H-row (all are Luke's), with status | 31 | 4,049 |
| `superseding [D-U4]` | sentences naming X that talk about superseding | 3 | 548 |
| `verdicts` | V-reports and their typed verdicts | 11 | 957 |
| `jev-egress` | Jev egress counts per day and sender | 4 | 758 |
| `open-claims` | claims nobody else has verified | 6 | 1,647 |
| `spend` | roster cost per agent and day (`cost_unmeasured` counted) | 4 | 560 |
| `module-card [store]` | everything a module card funnels | 10 | 759 |
| `ap-users [AP-01]` | modules citing an AP | 41 | 1,202 |
| `stale-sources` | sources with their sha12 and ingest time | 38 | 6,884 |
| `skills` (V4-70) | v4-relevant skills to load, with paths, and the rule | see V4-70 | |
| `agents` (V4-70) | `agent_status` | see V4-70 | |
| `reflexes` (V4-70) | every reflex, Jev senders first, and the rule | see V4-70 | |
| `restart` (V4-70, composite) | ONE bounded call: H-5, open P0 holds, newest `just verify` step logs (the summary line is stdout-only, so it is reported UNMEASURED with each step's last verdict line), `check` and readiness counts, skills to load, `agent_status`, newest `~/handoffs/HEE4_HANDOVER_*`, `~/handoffs/HEE4_RESTART.md` (exists?), and the five entry routes; refused over 256 KiB | see V4-70 | |
| `search verdict authority` | ranked full-text search (FTS) | 19 of 273 | 2,339 |

The row counts are from the run quoted in the build receipt. `verdicts` and `stale-sources` have each grown by 1 since then, because V12 was ingested.

**Agents may also use the engine directly**, read-only and one-shot: `tursodb --readonly -m list ~/hee4-evidence/db/hee4-ops.db "SELECT …" </dev/null`.
- Always pass both the DB and the SQL, with stdin closed.
- Never open the file read-write with `tursodb`.
- Never hold a long-lived `--readonly` session: it reads stale data.

## Writer safety: the finding (measured 2026-10-01, tursodb 0.7.2, Python 3.14.7 / SQLite 3.51.2, scratch files only)

1. **Python `sqlite3` is the safe writer** on a file `tursodb` created. While a Python `BEGIN IMMEDIATE` was open for 3 s:
   - `tursodb --readonly` read the last commit (`2`, rc=0);
   - a Python `mode=ro` reader read `(2,)`;
   - a second Python writer got `database is locked`;
   - after the COMMIT, both engines read `3`;
   - `PRAGMA integrity_check` was `ok` on both.
2. **`tursodb` read-write is unsafe beside Python.**
   - An active `tursodb` rw holder locked out Python readers and writers (`database is locked`).
   - A `tursodb` rw open failed while any Python connection had the file open (`Locking error … File is locked by another process`).
   - The habitat finding also stands: its stdin `BEGIN…COMMIT` batch is not atomic.
3. **`tursodb --readonly` coexists with the Python writer.** It is the read-back and check engine.
4. **`PRAGMA foreign_key_check` in `tursodb` 0.7.2 is a false pass.** It returned no rows (rc=0) on a planted violation that Python reports as `[('c', 1, 'p', 0)]`. COMPAT.md lists it as unsupported. `hee4db check` therefore runs foreign keys in Python only, and `integrity_check` on both engines, requiring the exact text `ok`.
5. **Durability is configuration, read back, not observed.** `synchronous=FULL` reads back as 2, so SQLite syncs the WAL at COMMIT. A power-loss test was not run, and `strace` is absent, so the fsync itself is **UNMEASURED**. The sidecar is fsynced explicitly, along with its directory, before the rename.

The six findings are rows in `claims`, `who="opus-5.5 hee4db-build 2026-10-01"`. **All six were re-measured independently on 2026-10-01** (scratch files, same binary; claim 5 over MCP stdio, unconfined and inside `bwrap`) and are `status=verified`, `verified_by=integrator-batchA`; each row's `result` holds the re-run's numbers. Claim 4's "false" `integrity_check` error is the observation on a fresh FTS file, not a proof of falseness. `hee4db recipe open-claims` → `measured=0/0 world_rows=6`.

## Turso features used, and verified on the binary (docs lose to the binary)

| Feature | Used | Verified on 0.7.2 | Docs say |
|---|---|---|---|
| `--readonly` one-shot | every read-back, `check`, `--engine turso`, snapshots | yes | writes error |
| `-m list`, `-q` | parsing output | yes | — |
| `VACUUM INTO` under `--readonly` | MCP snapshot; the control's baseline | yes | — |
| STRICT tables, CHECK, FK, views, `json_valid` | schema | yes (both engines read it) | supported (COMPAT.md) |
| FTS (`CREATE INDEX … USING fts`, `fts_match`, `fts_score`, weights, `OPTIMIZE INDEX`) | the sidecar only | needs `--experimental-index-method`; refused under `--readonly`, so a private per-query copy is used; prefix `term*` silently 0 rows; `integrity_check` gives a false error on an FTS file; no stemming (`door` ≠ `doors`) | "no flags", prefix supported |
| SQLite FTS5 | no | absent (`no such module: fts5`) | unsupported |
| `WITH RECURSIVE` | no | refused (`Recursive CTEs are not yet supported`) | partial |
| `PRAGMA foreign_key_check` | no (Python instead) | false pass (empty, rc=0) | unsupported |
| `--mcp` | `hee4db mcp`, inside bwrap | 9 tools; `--readonly` alone is no boundary | tools unspecified |

The main file holds **no turso-only DDL**, so stock `sqlite3`, Python and `tursodb` all open it. FTS lives in the derived sidecar, which stock SQLite cannot read (`malformed database schema … near "USING"`).

## MCP

`hee4db mcp` serves a **snapshot** over MCP stdio, confined by `bwrap`. It is never the live file, and it never runs unconfined. The steps:
1. `tursodb --readonly … "VACUUM INTO '<tmp>/hee4-ops.db'"`, then `chmod 0444`.
2. `bwrap --unshare-all --die-with-parent --clearenv`, binding read-only only `/usr`, the `tursodb` binary and the snapshot directory, with a private tmpfs `/tmp`.
3. `tursodb --mcp --readonly /db/hee4-ops.db`.

**Measured on 2026-10-01:**
- **Unconfined**, `tursodb --mcp --readonly`'s `open_database` plus `delete_data` emptied another file. The flag is not a boundary.
- **Inside bwrap:**
  - `execute_query` works;
  - `insert_data` and `delete_data` give `Resource is read-only`;
  - `open_database` on the real DB path produced an empty tmpfs file (`no such table`), and the host file was unchanged;
  - creating a file under `/db` gave `read-only filesystem`.
- With no `bwrap`, the verb refuses (`mcp_unconfined_refused`, exit 3).
- **Client config** (proposal): `{"command": "/var/home/Louranicas/herdr-engineering-engine-v4/ops/db/hee4db", "args": ["mcp"]}`.
- **Point-in-time:** the snapshot is as of launch, so restart the server after an ingest.
- `--print-argv` prints the argv and leaves the snapshot directory for the caller to delete.

## Jev egress counts (`record jev-daily`)

These are counts only; no text is read into the DB. The selection rules:

| Sender | Log | Counted as sent | v4 test |
|---|---|---|---|
| **verifier** | `jev-verifier/verdicts.jsonl` | `usage` is non-null (V8's rule) | the `cwd` names the v4 repo or `hee4-evidence` |
| **router** | `jev-router/decisions.jsonl` | `votes` is present | the `cwd` |
| **read-gate** | `jev-read-gate/decisions.jsonl` | `sent_chars > 0` | none: the log has no cwd, so `rows_v4_cwd` is NULL (`measured=partial`) |
| **recall** | `jev-recall/sends.jsonl` (since 2026-10-02, V4-74; before it: no per-send log, V4-29) | the row's own `sent` | the `cwd` |

**2026-10-01, at 13:5x:**
- verifier: 78 rows, 58 sent, **30 with a v4 cwd**;
- router: 60 rows, 28 sent, **7 with a v4 cwd**;
- read-gate: 81 rows, 44 sent, v4 cwd UNMEASURED;
- recall: UNMEASURED.

The verifier's v4 rows include this build session's own turns. The user-level Stop hook sent them. *(rev 2026-10-02 V4-74)* Closed at the source: the door now refuses v4 names and paths for every sender and the verifier scans the session (F-B); `hee4db jev-entry` reads back `sent_engine_rows=0/<sent>` since the install.

## Backup

- The DB lives on the LUKS NVMe, under `~/hee4-evidence/db/`.
- **Backup coverage of `~/hee4-evidence` is SUSPECTED, not measured (gap R6, ATLAS H-28).** No v4 backup unit has been read back.
- The derived tables and the sidecar can be rebuilt from their files with `hee4db ingest`.
- The **primary** tables have no other home: `roster_runs`, `measurements`, `jev_egress_daily`, `claims` and `ingest_receipts`. The run logs and measure files can re-derive the first two (`--from-logs`, `--from-measure-files`). Nothing re-derives a past day's Jev counts once the jsonl logs rotate, or the claims.
- To snapshot safely beside the writer: `tursodb --readonly hee4-ops.db "VACUUM INTO '<dest>'"`. Never `cp` a WAL database.

## How the roster and the curator use it

- **`run-agent.sh`, after its typed exit line (INSTALLED 2026-10-01, CN-04):** `hee4db record run --log <its log>` and `hee4db record measurement --file <its measure file>`, then one log line `db_record=ok|failed rc=N run_rc=R measurement_rc=M` (hee4db's output goes to `db-<stamp>-<mode>.txt` beside the log). A DB failure never changes the runner's exit code (stub battery 36/36 identical exits with hee4db working and replaced by `/bin/false`).
- **`hee4-curator`:** start with `hee4db check` and `hee4db stale`. A stale source means a corpus block may describe an old file. Then answer corpus questions with `hee4db recipe …` and `hee4db search …` instead of whole-file reads.
- **`hee4-workflow-curator`:** use `recipe trend <name>` for loop measurements, `recipe spend`, and `recipe last-runs`. Record each proposal's predicted effect as a `claim`, which the next run or a reviewer verifies.
- **Daily (INSTALLED 2026-10-01, CN-04, on the Fedora host; rev 2026-10-05: this host has no cron, `command -v crontab` finds nothing, so the daily run is not scheduled here until wave 2 of U-stack-04 installs `systemd/hee4-daily.timer`, which this registry reads back as `kind=daily`):** host crontab `30 2 * * *` ran `ops/db/daily.sh` through the toolbox wrapper, before curator deep at 02:40: `ingest`, `record run --from-logs` (backfills any failed `db_record`), `record measurement --from-measure-files`, `record jev-daily --day <yesterday>` (counts only), then `check`. Log: `~/hee4-evidence/db/daily/daily-<stamp>.log`, last line `daily verdict=… exit=… steps_ok=K/5 worst_step=…`. It is PASS_WITH_GAPS by construction while jev-daily cannot measure the read gate's cwd (exit 10); recall is measured since 2026-10-02 (V4-74).
- **`settings.json` permission spelling (proposal):** read-only verbs only, `Bash(/var/home/Louranicas/herdr-engineering-engine-v4/ops/db/hee4db q:*)` and similar. The record verbs stay with the runner, not the model.

## Known gaps (2026-10-01, refreshed by CN-21)

- ~~`check=orphans` reports `orphan_h=H-10`.~~ Closed: `hee4db check` → `check=orphans verdict=PASS measured=744/744 orphan_refs=0` (read back 2026-10-01).
- ~~`hee4db` is not on PATH.~~ Closed: `ls -l ~/.local/bin/hee4db` → symlink to `ops/db/hee4db` (14:08, 2026-10-01).
- **Stale between ingests (CN-04).** The derived tables are as of the last `ingest`. The daily 02:30 run bounds the age to one day; during a day of planning edits `hee4db check` reads `check=stale verdict=FAIL` (e.g. 4 vault design notes changed between this refresh's 06:06 ingest and its re-check). Run `hee4db ingest` before trusting a derived answer the same day; `hee4db stale` names the changed sources.
- **Pre-contract run logs (CN-20).** 9 of 13 run logs predate V4-24/V4-26 (stamp < `20261001T013317Z`) and carry no `exit=`. They are never rewritten: their rows are marked `exit_status=legacy_no_exit` (migration 002) and `record run --from-logs` prints `excluded_pre_contract=9` instead of counting them as gaps; it exits 10 only for a contract-era log missing `exit=` or `cost_usd=` (`exit_status=missing`). The 4 logs without `cost_usd=` are all among the 9.
- 2 logs under `~/hee4-evidence/curator/` come from the retired `ops/curator` runner (V4-19). They are excluded with that reason.
- **`db_record` and a busy writer.** The Writer's flock is non-blocking (`writer_busy`): a roster run that finishes while another hee4db writer holds the lock logs `db_record=failed`; the next daily `--from-logs` backfills it. Not yet observed.
- `decision_mentions` are candidates, not supersession edges.
- FTS has no stemming and no prefix search. The search falls back to LIKE (exit 10) whenever the sidecar is stale or absent.
- **Five habitat learnings vanished from the source (2026-10-05, U-stack-04 ops-db-schedule).** The copy in `habitat_learnings` was made on the Fedora host (every row `migrated_at=2026-10-01T03:56:50Z`, 42 rows). On Omarchy the source holds fewer rows than the copy; the five below are the copy's record of them, kept here because `hee4db migrate-habitat` (the only writer of the `habitat_*` tables) re-copies the source as it is and drops them:

  | source_rowid | id | standing | scope | title | sha256(body) first 16 |
  |---|---|---|---|---|---|
  | 73 | `learning:ship-is-a-named-act-20260928` | 1 | v3-provenance | Shipping was a human-named act on something already running | `14d588dc5bade4be` |
  | 74 | `learning:generated-needs-a-reader-20260928` | 1 | general | A generated channel needs a named reader, or it only looks alive | `f888420b92ec012f` |
  | 75 | `learning:drift-predates-herdr-20260928` | 1 | general | The shipping drift predates herdr and Fedora | `e1b2be8ff00ffd52` |
  | 76 | `learning:brakes-did-not-travel-20260928` | 1 | general | The verification discipline travelled; the brakes did not | `e2a387d830d496e6` |
  | 77 | `learning:zellij-services-none-migrate-20260928` | 1 | v3-provenance | No Zellij-era service migrates; re-derive ideas only | `f05bd20e7ee9ba23` |

  - MEASURED (before the re-copy): `hee4db q --table "SELECT source_rowid, id, standing, scope, title FROM habitat_learnings WHERE source_rowid >= 70 ORDER BY source_rowid"` listed rowid 70-77, all `migrated_at=2026-10-01T03:56:50Z`; the body hashes above are `sha256(body)` of those rows, computed in Python over the same query's JSON.
  - MEASURED: `~/.local/bin/tursodb -q --readonly -m list ~/firstmate/data/habitat-ops.db "SELECT count(*), max(rowid), sum(standing) FROM learnings" </dev/null` → `72|72|37` (the source's last rowid is 72; rowids 73-77 are absent, not changed).
  - MEASURED: `hee4db stale` before the re-copy → `verdict=FAIL exit=20 measured=41/41 stale_sources=0 absent=0 new_files=0 habitat_stale=5 habitat=checked registry_stale=0`, five lines `stale_habitat_row table=learnings rowid=73..77 state=deleted`; `find /mnt/storage-10tb ~ -maxdepth 4 -name 'habitat-ops.db*'` finds only `~/firstmate/data`.
  - INFERRED (from the three facts above): the `habitat-ops.db` that came to Omarchy predates, or was trimmed relative to, the Fedora source the 2026-10-01 copy read. The rows cannot be restored here and the source is Firstmate's home (read through `tursodb --readonly` only), so this block is the record; the copy was then re-made with `hee4db migrate-habitat` (`habitat table=learnings rows=72 migrated=37 disposition=partial`) and `hee4db stale` reads `habitat_stale=0`.
