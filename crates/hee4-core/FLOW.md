# hee4-core: flow

K1. The SQLite ledger is the single home of task state; startup reconcile runs to completion
before any dispatch. Sources: `modules/hee4-core/{store,recovery,task}/MODULE.md`,
`gates/features/crash-restart.md` (R01–R14), `gates/features/task.submit.md`, STACK-MAP §3.5.

## The rule

**No SQL outside `src/store.rs`.** `tests/one_door.rs` walks `src/**` recursively and runs a
regex-free token census outside `store.rs`: `execute`, `execute_batch`, `prepare`, `query_row`,
`query_map`, `pragma_update`, `pragma`, each not preceded by an identifier character and followed
by optional spaces and `(`. It fails on a hit and proves it catches planted calls (including
`conn.query_row (`). Inside `store.rs`, `apply_in` is the only code that writes `events` or
`tasks`; `Store::apply` and `Store::admit` are its two callers.

## Writers (each one transaction, committed under WAL + `synchronous=FULL` before it returns)

| Door | Writes | Refuses (writes nothing) |
|---|---|---|
| `Store::apply(task, Event)` | one `events` row + the `tasks` cache row | `transition` refusal; `Dispatch` while `recovery_complete=0`; unreadable history |
| `Store::admit(task, OperationKey, bytes, f)` | `Admit` via `apply_in` + `operations` row with `f`'s result | same key, other sha256 → `Conflict`; same key, same sha → replay (no write) |
| `Store::record_observation(task, id, obs)` | `observations` row | same id, other body |
| `Store::append_receipt(receipt)` | `receipts` row | chain with it fails `Receipt::verify_chain`; cites an observation not ledgered for its task |
| `recovery::reconcile(store, observations)` | events only through `Store::apply`; the `meta.recovery_complete` flag | — |
| `Store::open(path)` | schema v1 (`user_version`), `meta.epoch`, `meta.recovery_complete=0` | newer `user_version` |

## Schema v2 (v1 plus `cache_heals`; v1 files migrate on open; all `STRICT`)

```
tasks(id PK, phase CHECK(11 spellings), cancel 0|1, generation >= 0, updated_ts)   -- cache
events(seq PK AUTOINCREMENT, task_id FK, event_json, ts)   -- source of truth; UPDATE/DELETE abort
operations(principal, action, version, idem_key, request_sha256, task_id FK, result_json, ts,
           UNIQUE(principal, action, version, idem_key))
observations(id PK, task_id FK, json, ts)
receipts(seq PK, id UNIQUE, task_id FK, json, hash_prev, hash_self UNIQUE, ts,
         UNIQUE(task_id, hash_prev))                       -- UPDATE/DELETE abort
meta(key PK, value)                                        -- epoch, recovery_complete
cache_heals(seq PK, task_id FK, cached_phase, replayed_phase, ts)   -- v2; heal record
```

`event_json` is `hee4_contracts::Event`'s own `Serialize` output; `codec.rs` decodes it with
`serde_json::from_str::<Event>`, so a foreign spelling (`"queued"`) is `StoreError::Corrupt`, never repaired (EX-05).
`tasks.phase` is `TaskState::replay(events).phase().as_str()`, rewritten in the same transaction
as each event; reconcile reports a mismatch as a finding. When `apply_in` finds the cached phase
divergent from replay it overwrites it (replay wins) and inserts a `cache_heals` row in the same
transaction; `Store::cache_heals()` reads them.

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

R03 (both closing records), R09 (workspace) and R13 (cursor) need facts this ledger does not hold
(no attempts table, no cursor); they never fire here. `complete = no findings`; only then is
`recovery_complete=1`, and only then does `apply(_, Dispatch)` succeed. A second pass applies
nothing.
