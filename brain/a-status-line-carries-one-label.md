# A status line carries one label

**Seen 2026-10-05.** `tools/doctor` at 2c7926b, line 98, emitted the `model` row as
`status=MEASURED` with the detail "endpoint reachable (/api/ps ok); GPU residency UNMEASURED: no
model loaded". The row aggregated as measured, and the UNMEASURED half only showed in the detail
string. Commit 6cf504e split it into two rows: `model_endpoint` (tools/doctor:165-167) and
`model_gpu` (:169-176, UNMEASURED by name, advisory unless `--require-gpu`).
MEASURED with `git log -S"GPU residency UNMEASURED" --oneline -- tools/doctor` and
`grep -n -E "model_endpoint|model_gpu|require-gpu" tools/doctor`.

**Rule.** One row, one label. A line that reports two halves with two labels must be two rows.
An UNMEASURED never hides in a MEASURED row's detail text: the detail is prose, and the
aggregate reads only the status field. An aggregate counts an UNMEASURED row as red or as named
excluded, never as zero (PROTOCOL §1).

**Also.** A detail string that matches a label word is a smell a check can catch: grep the
detail column for the labels.

Related: [[verification-spine]] (refuse to look at nothing), [[meta-goal-rungs]] (the split
moves the class from review to a row the aggregate reads).
