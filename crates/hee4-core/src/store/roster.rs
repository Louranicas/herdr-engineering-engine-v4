//! The roster family's store half (K1): the `roster_records` and `roster_revisions` tables
//! (migration `m005_roster`), the typed record, and the verbs K6 calls. Every write runs inside
//! one `Store::operate` closure, so each carries an `operations` row with a derived id and
//! replays by key; the key's action is always the caller's (this file spells no wire id). The
//! native model's own record is composed by [`Store::roster_compose_deploy`] under principal
//! `deploy`, action `deploy.install` (DECISIONS V4-62), never under an operator's key.
//!
//! The generation check of a revise or a disable runs before `operate` on the same connection:
//! this `Store` holds the ledger's one writing connection (every other connection is read-only),
//! so nothing can move a record between that read and the transaction that follows it; a replay
//! (same key, same bytes) is answered before the check, whatever the generation is now.

use hee4_contracts::bounds::MAX_TOKEN_BYTES;
use hee4_contracts::{Sha256Hex, TaskId, canonical_json};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::{Value, json};

use super::migrations::Migration;
use super::{Operation, OperationKey, Store, StoreError, now_ms, operation_id};

/// m005: the roster tables. `roster_revisions` is append-only (the events/receipts pattern).
const SCHEMA_M005: &str = "
CREATE TABLE roster_records(
  id TEXT PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('agent','model','runtime')),
  definition_json TEXT NOT NULL,
  capability TEXT NULL,
  locality TEXT NOT NULL CHECK (locality IN ('local','remote')),
  disabled INTEGER NOT NULL CHECK (disabled IN (0, 1)),
  generation INTEGER NOT NULL CHECK (generation >= 1),
  updated_ts INTEGER NOT NULL
) STRICT;
CREATE TABLE roster_revisions(
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  record_id TEXT NOT NULL REFERENCES roster_records(id),
  generation INTEGER NOT NULL CHECK (generation >= 1),
  definition_json TEXT NOT NULL,
  audit_reason TEXT NOT NULL,
  operation_id TEXT NOT NULL,
  ts INTEGER NOT NULL
) STRICT;
CREATE INDEX roster_revisions_by_record ON roster_revisions(record_id, seq);
CREATE TRIGGER roster_revisions_no_update BEFORE UPDATE ON roster_revisions
  BEGIN SELECT RAISE(ABORT, 'roster_revisions are append-only'); END;
CREATE TRIGGER roster_revisions_no_delete BEFORE DELETE ON roster_revisions
  BEGIN SELECT RAISE(ABORT, 'roster_revisions are append-only'); END;
";

/// The roster migration, registered by one line in `migrations::MIGRATIONS`.
pub(crate) const MIGRATION: Migration = Migration {
    name: "m005_roster",
    apply: m005_roster,
};

fn m005_roster(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute_batch(SCHEMA_M005)?;
    Ok(())
}

/// The principal the engine composes its own records under (V4-62).
const DEPLOY_PRINCIPAL: &str = "deploy";

/// The internal action the engine composes its own records under; not a catalogued id.
const DEPLOY_ACTION: &str = "deploy.install";

/// The audit reason of a composed record's revision.
const DEPLOY_AUDIT_REASON: &str = "composed by the engine at serve start";

/// The id prefix of a model record: `model:<name>`.
const MODEL_PREFIX: &str = "model:";

const COLUMNS: &str =
    "id, kind, definition_json, capability, locality, disabled, generation, updated_ts";

/// What a roster record is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RosterKind {
    /// An agent.
    Agent,
    /// A model the dispatcher may route to.
    Model,
    /// A runtime.
    Runtime,
}

impl RosterKind {
    /// Every kind, in wire order.
    pub const ALL: [Self; 3] = [Self::Agent, Self::Model, Self::Runtime];

    /// The wire spelling.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Model => "model",
            Self::Runtime => "runtime",
        }
    }

    /// The kind spelled `s`, if any.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.wire_name() == s)
    }
}

/// Where a record runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locality {
    /// On this machine.
    Local,
    /// Elsewhere.
    Remote,
}

impl Locality {
    /// Both localities, in wire order.
    pub const ALL: [Self; 2] = [Self::Local, Self::Remote];

    /// The wire spelling.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }

    /// The locality spelled `s`, if any.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.wire_name() == s)
    }
}

/// What a record can do (the route's `Capabilities`, declared).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RosterCaps {
    /// Largest context, in tokens.
    pub ctx_tokens: u32,
    /// Answers in constrained JSON.
    pub json_mode: bool,
    /// Calls tools.
    pub tool_use: bool,
    /// Runs on this machine.
    pub local: bool,
}

/// A record's definition: the wire `definition` object, every member required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterDefinition {
    /// What it is.
    pub kind: RosterKind,
    /// What it can do.
    pub caps: RosterCaps,
    /// Declared cost, milli-units.
    pub cost_milli: u32,
    /// Declared latency, ms.
    pub latency_ms: u32,
    /// Declared quality, 0..=1000.
    pub quality: u32,
    /// An optional capability tag the list filters by.
    pub capability: Option<String>,
    /// Where it runs.
    pub locality: Locality,
}

