# Edit a pinned source by appending, never in place

Cards cite pinned files by `KEY:line` (AT, DEC, MIG, UM…). Editing an existing line's text, or inserting a line above cited ones, breaks the cites: `repin` reports `keyline_unmapped` or `keyline_blank`, then refuses because the cites now "differ by hand". Measured 2026-10-05 on MIGRATION.md line 23 (8 cites broke; fixed by `git checkout` and appending the note at the end of the file).

Rule: a dated note goes at the END of a pinned file, or as a new row after the last one. If a cited line must change, change it, run `cite_pins.py repin KEY` immediately, and let the remap move the cites; never hand-edit a `KEY:?nn` marker back.

Related: [[verification-spine]], [[contradictions-2026-10-04]].
