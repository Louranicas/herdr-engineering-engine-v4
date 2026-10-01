-- hee4-ops.db · migration 002 · 2026-10-01 (CN-20)
-- roster_runs.exit_status says WHY a run row has or lacks exit_code, so a pre-contract log is not read as a defect:
--   recorded        the log carries `exit=N` (the V4-24/V4-26 runner contract, first log 20261001T013317Z)
--   legacy_no_exit  no `exit=` line AND the stamp predates 20261001T013317Z: the runner wrote none then; the log is
--                   never rewritten, so the row is marked rather than fixed
--   missing         no `exit=` line on a log at or after the contract: a real gap (`record run` exits 10)
-- NULL only on rows recorded before this migration; the next `record run --from-logs` sets every backfilled row.
ALTER TABLE roster_runs ADD COLUMN exit_status TEXT CHECK (exit_status IS NULL OR exit_status IN ('recorded','legacy_no_exit','missing'));
