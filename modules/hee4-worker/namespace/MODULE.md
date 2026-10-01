# hee4-worker · namespace
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-worker` | UM:75 |
| Cluster | K2 | UM:75 |
| v3 origin | `worker` → `worker/namespace.rs`, `worker/namespace_shim.rs`, `src/bin/namespace_shim.rs` | UM:82; IM:60; DP:51-52 |
| Status | PLANNING — HOLD; REFACTOR, not migrated | MIG:22 |
| Binary | **`hee4-namespace-shim`** (v3 `hee-namespace-shim`; in-namespace exec, reports `HEE3_NATIVE_STATUS_V1` lines) | UM:82 (V4-11); IM:60 |

**Design section:** [K2 hee4-worker › namespace](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K2%20hee4-worker%23namespace): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | v4 shape | Source |
|---|---|---|
| Purpose | The candidate namespace: bwrap `--unshare-all` (network-less), clearenv, cap-drop, tmpfs `/work` 4 GiB; the shim's status protocol and postrun. *(rev 2026-10-01 ratified V4-60, DC-38 N2/N4)* This module owns the **pure plan** (`namespace::spec`, a typed value). K0h renders the argv and flags from it (PH-4), and K0h spawn's one TERM→KILL is the only stop (EX-09). There is no stop loop here | AT:82; AT:110, AT:120; UM:173 |
| Observer | phase-enum observer replacing `NamespaceObserver` (~10 flags) | AR:10; UM:75; APX AP-03 row |
| Owned state | none durable | UM:75 |
| Spawn | only through the K0h spawn door (`process.rs:547` in v3) | UM:47, UM:72; IM:62 |
| Allowed deps | `hee4-contracts`, `hee4-host` | UM:62-66 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3 (namespace + musl/rust-lld self-contained candidate) | P2 | gate; plants per rule | none (AT:82) |
| P4 host record (R0 netns differs from host) | P3 + Tier-2 | host record | **H-9** TH-DEV (no seccomp until T15) re-affirmed for v4 (AT:83, AT:193) |
| P9 T15 isolation (seccomp, hostile matrix) | tag | — | **H-14** O-15 (AT:199) |
D-rows fed: D6 ("candidate (musl, netns separated)", AT:63); D1 (shim digest in manifest, AT:58).

## 4 · v3 basis
Flag (worker) **FINISHED** — assertion only, no mutants record (MA:7; ER §2). Recommendation (worker) **REFACTOR** — NamespaceObserver ~10 flags (AR:10).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Stop kill hard-codes `Duration::from_secs(5)`: a second door on `TERM_GRACE` | `namespace.rs:1364` (vs `resources.rs:46-48`) | DP:52; APX AP-01 row; A-3 | — |
| `NamespaceObserver` Option/flag soup | `namespace.rs:265-281` | APX AP-03 row; AR:10 | — |
| Shim status lines parsed | `namespace.rs:1215`; shim `namespace_shim.rs:226,291-296` | IM:60; UM:82 | — |
| bwrap spawned from `live_verifier` as well as worker | `live_verifier.rs:35` | IM:56 | — |
| Host has no `cc`; candidates must link musl + rust-lld from `~/.rustup` | — | AT:41, AT:111, AT:127 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| terminal_status `:1164` · native_status `:1205` · status_consistent `:1253` · stop mapping `:1343` · scratch mounts `:1691` · postrun `:1775` · EXACT · status protocol, mounts (DP:51) | not-Jev |
| stop kill `:1364` · THRESHOLD · **literal 5 s, a second door on TERM_GRACE** (DP:52) | not-Jev |

## 6 · Migrated inputs
None — REFACTOR (MIG:22). Namespace rename: binary `hee4-namespace-shim`; the status line prefix `HEE3_NATIVE_STATUS_V1` should become a K0 constant under the hee4 name (V4-11, DEC:31; UM:82). Seam strings in the shim count toward `seam_strings=0/0` (AT:58, AT:161).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | the 5 s literal re-spells TERM_GRACE (DP:52) |
| AP-03 | observer flag soup → phase enum (AR:10) |
| AP-10 | seams must not reach the release shim (AT:161) |
| AP-31 | every stop/wait path carries a budget with both numbers |
| AP-49 | read back mounts/limits after setup, not the call's success |
| EX-18 | one grace constant, compile-time checked, rendered into the other enforcer (EXX:32) |
| EX-09 | TERM/KILL once, revalidated (EXX:23) |
| EX-08 | verify pidfd before retaining (EXX:22) |
| A-3 | the anti-exemplar *is* this module's `:1364` (EXX:521) |
| D-09 | spawn only via K0h; no K4/K6 import |

## 8 · Interfaces
| Call | Door | Source |
|---|---|---|
| `/usr/bin/bwrap` candidate sandbox + `hee4-namespace-shim` | K0h spawn / K6 live-verifier adapter check run (V4-13, DC-01 applied via V4-39) | UM:173; IM:56, IM:60; DEC V4-13 |
| network | none (`--unshare-all` includes net) | AT:116 |
| scratch | tmpfs `/work` 4 GiB | AT:120 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | One grace constant; the stop path (K0h spawn's one TERM→KILL, *rev 2026-10-01 ratified V4-60*) reads it | census `duplicate_sites=0`; plant `from_secs(5)` re-spelling is refused (EX-18 compile-time check) |
| 2 | Observer is a phase enum: an illegal phase pairing does not compile | planted illegal state fails `cargo check` with the named error |
| 3 | Candidate netns ≠ host netns; mounts as declared | host record R0 row (AT:63 "R0 netns differs") |
| 4 | Release shim `seam_strings=0` with a positive control over a seamed build | D1 read-back (AT:58, AT:161) |
| 5 | Plants per status-protocol rule killed by named tests under `--cap-lints=warn`; scoped mutants | Tier-1 lines (AT:149) |

## 10 · Open decisions and risks
- H-9 (TH-DEV without seccomp) and H-14 (T15 isolation) are Luke's (AT:193, AT:199).
- Whether bwrap is spawned from the K6 live-verifier adapter or only via this module: V4-13 makes live_verifier an adapter (DEC:33); keep one spawn site.
- Risk: the shim's status format is a wire protocol between two binaries — pin it by content (P15, UM:49).

## 11 · Pull commands
```bash
grep -n 'namespace\|bwrap\|shim' ~/hee4-evidence/design/ULTRAMAP.md ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
grep -n 'worker/namespace' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 56,62p ~/hee4-evidence/reference/v3-evidence-b5367bc/interface-map-b5367bc.md
sed -n '/### A-3/,/### A-4/p;/### EX-18/,/## Durable/p' ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
grep -n 'H-9\|H-14' ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
```
