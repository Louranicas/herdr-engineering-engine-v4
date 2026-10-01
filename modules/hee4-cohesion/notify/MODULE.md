# hee4-cohesion · notify
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-cohesion` | UM:76 |
| Cluster | K3 | UM:76 |
| v3 origin | `notify` (`src/notify.rs`) | MA:25; AR:13 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K3 hee4-cohesion › notify](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K3%20hee4-cohesion%23notify): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | **Delivery policy only**: a pure function over K1 outbox rows deciding what `events.subscribe` streams and when a row is acknowledged | UM:76; UM:125 |
| Owned state | none — the outbox has one owner, the store (2 outboxes → 1) | UM:74; UM:76; UM:200 |
| Row types | outbox/event row types live in **K0** (K3 cannot import K1) | UM:56; V2:128 |
| Allowed deps | `hee4-contracts` | UM:63 |
| Keys | store's recipient key (`Principal::recipient`, `principal.rs:51`), sequence and `EventCursorV1` | UM:74; AR:13 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| **P5 system scenario** — `events.subscribe` + `Reply::Stream` with the outbox reader; `acknowledge_delivery` gains its production caller | P4 | l2 +3 incl. events.subscribe gate (AT:84) | — |
| P9 milestone-2 notify flow | tag | per-slice gates (AT:88) | — |
On the done-line (V4-14, DEC:34); feeds D8 (AT:65).

## 4 · v3 basis
Flag **NOT FINISHED** — two outboxes, public witness with no caller, not called from main (MA:25). Recommendation **REFACTOR** — two outboxes; recipient format incompatible (UUIDv4 vs store `"uid:role"`); two cursor/sequence models; witness public, no caller (AR:13).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Second outbox `struct Outbox` | `notify.rs:845` | UM:255; V2:38 | — |
| Public `witness` with no caller | `notify.rs:514` | AP-09; UM:255 | — |
| Store outbox inserted at 2 sites, never delivered | `store.rs:2024`, `store/terminal.rs:219` | AR:33 | E5 |
| `Reply` is {Close, Frame}: streaming needs a contract change | — | AR:34 | — |
| task.get `delivery` reads "pending" forever | `app/tasks.rs:701,971` | UM:124; AR:33 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| admits :373 · subscribe :945 · compact/restore :1143 — EXACT (DP:115) | not-Jev |
| retryable :426 · wake :1094 · enqueue :904 — THRESHOLD (DP:116) | not-Jev |
| record attempt category :1005 — JUDGMENT, caller maps a transport failure (DP:117) | **J11: not Jev** — typed transport errors mapped in code (JM:39) |

## 6 · Migrated inputs
None — REFACTOR (MIG:22). v3 suites `t13`/`t22`/`t11_notify` stay in v3, named only (MIG:25). Migrated schemas `control-v1.{request,result}.events.subscribe.schema.json` exist under `migrated/v3-b5367bc/schemas/actions/` (actions card owns them).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | two outboxes, two cursor models, two recipient formats |
| AP-13 | outbox written, never delivered — the reason this module exists in v4.0 |
| AP-09 | `witness` pub with no caller |
| AP-06 | delivery/retry decisions pure over rows + a supplied clock |
| AP-31 | stream loop ends on close/cursor end; every await on it budgeted |
| EX-06 | delivery predicates read their arms' values |
| EX-19 | one door for cursor/sequence identity |
| D-09 | notify must not reach K1: row types in K0 |
| D-01 | the P5 stack must move l2 (events.subscribe) |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Action | `events.subscribe` — v4.0, phase P5, done-line (v3 refused, B20) | UM:158; IM:25 (locator :457→:456, E10) |
| Transport | `Reply::Stream` arm in `serve_connection` (hop 8) | UM:101; UM:126 |
| Reads | K1 outbox rows via K6 port; writes acks via `acknowledge_delivery` | UM:125 |

## 9 · Done criteria
| # | Criterion | Evidence / read-back |
|---|---|---|
| 1 | One outbox (store), notify has no storage type | grep: no `Outbox` struct in hee4-cohesion; one-door census `duplicate_sites=0` |
| 2 | events.subscribe streams accept + stop rows and acks them | socket test through `main`; `acknowledge_delivery` production caller (AT:84) |
| 3 | task.get `delivery` reflects a real delivered count | read-back after subscribe: `delivered` ≠ pending (UM:124) |
| 4 | Recipient key = store's `Principal::recipient` | test over two principals, whole-row assertions (AP-20 avoided) |
| 5 | Plants | planted skip of ack / cursor off-by-one killed by named test under `--cap-lints=warn` |
| 6 | Mutation | scoped `cargo mutants` on delivery policy, `CARGO_TARGET_DIR` unset |

## 10 · Open decisions and risks
- D-U1 decided by V4-14 (DEC:12, DEC:34); "delete the inserts" alternative withdrawn (UM:158).
- Risk: stream framing is a contracts change (AR:34) — coordinate with hee4-contracts and control-socket cards.
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-31 (V4-65): Stream park budget. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=~/herdr-engineering-engine-v4
rg -n 'notify|outbox|events.subscribe|Reply::Stream' $E/design/ULTRAMAP.md $E/design/DEPLOYMENT_ATLAS.md
rg -n 'notify' $E/design/DECISION_POINTS-b5367bc.md $E/reference/v3-evidence-b5367bc/*.md
rg -n 'outbox|E5|E10' $E/reference/ERRATA-v3-evidence-b5367bc.md
ls $R/migrated/v3-b5367bc/schemas/actions/ | rg events
rg -n 'D-U1|V4-14' $R/plan/DECISIONS.md
```
