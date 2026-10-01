-- hee4-ops.db · migration 003 · 2026-10-01 (alignment + traversal highways)
-- Two DERIVED tables from vault files, rebuilt only by `hee4db ingest` (never edited by hand):
--   design_conflicts / conflict_modules  ← `15 Module Design/00 - Module Design Index.md`, the DC register
--                                          (columns | Id | Conflict | Found in | Phase | Modules | Status · recommendation |)
--   module_budgets                       ← `16 System Maps/Anti-Bloat Budget.md` §2, the single home of size budgets
-- Both source notes stay ingested as design_notes too; the composite parsers add these rows beside them.
-- Readiness is NOT a table: `hee4db readiness` computes it in code (the one home of the rules) from modules,
-- module_refs, held_items, held_item_phases, module_budgets and conflict_modules.

CREATE TABLE design_conflicts (
  id             TEXT PRIMARY KEY,
  title          TEXT NOT NULL,
  conflict       TEXT NOT NULL,
  found_in       TEXT NOT NULL,
  phase          TEXT,                          -- NULL for a closed (RESOLVED) row; the register writes '—'
  modules        TEXT NOT NULL,                 -- the register's Modules cell as written ('—' when closed)
  scope          TEXT NOT NULL CHECK (scope IN ('modules','all','none')),
  status_class   TEXT NOT NULL CHECK (status_class IN ('RESOLVED','PROPOSED','PARTIAL','OPEN')),
  recommendation TEXT NOT NULL,
  cites_um       INTEGER NOT NULL CHECK (cites_um IN (0,1)),   -- Conflict or Status text cites "UM §" or "UM-P"
  source_id      INTEGER NOT NULL REFERENCES sources(id),
  source_sha256  TEXT NOT NULL,
  source_line    INTEGER NOT NULL,
  ingested_at    TEXT NOT NULL,
  CHECK ((status_class = 'RESOLVED') = (phase IS NULL))
) STRICT;

CREATE TABLE conflict_modules (
  dc_id     TEXT NOT NULL REFERENCES design_conflicts(id),
  module    TEXT NOT NULL REFERENCES modules(name),
  source_id INTEGER NOT NULL REFERENCES sources(id),
  PRIMARY KEY (dc_id, module)
) STRICT;
CREATE INDEX conflict_modules_module ON conflict_modules(module);

CREATE TABLE module_budgets (
  module        TEXT PRIMARY KEY REFERENCES modules(name),
  cluster       TEXT NOT NULL,
  crate         TEXT NOT NULL,
  rec           TEXT NOT NULL,
  v3_size       TEXT NOT NULL,
  target        TEXT NOT NULL,
  scope         TEXT NOT NULL,
  driver        TEXT NOT NULL,
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  source_line   INTEGER NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;

CREATE VIEW v_open_conflicts_by_phase AS
SELECT d.phase, d.id, d.status_class, d.scope, d.title,
       (SELECT group_concat(c.module, ', ') FROM conflict_modules c WHERE c.dc_id = d.id) AS modules
FROM design_conflicts d
WHERE d.status_class <> 'RESOLVED';
