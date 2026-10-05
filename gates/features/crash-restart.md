# Crash, restart and restore

The charter's "survives a crash, a restart and a restore" (CHARTER §1) as one feature: what a caller of the socket and an operator at the CLI see across a `kill -KILL` mid-task, a unit stop/start (SIGTERM drain, RL-10) and a `hee4 restore --into` from a backup (E2E-06). Traces E2E-05, E2E-06, E2E-08; loops RL-7 (startup recovery), RL-10 (bounded shutdown); D-rows D3, D5, D7, D2. The drill commands are ATLAS §1 D7 (07-recover real-kill step), D5(b) and §6; where no command exists in the atlas this file says so.

## Sub-features

- kill-9-mid-task: `kill -KILL <MainPID>` after N acknowledged `task.submit` calls, with an attempt running; acked `admitted` rows survive ("recorded before the ack" = fsynced before the reply); the running attempt is reconciled on the next start.
- unit-restart: `systemctl --user stop hee4.service` (SIGTERM; SIGHUP handled the same) runs RL-10: draining word + wake every parked waiter → stop accept → stop dispatch → in-flight attempt finishes under its own deadline → close streams → seal → release custody; `systemctl --user start` runs E2E-05 and RL-7 to `complete`.
- startup-recovery: RL-7 classifies every open attempt and applies the pure reconcile policy R01–R14 by `transition`; `health` reads `recovery=complete` before S-1 is bound.
- restore-from-backup: `hee4 restore --into <dir> <backup-id>` writes a fresh generation from a backup/2 (manifest-last, chained page digests), runs recovery to `complete` inside the verb, persists `restored_from`, prints `restore backup=<id> ledger=<d> objects=<n>/<n> rto_s=<t> verdict=PASS`.
- effect-unknown-visibility: an attempt whose effect cannot be proven becomes task `effect_unknown{cancel}` and stays readable in `task.get` until `task.resolve`.
- custody-after-crash: a stale `control.sock` with no live holder is replaced only after the liveness probe; a live holder refuses a second `serve` by name.
- auto-restart-with-explicit-fallback: the unit carries `Restart=on-failure RestartSec=2` (systemd/hee4.service:12-13, V4-91), so a `kill -KILL` is followed by systemd's own restart in ~2 s; the drill still starts the unit explicitly (`systemctl --user start`) when it is not back within half its budget and records `explicit_start=yes` (V4-37: the drill never relies on `Restart=`).

## How to get to it (user POV)

An operator reaches all three through the host, never through an action:

- Crash: `kill -KILL $(systemctl --user show -p MainPID --value hee4.service)` during the 07-recover rehearsal (D7), or a host reboot/logout mid-attempt (ATLAS §6 "Host reboot or logout mid-attempt").
- Restart: `systemctl --user stop hee4.service` / `start`; in the gate world, closing stdin of `hee4 serve --until-stdin-closes`, or SIGHUP from a closed toolbox tab (RL-10 trigger; serve runs only under the unit in production, ATLAS §3.2).
- Restore: `hee4 restore --into <disposable HOME under ~/.cache/hee4-host/<run>/> <backup-id>` (never `/tmp`), then `health`. On the real root only after a loss (ATLAS §6 "Engine state lost or corrupt").

A socket caller sees: connection refused during the gap; after restart, `health` green, acked tasks present, every pre-restart cursor `resync_required`, and any mid-flight task in the state the recovery policy chose.

## Driving it with hee4

Preconditions: README `doctor` green before the drill; the unit installed (D2); a recent verified backup on the other disk for the restore leg; the rehearsal record open to receive each step's `rc=`.

Kill -9 mid-task (ATLAS D7 real-kill step; the executable form is `tools/drill --submit N` (rev 2026-10-05 drill), which runs exactly this over the JSON-line frame on `$XDG_RUNTIME_DIR/hee4/control.sock`):

