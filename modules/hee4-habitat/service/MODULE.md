# hee4-habitat · service
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-habitat` | UM:78 |
| Cluster | K5 (habitat operation and presentation) | UM:78; CMAP:3 |
| v3 origin | `service` (`service.rs`, `service/{observations,probe,local_probe,systemd}.rs`) | MA:28; AR:11; DP:15-18 |
| Status | PLANNING — HOLD; REFACTOR, not migrated | MIG:22 |

**Design section:** [K5 hee4-habitat › service (v4.2)](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K5%20hee4-habitat%23service%20%28v4.2%29): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | v4 shape | Source |
|---|---|---|
| Purpose | Probe **policy** over habitat services: pure policy over K0h I/O → a validated `Observation` that K6 commits via K1 | UM:78, UM:134 |
| Owned state | none durable; health facts persisted by K1 (`service_facts`, new, v4.2) via app | UM:78, UM:206 |
| Target shape | return a validated observation (removes service→store), one `bind()`, `LocalProbe` phase enum, **delete `service/systemd.rs`** | AR:11; UM:78, UM:250 |
| Allowed deps | `hee4-contracts`, `hee4-host` | UM:62-66 |
| Removed edges | service→store, service→worker (spawn goes via K0h) | CMAP:13; ER E1 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 / v4.2 (service.inspect/probe/action; needs op key + runner) | tag pushed | per-slice gate; l2 move (C02/C03) | H-1; **H-8** only for a J12 usefulness Score (AT:88, AT:192) |
Not on the v4.0 done-line (UM:161, UM:269). D-rows fed: none in v4.0.

## 4 · v3 basis
Flag **NOT FINISHED** — "read side built; the systemd adapter is comments only, no runner or op key, 3 actions unserved" (MA:28). Recommendation **REFACTOR** — writes the store directly; any caller can assert Useful health (AR:11).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Writes the store directly; undeclared service→store, service→worker edges | — | AR:11; CMAP:13 | **E1** (service→worker missing from CMAP Stage 2) |
| Profile binding written 3×; health not rebuildable after restart | — | AR:11 | — |
| `LocalProbe` option-soup: six `Option`s on one probe | `service/local_probe.rs:47-57` | APX AP-03 row; A-7 | — |
| `service/systemd.rs` is dead comments | — | AR:11; APX AP-09 row | — |
| Settle check + pinned-hash loop copied into numerical | — | AR:23 | — |
| Sites cited at `service.rs:93/130/174/277` are anchor comments; real sites in submodules | `service/observations.rs:93`, `service/probe.rs:130`, `service/local_probe.rs:174,277` | DP:15-18 | **E16** |
| Probe observations also land in the roster ledger (one row per probe) | — | AR:17 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| useful/health · inspect ladder `:408` · local probe verdict `service/local_probe.rs:174` · admit_recipe `service/local_probe.rs:277` · prepare_probe `service/probe.rs:130` · EXACT (DP:119) | not-Jev |
| validate `service/observations.rs:93` · TTL `:494` · THRESHOLD (DP:120) | not-Jev |
| UsefulResult for non-exact classes `:11-40` · JUDGMENT · no producer (DP:121) | **J12: Jev (grant), advisory** — Score "useful for its declared purpose" + Noul "responded at all"; expected-output contract stays code (JM:40) |

## 6 · Migrated inputs
None — REFACTOR (MIG:22). Service action schemas (`control-v1.{request,result}.service.{inspect,probe,action}.schema.json`) were migrated with actions and will be regenerated from K0 (MIG:16; UM:80).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-03 | `LocalProbe` option-soup (APX; A-7) |
| AP-09 | dead `systemd.rs`; public fields letting any caller assert Useful |
| AP-11 | service→store / service→worker edges (CMAP:13) |
| AP-01 | profile binding ×3; settle/hash copies (AR:11, AR:23) |
| AP-16 | usefulness is JUDGMENT — declare the kind; advisory only (J12) |
| A-7 | anti-exemplar is this module's probe (EXX:574) |
| EX-17 | fail-closed, sealed output (EXX:31) |
| EX-08 | "Verify pidfd before retaining" (EXX:22, :357): a value enters its retained phase only after verification. Kept because A-7's own **Use instead** names it for this probe's phase enum (EXX:590) |
| EX-04 | probe policy as pure named rules |
| D-09 | K5 cannot reach K1; the compiler enforces it |

## 8 · Interfaces
| Kind | Item | Status | Source |
|---|---|---|---|
| Actions | `service.inspect`, `service.probe`, `service.action` | v4.2 (C02/C03) | UM:161; IM:23 |
| Flow | K5 probe → validated `Observation` → K6 commits via K1 | new | UM:134 |
| Table | `service_facts` (K1) | v4.2 | UM:206 |
| Advice | J12 via `Advised<T>`, only after H-8 | held | JM:40; UM:133 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | No store import from K5 | planted `use hee4_core` fails with E0432/E0433 (UM:84) |
| 2 | `LocalProbe` is a phase enum; Useful health constructible only through the validator | planted illegal construction fails to compile |
| 3 | Health rebuildable after restart from `service_facts` | gate test: restart → same health line |
| 4 | `systemd.rs` absent | grep 0 |
| 5 | One `bind()` | census `duplicate_sites=0` (DEC:37) |
| 6 | Plants per probe rule killed by named tests; scoped mutants | Tier-1 (AT:149) |

## 10 · Open decisions and risks
- v4.2 scope; op key + runner undesigned (UM:161).
- J12 held for H-8/H-10/H-11 (AT:192-196).
- Risk: probe rows feeding the roster ledger reproduce the 4096 growth issue unless retention covers them (AR:17).

## 11 · Pull commands
```bash
grep -n 'service' ~/hee4-evidence/design/ULTRAMAP.md ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 28p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
sed -n 11p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
sed -n 40p ~/hee4-evidence/reference/v3-evidence-b5367bc/jev-decision-map-b5367bc.md
grep -n '^| E1 \|^| E16 ' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '/### A-7/,/### A-8/p' ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
```
