//! Budgets: every tunable the runtime reads, as one typed value with a cited default and exactly
//! one ceiling per field. No IO here: [`Budgets::parse`] takes the file's text.
//!
//! Host and worker depend on this crate only, so the value lives in K0; K1's `budget` is the
//! charge door (accounting), a different concern. Wave 2 replaces the host's local `DoorBudget`
//! with [`DoorBudget`]; wave 3 loads the file once at serve and hands the validated value down.
//!
//! One table, [`FIELDS`], holds every field's dotted name, getter, floor and ceiling; `validate`
//! and `render` both walk it, so a field cannot be checked under one name and printed under
//! another.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Ceiling for `socket.max_connections`.
pub const MAX_CONNECTIONS_CEILING: u64 = 65_535;
/// Ceiling for `socket.frame_bytes`: 64 MiB.
pub const MAX_FRAME_CEILING: u64 = 67_108_864;
/// Floor for `socket.frame_bytes`: a frame must hold a refusal.
pub const MIN_FRAME_BYTES: u64 = 1024;
/// Ceiling for every `*_ms` field: one day.
pub const MAX_DEADLINE_MS: u64 = 86_400_000;
/// Ceiling for `door.pool`.
pub const MAX_POOL: u64 = 1_024;
/// Ceiling for `stream.queue_frames` and `stream.batch_rows`.
pub const MAX_QUEUE_FRAMES: u64 = 65_536;
/// Ceiling for every byte-sized field (`door.max_body_bytes`, `door.max_total_bytes`,
/// `door.max_header_bytes`, `recovery.workspace_readback_bytes`): 16 GiB.
pub const MAX_BYTES_CEILING: u64 = 17_179_869_184;
/// Ceiling for every plain count (`door.max_requests`, `recovery.open_attempt_limit`,
/// `ledger.checkpoint_every`).
pub const MAX_COUNT_CEILING: u64 = 1_000_000;
/// Ceiling for `attempt.ctx_tokens`.
pub const MAX_CTX_TOKENS: u64 = 1_048_576;

/// The engine's Unix socket: connection count, frame size and the per-stream deadlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SocketBudget {
    /// Concurrent client connections (replaces `MAX_CONNECTIONS = 256`, hee4-app/src/socket.rs:115).
    pub max_connections: u64,
    /// Largest wire frame, bytes (replaces `MAX_FRAME_BYTES = 1_048_576`, hee4-app/src/wire.rs:10).
    pub frame_bytes: u64,
    /// Read deadline on a client stream (replaces `from_secs(60)`, hee4-app/src/socket.rs:166).
    pub read_deadline_ms: u64,
    /// Write deadline on a client stream (replaces `from_secs(10)`, hee4-app/src/socket.rs:167).
    pub write_deadline_ms: u64,
    /// Write deadline for an over-capacity refusal (replaces `from_millis(200)`,
    /// hee4-app/src/socket.rs:137).
    pub refusal_write_ms: u64,
    /// Read deadline the client side applies (replaces `from_secs(30)`, hee4-app/src/socket.rs:204).
    pub client_read_ms: u64,
}

impl SocketBudget {
    /// The literals the socket code carries today.
    pub const DEFAULT: Self = Self {
        max_connections: 256,
        frame_bytes: 1_048_576,
        read_deadline_ms: 60_000,
        write_deadline_ms: 10_000,
        refusal_write_ms: 200,
        client_read_ms: 30_000,
    };

    /// `read_deadline_ms` as a [`Duration`].
    #[must_use]
    pub const fn read_deadline(&self) -> Duration {
        Duration::from_millis(self.read_deadline_ms)
    }

    /// `write_deadline_ms` as a [`Duration`].
    #[must_use]
    pub const fn write_deadline(&self) -> Duration {
        Duration::from_millis(self.write_deadline_ms)
    }

    /// `refusal_write_ms` as a [`Duration`].
    #[must_use]
    pub const fn refusal_write(&self) -> Duration {
        Duration::from_millis(self.refusal_write_ms)
    }

    /// `client_read_ms` as a [`Duration`].
    #[must_use]
    pub const fn client_read(&self) -> Duration {
        Duration::from_millis(self.client_read_ms)
    }
}

