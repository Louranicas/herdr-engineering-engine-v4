-- hee4-ops.db · migration 001 · 2026-10-01
-- Planning/ops index for HEE v4. NOT the engine ledger: the engine's own SQLite store is product,
-- owned by K1 hee4-core::store (ULTRAMAP §5a), and no engine table is modelled here.
--
-- Two kinds of table:
--   DERIVED  (decisions … design_notes): a regenerable index of a file that is the one home.
--            Every row carries source_id + source_sha256 + ingested_at; `hee4db ingest` rebuilds them,
--            `hee4db stale` lists rows whose source file changed. Never edit a derived row by hand.
--   PRIMARY  (roster_runs, measurements, jev_egress_daily, claims, ingest_receipts): no file home;
--            this DB is the home. Written only through named `hee4db record …` verbs.
--   IMPORTED (habitat_*): frozen copies of habitat-ops.db rows with source_db/source_table/source_rowid
--            and the source row's sha256; `hee4db stale` re-reads the source and reports drift.
--
-- Plain SQLite only: STRICT tables, CHECKs, FKs, indexes, views. No turso-only DDL (FTS, MVCC,
-- custom types, generated columns, encryption) in this file, so Python sqlite3 and stock sqlite3
-- can always read and write it. Full-text search lives in a derived sidecar (hee4-ops-search.db).

CREATE TABLE schema_migrations (
  version     INTEGER PRIMARY KEY,
  name        TEXT NOT NULL,
  sha256      TEXT NOT NULL CHECK (length(sha256) = 64),
  applied_at  TEXT NOT NULL
) STRICT;

-- ── derived: the file world ─────────────────────────────────────────────────────────────
CREATE TABLE sources (
  id           INTEGER PRIMARY KEY,
  root         TEXT NOT NULL CHECK (root IN ('repo','evidence','vault')),
  rel_path     TEXT NOT NULL,
  parser       TEXT NOT NULL,
  sha256       TEXT NOT NULL CHECK (length(sha256) = 64),
  bytes        INTEGER NOT NULL CHECK (bytes >= 0),
  rows_derived INTEGER NOT NULL CHECK (rows_derived >= 0),
  ingested_at  TEXT NOT NULL,
  UNIQUE (root, rel_path)
) STRICT;