/// Why a wire `definition` was refused: the member (a pointer under `/body/definition`) and
/// what it needed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("definition{member}: {need}")]
pub struct DefinitionFault {
    /// The member's pointer under the definition (`/kind`, `/caps/ctx_tokens`, ...).
    pub member: &'static str,
    /// What the member needed.
    pub need: &'static str,
}

const DEFINITION_MEMBERS: [&str; 7] = [
    "kind",
    "caps",
    "cost_milli",
    "latency_ms",
    "quality",
    "capability",
    "locality",
];

const CAPS_MEMBERS: [&str; 4] = ["ctx_tokens", "json_mode", "tool_use", "local"];

fn member_u32(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    member: &'static str,
) -> Result<u32, DefinitionFault> {
    obj.get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(DefinitionFault {
            member,
            need: "unsigned 32-bit integer required",
        })
}

fn member_bool(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    member: &'static str,
) -> Result<bool, DefinitionFault> {
    obj.get(key)
        .and_then(Value::as_bool)
        .ok_or(DefinitionFault {
            member,
            need: "boolean required",
        })
}

impl RosterDefinition {
    /// Parse the wire object: every member required, no other member accepted.
    ///
    /// # Errors
    /// [`DefinitionFault`] naming the first member that is missing, mistyped or unknown.
    pub fn from_json(v: &Value) -> Result<Self, DefinitionFault> {
        let Value::Object(obj) = v else {
            return Err(DefinitionFault {
                member: "",
                need: "object required",
            });
        };
        if obj
            .keys()
            .any(|k| !DEFINITION_MEMBERS.contains(&k.as_str()))
        {
            return Err(DefinitionFault {
                member: "",
                need: "only kind, caps, cost_milli, latency_ms, quality, capability, locality",
            });
        }
        let kind = obj
            .get("kind")
            .and_then(Value::as_str)
            .and_then(RosterKind::parse)
            .ok_or(DefinitionFault {
                member: "/kind",
                need: "agent, model or runtime",
            })?;
        let Some(Value::Object(caps)) = obj.get("caps") else {
            return Err(DefinitionFault {
                member: "/caps",
                need: "object {ctx_tokens, json_mode, tool_use, local} required",
            });
        };
        if caps.keys().any(|k| !CAPS_MEMBERS.contains(&k.as_str())) {
            return Err(DefinitionFault {
                member: "/caps",
                need: "only ctx_tokens, json_mode, tool_use, local",
            });
        }
        let caps = RosterCaps {
            ctx_tokens: member_u32(caps, "ctx_tokens", "/caps/ctx_tokens")?,
            json_mode: member_bool(caps, "json_mode", "/caps/json_mode")?,
            tool_use: member_bool(caps, "tool_use", "/caps/tool_use")?,
            local: member_bool(caps, "local", "/caps/local")?,
        };
        let capability = match obj.get("capability") {
            Some(Value::Null) => None,
            Some(Value::String(s)) if !s.is_empty() && s.len() <= MAX_TOKEN_BYTES => {
                Some(s.clone())
            }
            _ => {
                return Err(DefinitionFault {
                    member: "/capability",
                    need: "null or a non-empty string of at most MAX_TOKEN_BYTES",
                });
            }
        };
        let locality = obj
            .get("locality")
            .and_then(Value::as_str)
            .and_then(Locality::parse)
            .ok_or(DefinitionFault {
                member: "/locality",
                need: "local or remote",
            })?;
        Ok(Self {
            kind,
            caps,
            cost_milli: member_u32(obj, "cost_milli", "/cost_milli")?,
            latency_ms: member_u32(obj, "latency_ms", "/latency_ms")?,
            quality: member_u32(obj, "quality", "/quality")?,
            capability,
            locality,
        })
    }

    /// The wire object (the inverse of [`RosterDefinition::from_json`]).
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "kind": self.kind.wire_name(),
            "caps": {
                "ctx_tokens": self.caps.ctx_tokens,
                "json_mode": self.caps.json_mode,
                "tool_use": self.caps.tool_use,
                "local": self.caps.local,
            },
            "cost_milli": self.cost_milli,
            "latency_ms": self.latency_ms,
            "quality": self.quality,
            "capability": self.capability,
            "locality": self.locality.wire_name(),
        })
    }

    /// SHA-256 over the canonical JSON of the definition.
    #[must_use]
    pub fn digest(&self) -> Sha256Hex {
        Sha256Hex::digest(canonical_json(&self.to_json()).as_bytes())
    }
}

/// Why a record id was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdFault {
    /// Not `<lowercase prefix>:<name of [A-Za-z0-9._:-]>`.
    #[error(
        "record id must be <prefix>:<name> with a lowercase prefix and a name of [A-Za-z0-9._:-]"
    )]
    Grammar,
    /// Over the token bound.
    #[error("record id is {bytes} bytes, at most {max}")]
    TooLong {
        /// The id's length.
        bytes: usize,
        /// [`MAX_TOKEN_BYTES`].
        max: usize,
    },
}