impl Default for SocketBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The receipt stream: queue depth, batch size, poll and close deadlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct StreamBudget {
    /// Frames queued per subscriber (replaces `QUEUE_FRAMES = 256`, hee4-app/src/stream.rs:27).
    pub queue_frames: u64,
    /// Rows read from the ledger per poll (replaces `BATCH = 256`, hee4-app/src/stream.rs:30).
    pub batch_rows: u64,
    /// Poll interval (replaces `POLL = from_millis(100)`, hee4-app/src/stream.rs:32).
    pub poll_ms: u64,
    /// Time to flush on close (replaces `CLOSE_DEADLINE = from_millis(200)`,
    /// hee4-app/src/stream.rs:148).
    pub close_deadline_ms: u64,
}

impl StreamBudget {
    /// The literals the stream code carries today.
    pub const DEFAULT: Self = Self {
        queue_frames: 256,
        batch_rows: 256,
        poll_ms: 100,
        close_deadline_ms: 200,
    };

    /// `poll_ms` as a [`Duration`].
    #[must_use]
    pub const fn poll(&self) -> Duration {
        Duration::from_millis(self.poll_ms)
    }

    /// `close_deadline_ms` as a [`Duration`].
    #[must_use]
    pub const fn close_deadline(&self) -> Duration {
        Duration::from_millis(self.close_deadline_ms)
    }
}

impl Default for StreamBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The model door the host opens for an attempt: request count, sizes, deadlines and pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DoorBudget {
    /// Requests forwarded before the door answers `429` (replaces `64`,
    /// hee4-host/src/model_door.rs:90).
    pub max_requests: u64,
    /// Largest request body, bytes; a larger one is `413` (replaces `1024 * 1024`,
    /// hee4-host/src/model_door.rs:91).
    pub max_body_bytes: u64,
    /// Sum of request bodies over the attempt, bytes (replaces `8 * 1024 * 1024`,
    /// hee4-host/src/model_door.rs:92).
    pub max_total_bytes: u64,
    /// Time a client has to send its header block (replaces `from_secs(2)`,
    /// hee4-host/src/model_door.rs:93).
    pub header_deadline_ms: u64,
    /// Time a client has to send its body after the header (replaces `from_secs(10)`,
    /// hee4-host/src/model_door.rs:94).
    pub body_deadline_ms: u64,
    /// Read, write and connect timeout on each side (replaces `from_secs(120)`,
    /// hee4-host/src/model_door.rs:95).
    pub io_timeout_ms: u64,
    /// Largest header block, bytes (replaces `MAX_HEADER = 64 * 1024`,
    /// hee4-host/src/model_door.rs:25).
    pub max_header_bytes: u64,
    /// Door worker threads (replaces `POOL = 8`, hee4-host/src/model_door.rs:219).
    pub pool: u64,
}

impl DoorBudget {
    /// The host's `DoorBudget::DEFAULT` plus its header and pool constants; the host's comment
    /// already calls them UNMEASURED stand-ins.
    pub const DEFAULT: Self = Self {
        max_requests: 64,
        max_body_bytes: 1_048_576,
        max_total_bytes: 8_388_608,
        header_deadline_ms: 2_000,
        body_deadline_ms: 10_000,
        io_timeout_ms: 120_000,
        max_header_bytes: 65_536,
        pool: 8,
    };

    /// `header_deadline_ms` as a [`Duration`].
    #[must_use]
    pub const fn header_deadline(&self) -> Duration {
        Duration::from_millis(self.header_deadline_ms)
    }

    /// `body_deadline_ms` as a [`Duration`].
    #[must_use]
    pub const fn body_deadline(&self) -> Duration {
        Duration::from_millis(self.body_deadline_ms)
    }

    /// `io_timeout_ms` as a [`Duration`].
    #[must_use]
    pub const fn io_timeout(&self) -> Duration {
        Duration::from_millis(self.io_timeout_ms)
    }
}

impl Default for DoorBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One attempt: its default timebox, hard deadline and model context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AttemptBudget {
    /// TIMEBOX when the brief gives none (replaces the `from_secs(120)` fallback of `timebox`,
    /// hee4-app/src/dispatcher.rs:106-118).
    pub timebox_default_ms: u64,
    /// Hard deadline on an attempt: the unit's drain figure of 1,200 s
    /// (gates/features/crash-restart.md:95). UNMEASURED stand-in.
    pub deadline_ms: u64,
    /// Model context window offered to the attempt (replaces `ctx_tokens: 32_768`,
    /// hee4-app/src/dispatcher.rs:178).
    pub ctx_tokens: u64,
}

