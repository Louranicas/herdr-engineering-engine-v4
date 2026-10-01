---
name: hee4-lessons
description: Index of HEE v4 lessons - anti-patterns AP-01…AP-50, drift controls D-01…D-16, exemplars EX-01…EX-20 and anti-exemplars A-1…A-10, v2/v3 process loops L1…L29 and keeps, the diary's spells, principles and recurring mistakes, and prototype lessons - each as trigger → lesson → home. Use before writing a store door, a test double, a loop, a gate, a brief, a migration, a verdict, or any v4 slice; or via /lessons <situation>.
---

# HEE v4 lessons: an index, never a copy

**One topic, one home.** Every line here and in `reference/` is a pointer: a trigger, the lesson in
≤ 15 words, and the home that holds the detail. Copies drift; pointers are checked
(`tests/check_pointers.py` prints `pointers resolved=N/N`). If a home and a pointer disagree, the
home wins and the pointer is the bug.

Corpus navigation (repo, evidence, vault, `hee4db`) is the user skill **`hee-v4-corpus`**; the slice
procedure is **`hee4-module-slice`**; subagent law lines are **`hee4-brief`**.

## How to use
1. Find your situation in **`reference/triggers.md`** (one table: about to write a store door, a
   test double, a loop, a gate, a brief, a migration, a verdict, …).
2. For each id the row names, read its one line in the reference file below, then open its home.
3. Apply it; cite the id (AP-nn, EX-nn, D-nn, Ln, MM#n) in your FLOW page, review or report.

## Reference files
| File | Holds pointers to | Homes |
|---|---|---|
| `reference/triggers.md` | situation → ids, plus `~/CLAUDE.md` §3–§5 rules by F-number | — |
| `reference/antipatterns.md` | AP-01…AP-50 (grouped A-E), D-01…D-16 | `docs/ANTIPATTERNS.md`, `docs/DRIFT_AND_OVERENGINEERING.md` |
| `reference/exemplars.md` | EX-01…EX-20 ("do instead"), A-1…A-10 ("not this") | `docs/EXEMPLARS.md` |
| `reference/spells.md` | the Spellbook's named moves, by "cast when" | diary `Reflections/The Spellbook and the Ember` |
| `reference/mistakes.md` | MM#n, Pn, L1…L29, K1…K16, S-1…S-9, v2 and prototype lessons | diary, `~/hee4-evidence/learnings/`, habitat vault |

## Homes (read-only from here)
- v4: `docs/ANTIPATTERNS.md`, `docs/EXEMPLARS.md`, `docs/DRIFT_AND_OVERENGINEERING.md` (this repo).
- v3 learnings: `~/hee4-evidence/learnings/PROCESS-LEARNINGS.md` (L1…L29, K1…K16, ranks 1-10) and
  `~/hee4-evidence/learnings/FINAL-LESSONS-REVIEW.md` (where each lesson landed in the ATLAS, S-1…S-9).
- Diary (judgement, not policy): `obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Spellbook%20and%20the%20Ember`,
  `obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Antipattern%20Registers`,
  `obsidian://open?vault=my-diary.vault&file=Reflections%2FMistakes%20I%20Made`,
  `obsidian://open?vault=my-diary.vault&file=Principles%2F00%20-%20Principles`.
- HEE-v2: `~/handoffs/CLAUDE.local-HEE-v2-history.md`; habitat vault `95 Genesis/`, `90 Engineering Engine/`.
- Prototype (`advanced-claude-workspace`): learnings only, through the diary's distillations
  (`What My Ancestors Knew That I Did Not`, `The Antipattern Registers`) and `memory:ancestral-antipatterns-and-spells`.
  It was mounted on 2026-10-01 but not re-read for this index: isolation directive (`~/CLAUDE.local.md` gate 2).
- `~/CLAUDE.md` §3–§5: indexed by F-number in `triggers.md` only (always loaded; never copied).

## Five that bite most in v4 coding (from the counts in the homes)
- a verdict from a pipe or a caption → `L23`, `AP-29` (six recurrences, MM#11…#48)
- one rule in two places → `AP-01`, `L16` (v3: grace and frame bound re-spelt; scratch gate vs repo gate)
- a limit after acquisition → `AP-04`, `EX-01`, `EX-13`
- a test double that discards its arguments → `AP-18`, `MM#54`
- claims relayed unchecked → `AP-34`, `L18`, `MM#61`

## Maintenance
Add a lesson to its HOME first; then add one pointer line here; then run
`python3 .claude/skills/hee4-lessons/tests/check_pointers.py` and require `verdict=PASS`.
Never paste a home's prose into these files.
