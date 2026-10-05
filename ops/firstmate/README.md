# Firstmate database adapter

This directory contains HEEv4's `fm-db` adapter, its schema migrations and its controls. The [HEEv4 entry point](../../README.md) supplies the wider engine context. The [Firstmate source repository](</mnt/storage-10tb/firstmate/README.md>) owns its supervisor implementation and instructions.

[Poteto Weave](</mnt/storage-10tb/hee4-evidence/prototypes/turso-tool-context/assimilation-20261005/README.md>) links back here as the framework's Firstmate integration reference. It retrieves source-grounded context and records its own experiments separately. Firstmate keeps its existing orchestration database ownership, and HEE keeps verdict authority.

Read [fm-db](fm-db) for current command behavior and [the control suite](tests/control.py) for demonstrated contracts. This navigation link does not install the candidate into Firstmate or change adapter behavior.

## Doors and schema level

`fm-db record spawn` requires `--brief-sha`: a spawn that names no brief is refused as `brief_sha_missing` (exit 20), and `brief_sha_unknown` and `brief_sha_unit_mismatch` still apply. `schema/003_spawn_brief_required.sql` refuses the same NULL in SQLite, so raw SQL cannot go around the door. Rows recorded before `002_brief_link.sql` keep a NULL `brief_sha`.

Every verb except `init` compares `schema/*.sql` with `schema_migrations` before it writes or reads back, and refuses `schema_behind=<file>` (exit 3). After a new migration merges, run `FM_HOME=<home> ops/firstmate/fm-db init` on each home before its orchestrator records anything. The [orchestration plan](../../plan/FIRSTMATE-ORCHESTRATION-2026-10-05.md) lists every door.
