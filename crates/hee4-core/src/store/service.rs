//! The service family's store half (K1): the `service_facts` migration, its reader, the seed,
//! and the two `Store::operate` commits. Every byte of SQL for `service_facts` lives here (the
//! one-door rule, `FLOW.md`); the runner (K5) never sees this file, and the actions (K6) commit
//! through these verbs and never hold a `Transaction`.

use hee4_contracts::{Observation, Sha256Hex};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;

use super::migrations::Migration;
use super::{Operation, OperationKey, RawOperation, Store, StoreError, now_ms, operation_id};

/// `m005_service_facts`: one row per managed service, keyed by its service id. `generation`
/// starts at 1 and moves only through [`Store::service_action_commit`]; `owner_sha256` is the
/// digest of `owner_id`, the CAS guard an action names.
const SCHEMA_M005: &str = "
CREATE TABLE service_facts(
  service_id TEXT PRIMARY KEY NOT NULL,
  owner_id TEXT NOT NULL,
  unit_id TEXT NOT NULL,
  owner_sha256 TEXT NOT NULL CHECK (length(owner_sha256) = 64),
  generation INTEGER NOT NULL CHECK (generation >= 1),
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
}

/// A row read back, before its digest and JSON are parsed.
type RawFact = (String, String, String, String, i64, Option<String>, i64);

fn parse_fact(raw: RawFact) -> Result<ServiceFact, StoreError> {
    let (service_id, owner_id, unit_id, sha, generation, health, updated_ts) = raw;
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
        cached_health,
        updated_ts,
    })
}

fn fact_row(conn: &Connection, service_id: &str) -> Result<Option<ServiceFact>, StoreError> {
    conn.query_row(
        "SELECT service_id, owner_id, unit_id, owner_sha256, generation, cached_health_json,
                updated_ts
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

/// A `(service_id, owner_id, unit_id)` seed row.
pub type Seed<'a> = (&'a str, &'a str, &'a str);

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
        for (service_id, owner_id, unit_id) in seeds {
            inserted += tx.execute(
                "INSERT OR IGNORE INTO service_facts(service_id, owner_id, unit_id, owner_sha256,
                   generation, cached_health_json, updated_ts)
                 VALUES (?1, ?2, ?3, ?4, 1, NULL, ?5)",
                params![
                    service_id,
                    owner_id,
                    unit_id,
                    Sha256Hex::digest(owner_id.as_bytes()).to_string(),
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
        }
        let health = serde_json::to_string(health).map_err(StoreError::Json)?;
        let generation = i64::try_from(expected.generation).map_err(|_| StoreError::Corrupt {
            task: service_id.to_owned(),
            detail: "expected generation overflows i64".into(),
        })?;
        Ok(self.operate(op, request, |tx| {
            let changed = tx.execute(
                "UPDATE service_facts
                 SET generation = generation + 1, cached_health_json = ?2, updated_ts = ?3
                 WHERE service_id = ?1 AND generation = ?4 AND owner_sha256 = ?5",
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

    const SEEDS: &[Seed<'static>] = &[("drive", "deploy", "hee4-drive.service")];

    #[test]
    fn service_seed_is_idempotent_and_digests_the_owner() -> R {
        let store = open("seed")?;
        assert_eq!(store.service_seed(SEEDS)?, 1);
        assert_eq!(store.service_seed(SEEDS)?, 0);
        let fact = store.service_get("drive")?.ok_or("no row")?;
        assert_eq!(fact.generation, 1);
        assert_eq!(fact.unit_id, "hee4-drive.service");
        assert_eq!(fact.owner_sha256, Sha256Hex::digest(b"deploy"));
        assert_eq!(fact.cached_health, None);
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
}