CREATE TABLE decisions (
  id            TEXT PRIMARY KEY,
  kind          TEXT NOT NULL CHECK (kind IN ('decision','proposal')),
  section       TEXT NOT NULL,
  title         TEXT NOT NULL,
  body          TEXT NOT NULL,
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  source_line   INTEGER NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;

-- Every sentence of a decision that names another decision id. `has_supersede` marks sentences
-- containing "supersed…". These are CANDIDATES for an agent to read, not parsed supersession edges:
-- the register is prose, and prose is not a declaration (the file decides).
CREATE TABLE decision_mentions (
  decision_id   TEXT NOT NULL REFERENCES decisions(id),
  mentioned_id  TEXT NOT NULL,
  has_supersede INTEGER NOT NULL CHECK (has_supersede IN (0,1)),
  sentence      TEXT NOT NULL,
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;
CREATE INDEX decision_mentions_mentioned ON decision_mentions(mentioned_id, has_supersede);

CREATE TABLE held_items (
  id            TEXT PRIMARY KEY,
  owner         TEXT NOT NULL,
  status        TEXT NOT NULL CHECK (status IN ('open','closed','merged')),
  title         TEXT NOT NULL,
  item          TEXT NOT NULL,
  why           TEXT NOT NULL,
  blocks        TEXT NOT NULL,
  answer_form   TEXT NOT NULL,
  cite          TEXT NOT NULL,
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  source_line   INTEGER NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;

CREATE TABLE held_item_phases (
  held_id   TEXT NOT NULL REFERENCES held_items(id),
  phase     TEXT NOT NULL CHECK (phase GLOB 'P[0-9]'),
  source_id INTEGER NOT NULL REFERENCES sources(id),
  PRIMARY KEY (held_id, phase)
) STRICT;
CREATE INDEX held_item_phases_phase ON held_item_phases(phase);

CREATE TABLE antipatterns (
  id            TEXT PRIMARY KEY,
  category      TEXT NOT NULL,
  name          TEXT NOT NULL,
  definition    TEXT NOT NULL,
  tell          TEXT NOT NULL,
  detector      TEXT NOT NULL,
  is_trigger    INTEGER NOT NULL CHECK (is_trigger IN (0,1)),
  v3_instance   TEXT NOT NULL,
  mirror        TEXT NOT NULL,
  cite          TEXT NOT NULL,
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  source_line   INTEGER NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;

CREATE TABLE exemplars (
  id            TEXT PRIMARY KEY,
  kind          TEXT NOT NULL CHECK (kind IN ('exemplar','anti-exemplar')),
  theme         TEXT NOT NULL,
  title         TEXT NOT NULL,
  v3_site       TEXT NOT NULL,
  prevents      TEXT NOT NULL,
  body          TEXT NOT NULL,
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  source_line   INTEGER NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;

CREATE TABLE drift_controls (
  id            TEXT PRIMARY KEY,
  title         TEXT NOT NULL,
  body          TEXT NOT NULL,
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  source_line   INTEGER NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;

CREATE TABLE modules (
  name           TEXT PRIMARY KEY,
  crate          TEXT NOT NULL,
  cluster        TEXT NOT NULL,
  flag           TEXT NOT NULL,
  recommendation TEXT NOT NULL,
  card           TEXT NOT NULL,
  source_id      INTEGER NOT NULL REFERENCES sources(id),
  source_sha256  TEXT NOT NULL,
  source_line    INTEGER NOT NULL,
  ingested_at    TEXT NOT NULL
) STRICT;

-- One row per list element of a [[module]] entry: phase, ap, ex, anti, d, jev, h, v4, errata,
-- depends_on, v3_origin, migrated_path.
CREATE TABLE module_refs (
  module    TEXT NOT NULL REFERENCES modules(name),
  kind      TEXT NOT NULL CHECK (kind IN ('phase','ap','ex','anti','d','jev','h','v4','errata',
                                          'depends_on','v3_origin','migrated_path')),
  ref       TEXT NOT NULL,
  ordinal   INTEGER NOT NULL,
  source_id INTEGER NOT NULL REFERENCES sources(id),
  PRIMARY KEY (module, kind, ordinal)
) STRICT;
CREATE INDEX module_refs_ref ON module_refs(kind, ref);

CREATE TABLE loops (
  id            TEXT PRIMARY KEY,
  kind          TEXT NOT NULL CHECK (kind IN ('loop','keep')),
  title         TEXT NOT NULL,
  evidence      TEXT NOT NULL,
  columns_json  TEXT NOT NULL CHECK (json_valid(columns_json)),
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  source_line   INTEGER NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;

CREATE TABLE verifications (
  id            TEXT PRIMARY KEY,
  subject       TEXT NOT NULL,
  verdict       TEXT NOT NULL CHECK (verdict IN ('PASS','PASS_WITH_GAPS','FAIL','UNTYPED')),
  verdict_text  TEXT NOT NULL,
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  source_line   INTEGER NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;

CREATE TABLE design_notes (
  path          TEXT PRIMARY KEY,
  folder        TEXT NOT NULL,
  title         TEXT NOT NULL,
  body          TEXT NOT NULL,
  body_bytes    INTEGER NOT NULL,
  truncated     INTEGER NOT NULL CHECK (truncated IN (0,1)),
  source_id     INTEGER NOT NULL REFERENCES sources(id),
  source_sha256 TEXT NOT NULL,
  ingested_at   TEXT NOT NULL
) STRICT;

-- ── primary: no file home ───────────────────────────────────────────────────────────────
CREATE TABLE roster_runs (
  id            INTEGER PRIMARY KEY,
  agent         TEXT NOT NULL,
  mode          TEXT NOT NULL,
  stamp         TEXT NOT NULL,
  log_path      TEXT UNIQUE,
  exit_code     INTEGER,
  measure_rc    TEXT,
  claude_rc     INTEGER,
  verdict       TEXT NOT NULL CHECK (verdict IN ('PASS','PASS_WITH_GAPS','FAIL','REFUSED','SKIPPED','NONE')),
  verdict_line  TEXT,
  cost_usd      REAL,
  is_error      TEXT,
  subtype       TEXT,
  degraded_by   TEXT,
  source        TEXT NOT NULL CHECK (source IN ('backfill','cli')),
  source_sha256 TEXT,
  recorded_at   TEXT NOT NULL
) STRICT;
CREATE INDEX roster_runs_agent_stamp ON roster_runs(agent, stamp);
CREATE INDEX roster_runs_stamp ON roster_runs(stamp);

CREATE TABLE measurements (
  id            INTEGER PRIMARY KEY,
  name          TEXT NOT NULL,
  value         REAL,
  value_text    TEXT,
  denominator   REAL,
  unmeasured    INTEGER NOT NULL CHECK (unmeasured IN (0,1)),
  measured_at   TEXT NOT NULL,
  source        TEXT NOT NULL,
  source_sha256 TEXT,
  recorded_at   TEXT NOT NULL,
  CHECK (value IS NOT NULL OR value_text IS NOT NULL),
  UNIQUE (name, measured_at, source)
) STRICT;
CREATE INDEX measurements_name_time ON measurements(name, measured_at);

-- Counts only, never text (V4-21, V4-29). rows_v4_cwd NULL means the sender logs no cwd (UNMEASURED).
CREATE TABLE jev_egress_daily (
  day           TEXT NOT NULL CHECK (day GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  sender        TEXT NOT NULL,
  rows_total    INTEGER,
  rows_sent     INTEGER,
  rows_v4_cwd   INTEGER,
  rows_no_cwd   INTEGER,
  measured      TEXT NOT NULL CHECK (measured IN ('full','partial','unmeasured')),
  method        TEXT NOT NULL,
  source_path   TEXT NOT NULL,
  source_sha256 TEXT,
  recorded_at   TEXT NOT NULL,
  PRIMARY KEY (day, sender)
) STRICT;

CREATE TABLE claims (
  id           INTEGER PRIMARY KEY,
  who          TEXT NOT NULL,
  claim        TEXT NOT NULL,
  evidence     TEXT NOT NULL,
  verified_by  TEXT,
  status       TEXT NOT NULL CHECK (status IN ('open','verified','refuted','unmeasured')),
  result       TEXT,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  CHECK (status = 'open' OR verified_by IS NOT NULL),
  CHECK (verified_by IS NULL OR verified_by <> who)
) STRICT;
CREATE INDEX claims_status ON claims(status);

CREATE TABLE ingest_receipts (
  id              INTEGER PRIMARY KEY,
  verb            TEXT NOT NULL,
  started_at      TEXT NOT NULL,
  finished_at     TEXT NOT NULL,
  status          TEXT NOT NULL CHECK (status IN ('ok','gaps','refused')),
  sources_seen    INTEGER NOT NULL,
  sources_changed INTEGER NOT NULL,
  rows_written    INTEGER NOT NULL,
  detail_json     TEXT NOT NULL CHECK (json_valid(detail_json))
) STRICT;

-- ── imported from habitat-ops.db (frozen copies with provenance) ────────────────────────
CREATE TABLE habitat_learnings (
  source_db         TEXT NOT NULL,
  source_table      TEXT NOT NULL CHECK (source_table = 'learnings'),
  source_rowid      INTEGER NOT NULL,
  source_row_sha256 TEXT NOT NULL,
  id                TEXT NOT NULL UNIQUE,
  title             TEXT NOT NULL,
  body              TEXT NOT NULL,
  standing          INTEGER NOT NULL CHECK (standing IN (0,1)),
  scope             TEXT NOT NULL CHECK (scope IN ('general','v3-provenance')),
  migrated_at       TEXT NOT NULL,
  PRIMARY KEY (source_table, source_rowid)
) STRICT;

CREATE TABLE habitat_engine_features (
  source_db         TEXT NOT NULL,
  source_table      TEXT NOT NULL CHECK (source_table = 'engine_features'),
  source_rowid      INTEGER NOT NULL,
  source_row_sha256 TEXT NOT NULL,
  id                TEXT NOT NULL UNIQUE,
  area              TEXT NOT NULL,
  status            TEXT NOT NULL,
  detail            TEXT NOT NULL,
  source_url        TEXT NOT NULL,
  migrated_at       TEXT NOT NULL,
  PRIMARY KEY (source_table, source_rowid)
) STRICT;

CREATE TABLE habitat_composition_rules (
  source_db         TEXT NOT NULL,
  source_table      TEXT NOT NULL CHECK (source_table = 'composition_rules'),
  source_rowid      INTEGER NOT NULL,
  source_row_sha256 TEXT NOT NULL,
  id                TEXT NOT NULL UNIQUE,
  rule              TEXT NOT NULL,
  rule_source       TEXT NOT NULL,
  how_to_check      TEXT NOT NULL,
  migrated_at       TEXT NOT NULL,
  PRIMARY KEY (source_table, source_rowid)
) STRICT;

CREATE TABLE habitat_doc_sources (
  source_db         TEXT NOT NULL,
  source_table      TEXT NOT NULL CHECK (source_table = 'doc_sources'),
  source_rowid      INTEGER NOT NULL,
  source_row_sha256 TEXT NOT NULL,
  id                TEXT NOT NULL UNIQUE,
  role              TEXT NOT NULL,
  title             TEXT NOT NULL,
  url               TEXT NOT NULL,
  deployed_here     INTEGER NOT NULL,
  jev_note          TEXT,
  migrated_at       TEXT NOT NULL,
  PRIMARY KEY (source_table, source_rowid)
) STRICT;

-- One row per habitat-ops.db table: the world, each with a disposition and its reason.
CREATE TABLE habitat_migration_ledger (
  source_table   TEXT PRIMARY KEY,
  rows_in_source INTEGER NOT NULL,
  rows_migrated  INTEGER NOT NULL,
  disposition    TEXT NOT NULL CHECK (disposition IN ('migrated','partial','not_migrated')),
  reason         TEXT NOT NULL CHECK (length(reason) > 0),
  target_table   TEXT,
  migrated_at    TEXT NOT NULL
) STRICT;

-- ── views for agents ────────────────────────────────────────────────────────────────────
CREATE VIEW v_module_phase_ap AS
SELECT m.name AS module, m.crate, m.cluster, p.ref AS phase,
       (SELECT group_concat(a.ref, ',') FROM module_refs a
         WHERE a.module = m.name AND a.kind = 'ap') AS ap_ids
FROM modules m JOIN module_refs p ON p.module = m.name AND p.kind = 'phase';

CREATE VIEW v_open_held_by_phase AS
SELECT ph.phase, h.id, h.title, h.answer_form
FROM held_item_phases ph JOIN held_items h ON h.id = ph.held_id
WHERE h.status = 'open';
