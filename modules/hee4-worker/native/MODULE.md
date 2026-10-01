# hee4-worker · native
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-worker` | UM:75 |
| Cluster | K2 | UM:75 |
| v3 origin | `worker` → `worker/native.rs` (the native ollama provider) | MA:7; AR:10 |
| Status | PLANNING — HOLD; worker is REFACTOR, **not migrated** (redesign from findings) | MIG:22 |
| Binary | none (runs inside `hee4`) | UM:82 |

**Design section:** [K2 hee4-worker › native](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K2%20hee4-worker%23native): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | v4 shape | Source |
|---|---|---|
| Purpose | The native model provider: profile, daemon identity, endpoint resolution, transport, ollama exchange → `native::execute` on the spine | UM:75, UM:114 |
| Split | `native/{profile,daemon,endpoint,transport,ollama}` (v3 native.rs mixes ~7 concerns) | AR:10; UM:75 |
| Owned state | none durable; `ProviderState` enum moves to K0 | UM:75, UM:216 |
| I/O doors | curl (loopback only), `/proc/net/tcp` + fd scan live in K0h; native holds the pure policy | UM:72, UM:169-170 |
| Allowed deps | `hee4-contracts`, `hee4-host` | UM:62-66 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3 Worker + native + verdict | P2 | gate; plants per rule; **live-reply fixture committed from a recording**, not hand-typed | none — loopback to ollama not held (HO-04) (AT:82) |
| P4 first host record (spine F06) | P3 + Tier-2 | host record, scoreboard `host=1` recomputed | H-9 (TH-DEV + loopback re-affirmed for v4) (AT:83, AT:193) |
| P7 first real GPU task | P6 | `/api/ps` `size_vram > 0`; daemon exe pin `12ff8654…` | H-16 only if topology changes (AT:86, AT:201) |
D-rows fed: **D6** (real task on the GPU model, AT:63), D9 (AT:66).

## 4 · v3 basis
Flag (worker) **FINISHED** — "limits door, native execute, one settle, readback not redispatch, custody; no mutants record" (MA:7) — **assertion only: nothing was run** (ER §2 row 1). Recommendation **REFACTOR** — native.rs mixes ~7 concerns (AR:10).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Run limit spelled 4× (15 min here) | `native.rs:394` (+ `mod.rs:713,737`, `process.rs:541`) | AR:10; UM:39 | — |
| curl 180 s / 5 s literal | `native.rs:1300` | AR:10; DP:45; UM:169 | — |
| fd readlink bytes unbounded (count bounded, bytes not) | `native.rs:1074` | RR:24 (security-2); UM:42 | — |
| A same-uid unreadable co-holder is invisible to `holder()` | `native.rs:956` | RR:23 (security-1); UM:170 | — |
| `resolve` no-walk branch never exercised (doubles hard-code true) | `native.rs:1116` | RR:13 (tests-1) | — |
| endpoint_holder wiring pinned by nothing | `native.rs:1084` | RR:26 (tests-2) | — |
| Tools arm unreachable (`has_tool_proposals: false`) | `native.rs:1636` | DP:40; JM:7 | — |
| Oversized reply reads as `Error::Json` | `native.rs:663` | RT:43 (S1) | — |
| `native::Systemd` + native.toml `[daemon] unit/scope` → **retired** (N6d) | — | UM:251; RT:43 | — |
| curl pin cited at `native_provider.rs:505` is a `#[cfg(test)]` fixture; production path comes from `native.toml` | — | IM:39 | **E9** |
| Daemon is not a descendant of its unit's MainPID → resolve by the LISTEN holder of :11434 | — | AT:116, AT:137; IM:42 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| execute window `:1531` · THRESHOLD · MAX_RUN 15 min (`:394`) (DP:42) | not-Jev |
| execute selection `:1538` · EXACT · profile equality, name charset (DP:43) | not-Jev |
| ADAPTERS `:373` · THRESHOLD · num_ctx/num_predict table (DP:44) | not-Jev |
| exchange timeout `:1300` · THRESHOLD · 180 s / 5 s literal (DP:45) | not-Jev |
| clean `:1266` · catalogue `:1368` · resident `:1430` · response `:1606` · usage `:1613` · done_reason `:1618` · identity `:1626` · cancel `:1563` · EXACT (DP:46) | not-Jev |
| select_daemon `:855` · listener `:936` · holder `:967` · hash_file `:686` · daemon/resolve `:758,:1145` · EXACT · identity, pid-reuse guard (DP:47) | not-Jev |
(J1-J3 candidate-loop JUDGMENT sites live in K6 `candidates`, not here; JM:29-31.)

