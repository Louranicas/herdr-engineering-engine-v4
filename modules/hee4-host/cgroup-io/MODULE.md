# hee4-host · cgroup-io
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-host` | UM:72 |
| Cluster | K0h | UM:72 |
| v3 origin | `worker::aggregate` (transient slices/scopes via busctl, limit read-back), `worker::resources` (systemd-run, `validate_limits`, `parse_population`), `worker::namespace` I/O used by the live verifier | UM:72; IM:45-51; DP:51-54 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K0h hee4-host › cgroup-io](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0h%20hee4-host%23cgroup-io): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | The aggregate/namespace I/O the **K6 live-verifier adapter** needs: create/stop transient units, read back CPUQuota/MemoryMax/TasksMax/IOWeight/ControlGroup, read `/proc/self/cgroup` | UM:72; IM:48-50; UM:194 |
| Owned state | None durable | UM:72 |
| Allowed deps | K0 (and `spawn`/`clients` in this crate) | UM:62 |
| Why here | the adapter needs worker I/O without K4 depending on K2 (V4-13) | UM:77; DEC:33 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P3 | P2 | candidate runs in a transient slice with limits read back (CPUQuota 4 s/s, MemoryMax 16 GiB, TasksMax 256); N6d retirements applied | none — AT:82, AT:118 |
| P4 | P3 + Tier-2 | host record shows the slice read-back | H-9 — AT:83 |
| P6 (value; the `serve_cgroup` column itself is built in P2 as store work, ATLAS P2 Work cell) *(rev 2026-10-01 V11-fix; rev 2026-10-01 alignment: phases agree with MODULES.toml P3, P4, P6)* | P1 / P5 | `serve_cgroup` column built in P2 (ATLAS P2 Work cell); written at admission ends in `/hee4.service` | H-3 (enable) — AT:85, AT:187 |
Feeds D2 (`/proc/<MainPID>/cgroup` not under `libpod-`, AT:59), D6 (AT:63), D9 (`serve_cgroup`, AT:66).

## 4 · v3 basis
- **Flag:** worker FINISHED (MA:7; unverified, ER §2). **Recommendation:** worker REFACTOR — phase-enum observer instead of ~10 flags (AR:10).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Two parsers for one population format | `resources.rs:487`, `aggregate.rs:639` | DP:53 | — |
| `NamespaceObserver` ~10 flags (option-soup) | `namespace.rs:265-281` | AP-03; AR:10 | — |
| Stop kill literal 5 s (second door on TERM_GRACE) | `namespace.rs:1364` | DP:52; A-3 | — |
| `aggregate::main_pid`, `native::Systemd`, Manager arms retired (N6d) | `aggregate.rs:514` | UM:251; RT:43 (S1) | — |
| `stop_empty` locator | `worker/aggregate.rs:432` (was :443) | DP:54 | **E16** |
| Transient units from either side land under `user@1000.service`, outside the toolbox scope | — | AT:118 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| worker/namespace `terminal_status :1164 · native_status :1205 · status_consistent :1253 · stop mapping :1343 · scratch mounts :1691 · postrun :1775` · EXACT (DP:51) | not-Jev |
| worker/namespace `stop kill :1364` · THRESHOLD · literal 5 s (DP:52) | not-Jev |
| worker/resources `validate_limits :471 · parse_population :487` · EXACT · two parsers (DP:53) | not-Jev |
| worker/aggregate `unit_reply :589 · settled :663 · create_outcome :673 · stop_step :713 · resolve :730 · stop_empty :432` · EXACT · systemd readback (DP:54) | not-Jev |

## 6 · Migrated inputs
None — worker is REFACTOR (MIG:22). The namespace **policy** (status protocol, mounts) goes to K2 `hee4-worker/namespace`; only the I/O comes here (UM:72, UM:75).

## 7 · Quality guard
| id | why here |
|---|---|
| AP-01 | one population parser; grace from `bounds` |
| AP-03 | phase enum, not flag soup |
| AP-49 | every StartTransientUnit is read back via get-property |
| AP-06 | parse functions pure over bytes; busctl I/O thin |
| EX-08, EX-09, EX-18 | verify before retain; TERM/KILL once; one grace constant |
| A-3 | literal 5 s |
| D-09 | K4 reaches this only through K6's adapter |

## 8 · Interfaces
D-Bus `org.freedesktop.systemd1.Manager.StartTransientUnit` / `StopUnit` / `get-property` via pinned busctl (IM:48-51); `systemd-run` scopes (IM:57); `/proc/self/cgroup` read for `serve_cgroup` (UM:194, V4-15 DEC:35).

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | One population parser | census `duplicate_sites=0` (DEC:37) |
| 2 | Every created unit's limits read back and compared; mismatch refuses by name | host record line (AT:118) |
| 3 | No Manager/MainPID walk code remains | grep 0 for `main_pid` (UM:251) |
| 4 | Parsers tested with fixtures recorded from `busctl` on the host (independent source) | fixture provenance |
| 5 | Plants per rule killed by named tests; scoped mutants, CARGO_TARGET_DIR unset | `plants=k/k`, `mutants survivors=` (AT:149) |

## 10 · Open decisions and risks
- ~~Whether this card should be a K6-private module instead of K0h (only the live-verifier adapter uses it) — UM:72 puts it in K0h.~~ Settled (*rev 2026-10-01 ratified V4-60, DC-38 N3/N5*): it stays in K0h, which holds every I/O door. It makes every D-Bus and `systemd-run` call (S-4/S-6), with K6 (the adapter, admission) as the caller and K2's plan as the input, and it holds the **one population parser**. The limits comparison is K2's pure `aggregate::readback` (PR-K2-08), and K0h `limits_match` is withdrawn.
- Host vs toolbox busctl digests differ (AT:114): the pin table must be per side.
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-42 (V4-58): When `serve_cgroup` is read. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 72p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 192p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 249p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 45,51p ~/hee4-evidence/reference/v3-evidence-b5367bc/interface-map-b5367bc.md
sed -n 51,54p ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 43p ~/hee4-evidence/reference/v3-evidence-b5367bc/route-to-v010-b5367bc.md
sed -n 54p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md; sed -n 113p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
rg -n '### (EX-08|EX-09|EX-18|A-3)' docs/EXEMPLARS.md
```
