-- hee4-ops.db · migration 005 · 2026-10-01 (rev 2026-10-01 registry, V4-70)
-- A registry of the skills, agents and reflexes a v4 session or roster run works with. Every table here is
-- DERIVED: each source file stays the home of its facts, and `hee4db ingest` rebuilds these rows in the
-- same transaction as the planning tables. Rows carry the absolute source path, its sha256 and the ingest
-- time directly (the sources table's root CHECK names only the three v4 homes; ~/.claude and the host
-- crontab are not v4 homes, they are read-only inputs). A location is (source_root, source_rel) with
-- source_root in claude (~/.claude) | repo (the v4 repo) | host (the crontab) | abs (anything else), resolved
-- by hee4db at check time, as `sources` does; source_path is the absolute path as ingested, for display only.
--   registry_sources  one row per location the registry ingest looked at (a directory, a file, the
--                     crontab): status ok | absent | unreadable, so an absent project dir or an unreadable
--                     crontab is a recorded fact, never zero rows read as "none"
--   skills            ~/.claude/skills/*/SKILL.md (scope user) and <repo>/.claude/skills/*/SKILL.md (project)
--   agents            <repo>/.claude/agents/*.md (project) and ~/.claude/agents/*.md (user)
--   agent_modes       <repo>/ops/roster/<agent>/modes.conf, one row per `mode|budget_usd|prompt` line
--   agent_schedules   the host crontab (`flatpak-spawn --host crontab -l`, read-only), one row per job line
--   reflexes          ~/.claude/hooks/*.sh (scope user) joined with the user settings.json hooks block, plus
--                     the project settings.json hooks (scope project); jev_sender is a fact for H-8/H-19
-- v4_relevant and jev_sender are computed by rules stated in code (hee4db: SKILL_V4_RULE, JEV_SENDER_RULE).

CREATE TABLE registry_sources (
  id           INTEGER PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('skills_dir','agents_dir','hooks_dir','roster_dir','modes','settings','crontab')),
  scope        TEXT NOT NULL CHECK (scope IN ('user','project','host')),
  source_root  TEXT NOT NULL CHECK (source_root IN ('claude','repo','host','abs')),
  source_rel   TEXT NOT NULL,
  path         TEXT NOT NULL,                                        -- absolute, as ingested (display)
  status       TEXT NOT NULL CHECK (status IN ('ok','absent','unreadable')),
  sha256       TEXT CHECK (sha256 IS NULL OR length(sha256) = 64),   -- files and the crontab text; NULL for a directory
  entries      INTEGER NOT NULL CHECK (entries >= 0),                -- rows derived (files/lines) from this location
  detail       TEXT NOT NULL,
  ingested_at  TEXT NOT NULL,
  UNIQUE (source_root, source_rel)
) STRICT;

CREATE TABLE skills (
  scope          TEXT NOT NULL CHECK (scope IN ('user','project')),
  dir_name       TEXT NOT NULL,
  name           TEXT NOT NULL,                       -- frontmatter name ('' when it did not parse)
  description    TEXT NOT NULL,
  parse_error    TEXT,                                -- NULL when the frontmatter parsed with name + description
  lines          INTEGER NOT NULL CHECK (lines >= 0),
  has_reference  INTEGER NOT NULL CHECK (has_reference IN (0,1)),
  v4_relevant    INTEGER NOT NULL CHECK (v4_relevant IN (0,1)),
  v4_reason      TEXT NOT NULL,                       -- rule:<token> | curated:<reason> | excluded:<reason> | ''
  source_root    TEXT NOT NULL CHECK (source_root IN ('claude','repo','host','abs')),
  source_rel     TEXT NOT NULL,
  source_path    TEXT NOT NULL,
  source_sha256  TEXT NOT NULL CHECK (length(source_sha256) = 64),
  ingested_at    TEXT NOT NULL,
  PRIMARY KEY (scope, dir_name),
  UNIQUE (source_root, source_rel)
) STRICT;

CREATE TABLE agents (
  scope          TEXT NOT NULL CHECK (scope IN ('user','project')),
  name           TEXT NOT NULL,                       -- frontmatter name, else the file stem
  model          TEXT,
  tools          TEXT,
  description    TEXT NOT NULL,
  parse_error    TEXT,
  source_root    TEXT NOT NULL CHECK (source_root IN ('claude','repo','host','abs')),
  source_rel     TEXT NOT NULL,
  source_path    TEXT NOT NULL,
  source_sha256  TEXT NOT NULL CHECK (length(source_sha256) = 64),
  ingested_at    TEXT NOT NULL,
  PRIMARY KEY (scope, name),
  UNIQUE (source_root, source_rel)
) STRICT;

CREATE TABLE agent_modes (
  agent          TEXT NOT NULL,                       -- the roster directory name; not a FK: a missing definition is what registry_agents reports
  mode           TEXT NOT NULL,
  budget_usd     REAL NOT NULL CHECK (budget_usd >= 0),
  prompt         TEXT NOT NULL,
  source_root    TEXT NOT NULL CHECK (source_root IN ('claude','repo','host','abs')),
  source_rel     TEXT NOT NULL,
  source_path    TEXT NOT NULL,
  source_sha256  TEXT NOT NULL CHECK (length(source_sha256) = 64),
  source_line    INTEGER NOT NULL,
  ingested_at    TEXT NOT NULL,
  PRIMARY KEY (agent, mode)
) STRICT;

CREATE TABLE agent_schedules (
  id             INTEGER PRIMARY KEY,
  kind           TEXT NOT NULL CHECK (kind IN ('roster','other')),
  schedule       TEXT NOT NULL,                       -- the five cron fields (or an @word), verbatim
  agent          TEXT,                                -- run-agent.sh's first argument (NULL for kind other)
  mode           TEXT,                                -- run-agent.sh's second argument
  command        TEXT NOT NULL,
  source_root    TEXT NOT NULL CHECK (source_root IN ('claude','repo','host','abs')),
  source_rel     TEXT NOT NULL,
  source_path    TEXT NOT NULL,                       -- 'crontab:host' (flatpak-spawn --host crontab -l) or the HEE4DB_CRONTAB_FILE path
  source_sha256  TEXT NOT NULL CHECK (length(source_sha256) = 64),
  source_line    INTEGER NOT NULL,
  ingested_at    TEXT NOT NULL,
  CHECK ((kind = 'roster') = (agent IS NOT NULL AND mode IS NOT NULL))
) STRICT;
CREATE INDEX agent_schedules_agent ON agent_schedules(agent);

CREATE TABLE reflexes (
  scope          TEXT NOT NULL CHECK (scope IN ('user','project')),
  name           TEXT NOT NULL,                       -- script file name (e.g. pipe-verdict-guard.sh)
  path           TEXT NOT NULL,
  events         TEXT NOT NULL,                       -- comma-joined settings.json events wiring it ('' when not wired)
  matchers       TEXT NOT NULL,                       -- comma-joined event:matcher pairs ('' when not wired)
  wired          INTEGER NOT NULL CHECK (wired IN (0,1)),
  jev_sender     INTEGER NOT NULL CHECK (jev_sender IN (0,1)),
  jev_reason     TEXT NOT NULL,                       -- name | code:<token> | name+code:<token> | ''
  settings_path  TEXT NOT NULL,
  source_root    TEXT NOT NULL CHECK (source_root IN ('claude','repo','host','abs')),
  source_rel     TEXT NOT NULL,
  source_path    TEXT NOT NULL,                       -- the script when readable, else the settings file
  source_sha256  TEXT NOT NULL CHECK (length(source_sha256) = 64),
  ingested_at    TEXT NOT NULL,
  PRIMARY KEY (scope, name)
) STRICT;

-- One row per agent named anywhere (definition, roster dir or crontab): its schedule, modes, the last run
-- recorded in roster_runs, and its spend over the last 24 h (stamps are UTC `YYYYMMDDTHHMMSSZ`).
CREATE VIEW agent_status AS
SELECT n.agent AS agent,
       (SELECT group_concat(a.scope || ':' || coalesce(a.model, '?'), ',') FROM agents a WHERE a.name = n.agent) AS definition,
       (SELECT group_concat(m.mode || '=$' || m.budget_usd, ',') FROM agent_modes m WHERE m.agent = n.agent) AS modes,
       (SELECT group_concat(s.mode || '@' || s.schedule, '; ') FROM agent_schedules s WHERE s.agent = n.agent) AS schedule,
       r.stamp AS last_stamp, r.mode AS last_mode, r.exit_code AS last_exit, r.verdict AS last_verdict, r.cost_usd AS last_cost_usd,
       (SELECT count(*) FROM roster_runs x WHERE x.agent = n.agent
          AND x.stamp >= strftime('%Y%m%dT%H%M%SZ', 'now', '-1 day')) AS runs_24h,
       (SELECT round(sum(x.cost_usd), 4) FROM roster_runs x WHERE x.agent = n.agent
          AND x.stamp >= strftime('%Y%m%dT%H%M%SZ', 'now', '-1 day')) AS spend_24h_usd,
       (SELECT sum(x.cost_usd IS NULL) FROM roster_runs x WHERE x.agent = n.agent
          AND x.stamp >= strftime('%Y%m%dT%H%M%SZ', 'now', '-1 day')) AS cost_unmeasured_24h
FROM (SELECT name AS agent FROM agents
      UNION SELECT agent FROM agent_modes
      UNION SELECT agent FROM agent_schedules WHERE agent IS NOT NULL) n
LEFT JOIN roster_runs r ON r.id = (SELECT y.id FROM roster_runs y WHERE y.agent = n.agent ORDER BY y.stamp DESC, y.id DESC LIMIT 1);
