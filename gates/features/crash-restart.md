# Crash, restart and restore

The charter's "survives a crash, a restart and a restore" (CHARTER §1) as one feature: what a caller of the socket and an operator at the CLI see across a `kill -KILL` mid-task, a unit stop/start (SIGTERM drain, RL-10) and a `hee4 restore --into` from a backup (E2E-06). Traces E2E-05, E2E-06, E2E-08; loops RL-7 (startup recovery), RL-10 (bounded shutdown); D-rows D3, D5, D7, D2. The drill commands are ATLAS §1 D7 (07-recover real-kill step), D5(b) and §6; where no command exists in the atlas this file says so.

## Sub-features

- kill-9-mid-task: `kill -KILL <MainPID>` after N acknowledged `task.submit` calls, with an attempt running; acked `admitted` rows survive ("recorded before the ack" = fsynced before the reply); the running attempt is reconciled on the next start.
- unit-restart: `systemctl --user stop hee4.service` (SIGTERM; SIGHUP handled the same) runs RL-10: draining word + wake every parked waiter → stop accept → stop dispatch → in-flight attempt finishes under its own deadline → close streams → seal → release custody; `systemctl --user start` runs E2E-05 and RL-7 to `complete`.
- startup-recovery: RL-7 classifies every open attempt and applies the pure reconcile policy R01–R14 by `transition`; `health` reads `recovery=complete` before S-1 is bound.
- restore-from-backup: `hee4 restore --into <dir> <backup-id>` writes a fresh generation from a backup/2 (manifest-last, chained page digests), runs recovery to `complete` inside the verb, persists `restored_from`, prints `restore backup=<id> ledger=<d> objects=<n>/<n> rto_s=<t> verdict=PASS`.
- effect-unknown-visibility: an attempt whose effect cannot be proven becomes task `effect_unknown{cancel}` and stays readable in `task.get` until `task.resolve`.
- custody-after-crash: a stale `control.sock` with no live holder is replaced only after the liveness probe; a live holder refuses a second `serve` by name.
- no-auto-restart: no `Restart=` is configured anywhere in the atlas (D7, CN-06); the drill starts the unit explicitly.

## How to get to it (user POV)

An operator reaches all three through the host, never through an action:

- Crash: `kill -KILL $(systemctl --user show -p MainPID --value hee4.service)` during the 07-recover rehearsal (D7), or a host reboot/logout mid-attempt (ATLAS §6 "Host reboot or logout mid-attempt").
- Restart: `systemctl --user stop hee4.service` / `start`; in the gate world, closing stdin of `hee4 serve --until-stdin-closes`, or SIGHUP from a closed toolbox tab (RL-10 trigger; serve runs only under the unit in production, ATLAS §3.2).
- Restore: `hee4 restore --into <disposable HOME under ~/.cache/hee4-host/<run>/> <backup-id>` (never `/tmp`), then `health`. On the real root only after a loss (ATLAS §6 "Engine state lost or corrupt").

A socket caller sees: connection refused during the gap; after restart, `health` green, acked tasks present, every pre-restart cursor `resync_required`, and any mid-flight task in the state the recovery policy chose.

## Driving it with hee4

Preconditions: README `doctor` green before the drill; the unit installed (D2); a recent verified backup on the other disk for the restore leg; the rehearsal record open to receive each step's `rc=`.

Kill -9 mid-task (ATLAS D7 real-kill step, verbatim where the atlas spells it):

```bash
for i in $(seq 1 $N); do hee4-sh task.submit 'spec:={…}' @idempotency_key=$(uuidgen); done   # N acks; record the task ids
MAINPID=$(systemctl --user show -p MainPID --value hee4.service)
kill -KILL $MAINPID                                                                           # rc recorded
systemctl --user start hee4.service                                                          # explicit; no Restart= exists
hee4 health                                                                                  # ready=true recovery=complete database=ready socket=owned
hee4-sh task.list 'states:=["admitted","running","effect_unknown","accepted"]' task_class:=null parent_task_id:=null 'page:={"limit":100,"cursor":null}'   # acked_present=N/N over the recorded ids
hee4-sh task.get 'selector:={"task_id":"<the one that was running>"}' evidence=none            # reconciled state
```

- Evidence: `acked_present=N/N` (D7); the rehearsal record lists the N ids so D9 excludes them; `/proc/<MainPID>/exe` digest unchanged across the restart (D2).
- Side effects: `tasks` rows for all N present in a host `mode=ro` read; the running attempt's row is `unknown` or `settled` per the policy; no `operations` row lost.

Unit restart (RL-10 then RL-7):

