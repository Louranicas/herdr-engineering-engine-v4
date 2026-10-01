-- hee4-ops.db · migration 004 · 2026-10-01 (rev 2026-10-01 jev-fit)
-- Two DERIVED tables from ONE vault file, rebuilt only by `hee4db ingest` (never edited by hand):
--   jev_fits / jev_fit_modules  ← `50 Jev/Jev Fit Map.md` §3, the single home of the 51 decision intersections
--                                 (tables 3a–3d: | # | Decision | Where | Type … | Data class; allowed now? | Code baseline … |
--                                  Threshold source | Cost/day … | Grade and reason | Today … |)
-- Derived in code (parse_jev_fit_map), never typed by hand:
--   allowed_now  from the bold yes/no in the data-class cell ('—' → 'n/a'; no bold verdict → 'conditional')
--   engine_data  1 when the data-class cell names engine data, engine logs/source, or v4 planning/report text
--   phases       the P-tokens of the Where cell (comma-joined, may be '')
--   gates        H-ids the row waits on: its data-class cell, §7 "Fits that wait on it" (explicit ids and
--                "every J row" → non-X J rows), and, for non-X §3d rows, the §3d session rule
--   modules      a cluster token directly before a backticked module name (`K6 \`candidates\``), a '/' or ','
--                continuing it; 'ALL' for "all crates"; 'UNMAPPED' when the row names none
-- Refusals at ingest: an unknown grade, an unknown module (or one in another cluster), a duplicate id.

CREATE TABLE jev_fits (
  id               TEXT PRIMARY KEY,
  section          TEXT NOT NULL CHECK (section IN ('3a','3b','3c','3d')),
  decision         TEXT NOT NULL,
  where_cell       TEXT NOT NULL,
  question_type    TEXT NOT NULL,
  data_class       TEXT NOT NULL,
  engine_data      INTEGER NOT NULL CHECK (engine_data IN (0,1)),
  allowed_now      TEXT NOT NULL CHECK (allowed_now IN ('yes','no','conditional','n/a')),
  baseline         TEXT NOT NULL,
  threshold_source TEXT NOT NULL,
  cost_estimate    TEXT NOT NULL,
  grade            TEXT NOT NULL CHECK (grade IN ('A','B','C','X')),
  grade_reason     TEXT NOT NULL,
  measured_value   TEXT,                         -- the "Today" cell; NULL when it reads '—'
  phases           TEXT NOT NULL,
  gates            TEXT NOT NULL,
  modules          TEXT NOT NULL,                -- comma-joined names, 'ALL' or 'UNMAPPED'
  source_id        INTEGER NOT NULL REFERENCES sources(id),
  source_sha256    TEXT NOT NULL,
  source_line      INTEGER NOT NULL,
  ingested_at      TEXT NOT NULL
) STRICT;

CREATE TABLE jev_fit_modules (
  fit_id    TEXT NOT NULL REFERENCES jev_fits(id),
  module    TEXT NOT NULL REFERENCES modules(name),
  source_id INTEGER NOT NULL REFERENCES sources(id),
  PRIMARY KEY (fit_id, module)
) STRICT;
CREATE INDEX jev_fit_modules_module ON jev_fit_modules(module);
