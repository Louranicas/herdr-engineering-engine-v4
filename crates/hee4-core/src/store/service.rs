//! The service family's store half (K1): the `service_facts` and `service_claims` migrations,
//! their readers, the seed, the durable act claim, and the two `Store::operate` commits. Every byte of SQL for `service_facts` lives here (the
//! one-door rule, `FLOW.md`); the runner (K5) never sees this file, and the actions (K6) commit
//! through these verbs and never hold a `Transaction`.

use hee4_contracts::{Observation, Sha256Hex};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;

use super::migrations::Migration;
use super::{Operation, OperationKey, RawOperation, Store, StoreError, now_ms, operation_id};

/// `m005_service_facts`: one row per managed service, keyed by its service id. `generation`
/// starts at 1 and moves only through [`Store::service_action_commit`]; `owner_sha256` is the
/// digest of `owner_id`, the CAS guard an action names. `actable` is 0 for a service no
/// `service.action` may start or stop (the engine itself, the model): the refusal is data.
const SCHEMA_M005: &str = "
CREATE TABLE service_facts(
  service_id TEXT PRIMARY KEY NOT NULL,
  owner_id TEXT NOT NULL,
  unit_id TEXT NOT NULL,
  owner_sha256 TEXT NOT NULL CHECK (length(owner_sha256) = 64),
  generation INTEGER NOT NULL CHECK (generation >= 1),
  actable INTEGER NOT NULL CHECK (actable IN (0, 1)),
  cached_health_json TEXT NULL,
  updated_ts INTEGER NOT NULL
) STRICT;
";

/// The service family's migration: the one line it registers in `MIGRATIONS`.
pub(super) const MIGRATION: Migration = Migration {
    name: "m005_service_facts",
    apply: m005_service_facts,
};

fn m005_service_facts(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute_batch(SCHEMA_M005)?;
    Ok(())
}

/// `m006_service_claims`: at most one act claim per service, by primary key (a second row for
/// one service is unrepresentable). [`Store::service_claim`] inserts it before `runner.act`;
/// [`Store::service_release`] deletes it after the commit. The row is the cross-process guard:
/// two serves on one ledger (or a serve restarted mid-act) cannot both hold it.
const SCHEMA_M006: &str = "
CREATE TABLE service_claims(
  service_id TEXT PRIMARY KEY NOT NULL REFERENCES service_facts(service_id),
  generation INTEGER NOT NULL CHECK (generation >= 1),
  operation_id TEXT NOT NULL,
  claimed_ts INTEGER NOT NULL
) STRICT;
";

/// The act-claim migration: the one line it registers in `MIGRATIONS`.
pub(super) const CLAIM_MIGRATION: Migration = Migration {
    name: "m006_service_claims",
    apply: m006_service_claims,
};

fn m006_service_claims(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute_batch(SCHEMA_M006)?;
    Ok(())
}

/// How old (ms) a claim may grow before [`Store::service_claim`] supersedes it and
/// [`Store::service_stale_claims`] reports it. It bounds how long a serve that died mid-act
/// blocks its service. It must exceed one act: the busctl call and its settle share the
/// runner's 5 s probe timeout, and the commit waits at most the 5 s busy timeout.
/// `Budgets` has no service section; the proposed field is `service.claim_stale_ms`.
pub const CLAIM_STALE_MS: i64 = 60_000;

/// One `service_claims` row: who holds the act on a service, at which generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceClaim {
    /// The claimed service.
    pub service_id: String,
    /// The generation the holder acts on.
    pub generation: u64,
    /// The holder's derived operation id (`op-...`).
    pub operation_id: String,
    /// When the claim was taken (ms since the Unix epoch).
    pub claimed_ts: i64,
}

/// A claim no live act holds any more: older than [`CLAIM_STALE_MS`], or at a generation the
/// service has already left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleClaim {
    /// The claim as stored.
    pub claim: ServiceClaim,
    /// Its age when it was found (ms).
    pub age_ms: i64,
    /// Why it is stale.
    pub reason: StaleReason,
}

/// Why a claim is stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleReason {
    /// Older than [`CLAIM_STALE_MS`]: its holder died or hung mid-act.
    Expired,
    /// The service's generation is no longer the claim's: its act already committed.
    GenerationMoved,
}

impl StaleReason {
    /// The reason's stable spelling for a log line.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Expired => "expired",
            Self::GenerationMoved => "generation_moved",
        }
    }
}

