# HEE v4 controls against drift and over-engineering (D-01 … D-16)

**Written:** 2026-10-01 · planning only (HOLD: no code until Luke says "start coding") · author Opus 5.5 (same lineage as every source).
**Companions:** [ANTIPATTERNS.md](ANTIPATTERNS.md) (code/process defects, AP ids) · [EXEMPLARS.md](EXEMPLARS.md) (code to imitate) · `gates/REQUIREMENTS.md` (the ten scaffolding obligations; this file does not restate them, it points at them) · `plan/DECISIONS.md` (where a control's adoption is recorded).
**Tags.** **FACT** = a number or quote at the cited path. **INTERP** = my inference. **INTENT** = a v4 mechanism that does not exist yet (nothing is built under the HOLD). **EXISTS** = a habitat tool or reflex that runs today.
**Source keys** are those of ANTIPATTERNS.md (CM = `~/CLAUDE.md`, SA = Stall Assessment, AH = The Adaptive Habitat seed table, PL = `~/hee4-evidence/learnings/PROCESS-LEARNINGS.md`, MM = Mistakes I Made, FF = Field Findings, …).

## Why this file exists (FACT)

- v3: **311 commits in 11 days, 0 tags**, flows proven through the running engine **2 of 20** after 46 landings (SA:12, SA:38, SA:46).
- The last ship, `deep-diff-forge`, reached v0.1.0 **11 h 53 m** after its first commit: one prose spec, one layer per commit, no design-review rounds, reviews after shipping, process files **0.11×** the product (SA:29-35).
- v3 verification: 105 gate/cold runs, 38.6 machine-hours, 19 measured nothing, **0 confirmed product defects** caught by the gate; nearly every real defect came from independent review, plants and mutation (SA:57-59).
- v2: scripts +683% vs crates +78%, gate steps 7 → 26, apparatus:product 1.98 : 1 (`~/.claude/hooks/new-artifact-guard.sh:6-8`).
- Verified stall causes: the product changed kind (C1), nothing forces a ship (C2), verification coupled to every commit (C3), rules accreted in kind with the June brake gone (C5); herdr refuted as the cause (SA:44-55).

**The two failure shapes** this file controls:
- **Over-engineering** — apparatus (detectors, gates, design rounds, corpus) growing faster than the product, where no number says it pays.
- **Drift** — work, state or rules moving away from the charter's done-line without anyone measuring the distance.

**What is never braked** (CM:31-33, RA6 via PL:112): the quality floor (zero warnings, `forbid(unsafe)`, no `unwrap` in library code) and the discovery layer (independent review, plant batteries, scoped mutation; PL K1-K3). Where a control below conflicts with them, quality has primacy (CM:19-20). A control that reduces *discovery* to save time is itself drift.

---

## The controls

Each control: **Symptom** it prevents · **Incident** that earned it · **v4 mechanism** (gate step / file / reflex) · **Measure** (the printed number; a control with no number measured nothing, CM:324-328).

### D-01 · The brake: every stack names the flow it moves
- **Symptom:** verification and apparatus grow while the product's done-line does not move.
- **Incident (FACT):** 2,319 lines and three designs for a detector on one rule a raw-v0 symbol check settles, while `l2` stayed at 2/20 across 46 landings (CM:28-34; SA:61; MEM/the-brake-quality-serves-shipping:16). After the flow ratchet was adopted, `l2` moved 2 → 3 at `dfb32d5`, the first move in 46+ landings (PL K4).
- **v4 mechanism (INTENT):** every landing record quotes the scoreboard line `flows= l2= partial=` **before and after**; a stack with delta 0 names the flow it enables or the check it retires, with a line count (SA:67-70). A version cut is an annotated tag whose message fields have one home: ATLAS §1 row D10 (`~/hee4-evidence/design/DEPLOYMENT_ATLAS.md`) *(rev 2026-10-01 V7/V8/V9/V10-fix, F21)*. One review round and one session per detector; after that the rule becomes a written trigger plus its one-line measurement (CM:30-31).
- **Measure:** `l2_before= l2_after= delta=` in every landing record; `gate_minutes_per_l2_delta=` (PL L1). EXISTS today in v3: `python3 tools/check-flow-scoreboard | tail -2`.

### D-02 · At most two design rounds, then land the buildable cut
- **Symptom:** prose design reviewed adversarially round after round; each round finds what a compiler states in seconds.
- **Incident (FACT):** B14a-2 took 4 rounds, all FAIL (HIGH 5 → 4 → 3); R14–R23 each took exactly 2 and none passed round 1; R23 alone cost 4 agents, 659,219 tokens, 187 tool calls; B14's DESIGN.md reached 4,412 lines for one route item vs 6,502 lines for the whole forge spec (PL L12; CA:85-91).
- **v4 mechanism (INTENT):** the review tool refuses round 3 with `refused: round cap 2; land buildable cut` and records the open items as a split (PL L12; gates/REQUIREMENTS.md rank 4). Applied already once in v3: B14c-E was closed as a disposition instead of a third round (PL K5, K6).
- **Measure:** `design_rounds=N` per slice (must be ≤ 2); `design_tokens=` per round from the fan-out journal; `design_lines/product_lines` at the cut.

### D-03 · Skeleton first: nothing is reviewed until it compiles
- **Symptom:** interfaces argued in prose; defects that are type errors found by reading.
- **Incident (FACT):** the forge had no design rounds and shipped (SA:33); C4 "design in prose over adversarial rounds before a skeleton runs" was contested 1/1 (SA:50) — so this control is supported, not proven (INTERP). Luke's 2026-09-26 ROI reset made it the rule (MEM/hee3-new-way-of-working:15-17).
- **v4 mechanism (INTENT):** `slice new` creates `FLOW.md` (≤ 1 page: flow id, inputs, outputs, reader, refusal names) and a compiling skeleton branch; the review tool refuses a subject sha that does not pass `cargo check` with `refused: no compiled skeleton` (PL L12).
- **Measure:** `skeleton_compiles=yes|no` at review admission; `flow_md_lines=` (≤ 1 page).

### D-04 · Detector admission header: trigger, budget, reader
- **Symptom:** bespoke detectors and gate steps accrete; each F-numbered finding becomes a permanent obligation on every slice (C5).
- **Incident (FACT):** the admission rule of the seed table — a trigger with a number; a signal command with a baseline taken before the block exists **and a named reader**; a cost bound; a detector budget of at most one review round and one session; one-in-one-out on always-loaded bytes (AH:222-227). A 42-control census still had 5 compiling false passes (MEM/census-receding-horizon:8-15).
- **v4 mechanism (INTENT):** every new `tools/check-*` or gate step opens with a header the gate parses:
  ```
  # admission: trigger=<number and where it was read> baseline=<value, date>
  # admission: budget=1 round/1 session  cost=<minutes per run>  reader=<who reads the output, where>
  # admission: retires=<check or line count it replaces, or none>
  ```
  A check file without it is refused by name (PL L13 proposes the `new-artifact` reflex **block** in the v4 repo; today the reflex only warns — EXISTS, warn-only, `~/.claude/hooks/new-artifact-guard.sh:20`).
- **Measure:** `checks=N admitted=N headerless=0`; per detector `survivors_round_1=` (feeds D-10).

### D-05 · Apparatus ratio printed at every cut
- **Symptom:** what is not counted grows — scripts, gate steps, governance, planning notes.
- **Incident (FACT):** v2 scripts 6 → 47 (+683%) while crates 9 → 16 (+78%); `AUTHORIZATION.md` the most-edited file (47 touches); 1,246 planning notes created in one day (PR P28 :600-611; PL L27). forge baseline 0.11× (SA:35).
- **v4 mechanism (INTENT):** the cut prints layer denominators — product, tests, apparatus (tools, gates, hooks), governance, planning — each as lines and as a ratio to product. The tag message carries `apparatus_ratio=` (one of the fields listed once at ATLAS §1 D10, *(rev 2026-10-01 V7/V8/V9/V10-fix, F21)*); a cut above the declared bound refuses without a reason line (PL L27; gates/REQUIREMENTS.md rank 10). Also print `largest_file=<path> lines=` (AP-08). EXISTS: `new-artifact` reflex asks at the moment an artifact is born.
- **Measure:** `apparatus_ratio=<x>` (baseline to beat: forge 0.11×; v2 1.98×); `largest_file=`.

### D-06 · Declare the scope fence at charter time
- **Symptom:** adjacent work started inside a session; the human detects drift and interrupts.
- **Incident (FACT):** 14 measured drift incidents, twice needing a mid-run interrupt in one session (CM:399-405); a whole session built on v2 while the work was v3 (MM #49, MM:1550).
- **v4 mechanism (EXISTS + INTENT):** `habitat-scope set --session "$HABITAT_SCOPE_SESSION" --charter "<x>" <roots>` at charter time; the `scope-fence` reflex warns on Write/Edit outside the roots (CLAUDE.local reflex table). v4 adds: `hee4.toml` declares module homes and the host/toolbox root, and `hee4 status` prints `root=<resolved> view=host|toolbox` (PL L24). Adjacent work found inside a fence is a **deferred item to raise**, never work to start (CM:402-403).
- **Measure:** `out_of_fence_writes=N` per session (reflex count). FACT from this document's own authoring: writing a temporary file in the session scratchpad fired the fence warning, because the scratchpad is not among the declared roots — a known false-positive class; record it rather than widen the fence silently.

### D-07 · One topic, one home (and a budget on always-loaded bytes)
- **Symptom:** the same rule or state written in two places; they drift; always-loaded surfaces bloat.
- **Incident (FACT):** two ancestral instruction files drifted for 480 sessions (CM:5-7). Always-loaded bytes 79,110 B before the seed; budget 77,144 B one-in-one-out; re-measured 75,547 B (AH:237).
- **v4 mechanism:** the crosswalk pattern (CM:5-13) — v4 already has `docs/INDEX.md` "Where things live", and `gates/REQUIREMENTS.md` owns obligations while PL owns evidence (FACT: `gates/REQUIREMENTS.md:3`). INTENT: `rules.toml` maps each rule id to exactly one enforcing site; a parsed census refuses a rule enforced at two sites (gates/REQUIREMENTS.md rank 9; AP-01).
- **Measure:** `rules=N sites=N duplicates=0`, **reader: `tools/gate`** (the REQ rank 9 census step refuses on `duplicates>0`). ~~`always_loaded_bytes=` (`wc -c ~/CLAUDE.md ~/CLAUDE.local.md ~/.claude/projects/-var-home-Louranicas/memory/MEMORY.md`) against the 77,144 B budget.~~ **No reader in v4 → dropped (D-08):** those files are habitat surfaces outside the v4 fence; the seed table (AH) owns that budget *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F13)*.

### D-08 · The frozen-corpus lesson: no reader, no row
- **Symptom:** a generated corpus and its publisher sit on the landing chain; the product acquires a second product nobody reads.
- **Incident (FACT):** publication took 18–44 min per run; 4 of 20 runs failed on the corpus agreeing with itself; 92.3% of inserted lines were hash bookkeeping; 296 GB reclaimed; 58,762 anchor lines in 79 tracked files, 2,403 in `Cargo.toml`; the store's migration identity required a corpus marker; code-writing subagents read the corpus 4 times against 952 code reads (PL L9, L10; SA:104: 88 GB that 0 of 7,782 transcripts opened). The corpus was frozen on 2026-09-28 (SA:89-92; MEM/hee3-plan-paradigm).
- **v4 mechanism (INTENT):** no publisher. Intent is small rows in the repo, each naming its reader; state is computed on read; the version cut is the tag (D-01). A scaffold lint refuses generated `BEGIN…END` spans in `src/`, `tests/`, `migrations/` and manifests (AP-45). Evidence never pins the diary (PL L11).
- **Measure:** `git log --grep='^Publication for' | wc -l` = 0; `orphan_intent_rows=0`; `generated_spans_in_src=0`.

### D-09 · Dependency law, enforced by the compiler
- **Symptom:** "dependency direction remains visible" stays a sentence; undeclared edges grow with each composed action.
- **Incident (FACT):** declared law "every module depends on `contracts` only"; measured 7 undeclared edges (CMAP's text says 6; ERRATA E1) *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F06)*, including the K2 → K6 inversion `worker → actions` (CMAP:3, :9-19); 12 unserved actions would each add composition edges (CMAP:22).
- **v4 mechanism (INTENT):** clusters are the **9 crates** of ULTRAMAP §2, so the compiler refuses an undeclared edge (UM §2 dependency law: K0 contracts ← K0h host ← {K2, K4, K5}; K0 ← {K1, K3}; K0 ← K0e egress ← K6 app only; K6 app is the only crate that sees more than one cluster; no Kn depends on another Kn). The law lives once: the `[workspace] members` list plus each crate's `[dependencies]` (UM §2). The control is ULTRAMAP §2's planted cross-crate `use`: it must fail `cargo check` with E0432/E0433 naming the planted path, plus a second plant, `use hee4_egress` in any crate but K6. **No gate step compares `cargo metadata` against a law table**: that was `tools/check-module-deps` under another name, dropped by V4-17 *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F06)*.
- **Measure:** the two planted-`use` controls each print the planted path from the compiler's own E0432/E0433 diagnostic (`cases=2/2`); reader: `tools/gate` (P0 Work cell, ATLAS §2) *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F06)*.

### D-10 · Receding-horizon rule: survivors per round must fall
- **Symptom:** a detector's rule surface grows faster than it can be covered; each round of review finds more ways to fool it.
- **Incident (FACT):** three rounds on one detector went 21 mutants all killed → 85 with 26 surviving → 63 clauses covered → 127 with 36 surviving (CM:122-128; F128, FF:2378). B14c-E: a hand-lexer census replaced by a compiler-decided symbol check (PL K16).
- **v4 mechanism:** record survivors per round in the detector's header (D-04). If round 1 does not bring survivors down, stop strengthening the check and ask what the compiler can refuse instead — types (EX-14, EX-05), privacy (EX-15, EX-17), crates (D-09), compiled symbols (MEM/census-receding-horizon:25). Prefer compiler-decided rules over source censuses (PL K16).
- **Measure:** `survivors_round_n=` series per detector; the detector is frozen when the series is not decreasing after its one budgeted round.

### D-11 · Stop rule: two unexcused zero-delta stacks stop the line
- **Symptom:** stack after stack lands with "enabler" claims while no flow moves.
- **Incident (FACT):** `l2=2` in 33/33 gate outputs across 46 landings (AH:234); the seed's rule: an "enabler for Fxx" claim expires if Fxx is not L2 within 2 stacks; **two unexcused zero-delta stacks stop the line and force a re-plan toward the nearest partial flow** (SA:67-71).
- **v4 mechanism (INTENT):** the landing tool reads the last two landing records; on the second unexcused `delta=0` it refuses the next slice with `stopped: zero_delta_streak=2; re-plan toward <nearest partial flow>` until a decision row in `plan/DECISIONS.md` names the re-plan.
- **Measure:** `zero_delta_streak=N` (stop at 2); `enabler_claims_open=N` with their expiry stack.

### D-12 · Checkpoint every ~3 phase boundaries
- **Symptom:** long autonomous runs compound drift and suppressions between checkpoints.
- **Incident (FACT):** "drift is checkpoint-bound; long unbroken runs are exactly when it compounds" (CM:406-408; PR P16 :300).
- **v4 mechanism:** after roughly three phase boundaries executed autonomously, pause and (1) re-assert the standard, (2) audit the range for suppressions (`git grep -n '#\[allow\|#!\[allow'` — v3 baseline 0 hits at b5367bc, FACT) and new `pub` items (`habitat-unused-pub`, EXISTS), (3) compare touched paths with the fence (D-06), (4) append a `checkpoint` row to the decision log. INTENT: `hee4 status` prints `phases_since_checkpoint=`.
- **Measure:** `new_allows=0` and `new_unused_pub=0`, **reader: the checkpoint row itself** (step 4 above quotes both lines). ~~`phases_since_checkpoint=` (≤ 3)~~ **no reader → dropped (D-08)**: nothing computes or reads it; the checkpoint stays a working-mode rule (`~/CLAUDE.md` §7) with its trigger, three phase boundaries *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F13)*.

