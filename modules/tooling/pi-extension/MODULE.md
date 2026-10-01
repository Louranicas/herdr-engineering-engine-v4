# tooling · pi-extension
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | none — **outside the crate DAG** (`integrations/pi`) | UM:80 |
| Cluster | outside (v3 design: K6) | CMAP:3; UM:80 |
| v3 origin | `pi_extension` (no Rust module; a Pi generator/register over the 21 action ids) | MA:29; AR:16 |
| Status | PLANNING — HOLD; the Pi lane is post-v1 (F17 declared absent in v1) | DEC:4; RT:44, RT:56 |

**Design section:** [T tooling › pi-extension (post-tag)](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/T%20tooling%23pi-extension%20%28post-tag%29): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Pi extension exposing engine actions as Pi tools; reaches the socket only through the `hee4` binary | IM:11; UM:151 |
| Owned state | None; generated artifact only | UM:80 |
| Reads | the tool projection K0 emits as JSON (one authority) | UM:71, UM:80 |
| Must | enumerate K0's JSON; refuse `duplicate_tool`; register only catalogue actions (PI-02) | UM:80; MA:29 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 Used (post-tag) (INTERP: F17 "Pi lane, post-v1", RT:44) | tag pushed | per-slice gate (AT:88) | **H-11** if a remote Pi needs a paid RC01 profile (AT:196) |
Not on the v4.0 done-line.

## 4 · v3 basis
Flag **NOT FINISHED** (MA:29): "No Rust module; register admits any action (PI-02 absent), no usage/evidence in render". Recommendation **REFACTOR** (AR:16): two tool-projection authorities (Rust `tools.rs` vs the Pi generator over all 21 ids) → projection in `actions::tools` emitted as JSON that the Pi generator enumerates; `duplicate_tool` refusal.

| Finding | Source | Errata |
|---|---|---|
| Two tool-projection authorities | AR:16; UM:49 (P15) | — |
| "PI-02 register admits any action" | MA:29 | **ERRATA §2**: UNVERIFIABLE from the tree (ER:36) |
| `worker/tools.rs` (the Rust half) is compiled with no production caller; its projection moves to K0 | UM:254 | **E6** (not "test-only") |
| Pi does not read the tool column | MA:20 (actions row) | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| pi register + Calls :128/:175 · EXACT (DP:131) | not-Jev |

## 6 · Migrated inputs
None: pi_extension is REFACTOR (MIG:22). Input to regenerate from: the `control-v1*.json` files in `migrated/v3-b5367bc/schemas/actions/` (the catalogue schemas, migrated with actions), which v4 re-emits from K0 (UM:80). Names follow `hee4` (DEC:31).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | two projection authorities (Rust + generator) is the v3 defect |
| AP-12 | lists of action ids that must agree: enumerate K0's JSON instead |
| AP-27 | the generator's denominator must be K0's catalogue, not a hand list |
| AP-10 | the Rust projection shipped as test-only-ish public API (A-10) |
| A-10 | do not re-create `worker/tools.rs` |
| EX-19 | imitate: one door for a digest — pin the catalogue by content (P15) |
| D-09 | K2 → K6 inversion came from the projection; K0 owns it now |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Uses | `hee4` binary as producer (no direct socket) | IM:11 |
| Reads | K0 emitted tool projection JSON; catalogue content digest | UM:71, UM:49 |
| Serves | Pi tools for the served actions ~~(v4.0: health, tools.*, task.*)~~. The catalogue's tool projection decides which: `health` and `task.resolve` have no tool there, so Pi serves none for them (*rev 2026-10-01 ratified V4-66, DC-20*) | UM:156-157 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | One projection authority | generator input is K0's JSON only; a planted second list is refused |
| 2 | `duplicate_tool` refusal | generator control plants a duplicate and asserts the named refusal (AP-22) |
| 3 | Register admits only catalogue actions (PI-02) | test registering an unknown action id fails by name |
| 4 | Catalogue pinned by content | digest mismatch refuses (UM:49) |
| 5 | usage/evidence in render | whole-line assertion over two fixtures differing in every field (AP-20) |

## 10 · Open decisions and risks
- Phase is INTERP; the ATLAS assigns no phase. Remote Pi cost mode is H-11.
- PI-02 status in v3 is unverifiable (ER:36), so v4 must prove it from scratch.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=$E/reference/v3-evidence-b5367bc
sed -n '49p;71p;80p;149p;154,155p;252p' $E/design/ULTRAMAP.md
sed -n '83p;187p' $E/design/DEPLOYMENT_ATLAS.md
sed -n '131p' $E/design/DECISION_POINTS-b5367bc.md
sed -n '16p' $R/architecture-review-b5367bc.md; sed -n '20p;29p' $R/module-audit-b5367bc.md
sed -n '44p;56p' $R/route-to-v010-b5367bc.md
grep -n 'E6 \|PI-02' $E/reference/ERRATA-v3-evidence-b5367bc.md
ls ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/schemas/actions/
```
