-- hee4-ops.db · migration 006 · 2026-10-05 (rev 2026-10-05 schedule, U-stack-04 ops-db-schedule)
-- Two CHECK widenings on the registry (005), so systemd user timers are a schedule source on a host with no cron:
--   registry_sources.kind   gains 'timers': the systemd user unit directory (~/.config/systemd/user, or
--                           HEE4DB_TIMERS_DIR for the control), read from its *.timer/*.service FILES only, never
--                           through systemctl; status ok | absent | unreadable as for the crontab, where absent is a
--                           measured fact about the host (no directory, or no crontab binary), never a zero
--   agent_schedules.kind    gains 'daily': an ExecStart whose command is ops/db/daily.sh (the wave-2 hee4-daily.timer);
--                           'roster' (run-agent.sh <agent> <mode>) keeps its CHECK, anything else stays 'other'
-- SQLite cannot ALTER a CHECK, and ALTER TABLE … RENAME validates dependent views (3.51.2), so each table is rebuilt:
-- the view agent_status is dropped first, the table is re-created beside the old one, the rows are copied (both tables
-- are DERIVED and `hee4db ingest` rebuilds them, but the copy keeps check=world and check=migrations measured straight
-- after migrate), the old table is dropped and the new one renamed; the index agent_schedules_agent and the view
-- agent_status are then recreated verbatim from 005.

DROP VIEW agent_status;

CREATE TABLE registry_sources_new (
  id           INTEGER PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('skills_dir','agents_dir','hooks_dir','roster_dir','modes','settings','crontab','timers')),
  scope        TEXT NOT NULL CHECK (scope IN ('user','project','host')),
  source_root  TEXT NOT NULL CHECK (source_root IN ('claude','repo','host','abs')),
  source_rel   TEXT NOT NULL,
  path         TEXT NOT NULL,                                        -- absolute, as ingested (display)
  status       TEXT NOT NULL CHECK (status IN ('ok','absent','unreadable')),
  sha256       TEXT CHECK (sha256 IS NULL OR length(sha256) = 64),   -- files, the crontab text, the timer+service bytes; NULL for a directory
  entries      INTEGER NOT NULL CHECK (entries >= 0),                -- rows derived (files/lines) from this location
  detail       TEXT NOT NULL,
  ingested_at  TEXT NOT NULL,
  UNIQUE (source_root, source_rel)
) STRICT;
INSERT INTO registry_sources_new (id, kind, scope, source_root, source_rel, path, status, sha256, entries, detail, ingested_at)
  SELECT id, kind, scope, source_root, source_rel, path, status, sha256, entries, detail, ingested_at FROM registry_sources;
DROP TABLE registry_sources;
ALTER TABLE registry_sources_new RENAME TO registry_sources;

CREATE TABLE agent_schedules_new (
  id             INTEGER PRIMARY KEY,
  kind           TEXT NOT NULL CHECK (kind IN ('roster','daily','other')),
  schedule       TEXT NOT NULL,                       -- the five cron fields (or an @word), or a timer's OnCalendar= value, verbatim
  agent          TEXT,                                -- run-agent.sh's first argument (NULL for kind daily/other)
  mode           TEXT,                                -- run-agent.sh's second argument
  command        TEXT NOT NULL,                       -- the cron command, or the paired .service ExecStart=, verbatim
  source_root    TEXT NOT NULL CHECK (source_root IN ('claude','repo','host','abs')),
  source_rel     TEXT NOT NULL,
  source_path    TEXT NOT NULL,                       -- 'crontab:host', the HEE4DB_CRONTAB_FILE path, or the .timer file
  source_sha256  TEXT NOT NULL CHECK (length(source_sha256) = 64),
  source_line    INTEGER NOT NULL,
  ingested_at    TEXT NOT NULL,
  CHECK ((kind = 'roster') = (agent IS NOT NULL AND mode IS NOT NULL))
) STRICT;
INSERT INTO agent_schedules_new (id, kind, schedule, agent, mode, command, source_root, source_rel, source_path, source_sha256, source_line, ingested_at)
  SELECT id, kind, schedule, agent, mode, command, source_root, source_rel, source_path, source_sha256, source_line, ingested_at FROM agent_schedules;
DROP TABLE agent_schedules;
ALTER TABLE agent_schedules_new RENAME TO agent_schedules;
CREATE INDEX agent_schedules_agent ON agent_schedules(agent);

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