/// A claim row read back, before its generation is checked.
type RawClaim = (String, i64, String, i64);

fn parse_claim(raw: RawClaim) -> Result<ServiceClaim, StoreError> {
    let (service_id, generation, operation_id, claimed_ts) = raw;
    let generation = u64::try_from(generation).map_err(|_| StoreError::Corrupt {
        task: service_id.clone(),
        detail: "service_claims.generation < 0".into(),
    })?;
    Ok(ServiceClaim {
        service_id,
        generation,
        operation_id,
        claimed_ts,
    })
}

fn claim_row(conn: &Connection, service_id: &str) -> Result<Option<ServiceClaim>, StoreError> {
    conn.query_row(
        "SELECT service_id, generation, operation_id, claimed_ts
         FROM service_claims WHERE service_id = ?1",
        [service_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .optional()?
    .map(parse_claim)
    .transpose()
}

/// Whether `claim` is stale at `now` against the service's `current` generation.
fn staleness(claim: ServiceClaim, current: u64, now: i64) -> Result<StaleClaim, ServiceClaim> {
    let age_ms = now.saturating_sub(claim.claimed_ts);
    let reason = if claim.generation != current {
        StaleReason::GenerationMoved
    } else if age_ms >= CLAIM_STALE_MS {
        StaleReason::Expired
    } else {
        return Err(claim);
    };
    Ok(StaleClaim {
        claim,
        age_ms,
        reason,
    })
}

fn generation_i64(service_id: &str, generation: u64) -> Result<i64, StoreError> {
    i64::try_from(generation).map_err(|_| StoreError::Corrupt {
        task: service_id.to_owned(),
        detail: "expected generation overflows i64".into(),
    })
}

/// One `service_facts` row, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFact {
    /// The service id (`self`, `model`, `drive`, ...).
    pub service_id: String,
    /// Who owns the service (the installer record's sense of `deploy`).
    pub owner_id: String,
    /// The systemd user unit the service runs as.
    pub unit_id: String,
    /// SHA-256 of `owner_id`: the CAS guard `service.action` names.
    pub owner_sha256: Sha256Hex,
    /// Bumped by every committed action; the precondition a caller reads from inspect.
    pub generation: u64,
    /// Whether `service.action` may act on this service (seeded; false for `self`, `model`).
    pub actable: bool,
    /// The last committed probe or action health, if any.
    pub cached_health: Option<Observation>,
    /// When the row last changed (ms since the Unix epoch).
    pub updated_ts: i64,
}

/// Why a service commit wrote nothing.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// The ledger refused (a key conflict, SQLite, JSON, an unreadable row).
    #[error(transparent)]
    Store(#[from] StoreError),
    /// No `service_facts` row carries this id.
    #[error("no service_facts row for {0}")]
    UnknownService(String),
    /// The caller's generation is not the row's; `current` is the row's now.
    #[error("stale generation: the service is at {current}")]
    StaleGeneration {
        /// The row's generation.
        current: u64,
    },
    /// The caller's expected owner digest is not the row's.
    #[error("owner digest mismatch")]
    OwnerMismatch,
    /// The row is seeded not actable (the engine itself, the model).
    #[error("service {0} is not actable")]
    NotActable(String),
    /// Another act holds the service's live claim (this or another serve on the ledger).
    #[error("service {} is claimed by {} at generation {}", .0.service_id, .0.operation_id, .0.generation)]
    ClaimHeld(ServiceClaim),
}

/// A row read back, before its digest and JSON are parsed.
type RawFact = (
    String,
    String,
    String,
    String,
    i64,
    bool,
    Option<String>,
    i64,
);

fn parse_fact(raw: RawFact) -> Result<ServiceFact, StoreError> {
    let (service_id, owner_id, unit_id, sha, generation, actable, health, updated_ts) = raw;
    let corrupt = |detail: String| StoreError::Corrupt {
        task: service_id.clone(),
        detail,
    };
    let owner_sha256 = sha
        .parse()
        .map_err(|e| corrupt(format!("service_facts.owner_sha256: {e}")))?;
    let generation =
        u64::try_from(generation).map_err(|_| corrupt("service_facts.generation < 0".into()))?;
    let cached_health = health
        .map(|j| serde_json::from_str::<Observation>(&j))
        .transpose()?;
    Ok(ServiceFact {
        service_id,
        owner_id,
        unit_id,
        owner_sha256,
        generation,
        actable,
        cached_health,
        updated_ts,
    })
}

fn fact_row(conn: &Connection, service_id: &str) -> Result<Option<ServiceFact>, StoreError> {
    conn.query_row(
        "SELECT service_id, owner_id, unit_id, owner_sha256, generation, actable,
                cached_health_json, updated_ts
         FROM service_facts WHERE service_id = ?1",
        [service_id],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        },
    )
    .optional()?
    .map(parse_fact)
    .transpose()
}