impl AttemptBudget {
    /// The dispatcher's literals and the drain figure.
    pub const DEFAULT: Self = Self {
        timebox_default_ms: 120_000,
        deadline_ms: 1_200_000,
        ctx_tokens: 32_768,
    };

    /// `timebox_default_ms` as a [`Duration`].
    #[must_use]
    pub const fn timebox_default(&self) -> Duration {
        Duration::from_millis(self.timebox_default_ms)
    }

    /// `deadline_ms` as a [`Duration`].
    #[must_use]
    pub const fn deadline(&self) -> Duration {
        Duration::from_millis(self.deadline_ms)
    }
}

impl Default for AttemptBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The dispatch loop: how long it idles with nothing to do and after an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DispatcherBudget {
    /// Sleep when no task is admitted (replaces `from_millis(100)`, hee4-app/src/lib.rs:140).
    pub idle_ms: u64,
    /// Sleep after a dispatch error (replaces `from_secs(1)`, hee4-app/src/lib.rs:143).
    pub error_backoff_ms: u64,
}

impl DispatcherBudget {
    /// The dispatch loop's literals.
    pub const DEFAULT: Self = Self {
        idle_ms: 100,
        error_backoff_ms: 1_000,
    };

    /// `idle_ms` as a [`Duration`].
    #[must_use]
    pub const fn idle(&self) -> Duration {
        Duration::from_millis(self.idle_ms)
    }

    /// `error_backoff_ms` as a [`Duration`].
    #[must_use]
    pub const fn error_backoff(&self) -> Duration {
        Duration::from_millis(self.error_backoff_ms)
    }
}

impl Default for DispatcherBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Crash recovery: how much workspace it reads back and how many open attempts it will adopt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RecoveryBudget {
    /// Bytes of workspace read back on restart: 1 GiB (`WORKSPACE_REMOVAL_BUDGET` family,
    /// gates/features/crash-restart.md:122). UNMEASURED stand-in.
    pub workspace_readback_bytes: u64,
    /// Open attempts recovery will adopt (`OPEN_ATTEMPT_LIMIT`,
    /// gates/features/crash-restart.md:122). UNMEASURED stand-in.
    pub open_attempt_limit: u64,
}

impl RecoveryBudget {
    /// The crash-restart feature file's named budgets, both UNMEASURED.
    pub const DEFAULT: Self = Self {
        workspace_readback_bytes: 1_073_741_824,
        open_attempt_limit: 64,
    };
}

impl Default for RecoveryBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The model host: how long a tags probe may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ModelBudget {
    /// Timeout on the `/api/tags` probe (replaces `from_secs(10)`, hee4-host/src/model.rs:52).
    pub tags_timeout_ms: u64,
}

impl ModelBudget {
    /// The host's literal.
    pub const DEFAULT: Self = Self {
        tags_timeout_ms: 10_000,
    };

    /// `tags_timeout_ms` as a [`Duration`].
    #[must_use]
    pub const fn tags_timeout(&self) -> Duration {
        Duration::from_millis(self.tags_timeout_ms)
    }
}

impl Default for ModelBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The ledger: its busy timeout and checkpoint cadence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LedgerBudget {
    /// `SQLite` busy timeout (replaces `busy_timeout(from_secs(5))`, hee4-core/src/store.rs:310;
    /// the follow-up is `Store::open_with(path, &LedgerBudget)`).
    pub busy_timeout_ms: u64,
    /// Receipts between WAL checkpoints (read by the wave-2 `Store::checkpoint_if_due(every)`).
    /// UNMEASURED stand-in.
    pub checkpoint_every: u64,
}

impl LedgerBudget {
    /// The store's literal and the checkpoint stand-in.
    pub const DEFAULT: Self = Self {
        busy_timeout_ms: 5_000,
        checkpoint_every: 64,
    };

    /// `busy_timeout_ms` as a [`Duration`].
    #[must_use]
    pub const fn busy_timeout(&self) -> Duration {
        Duration::from_millis(self.busy_timeout_ms)
    }
}

impl Default for LedgerBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Every budget the runtime reads. A value of this type passed [`Budgets::validate`] or is
/// [`Budgets::DEFAULT`]; a partial file overrides only the keys it names; an unknown key is a
/// parse failure, never a silent default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Budgets {
    /// The engine socket.
    pub socket: SocketBudget,
    /// The receipt stream.
    pub stream: StreamBudget,
    /// The model door.
    pub door: DoorBudget,
    /// One attempt.
    pub attempt: AttemptBudget,
    /// The dispatch loop.
    pub dispatcher: DispatcherBudget,
    /// Crash recovery.
    pub recovery: RecoveryBudget,
    /// The model host.
    pub model: ModelBudget,
    /// The ledger.
    pub ledger: LedgerBudget,
}