## 6 · Migrated inputs
None — worker is REFACTOR; redesign from findings (MIG:22). `tests/t03_contract` and `t28_actions` depend on worker and were migrated without it (MIG:28, MIG:32). Namespace for config: `~/.config/hee4/native.toml` (V4-11, DEC:31; UM:182).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | run limit ×4 and the curl literal → one `contracts::rc01` constant (AR:10) |
| AP-04 | readlink strings: the derived set needs a byte bound (RR security-2) |
| AP-08 | ~7 concerns in one file (AR:10) |
| AP-05 | N6c: a bound documented at `parse()` while `exchange()` truncates first — verify at the flow |
| AP-21 | the `Generated` decoder must be proven against the recorded live reply, not a hand-typed fixture (AT:82, AT:138) |
| AP-18 | endpoint doubles must record inputs (tests-1: every MainPid double hard-codes true) |
| EX-12 | pure listener/holder + bounded acquisition (EXX:26) |
| EX-13 | `take(N+1)` then refuse (EXX:27) |
| EX-05 | ProviderState one spelling (UM:216) |
| D-10 | endpoint checks: prefer types over more census |
| D-16 | done only when a real task is ACCEPTED on the GPU (D6) |

## 8 · Interfaces
| Call | Door | Source |
|---|---|---|
| ollama `127.0.0.1:11434` GET version/tags/ps, POST generate | K0h curl client (`--noproxy`) | UM:169; IM:38-41; AT:117 |
| endpoint holder identity (`/proc/net/tcp{,6}` + fd scan) | K0h bounded reader | UM:170; IM:42 |
| config `native.toml` | K6 loader | UM:182 |
| systemd Manager unit/scope for the daemon | **DROPPED** | UM:176 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | One run limit and one exchange timeout, read from K0 | one-door census `duplicate_sites=0` (DEC:37) |
| 2 | Endpoint resolution by LISTEN holder; unreadable co-holder → refusal by name; readlink bytes bounded | unit tests with recording doubles; plants for each rule killed by named tests under `--cap-lints=warn` |
| 3 | `Generated` decoder passes on the committed recorded reply | fixture provenance line (AT:82) |
| 4 | Host: daemon exe digest = pin, `/api/ps size_vram > 0` | HP-row (AT:230); D6 host record (AT:63) |
| 5 | D6: real task accepted through the unit | `hee4 task.get <id>` → `state=accepted`, P7 host record `evidence/host/F06u.json` made through the unit's socket (AT:63; F06/F07 are P4/P5 exit evidence only) *(rev 2026-10-01 V11-fix)* |
| 6 | Mutation: scoped `cargo mutants` on native policy (`CARGO_TARGET_DIR` unset) — worker had **no mutants record** in v3 | Tier-1 line (AT:149; UM:279) |

## 10 · Open decisions and risks
- UNMEASURED: whether the holder stays resolvable after a toolbox rebuild (pid/exe path change) → P3 (AT:240).
- H-9: TH-DEV + loopback-as-not-network need a v4 ruling (AT:193).
- H-16: ollama topology (toolbox-hosted vs host-native) (AT:201).
- Risk: host has no ROCm userland; a host copy silently falls back to CPU (AT:115) — guarded by the `size_vram` row.
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-10 (V4-60): Endpoint-holder identity: UM §4c says K0h; UM §2 K2 row lists `native/endpoint` in K2; DC-40 (V4-62): NF-ROSTER-INSTALL: phase and owner. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
grep -n 'native\|11434\|holder' ~/hee4-evidence/design/ULTRAMAP.md ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
grep -n 'worker/native' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 10p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
grep -n 'native.rs' ~/hee4-evidence/reference/v3-evidence-b5367bc/review-range-7ed3728-dfb32d5.md ~/hee4-evidence/reference/v3-evidence-b5367bc/interface-map-b5367bc.md
grep -n '^| E9 ' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '/### EX-12/,/## Types that refuse/p' ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
```
