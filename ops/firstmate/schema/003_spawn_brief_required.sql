-- 003: a new spawn names the brief it started from, in SQLite itself (V4-99), so a raw INSERT cannot go around
-- `fm-db record spawn`, foreign_keys on or off. Prospective: spawns recorded before 002 keep brief_sha NULL; these
-- triggers fire on INSERT only, plus an UPDATE that would blank a recorded link.
CREATE TRIGGER spawns_brief_sha_required BEFORE INSERT ON spawns
  WHEN NEW.brief_sha IS NULL
  BEGIN SELECT RAISE(ABORT, 'brief_sha_missing: a spawn names the brief it started from'); END;

CREATE TRIGGER spawns_brief_sha_of_unit BEFORE INSERT ON spawns
  WHEN NEW.brief_sha IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM briefs b WHERE b.brief_sha = NEW.brief_sha AND b.unit_id = NEW.unit_id)
  BEGIN SELECT RAISE(ABORT, 'brief_sha_not_of_unit: the spawn''s brief_sha is not a recorded brief of its unit'); END;

CREATE TRIGGER spawns_brief_sha_kept BEFORE UPDATE OF brief_sha ON spawns
  WHEN OLD.brief_sha IS NOT NULL AND (NEW.brief_sha IS NULL OR NEW.brief_sha <> OLD.brief_sha)
  BEGIN SELECT RAISE(ABORT, 'brief_sha_kept: a recorded spawn keeps the brief it started from'); END;
