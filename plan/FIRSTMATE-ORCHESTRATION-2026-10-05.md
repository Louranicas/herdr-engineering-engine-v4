# Firstmate as the orchestrator, one database per home

Luke, 2026-10-05: "we will use Firstmate as our orchestrator; each Firstmate is to have a turso database allocated to it." This file records how that lands against the roster (`.claude/agents/ROSTER.md`, `PROTOCOL.md`) and the stack map (`plan/STACK-MAP-2026-10-04.md`). Labels: MEASURED from `~/firstmate` at `e31bc6e6` (2026-10-03) unless marked.

## 1 · What Firstmate is, in this stack
An agent distro (the cloned repo is the distro: `AGENTS.md`, `bin/`, `skills/`), upstream `kunchenguid/firstmate`. The **captain** (Luke) talks to **the first mate** (the harness session in the repo root), which spawns **crew** (ships → project changes; scouts → `data/<id>/report.md`, never a PR) into isolated worktrees from a treehouse pool, inside a session backend (tmux default; herdr auto-detected via `HERDR_ENV=1`, which is this machine's case). **Secondmates** are persistent direct reports with their own `FM_HOME`. A zero-token bash **watcher** polls, queues wakes, and the first mate drains them. Hard rules: the first mate never does project work; crew never address the captain; a restart is a non-event.

Provenance: Luke's fork `Louranicas/firstmate` is gone and the 2026-10-03 migration dirs are empty; `~/firstmate` is the upstream clone plus the restored `data/habitat-ops.db` (SQLite, 30 tables, provenance copies that `hee4db` reads and `hee4db stale` diffs — **frozen**).

## 2 · Layer placement
Firstmate is **L1, the outer loop**, and the collaboration layer of the roster. It replaces the noodle-shaped orders file as the trigger path (backlog.md → brief → spawn) and the bespoke `hee4-coordinator` as the role. It is **not** the engine: HEE v4 (L2) still owns the unit of work, isolation, the verdict, receipts and recovery. The mode dial maps onto Firstmate's own grants: `manual` = default (captain in the loop), `supervised` = `yolo` (green merges only), `auto` = `/afk` with the captain's words verbatim.

## 3 · The database: `$FM_HOME/data/firstmate.db`
- **Why `data/`:** it is where Firstmate's durable state lives and where `hee4db:48` already expects `habitat-ops.db`. The new DB sits beside it; nothing in `habitat-ops.db` moves.
- **What it holds (orchestration only):** `units` (one captain intent, with `planned_agents`), `briefs` (content-hashed, with `head_sha` and the standing-orders hash), `spawns`, `claims` (label mandatory; a MEASURED claim needs its witness command), `verifications` (a trigger refuses the claimant verifying itself), `receipts` (fleet-ledger events, second sink), `exits` (the roster's typed vocabulary; Firstmate status verbs mapped), `andon` (STOP rows; an open andon blocks every further spawn in the unit).
- **What it never holds:** engine task state (HEE K1 `store` + `transition` is the single home — README "must never share a file, a schema or a writer"); `backlog.md` (tasks-axi owns it); `.wake-queue` (the watcher's, lock-bound).
- **Writer rules (from `ops/db/README.md` "Writer safety"):** one writer, `ops/firstmate/fm-db`, Python sqlite3 under a flock with `BEGIN IMMEDIATE`; `tursodb` only ever `--readonly` one-shot for read-back (`fm-db q`, `fm-db status`). Crew append claims through `fm-db record claim`, never raw SQL. Schema in `ops/firstmate/schema/*.sql`, applied by `fm-db init`, which refuses a changed applied file.
- **Allocation:** one DB per `FM_HOME`. A secondmate home gets its own by running `fm-db init` with `FM_HOME` set to that home. `hee4.env` exports `FM_HOME` (default `~/firstmate`) and `FM_DB`.
- **Doors this gives the meta goal (rung 2):** unlabelled claim → refused by CHECK; self-verification → refused by trigger; spawn beyond `planned_agents` → refused; spawn under an open andon → refused; STOP without `measured=1` → refused by CHECK.

## 4 · Reconciliations with Firstmate's conventions
| Topic | Firstmate | Roster/PROTOCOL | Resolution |
|---|---|---|---|
| Fresh vs resume | secondmates persist and relaunch into the same home; `fm-control relaunch` reuses the worktree | every spawn fresh | crew/scouts fresh; a secondmate relaunch is a new `spawns` row with `fresh=0` against the same brief |
| Concurrency | no cap ("dispatch isolated work immediately") | `planned_agents=` first, flat, stop at 70% | enforced in `fm-db record spawn`, plus `spend_max_concurrent_workers` in the away contract |
| Worker permissions | Claude workers default to `--dangerously-skip-permissions` | `hee4-watch-fence` flags bypass | `config/claude-permission-mode = auto` set in this home (gitignored) |
| Exit vocabulary | `done/failed/blocked/needs-decision/paused` | `PASS/PASS_WITH_GAPS/FAIL/BLOCKED/STOP` | mapped in `fm-db record exit`; the roster runner regex still rejects BLOCKED/STOP (exit 30) — a `ops/roster/README.md` fix for Luke |
| Halt | `fm-captain-hold.sh hold`, `fm-control interrupt`; no fleet-wide verb | STOP halts the unit | `andon` row blocks spawns; the first mate interrupts the unit's live panes |
| Authority of a chat sentence | intake treats it as authority if "current, explicit, concrete" | "no agent acts on a chat sentence; the brief is the only start" | both already require a brief before spawn; `HANDOFF-TO-FIRSTMATE.md` (2026-10-04) handed work via a pane, not a brief — it needs one |
| Watchers | scouts (knowledge, never a PR) | read-only, never fix, may STOP | roster watchers run as scouts; STOP is recorded in `andon`, surfaced to the captain as `needs-decision` |

## 5 · Still Luke's
- Confirm `FM_HOME=~/firstmate` as the primary home and whether HEE v4 gets a dedicated secondmate home (each with its own `firstmate.db`).
- Decide whether `treehouse.toml` (lost with the old home) is re-created, and with which pool size.
- The roster runner's verdict regex (BLOCKED/STOP) and the retirement of the cron curators (V4-80).
- Proposed register rows: **V4-82** Firstmate is the orchestrator; **V4-83** one orchestration DB per Firstmate home, `fm-db` the single writer, engine task state never inside it.