```bash
systemctl --user stop hee4.service          # SIGTERM; drain within TimeoutStopUSec
systemctl --user show -p Result,ExecMainStatus,TimeoutStopUSec hee4.service   # Result=success; TimeoutStopUSec = 1,200 s + measured seal time
stat $XDG_RUNTIME_DIR/hee4/control.lock     # custody released
systemctl --user start hee4.service
hee4 health
```

- Evidence: D2 (`TimeoutStopUSec`), D3 after start; a parked `events.subscribe` receives `server_draining` then close before the stop completes (RL-5 wake by RL-10). Read `systemctl show`, never `journalctl` (H-7).
- `UNWRITTEN: the exact drain read-back line (RL-10 P-1 names "the RL-10 drain test, printing both numbers" but no command or line format is in the atlas).`

Restore from backup (ATLAS D5(b), E2E-06):

```bash
ls /var/mnt/STORAGE-10TB/hee4-backups/<id>/                       # manifest last; objects=<n> with its bound
hee4 restore --into ~/.cache/hee4-host/<run>/state <backup-id>    # restore backup=<id> ledger=<d> objects=<n>/<n> rto_s=<t> verdict=PASS
HOME=~/.cache/hee4-host/<run> hee4 serve --until-stdin-closes &   # gate-world serve on the restored root (UNWRITTEN: how a serve is pointed at a non-default root on the host)
hee4 health                                                        # recovery=complete
hee4-sh task.list … 'page:={"limit":100,"cursor":<a pre-restore cursor>}'   # resync_required (new epoch)
```

- Evidence: D5(b) (the restore line with both object counts, RTO measured against ≤ 10 min); ledger digest and object count compared with the source at the backup cutoff.
- Side effects: `restored_from` persisted (card startup-coordinator §9 #4); a fresh generation under `generations/<g>/`; `active.json` points at it.
- Must not race a live `serve`: S-2 is checked by the restore verb (K6 PR-10).

## Gotchas

- `effect_unknown` is the honest end of a kill: "a clock anomaly or elapsed wait cannot turn an unknown external effect into `none`" (CD RC03 §6). A drill that expects every killed task to come back `admitted` or `failed` is wrong; one may come back `effect_unknown` and need `task.resolve`.
- The recovery policy's targets are not drawn as edges: RL-7 applies R01–R14 and the policy, not the machine, decides (State map §2a "Startup reconcile"). **What the policy leaves unspecified**: the R01–R14 rules themselves and their budgets (`OPEN_ATTEMPT_LIMIT`, `CLEANUP_BATCH`, `WORKSPACE_REMOVAL_BUDGET`) are not in the migrated set (RL-7 "UNMEASURED here"); the attempt-side events are PROPOSAL (State map P-1); `cleanup_pending` after a TERM/KILL of the owned cgroup is named in RL-10 but has no task-side state; no rule names what a running `analysis` (v4.2) becomes. Each is `UNWRITTEN: the recovery target for <that case>` until the P2 slice reads v3's `recovery.rs` (migrate decided, V4-59).
- `UNWRITTEN: the kill-mid-task outcome for an attempt whose candidate was running in a transient scope (S-6) outside hee4.service's cgroup; the scope outlives serve (Socket map S-6 failure mode) and the atlas names no reaper step in 07-recover.`
- A SIGKILL cannot drain: the owned-cgroup "confirmed empty within 10 s" rule is RL-10's (SIGTERM path). After a KILL the child candidate may outlive the engine; count it in the rehearsal record.
- `recovery=complete` can never be observed as anything else through the socket (K6 PR-6 binds S-1 after recovery). A not-ready engine is a connection refusal, not a health line; the only pre-bind read-back is the serve process's own exit line (`UNWRITTEN: its format`).
- v3 lost 0/1 … 187/200 acked rows under a real kill with a buffered writer while a double passed the test (D7). Only the real writer under a real kill is evidence (F132, AP-49); a kill test against a store double proves nothing here.
- The 128 GiB backup budget is fail-closed at admission; a restore drill run while the budget is exhausted meets a `resource_exhausted` on the submit leg, not a restore failure.
- RTO ≤ 10 min and RPO ≤ 35 min (CD:73) are the bounds the restore line is measured against; the 15-min backup cadence has two readings in the sources (Loop map C2) and which one v4 implements is undecided, so the RPO you measure depends on it.
- `TimeoutStopSec` is derived (1,200 s + the P4-measured seal time) and written as a unit comment; before P4 the number does not exist, so a drain budget asserted earlier is a guess.
- No off-site copy exists (ATLAS §6 "Whole-machine loss", H-15): a restore drill proves recovery from the other disk only.
- Must not: `serve` is never hand-started on the real root (SIGHUP from a closed tab would kill it, ATLAS §3.2); state, backups and worktrees never go under `/tmp` (tmpfs); v3 state is never read by v4 (D-U2).
