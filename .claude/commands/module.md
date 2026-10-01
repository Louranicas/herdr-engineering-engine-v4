---
description: Load one HEE v4 module - highway, card, design section, AP/EX ids and open questions
argument-hint: <module>
allowed-tools: Bash(hee4db highway:*), Bash(hee4db get:*)
---
Module: `$ARGUMENTS` (if empty, ask for the module name and stop).

1. Run `hee4db highway $ARGUMENTS`. If it refuses (`not_found`), quote the refusal and stop.
2. Read the card path it names (`modules/<crate>/<module>/MODULE.md`), whole.
3. Read the design section it names (vault `15 Module Design/<crate note>`, that heading only).
4. List, as one line each with its title: every AP-nn and EX-nn (and A-n, D-nn) the card names
   (`hee4db get ap AP-nn` / `get ex EX-nn` for titles), then the open DC-nn rows and held H-n items.
5. List the open questions: the card's and design section's open items, PROPOSAL tags, and any
   DC row not yet ratified (H-27).

Report ≤ 25 lines, stable ids only. Planning only under the HOLD (H-5): this command never writes code.
For the lessons that apply, run `/lessons <module's situation>`.
