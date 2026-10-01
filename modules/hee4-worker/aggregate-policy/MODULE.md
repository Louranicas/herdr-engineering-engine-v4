# hee4-worker · aggregate-policy
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-worker` | UM:75 |
| Cluster | K2 | UM:75 |
| v3 origin | `worker` → `worker/aggregate.rs` (+ `worker/resources.rs` limits) | UM:75; IM:48-50, IM:57; DP:53-54 |
| Status | PLANNING — HOLD; REFACTOR, not migrated | MIG:22 |
| Split | **policy** here; the busctl/systemd-run I/O the K6 live-verifier adapter needs lives in K0h (`hee4-host` cgroup-io) | UM:72, UM:171-172 |

**Design section:** [K2 hee4-worker › aggregate-policy](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K2%20hee4-worker%23aggregate-policy): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | v4 shape | Source |
|---|---|---|
| Purpose | Aggregate resource policy for candidates: transient slice/scope limits (CPUQuota 4 s/s, MemoryMax 16 GiB, TasksMax 256 read back), unit reply interpretation, stop steps | AT:118; DP:54 |
| Owned state | none durable | UM:75 |
| Dropped | `aggregate::main_pid` (`aggregate.rs:514`), Manager arms (N6d retirement) | UM:251; RT:43 |
| Allowed deps | `hee4-contracts`, `hee4-host` | UM:62-66 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3 (limits door in `contracts::rc01`) | P2 | gate; plants per rule | none (AT:82) |
| P4 host record (limits read back under `hee4…` slices) | P3 + Tier-2 | host record; digest-pinned busctl (host side) | H-9 (AT:193) |
D-rows fed: D6 (limits read back during the real task, AT:63).

## 4 · v3 basis
Worker flag **FINISHED** (assertion; MA:7, ER §2); recommendation **REFACTOR** (AR:10).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Two parsers for one population format | `resources.rs:487` twin `aggregate.rs:639` | DP:53 | — |
| `stop_empty` locator | `worker/aggregate.rs:432` (was cited :443) | DP:22, DP:54 | **E16** |
| `main_pid` retired with the daemon unit walk | `aggregate.rs:514` | UM:251; V2:40 | — |
| busctl digest-pinned by class profile | `class_profile.rs:166,541` | IM:51; UM:171 | — |
| Tool digests differ by side (host vs toolbox) | — | AT:114, AT:136 | — |
| One settle (`ProcessReport::settled()`) and one hasher belong in worker/host (copied in numerical and service) | — | AR:23 (numerical row); UM:72 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| unit_reply `:589` · settled `:663` · create_outcome `:673` · stop_step `:713` · resolve `:730` · stop_empty `:432` · EXACT · systemd readback (DP:54) | not-Jev |
| resources validate_limits `:471` · parse_population `:487` (twin `aggregate.rs:639`) · EXACT · two parsers for one format (DP:53) | not-Jev |

## 6 · Migrated inputs
None — REFACTOR (MIG:22). Slice/scope names move from v3 `hee3aggregate*` (AT:118) to the hee4 namespace (V4-11, DEC:31).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | two population parsers; limits spelled in several places (DP:53; AR:10) |
| AP-49 | a StartTransientUnit success is not a limit: read back CPUQuota/MemoryMax/TasksMax (IM:50) |
| AP-06 | policy over parsed properties must be pure; I/O stays in K0h |
| AP-26 | parse busctl replies structurally, never regex |
| EX-07 | pure settle step (EXX:21) |
| EX-18 | one constant, compile-time checked (EXX:32) |
| EX-11 | pure-over-values shape (EXX:25) |
| D-09 | busctl lives in K0h; this crate never spawns directly |

## 8 · Interfaces
| Call | Door | Source |
|---|---|---|
| `busctl` StartTransientUnit / StopUnit / get-property | K0h, digest-pinned | UM:171; IM:48-51 |
| `systemd-run` transient scopes | K0h | UM:172; IM:57 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | One population parser | census `duplicate_sites=0` (DEC:37) |
| 2 | Limits read back equal requested, refusal by name with both numbers otherwise | unit tests over recorded busctl replies (recording, not hand-typed — AP-21) |
| 3 | Host: slice limits read back during D6 | host record row (AT:63, AT:118) |
| 4 | No `main_pid`/Manager arm reachable | `habitat-unused-pub` + grep 0 |
| 5 | Plants per rule killed by named tests (`--cap-lints=warn`); scoped mutants | Tier-1 (AT:149) |

## 10 · Open decisions and risks
- ~~The brief's split name "aggregate-policy" vs K0h "cgroup-io" is a planning split; UM:72 places "worker aggregate/namespace I/O the K6 live-verifier adapter needs" in K0h — confirm the boundary at P3 skeleton.~~ Settled (*rev 2026-10-01 ratified V4-60, DC-38 N3/N5*). This module owns the pure aggregate limits plan and the pure `aggregate::readback` comparison (PR-K2-08). K0h cgroup-io owns the D-Bus/`systemd-run` calls and the one population parser. Done criterion #1's "one population parser" is K0h's, and this module consumes its value.
- Risk: pin digests valid on one side only; the manifest stores host pins (AT:136).

## 11 · Pull commands
```bash
grep -n 'aggregate\|busctl\|systemd-run' ~/hee4-evidence/design/ULTRAMAP.md ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
grep -n 'worker/aggregate\|worker/resources\|aggregate.rs' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 45,58p ~/hee4-evidence/reference/v3-evidence-b5367bc/interface-map-b5367bc.md
grep -n '^| E16 ' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
```
