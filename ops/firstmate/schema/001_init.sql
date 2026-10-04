-- firstmate.db: ONE per Firstmate home ($FM_HOME/data/firstmate.db), orchestration state only.
-- Not a home for engine task state (HEE v4 K1 store owns that; one topic, one home).
-- Plain SQLite WAL, no engine-specific DDL: written by fm-db (Python sqlite3, BEGIN IMMEDIATE, flock),
-- read back with `tursodb --readonly` one-shot. Mirrors ops/db/README.md "Writer safety".
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS schema_migrations (
  file TEXT PRIMARY KEY, sha256 TEXT NOT NULL, applied_ts TEXT NOT NULL
);

-- A unit of orchestration: one captain intent, one brief, N spawns.
CREATE TABLE IF NOT EXISTS units (
  unit_id TEXT PRIMARY KEY, project TEXT NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('ship','scout')),
  mode TEXT NOT NULL, yolo INTEGER NOT NULL DEFAULT 0 CHECK (yolo IN (0,1)),
  brief_path TEXT NOT NULL, planned_agents INTEGER NOT NULL CHECK (planned_agents >= 1),
  opened_ts TEXT NOT NULL, closed_ts TEXT
);

-- The brief that started a unit, pinned by content hash; standing orders pinned beside it.
CREATE TABLE IF NOT EXISTS briefs (
  brief_sha TEXT PRIMARY KEY, unit_id TEXT NOT NULL REFERENCES units(unit_id),
  path TEXT NOT NULL, head_sha TEXT NOT NULL, standing_sha TEXT NOT NULL, ts TEXT NOT NULL
);

-- Every spawn (crew, scout, secondmate relaunch). planned_agents is enforced here, not by Firstmate.
CREATE TABLE IF NOT EXISTS spawns (
  task_id TEXT PRIMARY KEY, unit_id TEXT NOT NULL REFERENCES units(unit_id),
  agent TEXT NOT NULL, harness TEXT NOT NULL, model TEXT, backend TEXT, worktree TEXT,
  fresh INTEGER NOT NULL DEFAULT 1 CHECK (fresh IN (0,1)), ts TEXT NOT NULL
);

-- A claim: one labelled sentence with its witness. Unlabelled claims cannot be inserted.
CREATE TABLE IF NOT EXISTS claims (
  claim_id INTEGER PRIMARY KEY, task_id TEXT NOT NULL REFERENCES spawns(task_id),
  text TEXT NOT NULL, label TEXT NOT NULL CHECK (label IN ('MEASURED','INFERRED','UNMEASURED')),
  witness_cmd TEXT, head_sha TEXT NOT NULL, ts TEXT NOT NULL,
  CHECK (label <> 'MEASURED' OR witness_cmd IS NOT NULL)
);

-- A verification of a claim by a DIFFERENT task (the claimant cannot verify its own claim).
CREATE TABLE IF NOT EXISTS verifications (
  claim_id INTEGER NOT NULL REFERENCES claims(claim_id), verifier_task TEXT NOT NULL REFERENCES spawns(task_id),
  verdict TEXT NOT NULL CHECK (verdict IN ('verified','refuted','unmeasured')), result TEXT, ts TEXT NOT NULL,
  PRIMARY KEY (claim_id, verifier_task)
);
CREATE TRIGGER IF NOT EXISTS verifications_not_self BEFORE INSERT ON verifications
  WHEN NEW.verifier_task = (SELECT task_id FROM claims WHERE claim_id = NEW.claim_id)
  BEGIN SELECT RAISE(ABORT, 'claimant cannot verify its own claim'); END;

-- Fleet-ledger events, second sink (Firstmate's state/fleet-ledger.jsonl stays the first).
CREATE TABLE IF NOT EXISTS receipts (
  receipt_id INTEGER PRIMARY KEY, task_id TEXT NOT NULL REFERENCES spawns(task_id),
  event TEXT NOT NULL CHECK (event IN ('task.dispatched','status','pr_ready','merged','cleaned_up','report')),
  detail TEXT, pr TEXT, head_sha TEXT, ts TEXT NOT NULL
);

-- Typed exits, in the roster's vocabulary; Firstmate status verbs map onto it (fm-db record exit).
CREATE TABLE IF NOT EXISTS exits (
  task_id TEXT PRIMARY KEY REFERENCES spawns(task_id),
  verdict TEXT NOT NULL CHECK (verdict IN ('PASS','PASS_WITH_GAPS','FAIL','BLOCKED','STOP')),
  cases_ok INTEGER, cases_total INTEGER, reason TEXT, report_path TEXT, head_sha TEXT, ts TEXT NOT NULL
);

-- The andon: a STOP raised by a watcher or refuter halts the unit until cleared by the captain.
CREATE TABLE IF NOT EXISTS andon (
  andon_id INTEGER PRIMARY KEY, unit_id TEXT NOT NULL REFERENCES units(unit_id),
  raised_by TEXT NOT NULL, reason TEXT NOT NULL, measured INTEGER NOT NULL CHECK (measured = 1),
  ts TEXT NOT NULL, cleared_by TEXT, cleared_ts TEXT
);

CREATE VIEW IF NOT EXISTS open_units AS
  SELECT u.unit_id, u.project, u.kind, u.planned_agents,
         (SELECT count(*) FROM spawns s WHERE s.unit_id = u.unit_id) AS spawned,
         (SELECT count(*) FROM andon a WHERE a.unit_id = u.unit_id AND a.cleared_ts IS NULL) AS open_andon
  FROM units u WHERE u.closed_ts IS NULL;