/// A record id: `<prefix>:<name>`, at most [`MAX_TOKEN_BYTES`]; a model's is `model:<name>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RosterId(String);

impl RosterId {
    /// Parse an id.
    ///
    /// # Errors
    /// [`IdFault`].
    pub fn parse(s: &str) -> Result<Self, IdFault> {
        if s.len() > MAX_TOKEN_BYTES {
            return Err(IdFault::TooLong {
                bytes: s.len(),
                max: MAX_TOKEN_BYTES,
            });
        }
        let Some((prefix, name)) = s.split_once(':') else {
            return Err(IdFault::Grammar);
        };
        let prefix_ok = !prefix.is_empty() && prefix.bytes().all(|b| b.is_ascii_lowercase());
        let name_ok = !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_:".contains(&b));
        if prefix_ok && name_ok {
            Ok(Self(s.to_owned()))
        } else {
            Err(IdFault::Grammar)
        }
    }

    /// The id of the model record for `name`.
    ///
    /// # Errors
    /// [`IdFault`] when the name does not fit the grammar.
    pub fn model(name: &str) -> Result<Self, IdFault> {
        Self::parse(&format!("{MODEL_PREFIX}{name}"))
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The model name of a `model:<name>` id; `None` for any other prefix.
    #[must_use]
    pub fn model_name(&self) -> Option<&str> {
        self.0.strip_prefix(MODEL_PREFIX)
    }
}

impl std::fmt::Display for RosterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A listing row (`RosterHeadV1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterHead {
    /// The record id.
    pub id: RosterId,
    /// What it is.
    pub kind: RosterKind,
    /// Bumped by every revise and disable; the precondition's resource generation.
    pub generation: u64,
    /// Set by `roster.disable`; never routed to.
    pub disabled: bool,
    /// The definition's capability tag.
    pub capability: Option<String>,
    /// The definition's locality.
    pub locality: Locality,
}

impl RosterHead {
    /// The wire object `{id, kind, generation, disabled, capability, locality}`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id.as_str(),
            "kind": self.kind.wire_name(),
            "generation": self.generation,
            "disabled": self.disabled,
            "capability": self.capability,
            "locality": self.locality.wire_name(),
        })
    }
}

/// A record in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterRecord {
    /// The listing row.
    pub head: RosterHead,
    /// The definition.
    pub definition: RosterDefinition,
    /// When the row was last written (ms since the Unix epoch).
    pub updated_ts: i64,
}

impl RosterRecord {
    /// The wire object `{id, kind, definition, generation, disabled, updated_ts}`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.head.id.as_str(),
            "kind": self.head.kind.wire_name(),
            "definition": self.definition.to_json(),
            "generation": self.head.generation,
            "disabled": self.head.disabled,
            "updated_ts": self.updated_ts,
        })
    }
}

/// What `roster.list` filters by. An empty `kinds` admits every kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterFilter {
    /// The kinds admitted (any when empty).
    pub kinds: Vec<RosterKind>,
    /// A capability tag the record must carry.
    pub capability: Option<String>,
    /// A locality the record must have.
    pub locality: Option<Locality>,
    /// Whether disabled records are listed.
    pub include_disabled: bool,
}

impl RosterFilter {
    /// The canonical JSON the page cursor is pinned to (kinds sorted and deduplicated).
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut kinds: Vec<&str> = self.kinds.iter().map(|k| k.wire_name()).collect();
        kinds.sort_unstable();
        kinds.dedup();
        json!({
            "kinds": kinds,
            "capability": self.capability,
            "locality": self.locality.map(Locality::wire_name),
            "include_disabled": self.include_disabled,
        })
    }

    /// SHA-256 over [`RosterFilter::to_json`]'s canonical form.
    #[must_use]
    pub fn digest(&self) -> Sha256Hex {
        Sha256Hex::digest(canonical_json(&self.to_json()).as_bytes())
    }
}

/// What `roster.disable` does with the record's active attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisablePolicy {
    /// Attempts run to their end; no new dispatch.
    LetFinish,
    /// Each active attempt's task gets a cancel intent.
    RequestCancel,
}

impl DisablePolicy {
    /// Both policies, in wire order.
    pub const ALL: [Self; 2] = [Self::LetFinish, Self::RequestCancel];

    /// The wire spelling.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::LetFinish => "let_finish",
            Self::RequestCancel => "request_cancel",
        }
    }

    /// The policy spelled `s`, if any.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.wire_name() == s)
    }
}

/// What a `roster_update` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterChange {
    /// The record did not exist: generation 1.
    Created,
    /// The record existed: generation + 1.
    Revised,
}

impl RosterChange {
    /// The wire spelling.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Revised => "revised",
        }
    }
}

/// Why a roster verb wrote nothing.
#[derive(Debug, thiserror::Error)]
pub enum RosterError {
    /// The ledger refused or failed ([`StoreError::Conflict`] is the same key with other bytes).
    #[error("ledger: {0}")]
    Store(#[from] StoreError),
    /// The caller's generation is not the record's (`stale_generation`, `current_generation`).
    #[error("record {id} is at generation {current}, not {expected}")]
    StaleGeneration {
        /// The record.
        id: RosterId,
        /// What the caller read.
        expected: u64,
        /// What the record is at now.
        current: u64,
    },
    /// No such record.
    #[error("no roster record {0}")]
    NotFound(RosterId),
    /// The id does not fit the grammar.
    #[error("record id: {0}")]
    Id(#[from] IdFault),
}

/// A `roster_records` row as SQLite hands it over.
struct RawRecord {
    id: String,
    kind: String,
    definition_json: String,
    capability: Option<String>,
    locality: String,
    disabled: i64,
    generation: i64,
    updated_ts: i64,
}

impl RawRecord {
    fn from_row(r: &rusqlite::Row<'_>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: r.get(0)?,
            kind: r.get(1)?,
            definition_json: r.get(2)?,
            capability: r.get(3)?,
            locality: r.get(4)?,
            disabled: r.get(5)?,
            generation: r.get(6)?,
            updated_ts: r.get(7)?,
        })
    }

    fn parse(self) -> Result<RosterRecord, StoreError> {
        let corrupt = |detail: String| StoreError::Corrupt {
            task: self.id.clone(),
            detail,
        };
        let id =
            RosterId::parse(&self.id).map_err(|e| corrupt(format!("roster_records.id: {e}")))?;
        let kind = RosterKind::parse(&self.kind)
            .ok_or_else(|| corrupt(format!("roster_records.kind: {}", self.kind)))?;
        let locality = Locality::parse(&self.locality)
            .ok_or_else(|| corrupt(format!("roster_records.locality: {}", self.locality)))?;
        let definition = serde_json::from_str::<Value>(&self.definition_json)
            .map_err(|e| corrupt(format!("roster_records.definition_json: {e}")))
            .and_then(|v| {
                RosterDefinition::from_json(&v)
                    .map_err(|e| corrupt(format!("roster_records.definition_json: {e}")))
            })?;
        let generation = u64::try_from(self.generation)
            .map_err(|_| corrupt(format!("roster_records.generation: {}", self.generation)))?;
        Ok(RosterRecord {
            head: RosterHead {
                id,
                kind,
                generation,
                disabled: self.disabled != 0,
                capability: self.capability,
                locality,
            },
            definition,
            updated_ts: self.updated_ts,
        })
    }
}

fn get_in(conn: &Connection, id: &RosterId) -> Result<Option<RosterRecord>, StoreError> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM roster_records WHERE id = ?1"),
        [id.as_str()],
        RawRecord::from_row,
    )
    .optional()?
    .map(RawRecord::parse)
    .transpose()
}

fn sql_count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

fn sql_generation(g: u64) -> Result<i64, StoreError> {
    i64::try_from(g).map_err(|_| StoreError::Corrupt {
        task: String::new(),
        detail: format!("roster generation {g} overflows"),
    })
}

/// Create (generation 1) or revise (generation + 1) `id` with `definition`, then append the
/// revision row; a revise keeps the `disabled` mark.
fn upsert_in(
    tx: &Transaction<'_>,
    id: &RosterId,
    definition: &RosterDefinition,
    audit_reason: &str,
    operation_id: &str,
) -> Result<(RosterChange, RosterRecord), StoreError> {
    let (generation, change) = match get_in(tx, id)? {
        None => (1, RosterChange::Created),
        Some(r) => (r.head.generation.saturating_add(1), RosterChange::Revised),
    };
    let definition_json = serde_json::to_string(&definition.to_json())?;
    let ts = now_ms();
    tx.execute(
        "INSERT INTO roster_records(id, kind, definition_json, capability, locality, disabled,
           generation, updated_ts)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET kind = excluded.kind,
           definition_json = excluded.definition_json, capability = excluded.capability,
           locality = excluded.locality, generation = excluded.generation,
           updated_ts = excluded.updated_ts",
        params![
            id.as_str(),
            definition.kind.wire_name(),
            definition_json,
            definition.capability,
            definition.locality.wire_name(),
            sql_generation(generation)?,
            ts
        ],
    )?;
    tx.execute(
        "INSERT INTO roster_revisions(record_id, generation, definition_json, audit_reason,
           operation_id, ts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id.as_str(),
            sql_generation(generation)?,
            definition_json,
            audit_reason,
            operation_id,
            ts
        ],
    )?;
    let record = get_in(tx, id)?.ok_or_else(|| StoreError::Corrupt {
        task: id.to_string(),
        detail: "roster_records row absent after its upsert".into(),
    })?;
    Ok((change, record))
}

/// Mark `id` disabled at generation + 1 and append the revision row (definition unchanged).
fn disable_in(
    tx: &Transaction<'_>,
    id: &RosterId,
    audit_reason: &str,
    operation_id: &str,
) -> Result<RosterRecord, StoreError> {
    let before = get_in(tx, id)?.ok_or_else(|| StoreError::Corrupt {
        task: id.to_string(),
        detail: "roster_records row absent inside the disable transaction".into(),
    })?;
    let generation = sql_generation(before.head.generation.saturating_add(1))?;
    let ts = now_ms();
    tx.execute(
        "UPDATE roster_records SET disabled = 1, generation = ?2, updated_ts = ?3 WHERE id = ?1",
        params![id.as_str(), generation, ts],
    )?;
    tx.execute(
        "INSERT INTO roster_revisions(record_id, generation, definition_json, audit_reason,
           operation_id, ts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id.as_str(),
            generation,
            serde_json::to_string(&before.definition.to_json())?,
            audit_reason,
            operation_id,
            ts
        ],
    )?;
    get_in(tx, id)?.ok_or_else(|| StoreError::Corrupt {
        task: id.to_string(),
        detail: "roster_records row absent after its disable".into(),
    })
}

/// The stored result of a create, revise or compose: what the wire reply carries.
fn change_result(change: RosterChange, record: &RosterRecord) -> Value {
    json!({"record": record.to_json(), "change": change.wire_name()})
}

impl Store {
    /// Rows in `roster_records`, disabled ones included (the route fallback reads it).
    ///
    /// # Errors
    /// SQLite errors.
    pub fn roster_count(&self) -> Result<u64, StoreError> {
        let n: i64 = self
            .conn
            .query_row("SELECT count(*) FROM roster_records", [], |r| r.get(0))?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// The record `id`, if any.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unreadable row.
    pub fn roster_get(&self, id: &RosterId) -> Result<Option<RosterRecord>, StoreError> {
        get_in(&self.conn, id)
    }

    /// Up to `limit` heads matching `filter`, in id order, after `after_id` (keyset).
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unreadable row.
    pub fn roster_list(
        &self,
        filter: &RosterFilter,
        after_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RosterHead>, StoreError> {
        let kinds: Vec<&str> = filter.kinds.iter().map(|k| k.wire_name()).collect();
        let kind_at = |i: usize| kinds.get(i).copied();
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM roster_records
             WHERE (?1 = 1 OR disabled = 0)
               AND (?2 IS NULL OR capability = ?2)
               AND (?3 IS NULL OR locality = ?3)
               AND (?4 IS NULL OR id > ?4)
               AND (?5 = 1 OR kind IN (?6, ?7, ?8))
             ORDER BY id LIMIT ?9"
        ))?;
        let rows = stmt.query_map(
            params![
                i64::from(filter.include_disabled),
                filter.capability,
                filter.locality.map(Locality::wire_name),
                after_id,
                i64::from(kinds.is_empty()),
                kind_at(0),
                kind_at(1),
                kind_at(2),
                sql_count(limit)
            ],
            RawRecord::from_row,
        )?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?.parse()?.head);
        }
        Ok(out)
    }

    /// Every record the dispatcher may route to: kind `model`, not disabled, in id order.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unreadable row.
    pub fn roster_eligible(&self) -> Result<Vec<RosterRecord>, StoreError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM roster_records
             WHERE kind = ?1 AND disabled = 0 ORDER BY id"
        ))?;
        let rows = stmt.query_map([RosterKind::Model.wire_name()], RawRecord::from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?.parse()?);
        }
        Ok(out)
    }

    /// Refuse a revise or disable whose `expected` generation is not the record's. Runs before
    /// `operate` (see the module doc) and only when `op` is not already recorded, so a replay
    /// answers the stored result whatever the generation is now.
    fn roster_expect(
        &self,
        op: &OperationKey,
        id: &RosterId,
        expected: Option<u64>,
    ) -> Result<(), RosterError> {
        let Some(expected) = expected else {
            return Ok(());
        };
        if self.operation_by_key(op)?.is_some() {
            return Ok(());
        }
        match get_in(&self.conn, id)? {
            None => Err(RosterError::NotFound(id.clone())),
            Some(r) if r.head.generation != expected => Err(RosterError::StaleGeneration {
                id: id.clone(),
                expected,
                current: r.head.generation,
            }),
            Some(_) => Ok(()),
        }
    }

    /// Create or revise `id` under the caller's `op`, in one `operate` closure: the record row,
    /// one `roster_revisions` row carrying the operation id, and the `operations` row (subject =
    /// the record id, no task). The stored result is `{record, change}`.
    ///
    /// # Errors
    /// [`RosterError::StaleGeneration`] when `expected_generation` is not the record's;
    /// [`RosterError::NotFound`] when one was given for a record that does not exist;
    /// [`StoreError::Conflict`] (wrapped) for the same key with other bytes.
    pub fn roster_update(
        &self,
        op: &OperationKey,
        request: &[u8],
        id: &RosterId,
        definition: &RosterDefinition,
        audit_reason: &str,
        expected_generation: Option<u64>,
    ) -> Result<Operation, RosterError> {
        self.roster_expect(op, id, expected_generation)?;
        let op_id = operation_id(op);
        Ok(self.operate(op, request, |tx| {
            let (change, record) = upsert_in(tx, id, definition, audit_reason, &op_id)?;
            Ok((None, Some(id.to_string()), change_result(change, &record)))
        })?)
    }

    /// Disable `id` at `expected_generation` under the caller's `op`: `disabled = 1`,
    /// generation + 1, one revision row, the `operations` row. `active_attempts` is what K6
    /// found mid-attempt on this record; it is recorded in the stored result
    /// `{record, policy, active_attempts}` so a replay reports the same list.
    ///
    /// # Errors
    /// [`RosterError::NotFound`], [`RosterError::StaleGeneration`], [`StoreError::Conflict`]
    /// (wrapped).
    #[expect(
        clippy::too_many_arguments,
        reason = "the verb's inputs are the wire body plus the precondition and K6's attempt list"
    )]
    pub fn roster_disable(
        &self,
        op: &OperationKey,
        request: &[u8],
        id: &RosterId,
        expected_generation: u64,
        policy: DisablePolicy,
        audit_reason: &str,
        active_attempts: &[TaskId],
    ) -> Result<Operation, RosterError> {
        self.roster_expect(op, id, Some(expected_generation))?;
        let op_id = operation_id(op);
        let attempts: Vec<&str> = active_attempts.iter().map(TaskId::as_str).collect();
        Ok(self.operate(op, request, |tx| {
            let record = disable_in(tx, id, audit_reason, &op_id)?;
            Ok((
                None,
                Some(id.to_string()),
                json!({
                    "record": record.to_json(),
                    "policy": policy.wire_name(),
                    "active_attempts": attempts,
                }),
            ))
        })?)
    }

    /// Compose the engine's own record for `model_name` (id `model:<name>`) as an internal
    /// operation under principal `deploy`, action `deploy.install`, keyed by the digest of
    /// `{model, definition}`: the same name and definition replays (no second revision); a
    /// changed definition revises the record.
    ///
    /// # Errors
    /// [`RosterError::Id`] when the name does not fit the id grammar; the ledger's errors.
    pub fn roster_compose_deploy(
        &self,
        model_name: &str,
        definition: &RosterDefinition,
    ) -> Result<Operation, RosterError> {
        let id = RosterId::model(model_name)?;
        let request = canonical_json(&json!({
            "model": model_name,
            "definition": definition.to_json(),
        }));
        let op = OperationKey {
            principal: DEPLOY_PRINCIPAL.into(),
            action: DEPLOY_ACTION.into(),
            version: 1,
            idem_key: Sha256Hex::digest(request.as_bytes()).to_string(),
        };
        let op_id = operation_id(&op);
        Ok(self.operate(&op, request.as_bytes(), |tx| {
            let (change, record) = upsert_in(tx, &id, definition, DEPLOY_AUDIT_REASON, &op_id)?;
            Ok((None, Some(id.to_string()), change_result(change, &record)))
        })?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type R = Result<(), Box<dyn std::error::Error>>;

    fn path(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("hee4-core-roster-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{name}.sqlite"));
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
        Ok(path)
    }

    fn fresh(name: &str) -> Result<Store, Box<dyn std::error::Error>> {
        Ok(Store::open(&path(name)?)?)
    }

    fn key(idem: &str) -> OperationKey {
        OperationKey {
            principal: "uid:1".into(),
            action: "a.b".into(),
            version: 1,
            idem_key: idem.into(),
        }
    }

    fn definition(quality: u32) -> RosterDefinition {
        RosterDefinition {
            kind: RosterKind::Model,
            caps: RosterCaps {
                ctx_tokens: 4096,
                json_mode: true,
                tool_use: false,
                local: true,
            },
            cost_milli: 0,
            latency_ms: 0,
            quality,
            capability: None,
            locality: Locality::Local,
        }
    }

    fn id(s: &str) -> Result<RosterId, IdFault> {
        RosterId::parse(s)
    }

    fn revisions(store: &Store, id: &RosterId) -> Result<i64, rusqlite::Error> {
        store.conn.query_row(
            "SELECT count(*) FROM roster_revisions WHERE record_id = ?1",
            [id.as_str()],
            |r| r.get(0),
        )
    }

    fn operations_under(store: &Store, action: &str) -> Result<i64, rusqlite::Error> {
        store.conn.query_row(
            "SELECT count(*) FROM operations WHERE action = ?1",
            [action],
            |r| r.get(0),
        )
    }

    #[test]
    fn roster_update_creates_generation_1_then_revises_to_2() -> R {
        let store = fresh("create-revise")?;
        let id = id("model:alpha")?;
        let created = store.roster_update(&key("k1"), b"a", &id, &definition(1), "add", None)?;
        assert_eq!(created.result["change"], "created");
        assert_eq!(created.result["record"]["generation"], 1);
        assert_eq!(created.subject.as_deref(), Some("model:alpha"));
        assert_eq!(created.task_id, None);
        let revised =
            store.roster_update(&key("k2"), b"b", &id, &definition(2), "tune", Some(1))?;
        assert_eq!(revised.result["change"], "revised");
        assert_eq!(revised.result["record"]["generation"], 2);
        let record = store.roster_get(&id)?.ok_or("record")?;
        assert_eq!(record.head.generation, 2);
        assert_eq!(record.definition, definition(2));
        assert_eq!(
            revisions(&store, &id)?,
            i64::try_from(record.head.generation)?,
            "one revision row per generation"
        );
        Ok(())
    }

    #[test]
    fn roster_update_replay_returns_same_operation_id_and_no_second_revision() -> R {
        let store = fresh("replay")?;
        let id = id("model:beta")?;
        let first = store.roster_update(&key("k1"), b"same", &id, &definition(1), "add", None)?;
        let before = revisions(&store, &id)?;
        let again = store.roster_update(&key("k1"), b"same", &id, &definition(1), "add", None)?;
        assert!(again.replayed);
        assert_eq!(again.operation_id, first.operation_id);
        assert_eq!(again.result, first.result);
        assert_eq!(revisions(&store, &id)?, before);
        let stale_replay =
            store.roster_update(&key("k1"), b"same", &id, &definition(1), "add", Some(99))?;
        assert!(
            stale_replay.replayed,
            "a replay is answered before the generation check"
        );
        Ok(())
    }

    #[test]
    fn roster_update_other_bytes_is_conflict() -> R {
        let store = fresh("conflict")?;
        let id = id("model:gamma")?;
        store.roster_update(&key("k1"), b"one", &id, &definition(1), "add", None)?;
        let before = revisions(&store, &id)?;
        let other = store.roster_update(&key("k1"), b"two", &id, &definition(2), "add", None);
        assert!(
            matches!(&other, Err(RosterError::Store(StoreError::Conflict(k))) if *k == key("k1")),
            "{other:?}"
        );
        assert_eq!(revisions(&store, &id)?, before);
        Ok(())
    }

    #[test]
    fn roster_update_stale_expected_generation_names_current() -> R {
        let store = fresh("stale")?;
        let id = id("model:delta")?;
        store.roster_update(&key("k1"), b"a", &id, &definition(1), "add", None)?;
        store.roster_update(&key("k2"), b"b", &id, &definition(2), "tune", Some(1))?;
        let current = store.roster_get(&id)?.ok_or("record")?.head.generation;
        let before = revisions(&store, &id)?;
        let stale = store.roster_update(&key("k3"), b"c", &id, &definition(3), "tune", Some(1));
        assert!(
            matches!(&stale, Err(RosterError::StaleGeneration { expected: 1, current: c, .. }) if *c == current),
            "{stale:?}"
        );
        assert_eq!(
            revisions(&store, &id)?,
            before,
            "a stale revise writes nothing"
        );
        assert!(store.operation_by_key(&key("k3"))?.is_none());
        let absent = store.roster_update(
            &key("k4"),
            b"d",
            &RosterId::parse("model:none")?,
            &definition(1),
            "x",
            Some(1),
        );
        assert!(
            matches!(absent, Err(RosterError::NotFound(_))),
            "{absent:?}"
        );
        Ok(())
    }

    #[test]
    fn roster_disable_hides_from_eligible_and_bumps_generation() -> R {
        let store = fresh("disable")?;
        let id = id("model:eps")?;
        let other = RosterId::parse("model:zeta")?;
        store.roster_update(&key("k1"), b"a", &id, &definition(1), "add", None)?;
        store.roster_update(&key("k2"), b"b", &other, &definition(1), "add", None)?;
        let eligible_before: Vec<RosterId> = store
            .roster_eligible()?
            .into_iter()
            .map(|r| r.head.id)
            .collect();
        assert!(eligible_before.contains(&id));
        let task: TaskId = "t-1".parse()?;
        let op = store.roster_disable(
            &key("k3"),
            b"c",
            &id,
            1,
            DisablePolicy::RequestCancel,
            "retire",
            std::slice::from_ref(&task),
        )?;
        assert_eq!(op.result["record"]["disabled"], true);
        assert_eq!(op.result["record"]["generation"], 2);
        assert_eq!(op.result["active_attempts"], json!([task.as_str()]));
        assert_eq!(op.result["policy"], "request_cancel");
        let eligible_after: Vec<RosterId> = store
            .roster_eligible()?
            .into_iter()
            .map(|r| r.head.id)
            .collect();
        assert!(!eligible_after.contains(&id));
        assert!(eligible_after.contains(&other));
        assert_eq!(store.roster_count()?, u64::try_from(eligible_before.len())?);
        let all = RosterFilter {
            kinds: vec![],
            capability: None,
            locality: None,
            include_disabled: true,
        };
        let hidden = RosterFilter {
            include_disabled: false,
            ..all.clone()
        };
        let shown: Vec<RosterId> = store
            .roster_list(&all, None, 10)?
            .into_iter()
            .map(|h| h.id)
            .collect();
        assert!(shown.contains(&id));
        let listed: Vec<RosterId> = store
            .roster_list(&hidden, None, 10)?
            .into_iter()
            .map(|h| h.id)
            .collect();
        assert!(!listed.contains(&id));
        assert!(listed.contains(&other));
        let again = store.roster_disable(
            &key("k4"),
            b"d",
            &id,
            1,
            DisablePolicy::LetFinish,
            "again",
            &[],
        );
        assert!(
            matches!(again, Err(RosterError::StaleGeneration { current: 2, .. })),
            "{again:?}"
        );
        let unknown = store.roster_disable(
            &key("k5"),
            b"e",
            &RosterId::parse("model:none")?,
            1,
            DisablePolicy::LetFinish,
            "x",
            &[],
        );
        assert!(
            matches!(unknown, Err(RosterError::NotFound(_))),
            "{unknown:?}"
        );
        let after = store
            .roster_list(&all, Some(id.as_str()), 10)?
            .into_iter()
            .map(|h| h.id)
            .collect::<Vec<_>>();
        assert_eq!(after, [other], "keyset: ids after the given one");
        Ok(())
    }

    #[test]
    fn roster_revisions_are_append_only() -> R {
        let store = fresh("append-only")?;
        let id = id("model:eta")?;
        store.roster_update(&key("k1"), b"a", &id, &definition(1), "add", None)?;
        let update = store
            .conn
            .execute("UPDATE roster_revisions SET audit_reason = 'edited'", []);
        assert!(
            matches!(&update, Err(e) if e.to_string().contains("append-only")),
            "{update:?}"
        );
        let delete = store.conn.execute("DELETE FROM roster_revisions", []);
        assert!(
            matches!(&delete, Err(e) if e.to_string().contains("append-only")),
            "{delete:?}"
        );
        assert!(revisions(&store, &id)? > 0);
        Ok(())
    }

    #[test]
    fn roster_compose_deploy_is_idempotent_across_two_opens() -> R {
        let path = path("compose")?;
        let first = {
            let store = Store::open(&path)?;
            let op = store.roster_compose_deploy("qwen:1b", &definition(0))?;
            assert!(!op.replayed);
            assert_eq!(op.subject.as_deref(), Some("model:qwen:1b"));
            assert_eq!(op.result["change"], "created");
            op
        };
        let store = Store::open(&path)?;
        let again = store.roster_compose_deploy("qwen:1b", &definition(0))?;
        assert!(again.replayed);
        assert_eq!(again.operation_id, first.operation_id);
        assert_eq!(again.result, first.result);
        let id = RosterId::model("qwen:1b")?;
        let record = store.roster_get(&id)?.ok_or("record")?;
        assert_eq!(record.head.generation, 1);
        assert_eq!(
            revisions(&store, &id)?,
            i64::try_from(record.head.generation)?
        );
        assert_eq!(
            operations_under(&store, DEPLOY_ACTION)?,
            i64::try_from(record.head.generation)?
        );
        let last = store
            .last_operation_for(id.as_str())?
            .ok_or("last operation")?;
        assert_eq!(last.action, DEPLOY_ACTION);
        let other_name = store.roster_compose_deploy("qwen:7b", &definition(0))?;
        assert!(!other_name.replayed, "another model is another key");
        let changed = store.roster_compose_deploy("qwen:1b", &definition(5))?;
        assert!(!changed.replayed);
        assert_eq!(changed.result["change"], "revised");
        assert!(RosterId::model("bad name").is_err());
        Ok(())
    }

    #[test]
    fn definition_parse_refuses_unknown_and_missing_members_by_pointer() {
        let good = definition(3).to_json();
        assert_eq!(RosterDefinition::from_json(&good), Ok(definition(3)));
        let mut extra = good.clone();
        extra["extra"] = json!(1);
        assert!(RosterDefinition::from_json(&extra).is_err());
        let mut no_caps = good.clone();
        no_caps["caps"] = json!({"ctx_tokens": 1});
        assert_eq!(
            RosterDefinition::from_json(&no_caps).map_err(|e| e.member),
            Err("/caps/json_mode")
        );
        assert_eq!(
            RosterDefinition::from_json(&json!({})).map_err(|e| e.member),
            Err("/kind")
        );
        let mut bad_locality = good;
        bad_locality["locality"] = json!("moon");
        assert_eq!(
            RosterDefinition::from_json(&bad_locality).map_err(|e| e.member),
            Err("/locality")
        );
        assert!(RosterId::parse("Model:x").is_err());
        assert!(RosterId::parse("model:").is_err());
        assert!(RosterId::parse(&format!("model:{}", "x".repeat(MAX_TOKEN_BYTES))).is_err());
        assert_eq!(
            RosterId::parse("model:qwen2.5-coder:7b").map(|i| i.model_name().map(str::to_owned)),
            Ok(Some("qwen2.5-coder:7b".into()))
        );
    }
}