```bash
tools/drill --submit N          # the steps below, one drill_step= line each; last line drill verdict=... submitted=N acked_present=N/N
# what it does, step by step:
for i in $(seq 1 $N); do printf '%s\n' '{"request_id":"...","action":"task.submit","action_version":1,"idempotency_key":"<uuid4>","body":{"brief":"<eleven fields>"}}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/hee4/control.sock; done   # N acks (phase=admitted); the task ids are recorded
MAINPID=$(systemctl --user show -p MainPID --value hee4.service)
kill -KILL $MAINPID                                                                           # rc recorded
# Restart=on-failure RestartSec=2 brings the unit back; if no new MainPID by half the budget the drill runs the explicit start (V4-37) and records explicit_start=yes
systemctl --user start hee4.service                                                          # explicit fallback only
hee4 health                                                                                  # ready=true recovery=complete database=ready socket=owned
printf '%s\n' '{"request_id":"...","action":"task.list","action_version":1,"idempotency_key":null,"body":{}}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/hee4/control.sock   # acked_present=N/N over the recorded ids
```

- Evidence: `acked_present=N/N` (D7); the rehearsal record `~/.cache/hee4-drill/<sha12>/rehearsal.json` (`{tree, unit, submitted, acked_present, task_ids, steps, ts}`, written on every drill run) is what `tools/check-deployed` D7 reads and whose `task_ids` D9 excludes; `/proc/<MainPID>/exe` digest unchanged across the restart (D2).
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