### D-13 · State is a query, never a field
- **Symptom:** always-loaded prose says one state while the tree says another; resumes start from the wrong place.
- **Incident (FACT):** `CLAUDE.local.md` said `main = origin/main = 9d8c983` while main was `c82e0d2` (JU:74); 18 of 36 sampled status claims stale or wrong, all authored state (MEM/hee3-plan-paradigm; PL K12); updates appended to a superseded handover (MM #57, MM:1721); 21 `HEE3_*` handoff files, 714,986 B (PL L17).
- **v4 mechanism (INTENT):** `hee4 status` computes tree, dirty count, scoreboard, open asks and newest decisions; the handover is an append-only `decisions.jsonl` plus an `asks` list; the append target is computed at write time (PL L17; gates/REQUIREMENTS.md rank 10).
- **Measure:** **no reader → dropped (D-08)** for both `sha_literals_in_always_loaded=0` and `handoff_files=`: no v4 step reads either, and the always-loaded files are outside the v4 fence. The mechanism stays INTENT under REQ rank 10; its reader, when built, is the session resume step that runs `hee4 status` *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F13)*.

### D-14 · Admission's twin: pruning
- **Symptom:** a check that stopped catching anything keeps running, costing minutes on every landing.
- **Incident (FACT):** the seed table's pruning rule — freeze after 3 weekly re-measures at zero; retire after 6 weeks frozen (AH:229). The orchestration layer was frozen at ≤ 6 mentions a week and 7 cockpit calls ever (AH:240).
- **v4 mechanism (INTENT):** every admitted check carries `last_catch=<date|never>`; the cut report lists checks with `never` after 3 weekly re-measures as `freeze_candidates=`. The discovery layer is exempt (never pruned; AH:242).
- **Measure:** **no reader → dropped (D-08)** until a check exists to prune: `freeze_candidates=` and `retired_this_cut=` are deferred. Trigger: the first admitted check (D-04) reaching 3 weekly re-measures with `last_catch=never` *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F13)*.

