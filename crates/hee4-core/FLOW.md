# hee4-core: flow

K1. The SQLite ledger is the single home of task state; startup reconcile runs to completion
before any dispatch. Sources: `modules/hee4-core/{store,recovery,task}/MODULE.md`,
`gates/features/crash-restart.md` (R01–R14), `gates/features/task.submit.md`, STACK-MAP §3.5.

## The rule

**No SQL outside `src/store/`.** `tests/one_door.rs` walks `src/**` recursively and runs a
regex-free token census outside `src/store/`, two families: the identifier `rusqlite` as a whole
token anywhere (a `use`, a path, a type, a comment: a module cannot call a `Connection` method
without naming the crate, so this closes `prepare_cached`, `query`, `query_one`, `raw_execute`
and the rest without chasing names), and the call tokens `execute`, `execute_batch`, `prepare`,
`query_row`, `query_map`, `pragma_update`, `pragma` and `operate` (the one door that hands a
`Transaction` to a closure, which needs no `rusqlite` token), each not preceded by an identifier
character and followed by optional spaces and `(`. It fails on a hit, requires at least one file
under `src/store/` to hold a call (the door exists), and proves the walk catches planted calls: a
copy of `src` with one `conn.execute(` in `recovery.rs` and one `conn.prepare_cached(..)` +
`st.query(..)` (no listed verb) in `backup.rs` names both as offenders, the latter by the crate
name alone. The shell form of the same door:
`grep -rn -E '(^|[^A-Za-z0-9_])rusqlite([^A-Za-z0-9_]|$)' crates/hee4-core/src --include=*.rs |
grep -v src/store/` prints nothing.

Inside the door, `apply_in` is the only code that writes `events` or `tasks`; `Store::apply` and
`Store::admit` (through `operate`) are its two callers. `src/backup.rs` is the non-SQL half of
the backup door: it imports no rusqlite item and calls `Store` doors only.

## Writers (each one transaction, committed under WAL + `synchronous=FULL` before it returns)

