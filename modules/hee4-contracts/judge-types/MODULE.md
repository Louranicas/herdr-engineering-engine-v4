# hee4-contracts · judge-types
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub. **Doubly held:** Jev in the engine needs Luke's Engine Data Grant (H-8).

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-contracts` | UM:71 |
| Cluster | K0 | UM:71 |
| v3 origin | **none (new in v4)**; designed in the v3 jev-decision map §4.1/§4.3/§4.5 | JM:58, JM:66-69, JM:71-75 |
| Status | PLANNING — HOLD; types ship dark (no consumer calls out until granted) | AT:88 |

**Design section:** [K0 hee4-contracts › judge-types](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0%20hee4-contracts%23judge-types): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | The port types for advisory typed judgment: `Question{Noul,Choice,Score}`, `Answer`, `QuestionSet{id,version,digest}`, `Advised<T>{decision, advice, agreed}` (UM's shape; the K0 note's `Advised { advice: Advice }` sketch line is struck, *rev 2026-10-01 ratified V4-66, DC-46*), and `DataClass{public, synthetic, projection, local_private}` | UM:71, UM:219; JM:58, JM:66-69, JM:72 |
| Principle | P12: advice is never authority; a port behind a DataClass boundary | UM:46 |
| Owned state | None. The append-only `judgments` table is K1 (held) | UM:207; JM:70 |
| Allowed deps | serde only | UM:71 |
| Invariants carried by types | question sets are committed, pre-registered, digest-pinned (never task-derived); bounds 32k state / 255 choices / 2-10 levels live in `bounds` | JM:58, JM:62 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P0 | H-5 | types compile in K0 | H-5 (AT:189) |
| P9 | tag pushed **and** grant | Jev ports in shadow mode only, calling out only through K0e from K6 | **H-8** grant, **H-10** ZDR/DPA, **H-11** RC01 cost mode, **H-12** egress (AT:88, AT:192-197) |
No D-row depends on it (D1-D10 are v4.0; Jev is post-tag, AT:88).

## 4 · v3 basis
- **Flag / recommendation:** n/a (new). No v3 module; v3 had no engine Jev path (JM:3, JM:56).
- Counts behind it: 14 JUDGMENT vs 145 EXACT + 42 THRESHOLD (UM:46; JM:20-24); only JUDGMENT=14 is recountable (**E17**).
- Tally corrected: Jev (grant) 9 — J1,J2,J3,J7,J8,J9,J10,J12,J13; not-Jev 5 — J4,J5,J6,J11,J14 (JM:44).
- J1 correction: language-tagged fences already admitted; the gap is indented fences (**E11**).

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| Consumers of these types (JUDGMENT rows): candidates grammar :220-236 (DP:38), judge :403-452 (DP:40), render history :158-176 (DP:35), workload stderr :556 (DP:85), cohort outcome :843→954, context selection :1118, skills :230/:263, service usefulness :11-40, workflows :269 | J1, J2, J3, J7, J8, J9, J10, J12, J13 — **grant** (JM:29-41) |
| trim-empty, R11 ranking, diagnostics_of, notify category, Cohesion.jl | J4, J5, J6, J11, J14 — **not-Jev**, never get a `Question` (JM:32-42) |

## 6 · Migrated inputs
None — new in v4 (MIG lists no judge module). Design sources only: JM §4, UM:133.

## 7 · Quality guard
| id | why here |
|---|---|
| AP-16 | JUDGMENT sites must declare their kind and route through `Advised<T>`, never an EXACT parse |
| AP-13 | a `judgments` row with no reader is not written (calibration + `judge.inspect` are the readers) |
| AP-21 | labelled sets must come from an independent head (F113) before any gating (JM:74) |
| AP-02 | DataClass is an enum, never a string tag |
| A-6 | fence counting treated as exact |
| D-02, D-04 | no detector/design rounds on held work; admission header for any gate over advice |

## 8 · Interfaces
Used by K6 `DataClass` check before K0e (UM:133, UM:79), K1 `judgments` table (UM:207), K4 admission/calibration (UM:77), `judge.inspect` (held, UM:164), `~/.config/hee4/judge/` question sets + grant record (UM:182).

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | `Advised<T>` cannot yield authority: `decision: T` is always the code's value | type-level: no constructor takes an `Answer` as `decision` (compile-fail test) |
| 2 | `QuestionSet` carries id+version+digest; a set whose digest differs refuses by name | test over two fixture sets differing in every field (AP-19/AP-20) |
| 3 | Every state field consumed by a question carries a `DataClass` | K6 check refuses an unlisted class by name (JM:67-68) |
| 4 | Outage/refusal ⇒ `UNMEASURED`, code decision stands | variant exists and is asserted (JM:75) |

## 10 · Open decisions and risks
- All deployment is Luke's: grant (H-8), ZDR/DPA (H-10), cost mode (H-11), egress (H-12) — JM:86-91.
- Risk of over-engineering held work (D-01 brake): build types only at P0; no consumer wiring before the grant.
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-24 (V4-56): `judge.inspect` owner. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 56,91p ~/hee4-evidence/reference/v3-evidence-b5367bc/jev-decision-map-b5367bc.md
sed -n 29,44p ~/hee4-evidence/reference/v3-evidence-b5367bc/jev-decision-map-b5367bc.md
sed -n 46p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 133p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 217p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 184,188p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
rg -n '^\| E1[17] ' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
rg -n 'JUDGMENT' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
```
