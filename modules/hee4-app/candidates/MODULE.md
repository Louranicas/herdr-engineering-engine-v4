# hee4-app · candidates
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 | UM:65 |
| v3 origin | `app` → `src/app/candidates.rs` (ClassPrompt, render, grammar, judge, custody/settle) | DP:34-41 |
| Status | PLANNING — HOLD | DEC:4 |
| Jev placement | K2/K6 are the J1/J2/J3/J7 consumers in the candidate loop | JM:82 |

**Design section:** [K6 hee4-app › candidates](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23candidates): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | Content | Source |
|---|---|---|
| Purpose | The candidate loop: render the class prompt (with repair history), send to the native provider, parse the model reply (grammar), judge it into Replacement/Refused, settle child custody | DP:34-41 |
| Owned state | None durable; attempt rows are K1's | UM:74, UM:195 |
| Advisory (held) | Any Jev advice arrives as `Advised<T>{decision, advice, agreed}`; code decides | JM:72; UM:46 |
| Allowed deps | K0, K0h, K1, K2 (native execute), K3 (context, if composed) — via K6 | UM:65 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3: candidate namespace; U64 class builds musl + rust-lld; decoder proven on a recorded reply | P2 | gate; plants | none at P3; H-9 (TH-DEV: model-generated candidates without seccomp) is needed by P4 (its ATLAS §5 row reads "P4 R-exec row") |
| P5: fail → repair → verify → accept (B16 / F07) | P4 | host record F07 | none |
| P9: Jev ports J1/J2/J3 in shadow mode only if granted | tag | per-slice gate | H-8, H-10, H-11, H-12 |
Feeds D6 (AT:63). Sources: AT:82, AT:84, AT:88, AT:192-197.

## 4 · v3 basis
- **Flag:** `app` PARTIAL (MA:19). **Recommendation:** `app` REFACTOR; `candidates.rs:15` imports runtime — part of the runtime cycle (AR:7; A-5, EXX:543).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Fence parse counts column-0 ``` lines (a JUDGMENT treated as EXACT) | `candidates.rs:220-236` (as DP:38, JM:29; A-6's header says `:219-236` because its excerpt starts at the `let mut start = 0;` initialiser on `:219`) | DP:38; AP-16; A-6 (EXX:556) | — |
| `starts_with("```")` already admits a language-tagged fence; **the gap is indented fences** | `candidates.rs:224` | JM:29 | **E11** (ER:21) |
| `judge` has no plausibility check; Tools arm unreachable (`has_tool_proposals: false`) | `:403-452`; `native.rs:1636` | DP:40; JM:7 | — |
| Repair feedback is a fixed template | `:158-176` | DP:35; JM:31 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `ClassPrompt::new :133` · EXACT · empty/sha/utf8 refuse (DP:34) | — |
| `render history :158-176` · JUDGMENT · fixed template (DP:35) | **J3: grant** (JM:31) |
| `render bound :187` · THRESHOLD · MAX_PROMPT_BYTES (DP:36) | not-Jev (JM:23) |
| `grammar truncation :213` · EXACT · Finish==Length (DP:37) | — |
| `grammar fence parse :220-236` · JUDGMENT (DP:38) | **J1: code first, then grant** (JM:29; E11) |
| `grammar emptiness :237` · JUDGMENT(weak) · trim-empty (DP:39) | **J4: not-Jev** (JM:32) |
| `judge :403-452` · JUDGMENT · no plausibility (DP:40) | **J2: grant, advisory** (JM:30) |
| `custody_settled :458 · settle_children :492 · ready :531` · EXACT (DP:41) | — |

## 6 · Migrated inputs
None — `app` is REFACTOR (MIG:22).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-16 | fence counting is a judgment; declare the kind, give a labelled fixture set (A-6) |
| AP-21 | reply fixtures come from recordings, never hand-typed (AT:138) |
| AP-04 | prompt bound at acquisition; model reply bytes bounded before parse |
| AP-05 | verify the fence rule at the flow: what truncation already cut before `grammar` sees it |
| AP-08 | break the `candidates → runtime` import (AR:7) |
| AP-14 | an empty/odd reply is refused by name, never defaulted |
| A-6 | model-output fence counting as exact rule — the v3 instance |
| EX-05 | Replacement/Refused as a closed enum, unknown → None |
| EX-07 | custody settle as a pure function of one poll |
| D-02 / D-03 | the five-step loop of §2 (render, send, parse, judge, settle) is written first as a compiling skeleton with `MAX_ATTEMPTS = 3` (§9 #4), then at most 2 design rounds *(rev 2026-10-01 open-tasks CN-19: V5 generic row made module-specific)* |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| provider | native provider → ollama `POST generate` via K0h curl | UM:169 |
| ports (held) | `Advised<T>` → K6 DataClass check → K0e egress (J1/J2/J3) | UM:133; JM:72 |
| consumers | runtime lifecycle (execute), repair (history), u64-class (class prompt) | UM:114 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Fence parser handles indented fences and nested fences; ambiguous remainder refused by name | fixtures from recorded replies incl. indented fence (E11); independent source (F113, AT:138) |
| 2 | Every decision site states its kind in its doc | a census over the 7 DP sites (DP:34-40) prints `kinds_stated=7/7` (**to build at "start coding"**) |
| 3 | No import of runtime from candidates | the `use`-graph census over `crates/hee4-app/src` (runtime §9 #1) prints `cycles=0` and no `candidates → runtime` edge (**to build at "start coding"**) |
| 4 | Loop spends ≤ MAX_ATTEMPTS (3) candidates; each refusal names which rule | test over 3 attempts, rows 2 and 3 asserted whole (AP-19 off-origin) |
| 5 | Plants on truncation/fence/judge killed by named tests under `--cap-lints=warn`; scoped mutants, `CARGO_TARGET_DIR` unset | `plants=k/k killers=named`; `mutants caught= survivors=` (AT:149) |

## 10 · Open decisions and risks
- J1/J2/J3 need H-8 grant, H-10 DPA/ZDR, H-11 cost mode, H-12 egress; ship dark until then (AT:192-197; JM:86-91).
- Labelled evaluation sets do not exist (JQ:33 F-E, habitat-side; engine-side none) — no gating before them (JM:74).
- H-9 TH-DEV for v4 unmeasured; it blocks P4, not P3 (ATLAS §5, H-9 row).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-45 (V4-66): Who consumes Jev advice for J1/J2/J3/J7. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
V4=~/herdr-engineering-engine-v4; EV=~/hee4-evidence; R=$EV/reference/v3-evidence-b5367bc
sed -n '34,41p' $EV/design/DECISION_POINTS-b5367bc.md
sed -n '29,32p;72p;82p;86,91p' $R/jev-decision-map-b5367bc.md
sed -n 21p $EV/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '77p;79p;83p;133p;184,188p' $EV/design/DEPLOYMENT_ATLAS.md
sed -n '556,573p' $V4/docs/EXEMPLARS.md
rg -n '^\| \*\*AP-(04|05|08|14|16|21)\*\*' $V4/docs/ANTIPATTERNS.md
```
