# hee4-contracts · catalogue-data
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-contracts` | UM:71 |
| Cluster | K0 | UM:71 |
| v3 origin | `actions` catalogue data (`actions.rs` `Owner :345`, `Effect :404`, `mutates :485`, `Action :498`, `CATALOGUE :568`) + the tool projection from `worker/tools.rs` | UM:71, UM:75; migrated `src/actions.rs` |
| Not here | dispatch, the owner registry and `actions/control.rs` are K6 (`hee4-app/actions`) | UM:79 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K0 hee4-contracts › catalogue-data](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0%20hee4-contracts%23catalogue-data): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | The ~~21-id~~ 22-id action catalogue (21 + held `judge.inspect`, owner `Owner::Judge`, unregistered; *rev 2026-10-01 ratified V4-55 DC-09, V4-56 DC-24*) as data: id, `Owner`, `Effect`, schema, tool projection; **pinned by content digest**, emitted as JSON that bash/Pi generators enumerate | UM:49 (P15), UM:71, UM:80 |
| Replay | `Effect::mutates()` is what the K1 idempotent-op primitive derives replay from | UM:43 (P9) |
| Owned state | None | UM:71 |
| Allowed deps | serde only | UM:71 |
| Removes | the K2→K6 inversion `worker → actions` (tools projection moves here) | UM:75, UM:254; CMAP:15 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P0 | H-5 | skeleton | H-5 (AT:189) — AT:79 |
| P1 | P0 green | K6 dispatches by the `Owner` registry over this catalogue; `health`, `tools.*` served | none — AT:80 |
| P5 | P4 | `events.subscribe` entry served | none — AT:84, DEC:34 |
| P9 | tag | roster.* (v4.1), service.*/thread.*/analysis.* (v4.2) move from refused to served | O-15 (H-14) for analysis prod — AT:88, UM:159-163 |
Feeds D3 (`health`, AT:60) and D8 (every flow names its L2, AT:65).

## 4 · v3 basis
- **Flag:** actions PARTIAL — one 21-action catalogue, named unavailability; Pi doesn't read the tool column, 2 of the needed owner traits, no mutation gate (MA:20).
- **Recommendation:** actions HARDEN — dispatch on strings with 3 lists that must agree; `cli` field never read (AR:25). pi_extension REFACTOR — two tool-projection authorities (AR:16).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Two catalogues (JSON schema vs CATALOGUE) pinned by count only | — | AR:36; UM:49 | — |
| `Owner` on every `Action` already | `actions.rs:504` | UM:44 | — |
| `Action.cli` never read → dropped | — | AR:25; UM:253 | — |
| `worker/tools.rs` compiled into the lib, **no production caller** (not "test-only"); imports `crate::actions` | `worker/mod.rs:1005`, `worker/tools.rs:620` | AP-10; A-10 | **E6** |
| Undeclared edges incl. worker→actions: 7, not 6 | — | CMAP:19 | **E1** |
| Installer records roster revisions under unserved `roster.update` | `app/native_provider.rs:273` | AR:35; UM:135 | — |
| UNSERVED lines | `actions/control.rs:420/424/456` | IM:21-25 | **E10** |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| actions `admit` `actions/control.rs:343`, `replay_expired` `:313` · EXACT (DP:100; corrected locators) — consumers of this data in K6 | not-Jev; ER **E16** |
| None in DP for the catalogue table itself | — |

## 6 · Migrated inputs
| Path (under `migrated/v3-b5367bc/`) | Note |
|---|---|
| `src/actions.rs` (1,131 lines; 255 anchor lines) | data half comes here; strip anchor block (DEC:27) |
| `schemas/actions/*.json` (44 files), `generate_control_schema.py`, `README.stub.md` (999 anchor lines) | the emitted schema becomes a generated output of this crate (UM:80); stub README anchors dropped |
| tests: `tests/control_schema.py`, `tests/t28_actions.rs` (→ worker, MIG:32) | t28_actions waits for K2/K6 |
**On import:** `hee3.control` protocol name → `hee4` (V4-11, DEC:31); drop `cli`; add a content digest; tools projection added from `worker/tools.rs` (REFACTOR, not migrated).

## 7 · Quality guard
| id | why here |
|---|---|
| AP-12 | three lists that must agree → one table + registry |
| AP-28 | count pins on catalogues → content digest |
| AP-10, AP-11 | tools.rs projection inverted the layers |
| AP-09 | `cli` field never read |
| AP-01 | schema and CATALOGUE were two doors |
| EX-05, EX-19 | one spelling per id; one door for a digest |
| A-10 | test-only module shipped as public API |
| D-09, D-07 | compiler dependency law; one topic, one home |

## 8 · Interfaces
Serves: control API action table (UM:153-164); `tools.list`/`tools.inspect` (UM:156); schema emitted for bash wrapper and Pi (UM:151, UM:80). Used by K6 owner registry (UM:106) and K1 replay (UM:104).

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | Catalogue pinned by content digest; the JSON schema is generated from it | a planted edit to one entry changes the digest and fails the named test |
| 2 | No second id list anywhere (control, schema, Pi, bash) | one-door census `duplicate_sites=0` (DEC:37) |
| 3 | Pi generator refuses a duplicate tool (`duplicate_tool`) | control case (UM:80) |
| 4 | `use hee4_worker` impossible from here; worker has no edge to actions | P1 compiler control E0432/E0433 (UM:84) |
| 5 | Every `Effect` variant's `mutates()` asserted whole over two fixtures | mutants on `mutates` killed (AP-19) |

## 10 · Open decisions and risks
- `judge.inspect` held (UM:164, H-8); catalogue must be able to carry a held id without serving it.
- Installer operation kind `Owner::Deploy` (UM:135) must not reuse a wire id — owned by K1/K6, pinned here as an `Owner` variant.

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 43,44p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 49p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 151,162p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 20p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
sed -n 16p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md; sed -n 25p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
rg -n '^\| E(1|6|10) ' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
rg -n 'pub (enum|struct) (Owner|Effect|Action)|fn mutates|pub const CATALOGUE' migrated/v3-b5367bc/src/actions.rs
ls migrated/v3-b5367bc/schemas/actions | wc -l
```