### D-15 · Fan-out budgets (over-engineering by agent count)
- **Symptom:** review and refutation scale with an unbounded count of raised items; open briefs stall.
- **Incident (FACT):** 113 defects → 226 refuters, ~9.6 M tokens, 140 agents and the synthesis failed (MEM/size-workflows-to-usage-headroom:8-10); six reviewers on an answered question (MM #53, MM:1632); an open-brief reviewer 2 h 40 min vs 6 commands with a 20-command budget (PL L19).
- **v4 mechanism (INTENT):** the fan-out kernel (gates/REQUIREMENTS.md rank 3): `planned_agents=` required before the first launch; severity admission (HIGH/MED two lenses, LOW ≤ 1); refuters batched; every brief carries a command budget; a pre-fan-out search of same-day handoffs/evidence for the question.
- **Measure:** `planned_agents= launched= judged= unjudged=` (a vote nobody cast is UNJUDGED, never REFUTED — MEM/size-workflows-to-usage-headroom:23); regression fixture 98 planned → 18 agents, 49/49 judged (PL K9).

### D-16 · Ship ratchet: "deployed" is the only done
- **Symptom:** publication or a green gate stands in for release; the developer never runs the product on real input (C2).
- **Incident (FACT):** 0 tags in 311 commits (AH:236); "use" row: 0 tasks accepted through a deployed unit (AH:238); the Zellij-era lesson "shipping is a terminal act a person names, on something that already runs" (SA:102-104).
- **v4 mechanism:** CHARTER.md §4 done-line — each criterion a read-back command, not a claim; the first tag goes on the commit whose content-addressed release is installed and has ACCEPTED one task (SA:76); after that, a cut requires `l2` to rise. Pushing still needs Luke's word (CLAUDE.local gate 1).
- **Measure:** `tags=N`; `accepted_tasks_per_week=` read from the engine ledger (AH:238).

---

## Dashboard: the numbers that must move (or stay at zero)

| Control | Printed line | Today's baseline (source) | Direction | Status |
|---|---|---|---|---|
| D-01 | `flows= l2= partial=` | `flows=20 l2=2` at `c82e0d2`; `l2` 2 → 3 at `dfb32d5` (AH:234; PL K4) | up per stack | v3 EXISTS; v4 INTENT |
| D-02 | `design_rounds=` | 2 per design since the cap; 4 before (PL L12) | ≤ 2 | INTENT |
| D-04 | `checks= admitted= headerless=` | none | headerless = 0 | INTENT (reflex warns today) |
| D-05 | `apparatus_ratio=` | forge 0.11×; v2 1.98× (SA:35; new-artifact-guard.sh:7) | near forge | INTENT |
| D-07 | `rules= sites= duplicates=` | none (no rules census yet) | duplicates = 0 | INTENT; `always_loaded_bytes=` dropped, no v4 reader *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F13)* |
| D-08 | `Publication for` commits | 11/week at freeze (AH:239) | 0 | frozen in v3; INTENT in v4 |
| D-09 | planted-`use` controls `cases=2/2` *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F06)* | 7 edges (CMAP:19 lists 6; ERRATA E1 adds service→worker) *(rev 2026-10-01 gap-review R12)* | 0 | INTENT |
| D-10 | `survivors_round_n=` | 21 → 26/85 → 36/127 (CM:124-126) | falling, else freeze | INTENT |
| D-11 | `zero_delta_streak=` | 46 landings at l2=2 (SA:12) | < 2 | INTENT |
| D-12 | `new_allows=` · `new_unused_pub=` | v3 0 allows at b5367bc | 0 | INTENT; `phases_since_checkpoint=` dropped, no reader *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F13)* |
| D-16 | `tags=` · `accepted_tasks_per_week=` | 0 · 0 (AH:236, AH:238) | ≥ 1 | INTENT |
| fitness | `just friction` → `retry_rate=` | 0.041 (2026-09-04) → 0.0184 (PL:31) | down | EXISTS |

**Reading the dashboard.** An aggregate refuses as one unit (CM:321-323): a cut is not "mostly green". A line with no number beside it was never counted (CM:324-328). An `UNMEASURED` row is as red as a violation (MEM/verdicts-carry-denominators).

## Counter-evidence locator

This file could be wrong in these ways: (1) every mechanism marked INTENT is unbuilt, so none has a negative control yet — by its own D-04 rule none is admitted; (2) D-03's benefit rests on a contested cause (C4, SA:50) and one forge anecdote; (3) the costs quoted come from same-lineage records (PL:5); (4) the brake and quality primacy can conflict — CM:31-33 resolves it in favour of the floor and discovery, and a control here that says otherwise is wrong; (5) measurements such as `phases_since_checkpoint` have no baseline (UNMEASURED).