impl Default for Budgets {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A budget field out of bounds. Field names are the dotted declaration spelling, e.g.
/// `door.max_body_bytes`; callers match on the variant, the text is for humans.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum BudgetRefusal {
    /// A field is zero; every budget is at least one.
    #[error("budgets: {field} is zero")]
    Zero {
        /// The dotted field name.
        field: &'static str,
    },
    /// A field is above its ceiling (the `*_CEILING`, `MAX_*` consts of this module).
    #[error("budgets: {field} is {value}, over its ceiling {max}")]
    OverCeiling {
        /// The dotted field name.
        field: &'static str,
        /// The value found.
        value: u64,
        /// The ceiling.
        max: u64,
    },
    /// A field is below its floor (`socket.frame_bytes` must hold a refusal:
    /// `MIN_FRAME_BYTES`).
    #[error("budgets: {field} is {value}, below its floor {min}")]
    BelowFloor {
        /// The dotted field name.
        field: &'static str,
        /// The value found.
        value: u64,
        /// The floor.
        min: u64,
    },
    /// Two fields are out of order: `lesser` must not exceed `greater`.
    #[error("budgets: {lesser} must not exceed {greater}")]
    Order {
        /// The field that must be the smaller.
        lesser: &'static str,
        /// The field that must be the larger.
        greater: &'static str,
    },
}

/// Why [`Budgets::parse`] refused a text: the JSON did not fit the shape (serde's text, for
/// humans), or it fit and a field was out of bounds (typed, for callers).
#[derive(Debug, thiserror::Error)]
pub enum BudgetParseError {
    /// Not the JSON shape: malformed text, a wrong type, or an unknown (misspelled) key.
    #[error("budgets: {0}")]
    Json(#[from] serde_json::Error),
    /// The shape parsed; a field failed [`Budgets::validate`].
    #[error("{0}")]
    Refused(#[from] BudgetRefusal),
}

/// One row of the field table: the dotted name, where the value lives, its floor and ceiling.
struct Field {
    name: &'static str,
    get: fn(&Budgets) -> u64,
    floor: u64,
    max: u64,
}

/// Every field in declaration order; the only place a name, floor or ceiling is bound to a field.
const FIELDS: [Field; 28] = [
    Field {
        name: "socket.max_connections",
        get: |b| b.socket.max_connections,
        floor: 1,
        max: MAX_CONNECTIONS_CEILING,
    },
    Field {
        name: "socket.frame_bytes",
        get: |b| b.socket.frame_bytes,
        floor: MIN_FRAME_BYTES,
        max: MAX_FRAME_CEILING,
    },
    Field {
        name: "socket.read_deadline_ms",
        get: |b| b.socket.read_deadline_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "socket.write_deadline_ms",
        get: |b| b.socket.write_deadline_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "socket.refusal_write_ms",
        get: |b| b.socket.refusal_write_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "socket.client_read_ms",
        get: |b| b.socket.client_read_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "stream.queue_frames",
        get: |b| b.stream.queue_frames,
        floor: 1,
        max: MAX_QUEUE_FRAMES,
    },
    Field {
        name: "stream.batch_rows",
        get: |b| b.stream.batch_rows,
        floor: 1,
        max: MAX_QUEUE_FRAMES,
    },
    Field {
        name: "stream.poll_ms",
        get: |b| b.stream.poll_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "stream.close_deadline_ms",
        get: |b| b.stream.close_deadline_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "door.max_requests",
        get: |b| b.door.max_requests,
        floor: 1,
        max: MAX_COUNT_CEILING,
    },
    Field {
        name: "door.max_body_bytes",
        get: |b| b.door.max_body_bytes,
        floor: 1,
        max: MAX_BYTES_CEILING,
    },
    Field {
        name: "door.max_total_bytes",
        get: |b| b.door.max_total_bytes,
        floor: 1,
        max: MAX_BYTES_CEILING,
    },
    Field {
        name: "door.header_deadline_ms",
        get: |b| b.door.header_deadline_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "door.body_deadline_ms",
        get: |b| b.door.body_deadline_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "door.io_timeout_ms",
        get: |b| b.door.io_timeout_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "door.max_header_bytes",
        get: |b| b.door.max_header_bytes,
        floor: 1,
        max: MAX_BYTES_CEILING,
    },
    Field {
        name: "door.pool",
        get: |b| b.door.pool,
        floor: 1,
        max: MAX_POOL,
    },
    Field {
        name: "attempt.timebox_default_ms",
        get: |b| b.attempt.timebox_default_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "attempt.deadline_ms",
        get: |b| b.attempt.deadline_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "attempt.ctx_tokens",
        get: |b| b.attempt.ctx_tokens,
        floor: 1,
        max: MAX_CTX_TOKENS,
    },
    Field {
        name: "dispatcher.idle_ms",
        get: |b| b.dispatcher.idle_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "dispatcher.error_backoff_ms",
        get: |b| b.dispatcher.error_backoff_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "recovery.workspace_readback_bytes",
        get: |b| b.recovery.workspace_readback_bytes,
        floor: 1,
        max: MAX_BYTES_CEILING,
    },
    Field {
        name: "recovery.open_attempt_limit",
        get: |b| b.recovery.open_attempt_limit,
        floor: 1,
        max: MAX_COUNT_CEILING,
    },
    Field {
        name: "model.tags_timeout_ms",
        get: |b| b.model.tags_timeout_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "ledger.busy_timeout_ms",
        get: |b| b.ledger.busy_timeout_ms,
        floor: 1,
        max: MAX_DEADLINE_MS,
    },
    Field {
        name: "ledger.checkpoint_every",
        get: |b| b.ledger.checkpoint_every,
        floor: 1,
        max: MAX_COUNT_CEILING,
    },
];

/// Order rules, `(lesser, greater)` by field name: a body fits in the attempt's total; the
/// stream can close inside the socket's write deadline.
const ORDER: [(&str, &str); 2] = [
    ("door.max_body_bytes", "door.max_total_bytes"),
    ("stream.close_deadline_ms", "socket.write_deadline_ms"),
];

fn field(name: &str) -> Option<&'static Field> {
    FIELDS.iter().find(|f| f.name == name)
}

impl Budgets {
    /// Every sub-budget's `DEFAULT`: the literals the code carries today, each cited on its field.
    pub const DEFAULT: Self = Self {
        socket: SocketBudget::DEFAULT,
        stream: StreamBudget::DEFAULT,
        door: DoorBudget::DEFAULT,
        attempt: AttemptBudget::DEFAULT,
        dispatcher: DispatcherBudget::DEFAULT,
        recovery: RecoveryBudget::DEFAULT,
        model: ModelBudget::DEFAULT,
        ledger: LedgerBudget::DEFAULT,
    };

