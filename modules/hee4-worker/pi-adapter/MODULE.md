# hee4-worker · pi-adapter
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

**Thin evidence.** UM:75 lists "pi" among the worker members — "worker (native, namespace, aggregate, inference, pi)". The v3 `pi_extension` module has **no Rust module** (MA:29) and is a K6/tooling projection (UM:80); this card covers only a worker-side Pi execution adapter, whose v3 file is not named in any v4-owned source. The Pi extension itself has its own card (`modules/tooling/pi-extension/`).

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-worker` | UM:75 |
| Cluster | K2 | UM:75 |
| v3 origin | `worker` (sub-member "pi"); related: `pi_extension` (NOT FINISHED, REFACTOR) | UM:75; MA:29; AR:16 |
| Status | PLANNING — HOLD; not migrated | MIG:22 |

**Design section:** [K2 hee4-worker › pi-adapter](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K2%20hee4-worker%23pi-adapter): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | v4 shape | Source |
|---|---|---|
| Purpose | INTERP: execute a candidate via a Pi (remote/agent) provider behind the same execution port as native | UM:75 |
| Gating | a remote Pi lane needs RC01's paid profile (H-11); F17 "Pi lane, post-v1" | AT:196; RT:44 (S2 text) |
| Owned state | none durable | UM:75 |
| Allowed deps | `hee4-contracts`, `hee4-host`; **no network egress** (egress only via K0e from K6) | UM:62-66; DEC:32 (V4-12) |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 (post-tag, if at all) | tag pushed | per-slice gate | **H-11** RC01 cost mode (remote Pi); **H-12** egress (AT:196-197) |

## 4 · v3 basis
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| pi_extension: no Rust module; register admits any action (PI-02 absent — **unverifiable from the tree**) | — | MA:29; ER §2 row 2 | ER §2 |
| Two tool-projection authorities (Rust tools.rs vs the Pi generator over all 21 ids) | `worker/tools.rs` | AR:16 | **E6** (tools.rs compiled into lib, no production caller) |
| Pi `register` + Calls | `:128/:175` (integrations/pi) | DP:131 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| pi register + Calls `:128/:175` · EXACT (DP:131) — belongs to the tooling extension, listed here for the port contract | not-Jev |

## 6 · Migrated inputs
None (pi_extension is REFACTOR, MIG:22).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-10 | `worker/tools.rs` shipped as lib API with no production caller, inverting K2→K6 (A-10) |
| AP-11 | the worker→actions edge must not reappear (CMAP:15; E1) |
| AP-12 | any tool list must come from K0's emitted catalogue, pinned by content (UM:49, UM:71) |
| A-10 | anti-exemplar: tools.rs (EXX:616) |
| D-09 | no K6 import from K2 |
| D-05 | build nothing before F17 is scheduled |

## 8 · Interfaces
Tool catalogue read from K0 emitted JSON (UM:71, UM:80). No socket, no egress.

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Decision recorded: build (with H-11) or retire the member from UM:75 | `plan/DECISIONS.md` row |
| 2 | No `use hee4_egress` / `hee4_app` from K2 | planted-`use` control fails with E0432/E0433 (UM:84) |

## 10 · Open decisions and risks
- OPEN: v3 file for "pi" under worker is unnamed in v4 sources — UNMEASURED.
- Remote Pi needs H-11/H-12; out of the v4.0 done-line.

## 11 · Pull commands
```bash
grep -n 'pi\b\|Pi ' ~/hee4-evidence/design/ULTRAMAP.md | head
sed -n 29p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
sed -n 16p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
grep -n '^| E6 ' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '/### A-10/,$p' ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
```
