-- 002: every spawn names the brief it started from (PROTOCOL §2: the brief is the only way work starts).
-- Prospective: spawns recorded before this migration keep brief_sha NULL (the legal default under foreign_keys ON);
-- `fm-db record spawn` refuses a new spawn into a unit with no recorded brief, whatever this column holds.
ALTER TABLE spawns ADD COLUMN brief_sha TEXT REFERENCES briefs(brief_sha);

-- open_units gains `briefs` so `fm-db status` shows which open units can still be spawned into.
DROP VIEW IF EXISTS open_units;
CREATE VIEW open_units AS
  SELECT u.unit_id, u.project, u.kind, u.planned_agents,
         (SELECT count(*) FROM spawns s WHERE s.unit_id = u.unit_id) AS spawned,
         (SELECT count(*) FROM andon a WHERE a.unit_id = u.unit_id AND a.cleared_ts IS NULL) AS open_andon,
         (SELECT count(*) FROM briefs b WHERE b.unit_id = u.unit_id) AS briefs
  FROM units u WHERE u.closed_ts IS NULL;
