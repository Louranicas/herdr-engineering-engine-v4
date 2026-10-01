---
name: hee4-module-slice
description: DW-1, the HEE v4 module slice, as a procedure - card → FLOW → compiling skeleton → ≤ 2 design rounds → slice → Tier-1 → plants → Tier-2 + cold clone → regen + verify. Use ONLY after Luke has said "start coding" (H-5 closed); it refuses while the HOLD is open. Use when asked to build, implement or slice a v4 module.
---

# HEE v4 module slice (DW-1)

The procedure for writing one module's slice. Homes, not copies: DW-1 is defined in vault
`16 System Maps/Workflow and Loop Map` §3 (row DW-1, with DW-2 and DW-4), the tiers in ATLAS §4.1,
the README §3. Navigation of the corpus is the user skill **`hee-v4-corpus`**; the lessons index is
the project skill **`hee4-lessons`** (`/lessons`). Cite stable ids (AP-nn, EX-nn, D-nn, DC-nn,
H-n, V4-nn, Ln), never line numbers.

## 0 · Refuse while the HOLD is open (H-5, V4-0)
Run both; proceed only if BOTH say closed:
- `hee4db get held H-5` → `.row.status` must not be `open`;
- ATLAS §5 H-list row `H-5` (`~/hee4-evidence/design/DEPLOYMENT_ATLAS.md`, `| H-5 |`) must show it given.
- `hee4db highway <module>` → its `readiness` must not be blocked, and its phase's holds
  (`hee4db recipe blocks-phase <Pn>`) must be empty.
- For P0 (and after any change to the Jev door or a Jev hook): `just jev-entry` must print `verdict=PASS` with
  `senders_measured=4/4` and `sent_engine_rows=0/<sent>` (ATLAS P0 entry, H-8 read-back; DEC V4-74). Its value is the
  run you just made, never a quoted earlier line.
If any says open/blocked: print `slice verdict=REFUSED reason=hold_open id=<H-n>` and stop. Luke's
words are the only key; a note, a DB row edited by an agent, or an inferred yes is not (L20, S-6).

## 1 · Read (in this order; the highway names every path)
1. The card `modules/<crate>/<module>/MODULE.md`, whole.
2. Its design section (vault `15 Module Design/<crate note>`, the heading the card links), the
   system maps it names, `16 System Maps/Anti-Bloat Budget` (the module's line budget).
3. Every AP/EX/A/D id the card names: `docs/ANTIPATTERNS.md`, `docs/EXEMPLARS.md`,
   `docs/DRIFT_AND_OVERENGINEERING.md`. Then **`/lessons <situation>`** for the triggers that bite
   in this module (store door, test double, loop, gate…): the hee4-lessons skill indexes the v2/v3
   loops, the diary spells and recurring mistakes with their homes.
4. The decisions it cites (`plan/DECISIONS.md`, V4-nn / D-Unn) and its open DC rows.

## 2 · FLOW (one page)
Write the FLOW page in the slice's worktree: flow id, inputs, outputs, **reader** of every output
("no reader, no row", AP-13 / D-08), refusal names, the budget on every loop and exit-path await
(AP-31), the one door for each rule it enforces (AP-01). One page; more is a design round.

## 3 · Compiling skeleton (D-03)
Types, signatures and module wiring that compile with `forbid` lints and no `todo!()` in library
paths. Nothing is reviewed until it compiles. Print `skeleton_compiles=yes`.

## 4 · At most 2 design rounds (D-02, AP-36)
Independent review against the skeleton, not prose. After round 2, land the buildable cut and
record the rest as a DC row or a decision. Print `design_rounds=N` (N ≤ 2).

## 5 · Slice + Tier-1 (ATLAS §4.1)
Its own worktree per module slice (never edit a worktree a gate is running in, L8). Per commit:
targeted suites (a diff-scoped run prints every skipped step with its reason), open-code-review
coverage pass, independent review on committed objects (K1, AP-33). Print `l2_before= l2_after=`.

## 6 · Plant battery and mutants (DW-4)
- Plants under `--cap-lints=warn`, each requiring its **named** killing test (AP-23, K2); a plant
  that fails to compile is not a kill (L14).
- `cargo mutants` through the runner that **unsets `CARGO_TARGET_DIR`** and refuses if set, with a
  planted-survivor precheck (AP-32, L15); fresh-mtime restore + `cargo clean -p` before a verdict.
- Print `plants=k/k killers=named`, `mutants caught=a survivors=b` (each survivor named or
  equivalent-with-reason).

## 7 · Tier-2: repo gate + cold clone per stack (DW-2)
The repo's own full `tools/gate` (never a scratch gate, AP-01/L16) on a `git archive` export at the
stack tip, in parallel with `tools/cold-clone`. Both print `steps=N matched=N tree=<sha> dirty=0`.

## 8 · Regenerate and verify
`just regen` if a design-conflict register row changed; `just repin <KEY>` for every legend file
you edited (the PostToolUse hook names it); `hee4db ingest`; then `just verify` and quote its line.

## Report (typed, ≤ 12 lines)
`slice verdict=PASS|FAIL|PASS_WITH_GAPS module=<m> skeleton_compiles= design_rounds= plants=k/k
mutants caught= survivors= gate steps= cold steps= tree=<sha> verify=<verdict line>`, then each gap
by id. Commit only when asked; push only on Luke's word (H-1).
