---
description: Every planning hop for one module, phase or DC via `hee4db highway` (bounded JSON)
argument-hint: <module> | --phase Pn | --dc DC-nn | --jev
allowed-tools: Bash(hee4db highway:*)
---
Run `hee4db highway $ARGUMENTS` with the Bash tool.

If no argument was given, say so and show `hee4db highway --phase-table` instead.
Summarise the JSON in at most 12 lines: card path, design section, phase(s), readiness, open DCs,
held H-items, AP/EX/D ids, decisions, and any `gaps`/`refused` fields verbatim. Quote the final
`hee4db highway verdict=…` stderr line. Cite stable ids (AP-nn, DC-nn, H-n, V4-nn), never line numbers.
For the full traversal model see the user skill `hee-v4-corpus`.