Reconcile drill (STACK-MAP §3.5 and §7 #6: every interrupted task ends terminal or in a named quarantine; D7 strengthened). Runs after the kill -9 leg above, over the same recorded ids:

```bash
hee4-sh task.list 'states:=["admitted","running","verifying","repair_pending","cancellation_requested","effect_unknown"]' task_class:=null parent_task_id:=null 'page:={"limit":100,"cursor":null}'   # over the recorded ids: empty, or every id listed is in the transient class (R07 Unreadable/Unobserved, R11 NotRead) of the table below
hee4-sh task.list 'states:=["blocked"]' task_class:=null parent_task_id:=null 'page:={"limit":100,"cursor":null}'                   # every recorded id here is a named quarantine: its disposition names the R-row and the Unknown reason
hee4-sh task.get 'selector:={"task_id":"<a blocked id>"}' evidence=none                                                                # reason readable (UNWRITTEN: the task.get field that carries rule= and reason=; CD RC03 §4 has none)
systemctl --user stop hee4.service && systemctl --user start hee4.service && hee4 health                                               # second pass over the same ledger: recovery=complete again
hee4-sh task.list 'states:=["admitted","running","verifying","repair_pending","cancellation_requested","effect_unknown"]' task_class:=null parent_task_id:=null 'page:={"limit":100,"cursor":null}'   # identical to the first read minus the transient class: the policy is pure (EX-04), so a second pass changes nothing else
```

- Evidence: over the recorded ids, after the second pass, `non_terminal_unquarantined=0/N` (computed in the rehearsal record from the two lists; a printed counter is a proposal, no atlas line prints it); every `blocked` id carries an R-row reason; the first and second `task.list` reads differ only by ids that moved out of the transient class.
- Side effects: a disposition row per quarantined task (`task_dispositions`, S8) written by `transition` on `Resolve{quarantine}` with `by=recovery` (PROPOSED DC-nn, Gotchas); no `operations` row, because recovery is not a socket caller.
- Must not: the drill never issues `task.resolve` by hand before the second read; a hand-resolved task proves nothing about recovery.

Drive leg (`tools/drive --only crash-restart`, plugin `tools/drive.d/crash.py`) (rev 2026-10-05 drive): on a disposable serve only. The plugin starts its own `hee4 serve` (the binary of the serve behind `--socket`, read by SO_PEERCRED and `/proc/<pid>/exe`) on a socket, ledger and work dir under `~/.cache/hee4-crash/<run>/`, submits one task whose VERIFY is `/usr/bin/sleep 30` plus two quick ones, waits for `running`, sends SIGKILL to that serve only, starts it again on the same ledger, then stops it with SIGTERM and starts it a third time. Paths, each a line in the feature's evidence:

- `kill9_mid_attempt`: the task read `running` before the kill; the serve exited by signal 9.
- `recovery_complete`: `health` after the restart reads `ok=true recovery_complete=true`.
- `acked_present`: every acked task id answers `task.get` after the restart (k/N).
- `named_rule`: the restarted serve's stderr holds exactly one `recovery task=<id> rule=<R> reason=<why> workspace=<w> running -> <after>` line for the killed task, with `<R>` in {`R08WorkerAbsent`, `R12VerificationBoundary`} and `<after>` terminal, `blocked` or `effect_unknown`, and `task.get` reads `<after>`. Measured 2026-10-05: `rule=R08WorkerAbsent reason=AcknowledgedWorkerLost workspace=NotLeasedWritable running -> effect_unknown` (the R08 row of the table below; the R10 automatic quarantine is the PROPOSED DC-nn and is not served).
- `second_pass_pure`: after the SIGTERM stop and a third start, every recovery line naming the task leaves it where it was (measured 2026-10-05: `rule=R10EffectAmbiguity reason=none workspace=none effect_unknown -> effect_unknown`) and `task.get` reads the same phase.

On the live unit's socket every path is UNMEASURED `reason=disposable serve`: the live kill is `tools/drill`, which the captain runs.

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

### Reconcile table R01–R14 (what RL-7 does with each interrupted task)

Source: v3 `src/recovery.rs` at `b5367bca71a4`, rules `:874-1297`, read from the hee3 backup copy `/mnt/storage-10tb/hee3-backup/codebase/20261002T153315236497Z/src/recovery.rs` (the `migrated/v3-b5367bc/src/` set does not hold it; the card §6 says it is staged at "start coding"). Line numbers are that file's. The rule names are the `Rule` enum's doc lines verbatim; everything else in a row is labelled.

Reading the table. **Durable state** is the ledger row as `transition` last left it (TaskState, State map §2a; attempt row §2b: `state`, `effect`, `cleanup`, `acknowledgement`, `lease`) plus the committed terminal history. **Observation** is a field of v3's `Observations` (`claim`, `clock`, `process`, `pi_queue`, `cleanup`, `workspace`); the policy reads no clock and no `/proc` itself (EX-04). **Event** is the `TaskEvent` RL-7 feeds to `transition` after reading the policy's `Decision { rule, reconciliation }`; the policy writes nothing and `permits_execution()` is false for every arm but a `redispatch`/`replay` flag that v3 always sets false (EX-06). **Resulting state** is the task after RL-7. The policy is a pure function, so a second pass over the same ledger and observations yields the same decisions; the drill above asserts it. Provenance: MEASURED = the rule body at the cited lines; INFERRED = the Event mapping, from the rule's arm plus the State map §2c pairs (the only pair whose target is `effect_unknown` from `running`/`cancellation_requested` is `Settle{unsettled}`); UNMEASURED = named as such.

| R | Durable state (task · attempt) | Observation | Event into `transition` | Resulting state | Provenance |
|---|---|---|---|---|---|
| R01 StaleObservationEpoch | any non-terminal · any | `claim.epoch ≠ ledger.epoch` | none; `StaleObservationRefused`, the claim is dropped | unchanged this pass; RL-7 must re-run the attempt with `claim=None` to reach R03–R12 (UNWRITTEN in the atlas: whether RL-7 re-runs or reports the refusal) | MEASURED `:874-881`; INFERRED continuation |
| R02 StaleGeneration | any non-terminal · any | `claim.task_generation ≠ task.generation` or `claim.attempt_generation ≠ attempt.generation` | none; `StaleGenerationRefused`, the claim is dropped | as R01 | MEASURED `:882-900` |
| R03 CommitOrdering | terminal with both an acceptance and a cancellation record (`TaskHistory::Both`), or `Contradictory` | ordinals of the two records | none (I-01: a terminal takes no event). `Greater` → `AcceptanceStands`; `Less` → `RetainUnknown{CancellationPrecedesAcceptance}`; `Equal`/`Contradictory` → unknown | the terminal row stands; a contradiction cannot be transitioned away and is a recovery finding. UNWRITTEN: whether RL-7 withholds `recovery=complete` on it (proposed: yes; D3 is not green over a contradictory ledger) | MEASURED `:957-993`; INFERRED: S7 (one closing record per task) makes `Both` unwritable by `transition`, so in v4 this row guards a restored or corrupt ledger only |
| R04 AcceptanceStands | `accepted` · any | any | none | `accepted` | MEASURED `:994-1000` |
| R05 CancellationStands | `cancelled` with a prepared acceptance candidate (`AcceptanceCandidate::Prepared{verification_event}`) · any | any | none; `CancellationStands{rejected}` names the rejected verification event | `cancelled`; no acceptance is recorded (the live machine's I-03) | MEASURED `:1001-1020` |
| R06 LiveOwnedChild | `running` or `cancellation_requested` · `running` | `ProcessCustody::LiveSameIdentity`, `pi_queue` clear | none; `ReattachObservationOnly{redispatch:false}`: observe the child again, never redispatch (noodle "PID adoption") | unchanged; the adopted child's own `Settle{…}` ends it (→ `verifying` / `repair_pending` / `effect_unknown`, or the `cancellation_requested` self-edge). `pi_queue` `Unreconciled` or `ClearPending` → `RetainUnknown` and the R07 column applies | MEASURED `:1038-1060`; the S-6 transient-scope reaper gap (bullet above) stays |
| R07 ProcessNotOurs | `running` or `cancellation_requested` · `running`; also `settled` with `Unreadable` | `PidReused{differs}`, `Unreadable{error}`, `Unobserved` (on the unsettled path) | `Settle{unsettled}` | `effect_unknown{cancel: carried}`. `PidReused` is permanent (quarantine by R10); `Unreadable`/`Unobserved` are transient: "we could not look" is not evidence, the attempt is re-run on the next RL-7 pass and the drill's second read is where it leaves the list | MEASURED `:1061-1143`; INFERRED Event |
| R08 WorkerAbsent | `running` or `cancellation_requested` · `running` | `ProcessCustody::Absent` (the `kill -KILL` case) | `Settle{unsettled}`; the reason is the acknowledgement class: `NotSeen` → `DispatchUnacknowledged`, `Correlated{generation}` → `AcknowledgedWorkerLost`, `Unrecorded` → `AcknowledgementUnrecorded`; a `Writable` workspace attaches R09's lease refusal beside the unknown, never instead of it (REC-G2) | `effect_unknown{cancel: carried}`; then R10 | MEASURED `:1083-1120`; INFERRED Event |
| R09 WorkspaceReuse | any · `settled` or worker absent | `WorkspaceReadback::Writable{bytes}` | none for the task; `WorkspaceReuseRefused{NotLeasedWritable \| ClockUnavailable \| LeaseClockNotComparable \| LeaseExpiredWritable}`; attempt `cleanup` stays `pending` (attempt-side event is P-1 PROPOSAL) | unchanged; the next `Begin` from `repair_pending` takes a fresh workspace (S2). A lease is compared only in its own receiver clock epoch; expiry alone never licenses reuse | MEASURED `:1104-1140`, `:1144-1170`, `:1230-1245` |
| R10 EffectAmbiguity | `effect_unknown{cancel}` · `unknown` with `effect ∈ {unknown, pending}` | none consulted (durable values only) | v3: none, `RetainUnknown` ("stays explicit"); the only exits are the operator's `task.resolve` (`Resolve{quarantine}` → `blocked`, `Resolve{abandon}`) — the STACK-MAP §3.5 gap. **v4 PROPOSED (DC-nn, below): RL-7 issues `Resolve{quarantine}` itself, disposition `by=recovery reason=<Unknown variant>`, when the reason is in the permanent class {`DispatchUnacknowledged`, `AcknowledgedWorkerLost`, `AcknowledgementUnrecorded`, `ProcessIdentityReused`, `UnexpectedAttemptState`, `CancellationPrecedesAcceptance`}; the transient class {`ProcessUnreadable`, `ProcessUnobserved`, `PiQueueUnreconciled`, `PiClearPending`, cleanup `NotRead`} stays `effect_unknown` and is re-run next pass** | v3: `effect_unknown` (self-loop, manual exit only). PROPOSED: `blocked{cancel}` = the named quarantine `recovery:<reason>`, listed by `task.list states:=["blocked"]`; `Resolve{abandon}` remains the operator's exit from `blocked` | MEASURED `:918-930` (v3 keeps it unknown). **UNMEASURED: the observation that would let recovery re-decide instead of quarantining — an effect readback.** None exists: `Observations` has `claim`, `clock`, `process`, `pi_queue`, `cleanup`, `workspace` and no effect field (`recovery.rs` struct). STACK-MAP §3.5 names what would supply it: the candidate's external effect ledgered as an observation before the verdict. Until that field exists the automatic edge is the quarantine, not a re-decide |
| R11 CleanupReadback | `verifying`, `repair_pending` or terminal · `settled`, `cleanup=pending` | `CleanupReadback::NotRead` → `CleanupCandidate`, keep pending; `Partial{remaining}` → `CleanupCandidate` (beside `Writable` → R12's `boundary`); `Complete` with `attempt.cleanup ≠ settled` → settle by readback | task: none; attempt-side `CleanupSettled` (P-1 PROPOSAL) | task unchanged; attempt `cleanup=settled` only by readback, never by elapsed time (RL-10's "confirmed empty within 10 s" is the SIGTERM path's own rule) | MEASURED `:1171-1210` |
| R12 VerificationBoundary | `verifying` (task at its verification, evidence or acceptance boundary) · `settled`, cleanup read | any | none from recovery (`VerificationOutstanding`: "infers nothing"); the verdict is re-issued by K4 `decide` from the ledgered observations → `Verdict{…}` | `verifying` → by the re-issued verdict: `verifying` (Passed, awaits `Accept`), `repair_pending` (Failed), `failed` (Invalid/Error/Timeout), `effect_unknown` (unreconciled → R10) | MEASURED `:1211-1228`; INFERRED: the re-decide is this row's automatic exit (STACK-MAP §3.5 "re-decides"); UNWRITTEN: whether RL-7 calls `decide` before `complete` or the dispatcher does after it |
| R13 CursorEpoch | ledger `epoch`, `event_high_water`, `restored_from` · a subscriber cursor | `cursor.epoch == restored_from` → `PriorEpochOfRestore`; `cursor.epoch ≠ ledger.epoch` → `EpochChanged`; `cursor.sequence > event_high_water` → `FutureSequence`; else `CursorSnapshotOnly{replay:false}` | none (not a task rule) | no task change; `events.subscribe` answers `resync_required` (the "every pre-restart cursor" line above); never a replay authorization | MEASURED `:1262-1297` |
| R14 UnexpectedState | attempt `queued` (a state the store never produces), or a task state `boundary` has no arm for | any | v3: `RetainUnknown{UnexpectedAttemptState \| TaskStateUnexpected}` → `Settle{unsettled}`, then R10 permanent class | v3: `effect_unknown` → quarantine. v4: `queued` is not in the enum (State map §2b), so a row spelling it fails `parse → None` (EX-05) before the policy runs: recovery cannot read the row, `recovery=complete` is withheld, D3 stays red, the operator restores (ATLAS §6 "Engine state lost or corrupt") | MEASURED `:905-916`, `:1255-1261`; INFERRED v4 reading from EX-05 + §2b |

What the table settles for the drill: after RL-7 the only non-terminal states an interrupted task may hold are `verifying` (R12, until `decide` re-issues), `running` under an adopted live child (R06, until it settles), `repair_pending` awaiting its next `Begin`, `effect_unknown` in the transient class (R07/R11, cleared by the next pass), and `blocked` (the named quarantine, R10 PROPOSED). Everything else is terminal. The budgets (`OPEN_ATTEMPT_LIMIT`, `CLEANUP_BATCH`, `WORKSPACE_REMOVAL_BUDGET`) are not in `recovery.rs` (it is a pure fn; they belong to the caller) and stay UNMEASURED.

Proposed, not applied (conflicts with the State map §2a/§2c and the card; the coordinator or Luke places the row):

> **DC-nn · Recovery issues `Resolve{quarantine}`.** State map §2c sources `Resolve{quarantine}` only from the `task.resolve` operation (S8), and card recovery §2 keeps the policy pure (EX-04: "a decision here never resumes, settles, cleans up or replays anything by itself"). STACK-MAP §3.5 requires that no task is left `effect_unknown` with only a manual exit. Resolution proposed: the policy stays pure and returns `RetainUnknown{reason}` unchanged; the **caller** (RL-7 in K6 startup, not the policy and not a second writer) maps the permanent-class reasons to `Resolve{quarantine}` through `transition`, writing a disposition row `by=recovery reason=<Unknown variant> rule=R08|R07|R14|R03`, and maps the transient class to no event. `blocked{cancel}` becomes the named quarantine; `task.get` carries `rule=` and `reason=` (a CD RC03 §4 reply extension, UNWRITTEN). Touches: State map §2c (a second source for `Resolve{quarantine}`, "recovery (permanent unknown)"), card recovery §8 (Called by: RL-7 consumes `Decision.reconciliation`), card task.resolve (a disposition row not from an operation), ATLAS D7 wording ("terminal or `blocked` with an R-row reason"). Reversible; no code under H-5.
