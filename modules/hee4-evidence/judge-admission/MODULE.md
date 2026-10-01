# hee4-evidence · judge-admission
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub. **Doubly held:** Jev in the engine needs Luke's Engine Data Grant (H-8).

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-evidence` | UM:77 |
| Cluster | K4 | UM:77; JM:80 |
| v3 origin | **none (new in v4)** — no v3 module; designed in the Jev decision map §4 | JM:56-84 |
| Status | PLANNING — HOLD; engine Jev **held for grant** | DEC:4; UM:270; CH §3 |

**Design section:** [K4 hee4-evidence › judge-admission](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K4%20hee4-evidence%23judge-admission): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Admission and calibration statistics for Jev question sets: decides whether a question set may move from shadow (advice recorded) to *gating* | JM:74, JM:80; UM:77 |
| Admission record requires | labelled set, per-family threshold on p (no porting across families), negative control, an any-flag or all rule stated | JM:74 |
| Owned state | None durable. Reads K1 `judgments` rows (question-set digest, state digest, model, p, tokens) passed in by K6; never the state itself | UM:207; JM:70 |
| Allowed deps | `hee4-contracts` (K0: `Question{Noul,Choice,Score}`, `QuestionSet{id,version,digest}`, `DataClass`), `hee4-host` (K0h). Never K0e egress (only K6 may depend on it) | UM:62-66, UM:71, UM:73 |
| Law | Advice is never authority; outage/refusal = `UNMEASURED`, the code decision stands | UM:46 (P12); UM:133; JM:71-75 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 Used (post-tag): "Jev ports in shadow mode only if granted", calling out only through K0e from K6 | tag pushed + grant | per-slice gates (AT:88) | **H-8** Engine Data Grant, **H-10** ZDR/DPA, **H-11** RC01 cost mode, **H-12** network egress (AT:192-197) |
No D-row depends on it; the v4.0 done-line ships Jev ports dark (AT:192).

## 4 · v3 basis
Flag **n/a (new)**; recommendation **n/a (new)**. The v3 basis is the JUDGMENT census: 14 of 201 sites are JUDGMENT; 9 are Jev (grant) candidates, 5 not-Jev (JM:44). No labels exist anywhere yet, so no threshold can be inherited (JQ:33, F-E).

| Finding | Source | Errata |
|---|---|---|
| Only JUDGMENT=14 is recountable; EXACT 145 / THRESHOLD 42 is the agents' assertion | DP:10 | **E17** |
| J1's gap is indented fences, not language tags (affects the J1 question design) | JM:29 | **E11** |
| The J-list was first tallied 8/6, corrected to 9/5 | JM:44 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| The admission rule itself (labelled set, per-family threshold, negative control, any/all rule) · EXACT | not-Jev (code owns admission, JM:22, JM:74) |
| Consumers this module admits: J1, J2, J3, J7 (candidate loop), J8, J9, J10, J13 (K3), J12 (K5) | grant (JM:44, JM:81-83) |
| Not admitted ever: J4, J5, J6, J11, J14 | not-Jev (JM:44) |

## 6 · Migrated inputs
None (new module). Names follow `hee4`; config lives at `~/.config/hee4/judge/` (question sets, grant record, 0600 key) (UM:182; DEC:31).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-16 | a JUDGMENT site treated as EXACT is exactly what admission exists to prevent |
| AP-21 | labelled sets must be written by a different head than the question (F113; JQ:51) |
| AP-22 | a negative control must fail for the rule's own reason, never for an outage |
| AP-29 | an outage is `UNMEASURED`, never PASS (JQ:16) |
| AP-24 | the admission control must neuter each admission rule in turn |
| AP-42 | scope the grant rule to the harm (engine data leaving the machine), not wider |
| EX-17 | imitate: fail-closed lattice — missing admission = not admitted |
| EX-05 | imitate: closed enums for Question kinds and DataClass |
| D-04 | admission header (trigger, budget, reader) applies to every calibration check |
| D-15 | labelling fan-outs need a planned count |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Action | `judge.inspect` (question sets, admission state, shadow agreement) — **held**, K6 serves it | UM:164; JM:84 |
| Table | `judgments` (K1, append-only, digests not state) — held | UM:207 |
| Egress | none here; K6 → K0e `POST api.typesafe.ai/v1/systemone`, held | UM:175 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | No question set gates without an admission record | test: a set without a record yields `Advised{advice, agreed}` but the code decision; plant bypassing the check killed by a named test |
| 2 | Per-family thresholds, no porting | test with two families differing in threshold; swapped-threshold plant killed |
| 3 | Outage = `UNMEASURED` | test over a refused/timeout answer: output line prints `UNMEASURED`, code decision stands (JM:75) |
| 4 | K4 does not depend on K0e | planted `use hee4_egress` in K4 fails with E0432/E0433 (UM:84) |
| 5 | Statistics pinned | scoped `cargo mutants`, `CARGO_TARGET_DIR` unset (AT:132) |

## 10 · Open decisions and risks
- Entire module blocked on H-8/H-10/H-11/H-12 (AT:192-197); build only the port shapes before grant, if at all (D-01 brake).
- Labelled sets: none exist (JQ:33); UNMEASURED.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=$E/reference/v3-evidence-b5367bc
sed -n '46p;71p;73p;77p;133p;162p;173p;180p;205p;217p;268p' $E/design/ULTRAMAP.md
sed -n '83p;184,188p' $E/design/DEPLOYMENT_ATLAS.md
sed -n '19,44p;56,91p' $R/jev-decision-map-b5367bc.md
grep -n 'E11\|E17' $E/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '23,32p' $E/design/JEV_QUESTION_RESPONSES.md
```