/// The compare-and-set guard of an action commit: what the caller read from `service.inspect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expected {
    /// The generation the caller acts on (the envelope precondition).
    pub generation: u64,
    /// The owner digest the caller names.
    pub owner_sha256: Sha256Hex,
}

/// One seed row of `service_facts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seed<'a> {
    /// The service id.
    pub service_id: &'a str,
    /// Who owns the service.
    pub owner_id: &'a str,
    /// The systemd user unit.
    pub unit_id: &'a str,
    /// Whether `service.action` may act on it.
    pub actable: bool,
}

impl Store {
    /// The `service_facts` row for `service_id`, if any.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unparsable digest or generation;
    /// [`StoreError::Json`] for an unparsable cached health.
    pub fn service_get(&self, service_id: &str) -> Result<Option<ServiceFact>, StoreError> {
        fact_row(&self.conn, service_id)
    }

    /// Insert every seed whose id is absent, at generation 1 with `owner_sha256 =
    /// sha256(owner_id)` and no cached health; a present row is left as it is. Returns how
    /// many rows were inserted (0 on a second call: idempotent).
    ///
    /// # Errors
    /// SQLite errors.
    pub fn service_seed(&self, seeds: &[Seed<'_>]) -> Result<usize, StoreError> {
        let tx = self.begin()?;
        let mut inserted = 0;
        for seed in seeds {
            inserted += tx.execute(
                "INSERT OR IGNORE INTO service_facts(service_id, owner_id, unit_id, owner_sha256,
                   generation, actable, cached_health_json, updated_ts)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, NULL, ?6)",
                params![
                    seed.service_id,
                    seed.owner_id,
                    seed.unit_id,
                    Sha256Hex::digest(seed.owner_id.as_bytes()).to_string(),
                    seed.actable,
                    now_ms()
                ],
            )?;
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// The stored operation for `op` when its request digest matches `request` (a replay, so
    /// the caller runs no probe); `None` when the key is new.
    ///
    /// # Errors
    /// [`StoreError::Conflict`] for the same key with other bytes; SQLite or parse errors.
    pub fn service_replay(
        &self,
        op: &OperationKey,
        request: &[u8],
    ) -> Result<Option<Operation>, StoreError> {
        let sha = Sha256Hex::digest(request).to_string();
        let stored = self
            .conn
            .query_row(
                "SELECT operation_id, action, idem_key, task_id, subject, result_json, ts,
                        request_sha256
                 FROM operations
                 WHERE principal = ?1 AND action = ?2 AND version = ?3 AND idem_key = ?4",
                params![op.principal, op.action, op.version, op.idem_key],
                RawOperation::from_row,
            )
            .optional()?;
        let Some(raw) = stored else {
            return Ok(None);
        };
        if raw.request_sha256 != sha {
            return Err(StoreError::Conflict(op.clone()));
        }
        let row = raw.parse()?;
        Ok(Some(Operation {
            operation_id: row.operation_id,
            task_id: row.task_id,
            subject: row.subject,
            replayed: true,
            result: row.result,
        }))
    }

    /// Commit a probe: `cached_health_json` and `updated_ts` of `service_id`, plus the
    /// operations row (subject = the service id, no task), in one transaction. `result` builds
    /// the stored reply body from the derived operation id. A replay returns the stored body
    /// with `replayed = true`.
    ///
    /// # Errors
    /// [`ServiceError::UnknownService`] for an unseeded id; [`StoreError::Conflict`] through
    /// [`ServiceError::Store`] for the same key with other bytes; SQLite or JSON errors.
    pub fn service_probe_commit(
        &self,
        op: &OperationKey,
        request: &[u8],
        service_id: &str,
        observation: &Observation,
        result: impl FnOnce(&str) -> Value,
    ) -> Result<Operation, ServiceError> {
        if self.service_replay(op, request)?.is_none() && self.service_get(service_id)?.is_none() {
            return Err(ServiceError::UnknownService(service_id.to_owned()));
        }
        let health = serde_json::to_string(observation).map_err(StoreError::Json)?;
        Ok(self.operate(op, request, |tx| {
            let changed = tx.execute(
                "UPDATE service_facts SET cached_health_json = ?2, updated_ts = ?3
                 WHERE service_id = ?1",
                params![service_id, health, now_ms()],
            )?;
            if changed != 1 {
                return Err(StoreError::Corrupt {
                    task: service_id.to_owned(),
                    detail: "service_facts row vanished inside the probe commit".into(),
                });
            }
            Ok((None, Some(service_id.to_owned()), result(&operation_id(op))))
        })?)
    }

    /// Take the act claim on `service_id` at `generation` for `op` (single writer: one
    /// `BEGIN IMMEDIATE`). The row must be at `generation`; a live claim refuses; a stale one
    /// (older than [`CLAIM_STALE_MS`], or at a generation the row has left) is superseded and
    /// returned so the caller reports it by name.
    ///
    /// # Errors
    /// [`ServiceError::ClaimHeld`] with the live claim (also for the same `op`: a concurrent
    /// same-key request never acts twice); [`ServiceError::StaleGeneration`];
    /// [`ServiceError::UnknownService`]; SQLite errors.
    pub fn service_claim(
        &self,
        service_id: &str,
        generation: u64,
        op: &OperationKey,
    ) -> Result<Option<StaleClaim>, ServiceError> {
        let at = generation_i64(service_id, generation)?;
        let tx = self.begin().map_err(StoreError::from)?;
        let fact = fact_row(&tx, service_id)?
            .ok_or_else(|| ServiceError::UnknownService(service_id.to_owned()))?;
        if fact.generation != generation {
            return Err(ServiceError::StaleGeneration {
                current: fact.generation,
            });
        }
        let now = now_ms();
        let superseded = match claim_row(&tx, service_id)? {
            None => None,
            Some(held) => {
                Some(staleness(held, fact.generation, now).map_err(ServiceError::ClaimHeld)?)
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO service_claims(service_id, generation, operation_id, claimed_ts)
             VALUES (?1, ?2, ?3, ?4)",
            params![service_id, at, operation_id(op), now],
        )
        .map_err(StoreError::from)?;
        tx.commit().map_err(StoreError::from)?;
        Ok(superseded)
    }

    /// Release `op`'s claim on `service_id`: deletes the row only while `op` holds it (a claim
    /// superseded by another act is left alone). Idempotent; returns whether a row went.
    ///
    /// # Errors
    /// SQLite errors.
    pub fn service_release(&self, service_id: &str, op: &OperationKey) -> Result<bool, StoreError> {
        let tx = self.begin()?;
        let gone = tx.execute(
            "DELETE FROM service_claims WHERE service_id = ?1 AND operation_id = ?2",
            params![service_id, operation_id(op)],
        )?;
        tx.commit()?;
        Ok(gone == 1)
    }

    /// The live claim on `service_id`, if any (stale or not).
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for a negative generation.
    pub fn service_claim_get(&self, service_id: &str) -> Result<Option<ServiceClaim>, StoreError> {
        claim_row(&self.conn, service_id)
    }

    /// Every stale claim in the ledger, by service id: the recovery readback a serve prints at
    /// start. A stale claim does not block: the next [`Store::service_claim`] supersedes it.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unreadable row.
    pub fn service_stale_claims(&self) -> Result<Vec<StaleClaim>, StoreError> {
        let rows = {
            let mut stmt = self.conn.prepare(
                "SELECT c.service_id, c.generation, c.operation_id, c.claimed_ts, f.generation
                 FROM service_claims c JOIN service_facts f USING (service_id)
                 ORDER BY c.service_id",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?),
                    r.get::<_, i64>(4)?,
                ))
            })?;
            rows.collect::<Result<Vec<(RawClaim, i64)>, _>>()?
        };
        let now = now_ms();
        let mut stale = Vec::new();
        for (raw, current) in rows {
            let claim = parse_claim(raw)?;
            let current = u64::try_from(current).map_err(|_| StoreError::Corrupt {
                task: claim.service_id.clone(),
                detail: "service_facts.generation < 0".into(),
            })?;
            if let Ok(found) = staleness(claim, current, now) {
                stale.push(found);
            }
        }
        Ok(stale)
    }

    /// Commit an action: the row must still be at `expected.generation` with
    /// `expected.owner_sha256` (compare-and-set), then `generation + 1`, `cached_health_json =
    /// health`, `updated_ts`, plus the operations row, in one transaction. A replay returns the
    /// stored body without any check. The read and the write run on this store's one connection
    /// under the caller's lock, and the `UPDATE` repeats the two guards, so a row that moved
    /// between them is refused, never overwritten.
    ///
    /// # Errors
    /// [`ServiceError::StaleGeneration`] (with the row's generation),
    /// [`ServiceError::OwnerMismatch`], [`ServiceError::UnknownService`];
    /// [`StoreError::Conflict`] through [`ServiceError::Store`]; SQLite or JSON errors.
    pub fn service_action_commit(
        &self,
        op: &OperationKey,
        request: &[u8],
        service_id: &str,
        expected: Expected,
        health: &Observation,
        result: impl FnOnce(&str) -> Value,
    ) -> Result<Operation, ServiceError> {
        if self.service_replay(op, request)?.is_none() {
            let fact = self
                .service_get(service_id)?
                .ok_or_else(|| ServiceError::UnknownService(service_id.to_owned()))?;
            if fact.generation != expected.generation {
                return Err(ServiceError::StaleGeneration {
                    current: fact.generation,
                });
            }
            if fact.owner_sha256 != expected.owner_sha256 {
                return Err(ServiceError::OwnerMismatch);
            }
            if !fact.actable {
                return Err(ServiceError::NotActable(fact.service_id));
            }
        }
        let health = serde_json::to_string(health).map_err(StoreError::Json)?;
        let generation = generation_i64(service_id, expected.generation)?;
        Ok(self.operate(op, request, |tx| {
            let changed = tx.execute(
                "UPDATE service_facts
                 SET generation = generation + 1, cached_health_json = ?2, updated_ts = ?3
                 WHERE service_id = ?1 AND generation = ?4 AND owner_sha256 = ?5 AND actable = 1",
                params![
                    service_id,
                    health,
                    now_ms(),
                    generation,
                    expected.owner_sha256.to_string()
                ],
            )?;
            if changed != 1 {
                return Err(StoreError::Corrupt {
                    task: service_id.to_owned(),
                    detail: "service_facts guards moved inside the action commit".into(),
                });
            }
            Ok((None, Some(service_id.to_owned()), result(&operation_id(op))))
        })?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hee4_contracts::{Evidence, Outcome, ToolId};
    use serde_json::json;

    type R = Result<(), Box<dyn std::error::Error>>;

    fn open(name: &str) -> Result<Store, Box<dyn std::error::Error>> {
        let dir =
            std::env::temp_dir().join(format!("hee4-core-service-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        Ok(Store::open(&dir.join("ledger.sqlite3"))?)
    }

    fn observation(value: &str) -> Result<Observation, Box<dyn std::error::Error>> {
        Ok(Observation {
            source: "service.probe".parse()?,
            input_sha256: Sha256Hex::digest(b"{}"),
            tool: ToolId {
                name: "busctl".parse()?,
                version: "systemd-261".parse()?,
            },
            head_sha: "a".repeat(40).parse()?,
            outcome: Outcome::Pass,
            evidence: vec![Evidence {
                label: "active_state".parse()?,
                sha256: Sha256Hex::digest(value.as_bytes()),
            }],
            advisory: false,
            elapsed_ms: 3,
            budget_ms: 5000,
        })
    }

    fn key(action: &str, idem: &str) -> OperationKey {
        OperationKey {
            principal: "uid:1000".into(),
            action: action.into(),
            version: 1,
            idem_key: idem.into(),
        }
    }

    const SEEDS: &[Seed<'static>] = &[
        Seed {
            service_id: "drive",
            owner_id: "deploy",
            unit_id: "hee4-drive.service",
            actable: true,
        },
        Seed {
            service_id: "self",
            owner_id: "deploy",
            unit_id: "hee4.service",
            actable: false,
        },
    ];

    #[test]
    fn service_seed_is_idempotent_and_digests_the_owner() -> R {
        let store = open("seed")?;
        assert_eq!(store.service_seed(SEEDS)?, 2);
        assert_eq!(store.service_seed(SEEDS)?, 0);
        let fact = store.service_get("drive")?.ok_or("no row")?;
        assert_eq!(fact.generation, 1);
        assert_eq!(fact.unit_id, "hee4-drive.service");
        assert_eq!(fact.owner_sha256, Sha256Hex::digest(b"deploy"));
        assert_eq!(fact.cached_health, None);
        assert!(fact.actable);
        assert_eq!(store.service_get("self")?.map(|f| f.actable), Some(false));
        assert_eq!(store.service_get("nope")?, None);
        Ok(())
    }

    #[test]
    fn service_probe_commit_then_replay_returns_the_same_operation() -> R {
        let store = open("probe")?;
        store.service_seed(SEEDS)?;
        let obs = observation("active")?;
        let k = key("service.probe", "p1");
        let first = store.service_probe_commit(
            &k,
            b"{\"a\":1}",
            "drive",
            &obs,
            |id| json!({"operation_id": id, "service_id": "drive"}),
        )?;
        assert!(!first.replayed);
        assert_eq!(first.result["operation_id"], first.operation_id);
        assert_eq!(first.subject.as_deref(), Some("drive"));
        assert_eq!(
            store.service_get("drive")?.and_then(|f| f.cached_health),
            Some(obs.clone())
        );
        let again = store.service_probe_commit(&k, b"{\"a\":1}", "drive", &obs, |_| json!(null))?;
        assert!(again.replayed);
        assert_eq!(again.operation_id, first.operation_id);
        assert_eq!(again.result, first.result);
        assert!(matches!(
            store.service_probe_commit(&k, b"{\"a\":2}", "drive", &obs, |_| json!(null)),
            Err(ServiceError::Store(StoreError::Conflict(_)))
        ));
        assert!(matches!(
            store.service_probe_commit(
                &key("service.probe", "p2"),
                b"{}",
                "nope",
                &obs,
                |_| json!(null)
            ),
            Err(ServiceError::UnknownService(_))
        ));
        Ok(())
    }

    #[test]
    fn service_action_commit_is_a_compare_and_set_on_generation_and_owner() -> R {
        let store = open("action")?;
        store.service_seed(SEEDS)?;
        let obs = observation("inactive")?;
        let owner = Sha256Hex::digest(b"deploy");
        let wrong = Sha256Hex::digest(b"other");
        let stale = store.service_action_commit(
            &key("service.action", "a0"),
            b"{}",
            "drive",
            Expected {
                generation: 2,
                owner_sha256: owner,
            },
            &obs,
            |_| json!(null),
        );
        assert!(matches!(
            stale,
            Err(ServiceError::StaleGeneration { current: 1 })
        ));
        let mismatch = store.service_action_commit(
            &key("service.action", "a0"),
            b"{}",
            "drive",
            Expected {
                generation: 1,
                owner_sha256: wrong,
            },
            &obs,
            |_| json!(null),
        );
        assert!(matches!(mismatch, Err(ServiceError::OwnerMismatch)));
        let held = store.service_action_commit(
            &key("service.action", "s0"),
            b"{}",
            "self",
            Expected {
                generation: 1,
                owner_sha256: owner,
            },
            &obs,
            |_| json!(null),
        );
        assert!(matches!(held, Err(ServiceError::NotActable(id)) if id == "self"));
        assert_eq!(store.service_get("self")?.map(|f| f.generation), Some(1));
        assert_eq!(store.service_get("drive")?.map(|f| f.generation), Some(1));
        let k = key("service.action", "a1");
        let done = store.service_action_commit(
            &k,
            b"{}",
            "drive",
            Expected {
                generation: 1,
                owner_sha256: owner,
            },
            &obs,
            |id| json!({"operation_id": id}),
        )?;
        assert!(!done.replayed);
        let fact = store.service_get("drive")?.ok_or("no row")?;
        assert_eq!(fact.generation, 2);
        assert_eq!(fact.cached_health, Some(obs.clone()));
        let replay = store.service_action_commit(
            &k,
            b"{}",
            "drive",
            Expected {
                generation: 1,
                owner_sha256: owner,
            },
            &obs,
            |_| json!(null),
        )?;
        assert!(replay.replayed);
        assert_eq!(replay.result, done.result);
        assert_eq!(store.service_get("drive")?.map(|f| f.generation), Some(2));
        Ok(())
    }

    fn drive_at(generation: u64) -> Expected {
        Expected {
            generation,
            owner_sha256: Sha256Hex::digest(b"deploy"),
        }
    }

    #[test]
    fn service_claim_on_a_second_handle_is_refused_while_the_first_holds_it() -> R {
        let first = open("claim-two")?;
        let second = Store::open(first.path())?;
        first.service_seed(SEEDS)?;
        let (k1, k2) = (key("service.action", "c1"), key("service.action", "c2"));
        assert_eq!(first.service_claim("drive", 1, &k1)?, None);
        for k in [&k2, &k1] {
            match second.service_claim("drive", 1, k) {
                Err(ServiceError::ClaimHeld(held)) => {
                    assert_eq!(held.operation_id, operation_id(&k1));
                    assert_eq!((held.service_id.as_str(), held.generation), ("drive", 1));
                }
                other => return Err(format!("expected ClaimHeld, got {other:?}").into()),
            }
        }
        assert!(matches!(
            second.service_claim("drive", 2, &k2),
            Err(ServiceError::StaleGeneration { current: 1 })
        ));
        assert!(matches!(
            second.service_claim("nope", 1, &k2),
            Err(ServiceError::UnknownService(_))
        ));
        assert!(
            !second.service_release("drive", &k2)?,
            "a non-holder releases nothing"
        );
        assert!(second.service_release("drive", &k1)?);
        assert!(
            !first.service_release("drive", &k1)?,
            "release is idempotent"
        );
        assert_eq!(second.service_claim("drive", 1, &k2)?, None);
        assert_eq!(
            first.service_claim_get("drive")?.map(|c| c.operation_id),
            Some(operation_id(&k2))
        );
        assert_eq!(first.service_stale_claims()?, Vec::new());
        Ok(())
    }

    #[test]
    fn a_stale_claim_is_reported_by_name_then_superseded() -> R {
        let dead = open("claim-stale")?;
        dead.service_seed(SEEDS)?;
        let (k1, k2) = (key("service.action", "s1"), key("service.action", "s2"));
        dead.service_claim("drive", 1, &k1)?;
        let restarted = Store::open(dead.path())?;
        assert_eq!(restarted.service_stale_claims()?, Vec::new());
        assert!(matches!(
            restarted.service_claim("drive", 1, &k2),
            Err(ServiceError::ClaimHeld(_))
        ));
        dead.conn.execute(
            "UPDATE service_claims SET claimed_ts = claimed_ts - ?1",
            [CLAIM_STALE_MS],
        )?;
        let found = restarted.service_stale_claims()?;
        let [only] = found.as_slice() else {
            return Err(format!("expected one stale claim, got {found:?}").into());
        };
        assert_eq!(only.claim.service_id, "drive");
        assert_eq!(only.claim.operation_id, operation_id(&k1));
        assert_eq!(only.reason, StaleReason::Expired);
        assert!(only.age_ms >= CLAIM_STALE_MS);
        let superseded = restarted
            .service_claim("drive", 1, &k2)?
            .ok_or("the stale claim was not reported")?;
        assert_eq!(superseded.claim, only.claim);
        assert_eq!(restarted.service_stale_claims()?, Vec::new());
        assert!(
            !dead.service_release("drive", &k1)?,
            "the superseded holder releases nothing"
        );
        assert_eq!(
            restarted
                .service_claim_get("drive")?
                .map(|c| c.operation_id),
            Some(operation_id(&k2))
        );
        Ok(())
    }

    #[test]
    fn a_claim_left_behind_a_committed_act_is_stale_by_generation() -> R {
        let store = open("claim-moved")?;
        store.service_seed(SEEDS)?;
        let k1 = key("service.action", "m1");
        store.service_claim("drive", 1, &k1)?;
        let obs = observation("inactive")?;
        store.service_action_commit(&k1, b"{}", "drive", drive_at(1), &obs, |_| json!(null))?;
        let found = store.service_stale_claims()?;
        assert_eq!(
            found
                .iter()
                .map(|s| (s.claim.service_id.as_str(), s.reason))
                .collect::<Vec<_>>(),
            vec![("drive", StaleReason::GenerationMoved)]
        );
        let k2 = key("service.action", "m2");
        let superseded = store
            .service_claim("drive", 2, &k2)?
            .ok_or("not reported")?;
        assert_eq!(superseded.reason, StaleReason::GenerationMoved);
        Ok(())
    }
}
