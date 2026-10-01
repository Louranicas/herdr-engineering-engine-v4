# hee4-core · store
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-core` | UM:74 |
| Cluster | K1 (task ownership and recovery) | UM:74; CMAP:3 |
| v3 origin | `store` (`src/store.rs`, `store/{verification,terminal,recovery,roster,backup,schema}.rs`, `migrations/001-008.sql`) | UM:190; AR:6; RT:10-11 |
| Status | PLANNING — HOLD | DEC:4 |
| Binary | none (library; composed by `hee4-app`) | UM:79 |

**Design section:** [K1 hee4-core › store](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K1%20hee4-core%23store): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | The single owner of **all durable state**: ledger, objects, outbox, operations, roster rows, cohort/step/judgment tables. Writes only what `transition()` returns; hosts the one idempotent-op primitive | UM:74; UM:36 (P2); UM:43 (P9) |
| Owned tables | tasks, attempts, attempt_*, operations, events, artifacts/acceptances, outbox, verifications, task_stops/dispositions, roster_* (7), + new cohort_threads/reports/dissent_log, workflow_steps, service_facts, judgments | UM:193-207 |
| Files | `~/.local/state/hee4/` `active.json`, `generations/<g>/ledger.sqlite3`, `objects/sha256/`, `attempts/` (fresh root, no v3 import) | UM:181; DEC:13 (D-U2); DEC:31 (V4-11) |
| Allowed deps | `hee4-contracts` only (state enums, bounds, row types) | UM:63, UM:74 |
| Forbidden | K2-K5, K0h, K0e; service/cohort/roster rows arrive **via app** | UM:66; UM:134 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1 contract spine (`transition` consumer) | P0 green | gate + t28-style test through `main` (AT:80) | — |
| **P2 store + custody** (primary) | P1 | gate + scoped `cargo mutants` with `CARGO_TARGET_DIR` unset; restore drill on disposable root; object-ceiling wall time with its N (AT:81) | — |
| P5 outbox reader for `events.subscribe` | P4 | l2 +3 incl. events.subscribe gate (AT:84; V4-14 DEC:34) | — |
| P6 first real `commission` on Luke's root | P5 + backup/2 + restore | D4 read-back (AT:61, AT:85) | H-4 (AT:188) |
Feeds D-rows: **D4** (AT:61), **D5** (AT:62), **D6** accept closure (AT:63), **D9** `serve_cgroup` column (AT:66; V4-15 DEC:35).

## 4 · v3 basis
Flag **PARTIAL** — "upgrade behind a verified backup, atomic cut points; no restore, `restored_from` never persisted, two cursor doors" (MA:10). Recommendation **REFACTOR** — state is a String with no transition fn (AR:6).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Task state `String`, written at ≥ 9 sites, no transition fn | `store.rs:751,796,828` | AR:6; AP-02 | "≥9 sites" unverified (ER §2) |
| Cancellation held twice (flag + state) | `store.rs:2239-2245`; `:1458` | UM:37; V2:33,51 | UM hop 18 corrected :1457→:1458 |
| `queued` never written; `repair_pending` IS written | `store.rs:1536`, `store/verification.rs:248`, `:2637` | UM:22 (C1) | E4 |
| Outbox inserted at 2 sites, never delivered | `store.rs:2024`, `store/terminal.rs:219`; `pending_delivery`/`acknowledge_delivery` `:2032,:2054` 0 callers | AR:33; UM:45 (P11) | E5 |
| `INSERT INTO operations` 4 sites + 3 `replayed_*` | `store.rs:1254,1651,1802`; `store/roster.rs:271`; `:1268,1281,1294` | UM:43; V2:34 | — |
| Frame bound re-typed as literal | `store.rs:1236` (+`:1617,:1705`) | AP-01; A-4 | — |
| Generation written as `"1"` | `store.rs:1252-1253` | AP-02; A-2 | — |
| Charge ≤ reservation enforced at 3 SQL sites (not via Ledger) | — | AR:6; AR:9 | — |
| Roster observations capped 4096 globally, never pruned | `store/roster.rs:615` | UM:42 (P8); AR:17 | — |
| Backup/1 4096-object ceiling | `store/backup.rs:21,:327` | RT:10; AT:62 | — |
| `store.rs:2601` as flag-read site | — | AR:2 | **unverified** (E8) |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| settle_attempt :1531 · begin precondition :2612 · resolve_intent :1705 · `obligation_of` :2284 · waiting :2410 · accept :1980 · require_normal :2073 · next_dispatchable :1439 — EXACT, "state is a String; no transition fn" (DP:62; locator :2297→:2284 E16) | not-Jev (EXACT, JM:22) |
| store/verification record_verification :234 — EXACT verdict → state (DP:63) | not-Jev |
| store/terminal finish_with_preparation :99 — EXACT (DP:64) | not-Jev |
| store/recovery · reconciliation · roster: worker_settlement · evidence_available :339 · selected_pins :97 — EXACT (DP:65) | not-Jev |
| (new) `judgments` append-only table — held | holds J-records only after H-8 (JM:70) |

## 6 · Migrated inputs
None — REFACTOR, redesigned from findings, not carried verbatim (MIG:22). Cross-module store suites (t04/t05/t06) stay in v3 and are named only (MIG:25). Migrated `t06_receipt_import` and `t28_control` import `store` and cannot compile until this crate exists (MIG:29-30). Namespace: state root `~/.local/state/hee4/` (V4-11, DEC:31); migration series restarts at 001 (UM:193) and **must not** require the `-- HEE3-ANCHORS-END` corpus marker (AP-45; PL:46 L10).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-02 | v3's `state: String` at the one door; v4 writes only `transition()` output |
| AP-01 | frame limit, charge rule, cancellation each had ≥ 2 sites here |
| AP-13 | outbox written never read; `queued` readable never written |
| AP-04 | roster observations: derived per-attempt set capped globally, never pruned |
| AP-09 | `pending_delivery`/`acknowledge_delivery` pub with 0 callers |
| AP-19 | generation `"1"` cannot be told from a constant by a single-submit test |
| AP-49 | fsync before ack; kill-test the real writer (backup, accept) |
| AP-45 | migration identity leaked the corpus marker |
| AP-32 | the §9 #9 scoped `cargo mutants` over the store doors runs with `CARGO_TARGET_DIR` unset and a runner-owned target dir, and its precheck plant must go red first; with a shared dir every mutant links the unmutated store and reports success *(rev 2026-10-01 open-tasks CN-19: V5 generic row made module-specific)* |
| EX-02 | one write door, deadline before COMMIT — keep, but typed bodies |
| EX-03 | `cut_point!` fault injection absent from release |
| EX-14 | `Generation::FIRST`, never `"1"` |
| EX-16 | backup manifest-last: partial → fsync → rehash → rename |
| A-1, A-2, A-4 | text state in the door; literal `"1"`; re-typed frame bound |
| D-09 | K1 may import only K0 — compiler-enforced |
| D-01 | each store stack quotes `l2_before/after` |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Serves (via K6 ports) | task.submit/get/list/cancel/resolve reads & writes; events.subscribe outbox reader (P5) | UM:109-114, UM:122-125, UM:158 |
| New CLI | `hee4 restore --into <dir> <backup-id>` → fresh generation, recovery to `complete` | UM:149; V4-15 |
| Internal op kind | roster install under `Owner::Deploy`, **not** wire `roster.update` | UM:135; AR:35 |
| Files | state root + backups to `/var/mnt/STORAGE-10TB/hee4-backups/` (different disk) | UM:181; AT:226 |

## 9 · Done criteria
| # | Criterion | Evidence / read-back |
|---|---|---|
| 1 | Every task/attempt state write goes through `transition`; property test enumerates 11×\|Event\| pairs against UM §5c, with v3 CHECK constraints as independent source | test name + `pairs=N/N`; UM:241 |
| 2 | One cancellation source (no `cancellation` column) | schema diff; D-U5 (DEC:16); UM:194 |
| 3 | Exactly one `INSERT INTO operations` site and one outbox insert fn | one-door census `duplicate_sites=0` (V4-17, DEC:37) |
| 4 | Outbox rows delivered and acked through events.subscribe | `acknowledge_delivery` production caller; P5 gate (AT:84) |
| 5 | backup/2: count + chained page digest, keyset paging, measured object ceiling | ceiling wall time recorded with N (AT:81; RT:48) |
| 6 | `restore` + recovery to complete; `restored_from` persisted | `restore backup=<id> ledger=<d> objects=n/n rto_s=<t> verdict=PASS` (AT:62), RTO ≤ 10 min |
| 7 | Roster retention: latest per record + per-principal cap | `roster.list` count ≤ cap after N synthetic attempts (AT:227) |
| 8 | `serve_cgroup` written at admission | D9 query (AT:66) |
| 9 | Mutation | scoped `cargo mutants` on store doors, runner-owned target dir, precheck plant red, survivors named (AT:81; PL:82 K3; REQ rank 7) |
| 10 | Plants | one plant per door under `--cap-lints=warn`, killing test named (AT:149) |

## 10 · Open decisions and risks
- D-U2 fresh root (proposed, DEC:13); D-U5 (DEC:16); retention K/N values are PROPOSALs (AT:225-226).
- Engine-state backup timer would contradict RC01 → H-12 (AT:197); backups stay in `serve` (V4-6, DEC:21).
- Risk: backup/2 object ceiling UNMEASURED (AT:242); RTO UNMEASURED (AT:241).
- Risk: row types for K3/K5 must live in K0, not here (V2:128).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-05 (V4-58): Home of roster retention: K1 store/roster (UM, MIGRATION) vs a K2 pure policy the store calls (card roster), which the dependency law…. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=~/herdr-engineering-engine-v4
rg -n 'hee4-core|store' $E/design/ULTRAMAP.md | head -60
sed -n 186,239p $E/design/ULTRAMAP.md                      # state map + transitions
sed -n 74,80p $E/design/DEPLOYMENT_ATLAS.md; sed -n 198,206p $E/design/DEPLOYMENT_ATLAS.md
rg -n '^\| store' $E/design/DECISION_POINTS-b5367bc.md
rg -n 'store' $E/reference/ERRATA-v3-evidence-b5367bc.md $E/reference/v3-evidence-b5367bc/{architecture-review,module-audit,route-to-v010}-b5367bc.md
rg -n 'store' $E/verification/V2-design.md
rg -n 'AP-0[1249]|AP-1[39]|AP-45|AP-49|EX-0[23]|EX-1[46]|A-[124] ' $R/docs/ANTIPATTERNS.md $R/docs/EXEMPLARS.md
rg -n 'D-U2|D-U5|V4-6|V4-8|V4-14|V4-15' $R/plan/DECISIONS.md
```
