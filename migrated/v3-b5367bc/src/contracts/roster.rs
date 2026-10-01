//! Shared roster values. These are internal DTOs unless explicitly named V1.
//! Receiver timestamps are stamped by Store, never admitted from an import.

use super::{Generation, UuidV4, parse_u64_decimal};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

pub const MAX_RECORDS: usize = 256;
pub const MAX_HISTORY: usize = 4096;
pub const MAX_INPUT: usize = 1_048_576;
pub const MAX_PINS: usize = 16;
pub const MAX_TTL_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Invalid {
    Field,
    Bound,
    Identity,
    Precondition,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Agent,
    Model,
    Service,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Locality {
    Local,
    Remote,
    Hybrid,
}

fn required_nullable<'de, D, T>(decoder: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(decoder)
}

/// The exact RC03 closed definition; nullable endpoint remains required in JSON.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RosterDefinitionV1 {
    pub kind: Kind,
    pub display_name: String,
    pub owner_id: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub locality: Locality,
    #[serde(deserialize_with = "required_nullable")]
    pub endpoint_ref: Option<String>,
    pub limitations: String,
}

impl RosterDefinitionV1 {
    /// # Errors
    /// Rejects invalid RC03 byte bounds and references without normalization.
    pub fn validate(&self) -> Result<(), Invalid> {
        text(&self.display_name, 1, 256, false)?;
        text(&self.owner_id, 1, 128, true)?;
        text(&self.version, 1, 128, true)?;
        capabilities(&self.capabilities)?;
        optional_uuid(self.endpoint_ref.as_deref())?;
        text(&self.limitations, 0, 2048, false)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RosterHeadV1 {
    pub record_id: String,
    pub record_version: String,
    pub definition: RosterDefinitionV1,
    pub disabled: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub observation_cutoff_unix_ms: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Update {
    pub idempotency_key: String,
    pub record_id: Option<String>,
    pub expected_revision: Option<String>,
    pub definition: RosterDefinitionV1,
    pub audit_reason: String,
}

impl Update {
    /// # Errors
    /// Create requires both nullable fields absent; update requires both exact.
    pub fn validate(&self) -> Result<(), Invalid> {
        uuid(&self.idempotency_key)?;
        match (&self.record_id, &self.expected_revision) {
            (None, None) => {}
            (Some(id), Some(version)) => {
                uuid(id)?;
                generation(version)?;
            }
            _ => return Err(Invalid::Precondition),
        }
        self.definition.validate()?;
        text(&self.audit_reason, 1, 1024, false)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveAttemptPolicy {
    LeaveRunning,
    RequestCancel,
}

#[derive(Clone, Debug)]
pub struct Disable {
    pub idempotency_key: String,
    pub record_id: String,
    pub expected_revision: String,
    pub active_attempt_policy: ActiveAttemptPolicy,
    pub audit_reason: String,
}

impl Disable {
    /// # Errors
    /// Rejects malformed identity, precondition or audit text.
    pub fn validate(&self) -> Result<(), Invalid> {
        uuid(&self.idempotency_key)?;
        uuid(&self.record_id)?;
        generation(&self.expected_revision)?;
        text(&self.audit_reason, 1, 1024, false)
    }
}

/// Internal stored time; the boot epoch differs from the durable ledger epoch.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptTime {
    pub epoch: String,
    pub monotonic_ms: u64,
    pub unix_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    Worker,
    ServiceProbe,
    ProviderResponse,
    Requested,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Unavailable,
    Unknown,
}

/// A trusted worker/service owner supplies facts; Store supplies receipt time.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationInput {
    pub record_id: String,
    pub record_version: String,
    pub owner_id: String,
    pub endpoint_ref: Option<String>,
    pub instance_id: Option<String>,
    pub instance_generation: Option<String>,
    pub source: ObservationSource,
    pub observed_unix_ms: Option<u64>,
    pub availability: Availability,
    pub actual_identity: Option<String>,
    pub immutable_revision: Option<String>,
    pub capabilities: Vec<String>,
    pub evidence_ref: String,
}

impl ObservationInput {
    /// # Errors
    /// Refuses malformed references and incomplete instance binding.
    pub fn validate(&self) -> Result<(), Invalid> {
        uuid(&self.record_id)?;
        generation(&self.record_version)?;
        text(&self.owner_id, 1, 128, true)?;
        optional_uuid(self.endpoint_ref.as_deref())?;
        match (&self.instance_id, &self.instance_generation) {
            (None, None) => {}
            (Some(id), Some(version)) => {
                uuid(id)?;
                generation(version)?;
            }
            _ => return Err(Invalid::Identity),
        }
        for value in [&self.actual_identity, &self.immutable_revision]
            .into_iter()
            .flatten()
        {
            text(value, 1, 256, false)?;
        }
        if self.immutable_revision.is_some() && self.actual_identity.is_none() {
            return Err(Invalid::Identity);
        }
        capabilities(&self.capabilities)?;
        uuid(&self.evidence_ref)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub id: String,
    pub confirmed_source: Option<ObservationSource>,
    pub input: ObservationInput,
    pub received: ReceiptTime,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    Fresh,
    Missing,
    Expired,
    EpochMismatch,
    ClockRegression,
    FutureSource,
    UntrustedSource,
    WrongBinding,
}

/// The observation's age on the receiver's monotonic clock at `now`, or `None` when that clock
/// ran backwards. Meaningful only within one receiver epoch, which [`freshness`] checks first.
#[must_use]
pub fn age_ms(observation: &Observation, now: &ReceiptTime) -> Option<u64> {
    now.monotonic_ms
        .checked_sub(observation.received.monotonic_ms)
}

/// The capabilities `observation` evidences for `head`: those the definition declares **and**
/// the observation reports (RC03 keeps declared and observed capabilities separate), when the
/// observation is [`Freshness::Fresh`] under `ttl_ms`; `None` otherwise. A set, so a repeated
/// label counts once. The one rule for "declared and observed": [`Selection::permits`] reads it,
/// and route composition will when a task first requires a roster capability (B07 review G1).
#[must_use]
pub fn evidenced_capabilities<'a>(
    head: &'a RosterHeadV1,
    observation: Option<&'a Observation>,
    now: &ReceiptTime,
    ttl_ms: u64,
) -> Option<BTreeSet<&'a str>> {
    if freshness(head, observation, now, ttl_ms) != Freshness::Fresh {
        return None;
    }
    let observed = &observation?.input.capabilities;
    Some(
        head.definition
            .capabilities
            .iter()
            .filter(|capability| observed.contains(capability))
            .map(String::as_str)
            .collect(),
    )
}

/// Compare attributable facts; this function grants no worker or service control.
#[must_use]
pub fn freshness(
    head: &RosterHeadV1,
    observation: Option<&Observation>,
    now: &ReceiptTime,
    ttl_ms: u64,
) -> Freshness {
    let Some(observation) = observation else {
        return Freshness::Missing;
    };
    let input = &observation.input;
    if input.record_id != head.record_id
        || input.record_version != head.record_version
        || input.owner_id != head.definition.owner_id
        || input.endpoint_ref != head.definition.endpoint_ref
    {
        return Freshness::WrongBinding;
    }
    if input.source == ObservationSource::Requested
        || observation.confirmed_source != Some(input.source)
    {
        return Freshness::UntrustedSource;
    }
    if observation.received.epoch != now.epoch {
        return Freshness::EpochMismatch;
    }
    let Some(age) = age_ms(observation, now) else {
        return Freshness::ClockRegression;
    };
    if now.unix_ms < observation.received.unix_ms {
        return Freshness::ClockRegression;
    }
    if input
        .observed_unix_ms
        .is_some_and(|time| time > observation.received.unix_ms)
    {
        return Freshness::FutureSource;
    }
    if !(1..=MAX_TTL_MS).contains(&ttl_ms) || age >= ttl_ms {
        return Freshness::Expired;
    }
    Freshness::Fresh
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub record_id: String,
    pub expected_revision: String,
    pub capabilities: Vec<String>,
    pub local_only: bool,
    pub version: Option<String>,
    pub ttl_ms: u64,
}

impl Selection {
    /// # Errors
    /// Refuses malformed selection constraints or unbounded freshness policy.
    pub fn validate(&self) -> Result<(), Invalid> {
        uuid(&self.record_id)?;
        generation(&self.expected_revision)?;
        capabilities(&self.capabilities)?;
        if let Some(version) = &self.version {
            text(version, 1, 128, true)?;
        }
        if !(1..=MAX_TTL_MS).contains(&self.ttl_ms) {
            return Err(Invalid::Bound);
        }
        Ok(())
    }

    #[must_use]
    pub fn permits(
        &self,
        head: &RosterHeadV1,
        observation: Option<&Observation>,
        now: &ReceiptTime,
    ) -> bool {
        self.validate().is_ok()
            && !head.disabled
            && head.record_id == self.record_id
            && head.record_version == self.expected_revision
            && (!self.local_only || head.definition.locality == Locality::Local)
            && self
                .version
                .as_ref()
                .is_none_or(|version| version == &head.definition.version)
            && observation
                .is_some_and(|observed| observed.input.availability == Availability::Available)
            && evidenced_capabilities(head, observation, now, self.ttl_ms).is_some_and(
                |evidenced| {
                    self.capabilities
                        .iter()
                        .all(|cap| evidenced.contains(cap.as_str()))
                },
            )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub head: RosterHeadV1,
    pub observation: Option<Observation>,
}

/// One captured ledger view; editing a returned value cannot mutate the Store.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub epoch: String,
    pub cutoff: u64,
    pub now: ReceiptTime,
    pub records: Vec<Record>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceState {
    Starting,
    Ready,
    Busy,
    Waiting,
    Stopping,
    Exited,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    pub id: String,
    pub generation: String,
    pub revision: String,
    pub agent_record_id: String,
    pub agent_record_version: String,
    pub task_id: String,
    pub attempt_id: String,
    pub attempt_generation: String,
    pub session_id: String,
    pub workspace_ref: String,
    pub started: ReceiptTime,
    pub lease_expires_monotonic_ms: u64,
    pub state: InstanceState,
    pub usage_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub attempt_id: String,
    pub selection: Selection,
    pub record: Record,
    pub selected_at: ReceiptTime,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CancellationCause {
    pub disable_event: String,
    pub record_id: String,
    pub task_id: String,
    pub attempt_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub head: RosterHeadV1,
    pub event_id: String,
    pub epoch: String,
    pub sequence: u64,
    pub active_attempts: Vec<String>,
    pub cancellation_causes: Vec<CancellationCause>,
}

/// # Errors
/// Rejects a field outside its exact byte and ASCII contract.
pub fn text(value: &str, minimum: usize, maximum: usize, ascii: bool) -> Result<(), Invalid> {
    if value.len() < minimum || value.len() > maximum {
        return Err(Invalid::Bound);
    }
    if ascii && !value.is_ascii() {
        return Err(Invalid::Field);
    }
    Ok(())
}

/// # Errors
/// Rejects invalid lowercase nonnil `UUIDv4` syntax.
pub fn uuid(value: &str) -> Result<(), Invalid> {
    UuidV4::parse(value)
        .map(|_| ())
        .map_err(|_| Invalid::Identity)
}

fn optional_uuid(value: Option<&str>) -> Result<(), Invalid> {
    value.map(uuid).transpose().map(|_| ())
}

/// # Errors
/// Rejects noncanonical, zero or overflowing generation strings.
pub fn generation(value: &str) -> Result<(), Invalid> {
    value
        .parse::<Generation>()
        .map(|_| ())
        .map_err(|_| Invalid::Precondition)
}

/// # Errors
/// Rejects noncanonical or overflowing unsigned decimal text.
pub fn decimal(value: &str) -> Result<(), Invalid> {
    parse_u64_decimal(value)
        .map(|_| ())
        .map_err(|_| Invalid::Field)
}

/// # Errors
/// Rejects overlong lists/labels; repeated RC03 labels remain permitted.
pub fn capabilities(values: &[String]) -> Result<(), Invalid> {
    if values.len() > 128 {
        return Err(Invalid::Bound);
    }
    for value in values {
        text(value, 1, 128, true)?;
    }
    Ok(())
}