| Door | Writes | Refuses (writes nothing) |
|---|---|---|
| `Store::apply(task, Event)` | one `events` row + the `tasks` cache row | `transition` refusal; `Dispatch` while `recovery_complete=0`; unreadable history |
| `Store::operate(key, bytes, f)` (crate-private) | `f`'s writes inside the transaction + one `operations` row with the derived `operation_id` and `f`'s `(task_id, subject, result)` | same key, other sha256 → `Conflict`; same key, same sha → replay (no write); `f`'s `Err` → rollback, no row |
| `Store::admit(task, OperationKey, bytes, f)` | `operate`'s caller: `Admit` via `apply_in` (stamping `tasks.serve_cgroup`), subject = the task id | as `operate`; `Refused` if the task exists |
| `store/roster.rs`: `roster_update`, `roster_disable`, `roster_compose_deploy` (each one `operate` closure; m005_roster `roster_records` + append-only `roster_revisions`) | the record row (generation 1 or +1, `disabled`) + one `roster_revisions` row carrying the `operation_id`; the deploy record under principal `deploy`, action `deploy.install` | `StaleGeneration{current}`, `NotFound`, `Conflict`; a replay writes nothing |
| `Store::record_observation(task, id, obs)` | `observations` row | same id, other body |
| `Store::append_receipt(receipt)` | `receipts` row | chain with it fails `Receipt::verify_chain`; cites an observation not ledgered for its task |
| `Store::checkpoint_if_due(every: NonZeroU64)` (`src/store/receipts.rs`, migration `m005_checkpoints`) | one `checkpoints` row (`seq`, `upto_receipt_seq`, `count`, `root`; UPDATE/DELETE abort): `root` = `Receipt::checkpoint` (RFC 6962) over every `receipts.hash_self` in `seq` order up to `max(seq)`, so each root covers a prefix of the whole log across tasks; `every` is the caller's `ledger.checkpoint_every` budget field | fewer than `every` receipts since the last checkpoint → `None`, nothing written. Readers in the same file: `Store::open_read_only(path)` (`SQLITE_OPEN_READ_ONLY`, no pragma, migration or `meta` write; refuses a file without `migration:m005_checkpoints` as `Corrupt`; a WAL file in an unwritable directory is reopened `immutable=1` unless a non-empty `-wal` sits beside it), `verify_chain(task)` → `ChainReport{task, receipts, head}` and `verify_ledger()` → `LedgerReport{receipts, tasks, checkpoints, root}`, both `Err(VerifyError::Fault(ChainFault{receipt, seq, cause: Link|SelfHash|ColumnMismatch|Unparsable|RootMismatch|MissingReceipt|MissingCheckpoint}))` at the first row that does not re-derive (or `VerifyError::Store` when unreadable); `hee4 verify-ledger --ledger P` is their offline caller |
| `recovery::reconcile(store, observations)` | events only through `Store::apply`; the `meta.recovery_complete` flag | — |
| `Store::open(path)` | every `migration:<name>` row the file lacks (with its DDL), `PRAGMA user_version = count`, `meta.epoch` once at creation, `meta.boot += 1`, `meta.serve_cgroup` (this process's `/proc/self/cgroup` `0::` path), `meta.recovery_complete=0`; then ledger, `-wal`, `-shm` → 0600 and the dir → 0700 (idempotent) | a `migration:` row (or legacy `user_version`) this binary does not know → `UnknownMigration(name)` |
| `Store::mark_restored_from(epoch)` | `meta.restored_from` (its only writer; `backup::restore` calls it) | — |
| `Store::snapshot_into(dest)` | `dest`: one consistent snapshot of the committed state by `VACUUM INTO` (never a file copy of a live WAL ledger) | an existing `dest` |

Readers for the health body and R13: `schema_version()` (`PRAGMA user_version`),
`serve_cgroup()`, `boot()`, `epoch()`, `event_high_water()` (max `events.seq` or 0),
`cursor_check(epoch, seq)`, `operation_by_key(key)`, `last_operation_for(subject)` (newest `ts`,
ties by rowid).

## Migrations (`src/store/migrations.rs`; all tables `STRICT`)

`MIGRATIONS` is an append-only slice of `{name, apply: fn(&Transaction)}`; `open` applies every
entry whose `meta` row `migration:<name>` is absent, in slice order, writing the row in the same
transaction, then sets `user_version` to the count of applied rows (informational; the doctor
reads it). Seeding: a file with no `migration:` rows is read by its legacy `user_version`
(`2` → m001 and m002 applied, `1` → m001, `0` → none, other → `UnknownMigration`). A row naming
a migration not in the slice is refused by name. Never drop, rename or renumber an applied
migration; never edit the v1/v2 text.

| Name | Adds |
|---|---|
| `m001_v1` | `tasks`, `events` (+ append-only triggers), `operations`, `observations`, `receipts` (+ triggers), `meta` |
| `m002_cache_heals` | `cache_heals` |
| `m003_operations_subject` | `operations` recreated with `operation_id` (`op-` + 24 hex of sha256 of the four key fields joined by `\n`), nullable `task_id`, `subject` (= `task_id` for the copied rows) |
| `m004_serve_cgroup` | `tasks.serve_cgroup TEXT NOT NULL DEFAULT ''` |
| `m005_service_facts` | `service_facts(service_id PK, owner_id, unit_id, owner_sha256 CHECK len 64, generation CHECK >= 1, cached_health_json NULL, updated_ts)`; SQL only in `store/service.rs` |

```
tasks(id PK, phase CHECK(11 spellings), cancel 0|1, generation >= 0, updated_ts,
      serve_cgroup)                                        -- cache; serve_cgroup set on INSERT only
events(seq PK AUTOINCREMENT, task_id FK, event_json, ts)   -- source of truth; UPDATE/DELETE abort
operations(operation_id UNIQUE, principal, action, version, idem_key, request_sha256,
           task_id FK NULL, subject NULL, result_json, ts,
           UNIQUE(principal, action, version, idem_key))
observations(id PK, task_id FK, json, ts)
receipts(seq PK, id UNIQUE, task_id FK, json, hash_prev, hash_self UNIQUE, ts,
         UNIQUE(task_id, hash_prev))                       -- UPDATE/DELETE abort
meta(key PK, value)      -- epoch, boot, serve_cgroup, recovery_complete, restored_from, migration:*
cache_heals(seq PK, task_id FK, cached_phase, replayed_phase, ts)   -- heal record
```

`event_json` is `hee4_contracts::Event`'s own `Serialize` output; `codec.rs` decodes it with
`serde_json::from_str::<Event>`, so a foreign spelling (`"queued"`) is `StoreError::Corrupt`, never repaired (EX-05).
`tasks.phase` is `TaskState::replay(events).phase().as_str()`, rewritten in the same transaction
as each event; reconcile reports a mismatch as a finding. When `apply_in` finds the cached phase
divergent from replay it overwrites it (replay wins) and inserts a `cache_heals` row in the same
transaction; `Store::cache_heals()` reads them.

## Backup (`src/backup.rs` + `Store::snapshot_into`)

`backup_to(store, work, dest_root, SameDisk)` writes `<dest_root>/<id>/ledger.sqlite3` (by
`snapshot_into`), `objects/<name>.brief` for every `<work>/briefs/*.brief` (at most
`MAX_BACKUP_OBJECTS`, else `ObjectsOverBound`), and `manifest.json` LAST, renamed into place
(`id`, `ts_ms`, `epoch`, `boot`, `task_count`, `objects_n`, `objects_bound`, `files: {path:
sha256}`); `id` is `b-<ts_ms 12 hex>-<boot 8 hex>`, lexically sortable. A dir without a
manifest is incomplete by construction and `restore` refuses it (`Incomplete`). `SameDisk::Refuse`
(serve's default) refuses a `dest_root` on the ledger's device (`SameDevice`).
`restore(backup_dir, into)` refuses `TargetOccupied` when `<into>/ledger.sqlite3` exists,
verifies every sha256 before copying anything (`DigestMismatch{file}`, `ObjectsMissing{n,
total}`), then stages the ledger and `work/briefs/*.brief` under `<into>/.restore-<id>.tmp/`,
opens the staged ledger, records `restored_from` (the manifest's epoch), mints a fresh epoch and
runs `reconcile` with unobserved custody there, and only then renames the briefs and, LAST, the
ledger into place. A failure after the copy (`BackupError::Store`: a snapshot newer than the
binary answers `UnknownMigration`) removes the staging dir, so `<into>` holds no ledger and the
retry is not `TargetOccupied`; a `<into>/ledger.sqlite3` therefore means a completed restore.

Rules: backups stay in `serve`; there is no backup thread, timer or daemon (V4-6). The four
DC-22 triggers (freshness ≤ 15 min before any dispatch; after a task once freshness expired; at
every batch boundary ≤ 8 tasks; before an upgrade) are K6's to fire (wave-4
dispatcher-backups-ddf); no trigger policy lives here.

## Reconcile (pure policy `recovery::decide`, then `Store::apply`)

| Durable facts (replayed) | Observation | Rule | Event | After |
|---|---|---|---|---|
| non-terminal, claim epoch ≠ ledger | claim | R01 | — | unchanged |
| non-terminal, claim generation ≠ count(Dispatch) | claim | R02 | — | unchanged |
| accepted / cancelled | any | R04 / R05 | — | unchanged |
| running or cancellation_requested, attempt open | live same identity | R06 | — | unchanged |
| same | pid reused / unreadable / unobserved | R07 | `Recover(R07)` | effect_unknown{cr} |
| same | absent (`kill -KILL`) | R08 | `Recover(R08)` | effect_unknown{cr} |
| effect_unknown | any | R10 | — (quarantine is a DC proposal) | unchanged |
| repair_pending, failed, abandoned | any | R11 | — | unchanged |
| verifying | any | R12 | — (K4 re-decides) | unchanged |
| running with no open attempt; unreadable row; cache mismatch | any | R14 | — | finding |
| admitted, blocked, cancellation_requested with no attempt | any | none | — | unchanged |

R03 (contradiction between an `attempts` row and the events), R09 (workspace + lease) and R13 (cursor, `recovery::cursor`) fire over the attempts ledger: schema, hook and the reconcile rows are the module doc of `src/store/attempts.rs` (migration `m005_attempts`) and of `src/recovery.rs`; `src/probe.rs` is the only IO. R13 (cursor) has its inputs here now: `epoch`,
`event_high_water` and `restored_from`, read by `Store::cursor_check` (`PriorEpochOfRestore` →
`EpochChanged` → `FutureSequence` → `SnapshotOnly`, never a replay authorization); it fires in
the wave-2 attempts slice's `decide`, and `events.subscribe` answers `resync_required` from it.
`complete = no findings`; only then is `recovery_complete=1`, and only then does
`apply(_, Dispatch)` succeed. A second pass applies nothing.