    /// Parse the budgets file's text (JSON; `{}` is [`Budgets::DEFAULT`]; a partial object
    /// overrides only the keys it names) and validate the result.
    ///
    /// # Errors
    /// [`BudgetParseError::Json`] when the text is not the shape, including an unknown key;
    /// [`BudgetParseError::Refused`] with the first [`BudgetRefusal`] in field order.
    pub fn parse(json: &str) -> Result<Self, BudgetParseError> {
        let budgets: Self = serde_json::from_str(json)?;
        budgets.validate()?;
        Ok(budgets)
    }

    /// Check every field against the table: non-zero, at or above its floor, at or under its
    /// ceiling, then the order rules.
    ///
    /// # Errors
    /// The first [`BudgetRefusal`] in field order, naming the field.
    pub fn validate(&self) -> Result<(), BudgetRefusal> {
        for f in &FIELDS {
            let value = (f.get)(self);
            if value == 0 {
                return Err(BudgetRefusal::Zero { field: f.name });
            }
            if value < f.floor {
                return Err(BudgetRefusal::BelowFloor {
                    field: f.name,
                    value,
                    min: f.floor,
                });
            }
            if value > f.max {
                return Err(BudgetRefusal::OverCeiling {
                    field: f.name,
                    value,
                    max: f.max,
                });
            }
        }
        for (lesser, greater) in ORDER {
            if let (Some(l), Some(g)) = (field(lesser), field(greater))
                && (l.get)(self) > (g.get)(self)
            {
                return Err(BudgetRefusal::Order { lesser, greater });
            }
        }
        Ok(())
    }

    /// One line, `name=value` per field in declaration order, space separated: the doctor and
    /// health line.
    #[must_use]
    pub fn render(&self) -> String {
        FIELDS
            .iter()
            .map(|f| format!("{}={}", f.name, (f.get)(self)))
            .collect::<Vec<_>>()
            .join(" ")
    }
}
