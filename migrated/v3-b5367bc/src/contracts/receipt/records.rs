//! Explicit frozen record fields; local cross-field checks live in invariants.rs.
use super::primitives::{
    Bool, Count, Generation, Id, List, MapOnly, Maybe, Name, Payload, Reason, Ref, RelPath, Sha,
    Text, TypedRef, U64,
};
use super::{Address, Error, ReceiptRecord, Validate, sealed};
use serde::{Deserialize, Deserializer, Serialize};
#[path = "invariants.rs"]
mod invariants;
use invariants::LocalCheck;

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ObservationsV1Cancellation {
    #[serde(rename = "not_requested")]
    NotRequested,
    #[serde(rename = "requested")]
    Requested,
    #[serde(rename = "unknown")]
    Unknown,
}
impl Validate for ObservationsV1Cancellation {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ObservationsV1Cancellation {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "not_requested" => Ok(Self::NotRequested),
            "requested" => Ok(Self::Requested),
            "unknown" => Ok(Self::Unknown),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ObservationsV1Cleanup {
    #[serde(rename = "not_started")]
    NotStarted,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "settled")]
    Settled,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "unknown")]
    Unknown,
}
impl Validate for ObservationsV1Cleanup {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ObservationsV1Cleanup {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "not_started" => Ok(Self::NotStarted),
            "pending" => Ok(Self::Pending),
            "settled" => Ok(Self::Settled),
            "failed" => Ok(Self::Failed),
            "unknown" => Ok(Self::Unknown),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ProducerV1Status {
    #[serde(rename = "exited")]
    Exited,
    #[serde(rename = "signalled")]
    Signalled,
    #[serde(rename = "not_started")]
    NotStarted,
    #[serde(rename = "unknown")]
    Unknown,
}
impl Validate for ProducerV1Status {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ProducerV1Status {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "exited" => Ok(Self::Exited),
            "signalled" => Ok(Self::Signalled),
            "not_started" => Ok(Self::NotStarted),
            "unknown" => Ok(Self::Unknown),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum VerdictV1State {
    #[serde(rename = "PASS_CANDIDATE")]
    PassCandidate,
    #[serde(rename = "FAIL")]
    Fail,
    #[serde(rename = "INVALID")]
    Invalid,
    #[serde(rename = "ERROR")]
    Error,
    #[serde(rename = "TIMEOUT")]
    Timeout,
    #[serde(rename = "CANCELLED")]
    Cancelled,
    #[serde(rename = "UNMEASURED")]
    Unmeasured,
}
impl Validate for VerdictV1State {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for VerdictV1State {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "PASS_CANDIDATE" => Ok(Self::PassCandidate),
            "FAIL" => Ok(Self::Fail),
            "INVALID" => Ok(Self::Invalid),
            "ERROR" => Ok(Self::Error),
            "TIMEOUT" => Ok(Self::Timeout),
            "CANCELLED" => Ok(Self::Cancelled),
            "UNMEASURED" => Ok(Self::Unmeasured),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum AvailabilityV1State {
    #[serde(rename = "complete")]
    Complete,
    #[serde(rename = "incomplete")]
    Incomplete,
    #[serde(rename = "missing")]
    Missing,
}
impl Validate for AvailabilityV1State {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for AvailabilityV1State {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "complete" => Ok(Self::Complete),
            "incomplete" => Ok(Self::Incomplete),
            "missing" => Ok(Self::Missing),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum AvailabilityV1RetentionPolicy {
    #[serde(rename = "retain-v1")]
    RetainV1,
}
impl Validate for AvailabilityV1RetentionPolicy {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for AvailabilityV1RetentionPolicy {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "retain-v1" => Ok(Self::RetainV1),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ReceiptV1Protocol {
    #[serde(rename = "hee3.receipt")]
    Hee3Receipt,
}
impl Validate for ReceiptV1Protocol {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ReceiptV1Protocol {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "hee3.receipt" => Ok(Self::Hee3Receipt),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ReceiptV1Serialization {
    #[serde(rename = "json-exact-v1")]
    JsonExactV1,
}
impl Validate for ReceiptV1Serialization {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ReceiptV1Serialization {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "json-exact-v1" => Ok(Self::JsonExactV1),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ExpectedProducerV1Status {
    #[serde(rename = "exited")]
    Exited,
    #[serde(rename = "signalled")]
    Signalled,
}
impl Validate for ExpectedProducerV1Status {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ExpectedProducerV1Status {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "exited" => Ok(Self::Exited),
            "signalled" => Ok(Self::Signalled),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum CaseV1Outcome {
    #[serde(rename = "passed")]
    Passed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "skipped")]
    Skipped,
    #[serde(rename = "ignored")]
    Ignored,
    #[serde(rename = "broken")]
    Broken,
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "unmeasured")]
    Unmeasured,
}
impl Validate for CaseV1Outcome {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for CaseV1Outcome {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            "ignored" => Ok(Self::Ignored),
            "broken" => Ok(Self::Broken),
            "timeout" => Ok(Self::Timeout),
            "invalid" => Ok(Self::Invalid),
            "unmeasured" => Ok(Self::Unmeasured),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum SubjectFileV1Kind {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "directory")]
    Directory,
    #[serde(rename = "symlink")]
    Symlink,
    #[serde(rename = "submodule")]
    Submodule,
    #[serde(rename = "other")]
    Other,
}
impl Validate for SubjectFileV1Kind {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for SubjectFileV1Kind {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "file" => Ok(Self::File),
            "directory" => Ok(Self::Directory),
            "symlink" => Ok(Self::Symlink),
            "submodule" => Ok(Self::Submodule),
            "other" => Ok(Self::Other),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum SubjectFileV1Origin {
    #[serde(rename = "authored")]
    Authored,
    #[serde(rename = "generated")]
    Generated,
    #[serde(rename = "excluded")]
    Excluded,
}
impl Validate for SubjectFileV1Origin {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for SubjectFileV1Origin {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "authored" => Ok(Self::Authored),
            "generated" => Ok(Self::Generated),
            "excluded" => Ok(Self::Excluded),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ExpectationV1OracleClass {
    #[serde(rename = "contract")]
    Contract,
    #[serde(rename = "reference")]
    Reference,
    #[serde(rename = "round_trip_invariant")]
    RoundTripInvariant,
    #[serde(rename = "metamorphic")]
    Metamorphic,
    #[serde(rename = "differential")]
    Differential,
}
impl Validate for ExpectationV1OracleClass {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ExpectationV1OracleClass {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "contract" => Ok(Self::Contract),
            "reference" => Ok(Self::Reference),
            "round_trip_invariant" => Ok(Self::RoundTripInvariant),
            "metamorphic" => Ok(Self::Metamorphic),
            "differential" => Ok(Self::Differential),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ExpectationV1ExpectedOracle {
    #[serde(rename = "satisfied")]
    Satisfied,
}
impl Validate for ExpectationV1ExpectedOracle {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ExpectationV1ExpectedOracle {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "satisfied" => Ok(Self::Satisfied),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ObligationV1State {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "settled")]
    Settled,
    #[serde(rename = "unknown")]
    Unknown,
}
impl Validate for ObligationV1State {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ObligationV1State {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "open" => Ok(Self::Open),
            "settled" => Ok(Self::Settled),
            "unknown" => Ok(Self::Unknown),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ArtifactV1Availability {
    #[serde(rename = "available")]
    Available,
    #[serde(rename = "missing")]
    Missing,
}
impl Validate for ArtifactV1Availability {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ArtifactV1Availability {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "available" => Ok(Self::Available),
            "missing" => Ok(Self::Missing),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum CampaignV1Language {
    #[serde(rename = "rust")]
    Rust,
    #[serde(rename = "julia")]
    Julia,
}
impl Validate for CampaignV1Language {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for CampaignV1Language {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "rust" => Ok(Self::Rust),
            "julia" => Ok(Self::Julia),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum MutantV1Outcome {
    #[serde(rename = "caught")]
    Caught,
    #[serde(rename = "survived")]
    Survived,
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "unviable")]
    Unviable,
    #[serde(rename = "reviewed-equivalent")]
    ReviewedEquivalent,
    #[serde(rename = "excluded")]
    Excluded,
    #[serde(rename = "unmeasured")]
    Unmeasured,
}
impl Validate for MutantV1Outcome {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for MutantV1Outcome {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "caught" => Ok(Self::Caught),
            "survived" => Ok(Self::Survived),
            "timeout" => Ok(Self::Timeout),
            "unviable" => Ok(Self::Unviable),
            "reviewed-equivalent" => Ok(Self::ReviewedEquivalent),
            "excluded" => Ok(Self::Excluded),
            "unmeasured" => Ok(Self::Unmeasured),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum FindingV1Disposition {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "fixed")]
    Fixed,
    #[serde(rename = "accepted_residual")]
    AcceptedResidual,
    #[serde(rename = "not_reproduced")]
    NotReproduced,
    #[serde(rename = "out_of_scope")]
    OutOfScope,
}
impl Validate for FindingV1Disposition {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for FindingV1Disposition {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "open" => Ok(Self::Open),
            "fixed" => Ok(Self::Fixed),
            "accepted_residual" => Ok(Self::AcceptedResidual),
            "not_reproduced" => Ok(Self::NotReproduced),
            "out_of_scope" => Ok(Self::OutOfScope),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum OracleResultV1Result {
    #[serde(rename = "satisfied")]
    Satisfied,
    #[serde(rename = "violated")]
    Violated,
    #[serde(rename = "unavailable")]
    Unavailable,
    #[serde(rename = "error")]
    Error,
}
impl Validate for OracleResultV1Result {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for OracleResultV1Result {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "satisfied" => Ok(Self::Satisfied),
            "violated" => Ok(Self::Violated),
            "unavailable" => Ok(Self::Unavailable),
            "error" => Ok(Self::Error),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ReviewReceiptV1Protocol {
    #[serde(rename = "hee3.receipt.review")]
    Hee3ReceiptReview,
}
impl Validate for ReviewReceiptV1Protocol {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for ReviewReceiptV1Protocol {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "hee3.receipt.review" => Ok(Self::Hee3ReceiptReview),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed wire vocabulary from RC04."]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum AvailabilityReceiptV1Protocol {
    #[serde(rename = "hee3.receipt.availability")]
    Hee3ReceiptAvailability,
}
impl Validate for AvailabilityReceiptV1Protocol {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl<'de> Deserialize<'de> for AvailabilityReceiptV1Protocol {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "hee3.receipt.availability" => Ok(Self::Hee3ReceiptAvailability),
            _ => Err(serde::de::Error::custom("invalid receipt enum")),
        }
    }
}

#[doc = "Closed RC04 `IdentityV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IdentityV1 {
    pub run_id: Id,
    pub task_id: Id,
    pub attempt_id: Id,
    pub generation: Generation,
    pub module_id: Name,
    pub criterion_ids: List<Name>,
    pub profile_id: Name,
    pub parent_run: Maybe<Id>,
}
impl<'de> Deserialize<'de> for IdentityV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            run_id: Id,
            task_id: Id,
            attempt_id: Id,
            generation: Generation,
            module_id: Name,
            criterion_ids: List<Name>,
            profile_id: Name,
            parent_run: Maybe<Id>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            run_id: fields.run_id,
            task_id: fields.task_id,
            attempt_id: fields.attempt_id,
            generation: fields.generation,
            module_id: fields.module_id,
            criterion_ids: fields.criterion_ids,
            profile_id: fields.profile_id,
            parent_run: fields.parent_run,
        })
    }
}
impl Address for IdentityV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:IdentityV1";
}
impl sealed::Sealed for IdentityV1 {}
impl ReceiptRecord for IdentityV1 {}
impl Validate for IdentityV1 {
    fn validate(&self) -> Result<(), Error> {
        self.run_id.validate()?;
        self.task_id.validate()?;
        self.attempt_id.validate()?;
        self.generation.validate()?;
        self.module_id.validate()?;
        self.criterion_ids.validate()?;
        self.profile_id.validate()?;
        self.parent_run.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `SubjectsV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SubjectsV1 {
    pub seed_subject: TypedRef<SubjectV1>,
    pub result_subject: Maybe<TypedRef<SubjectV1>>,
    pub seed_to_result_patch: Maybe<Payload>,
    pub fixtures: TypedRef<SubjectV1>,
    pub oracle: TypedRef<SubjectV1>,
    pub harness: TypedRef<SubjectV1>,
    pub collector: TypedRef<SubjectV1>,
    pub launcher: TypedRef<SubjectV1>,
    pub locks: TypedRef<LockPageV1>,
    pub toolchain: TypedRef<ToolPageV1>,
    pub target_features_build_profile: TypedRef<BuildProfileV1>,
    pub standards: TypedRef<StandardPageV1>,
    pub isolation_profile: Payload,
}
impl<'de> Deserialize<'de> for SubjectsV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            seed_subject: TypedRef<SubjectV1>,
            result_subject: Maybe<TypedRef<SubjectV1>>,
            seed_to_result_patch: Maybe<Payload>,
            fixtures: TypedRef<SubjectV1>,
            oracle: TypedRef<SubjectV1>,
            harness: TypedRef<SubjectV1>,
            collector: TypedRef<SubjectV1>,
            launcher: TypedRef<SubjectV1>,
            locks: TypedRef<LockPageV1>,
            toolchain: TypedRef<ToolPageV1>,
            target_features_build_profile: TypedRef<BuildProfileV1>,
            standards: TypedRef<StandardPageV1>,
            isolation_profile: Payload,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            seed_subject: fields.seed_subject,
            result_subject: fields.result_subject,
            seed_to_result_patch: fields.seed_to_result_patch,
            fixtures: fields.fixtures,
            oracle: fields.oracle,
            harness: fields.harness,
            collector: fields.collector,
            launcher: fields.launcher,
            locks: fields.locks,
            toolchain: fields.toolchain,
            target_features_build_profile: fields.target_features_build_profile,
            standards: fields.standards,
            isolation_profile: fields.isolation_profile,
        })
    }
}
impl Address for SubjectsV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:SubjectsV1";
}
impl sealed::Sealed for SubjectsV1 {}
impl ReceiptRecord for SubjectsV1 {}
impl Validate for SubjectsV1 {
    fn validate(&self) -> Result<(), Error> {
        self.seed_subject.validate()?;
        self.result_subject.validate()?;
        self.seed_to_result_patch.validate()?;
        self.fixtures.validate()?;
        self.oracle.validate()?;
        self.harness.validate()?;
        self.collector.validate()?;
        self.launcher.validate()?;
        self.locks.validate()?;
        self.toolchain.validate()?;
        self.target_features_build_profile.validate()?;
        self.standards.validate()?;
        self.isolation_profile.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `InvocationV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InvocationV1 {
    pub argv: List<Text>,
    pub cwd_logical: Name,
    pub environment: TypedRef<EnvironmentPageV1>,
    pub grants: TypedRef<GrantPageV1>,
    pub expected: TypedRef<ExpectationV1>,
    pub oracle_id: Name,
    pub limits: TypedRef<LimitsV1>,
    pub allowed_effects: TypedRef<EffectPageV1>,
    pub cleanup_contract: TypedRef<CleanupContractV1>,
}
impl<'de> Deserialize<'de> for InvocationV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            argv: List<Text>,
            cwd_logical: Name,
            environment: TypedRef<EnvironmentPageV1>,
            grants: TypedRef<GrantPageV1>,
            expected: TypedRef<ExpectationV1>,
            oracle_id: Name,
            limits: TypedRef<LimitsV1>,
            allowed_effects: TypedRef<EffectPageV1>,
            cleanup_contract: TypedRef<CleanupContractV1>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            argv: fields.argv,
            cwd_logical: fields.cwd_logical,
            environment: fields.environment,
            grants: fields.grants,
            expected: fields.expected,
            oracle_id: fields.oracle_id,
            limits: fields.limits,
            allowed_effects: fields.allowed_effects,
            cleanup_contract: fields.cleanup_contract,
        })
    }
}
impl Address for InvocationV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:InvocationV1";
}
impl sealed::Sealed for InvocationV1 {}
impl ReceiptRecord for InvocationV1 {}
impl Validate for InvocationV1 {
    fn validate(&self) -> Result<(), Error> {
        self.argv.validate()?;
        self.cwd_logical.validate()?;
        self.environment.validate()?;
        self.grants.validate()?;
        self.expected.validate()?;
        self.oracle_id.validate()?;
        self.limits.validate()?;
        self.allowed_effects.validate()?;
        self.cleanup_contract.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ObservationsV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObservationsV1 {
    pub host: TypedRef<HostV1>,
    pub start_unix_ms: U64,
    pub end_unix_ms: U64,
    pub start_monotonic_ns: U64,
    pub end_monotonic_ns: U64,
    pub cutoff_unix_ms: U64,
    pub resources: TypedRef<ResourcePageV1>,
    pub producer: ProducerV1,
    pub cancellation: ObservationsV1Cancellation,
    pub cleanup: ObservationsV1Cleanup,
    pub unresolved_obligations: TypedRef<ObligationPageV1>,
}
impl<'de> Deserialize<'de> for ObservationsV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            host: TypedRef<HostV1>,
            start_unix_ms: U64,
            end_unix_ms: U64,
            start_monotonic_ns: U64,
            end_monotonic_ns: U64,
            cutoff_unix_ms: U64,
            resources: TypedRef<ResourcePageV1>,
            producer: ProducerV1,
            cancellation: ObservationsV1Cancellation,
            cleanup: ObservationsV1Cleanup,
            unresolved_obligations: TypedRef<ObligationPageV1>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            host: fields.host,
            start_unix_ms: fields.start_unix_ms,
            end_unix_ms: fields.end_unix_ms,
            start_monotonic_ns: fields.start_monotonic_ns,
            end_monotonic_ns: fields.end_monotonic_ns,
            cutoff_unix_ms: fields.cutoff_unix_ms,
            resources: fields.resources,
            producer: fields.producer,
            cancellation: fields.cancellation,
            cleanup: fields.cleanup,
            unresolved_obligations: fields.unresolved_obligations,
        })
    }
}
impl Address for ObservationsV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ObservationsV1";
}
impl sealed::Sealed for ObservationsV1 {}
impl ReceiptRecord for ObservationsV1 {}
impl Validate for ObservationsV1 {
    fn validate(&self) -> Result<(), Error> {
        self.host.validate()?;
        self.start_unix_ms.validate()?;
        self.end_unix_ms.validate()?;
        self.start_monotonic_ns.validate()?;
        self.end_monotonic_ns.validate()?;
        self.cutoff_unix_ms.validate()?;
        self.resources.validate()?;
        self.producer.validate()?;
        self.cancellation.validate()?;
        self.cleanup.validate()?;
        self.unresolved_obligations.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ProducerV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProducerV1 {
    pub status: ProducerV1Status,
    pub exit_code: Maybe<Count>,
    pub signal: Maybe<Count>,
    pub timeout: Bool,
    pub stdout: Maybe<Payload>,
    pub stderr: Maybe<Payload>,
}
impl<'de> Deserialize<'de> for ProducerV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            status: ProducerV1Status,
            exit_code: Maybe<Count>,
            signal: Maybe<Count>,
            timeout: Bool,
            stdout: Maybe<Payload>,
            stderr: Maybe<Payload>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            status: fields.status,
            exit_code: fields.exit_code,
            signal: fields.signal,
            timeout: fields.timeout,
            stdout: fields.stdout,
            stderr: fields.stderr,
        })
    }
}
impl Address for ProducerV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ProducerV1";
}
impl sealed::Sealed for ProducerV1 {}
impl ReceiptRecord for ProducerV1 {}
impl Validate for ProducerV1 {
    fn validate(&self) -> Result<(), Error> {
        self.status.validate()?;
        self.exit_code.validate()?;
        self.signal.validate()?;
        self.timeout.validate()?;
        self.stdout.validate()?;
        self.stderr.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `CasesV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CasesV1 {
    pub inventory: TypedRef<CasePageV1>,
    pub discovered: Count,
    pub selected: Count,
    pub executed: Count,
    pub passed: Count,
    pub failed: Count,
    pub skipped: Count,
    pub ignored: Count,
    pub broken: Count,
    pub timed_out: Count,
    pub invalid: Count,
    pub excluded: Count,
    pub unmeasured: Count,
    pub primary_credit: Count,
}
impl<'de> Deserialize<'de> for CasesV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            inventory: TypedRef<CasePageV1>,
            discovered: Count,
            selected: Count,
            executed: Count,
            passed: Count,
            failed: Count,
            skipped: Count,
            ignored: Count,
            broken: Count,
            timed_out: Count,
            invalid: Count,
            excluded: Count,
            unmeasured: Count,
            primary_credit: Count,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            inventory: fields.inventory,
            discovered: fields.discovered,
            selected: fields.selected,
            executed: fields.executed,
            passed: fields.passed,
            failed: fields.failed,
            skipped: fields.skipped,
            ignored: fields.ignored,
            broken: fields.broken,
            timed_out: fields.timed_out,
            invalid: fields.invalid,
            excluded: fields.excluded,
            unmeasured: fields.unmeasured,
            primary_credit: fields.primary_credit,
        })
    }
}
impl Address for CasesV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:CasesV1";
}
impl sealed::Sealed for CasesV1 {}
impl ReceiptRecord for CasesV1 {}
impl Validate for CasesV1 {
    fn validate(&self) -> Result<(), Error> {
        self.inventory.validate()?;
        self.discovered.validate()?;
        self.selected.validate()?;
        self.executed.validate()?;
        self.passed.validate()?;
        self.failed.validate()?;
        self.skipped.validate()?;
        self.ignored.validate()?;
        self.broken.validate()?;
        self.timed_out.validate()?;
        self.invalid.validate()?;
        self.excluded.validate()?;
        self.unmeasured.validate()?;
        self.primary_credit.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `DiagnosticsV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticsV1 {
    pub baseline: Bool,
    pub warning_count: Count,
    pub error_count: Count,
    pub by_tool: TypedRef<DiagnosticPageV1>,
    pub stdout_truncated: Bool,
    pub stderr_truncated: Bool,
    pub mismatch: Maybe<Text>,
}
impl<'de> Deserialize<'de> for DiagnosticsV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            baseline: Bool,
            warning_count: Count,
            error_count: Count,
            by_tool: TypedRef<DiagnosticPageV1>,
            stdout_truncated: Bool,
            stderr_truncated: Bool,
            mismatch: Maybe<Text>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            baseline: fields.baseline,
            warning_count: fields.warning_count,
            error_count: fields.error_count,
            by_tool: fields.by_tool,
            stdout_truncated: fields.stdout_truncated,
            stderr_truncated: fields.stderr_truncated,
            mismatch: fields.mismatch,
        })
    }
}
impl Address for DiagnosticsV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:DiagnosticsV1";
}
impl sealed::Sealed for DiagnosticsV1 {}
impl ReceiptRecord for DiagnosticsV1 {}
impl Validate for DiagnosticsV1 {
    fn validate(&self) -> Result<(), Error> {
        self.baseline.validate()?;
        self.warning_count.validate()?;
        self.error_count.validate()?;
        self.by_tool.validate()?;
        self.stdout_truncated.validate()?;
        self.stderr_truncated.validate()?;
        self.mismatch.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ArtifactsV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArtifactsV1 {
    pub inventory: TypedRef<ArtifactPageV1>,
    pub count: Count,
    pub total_bytes: U64,
    pub finalized: Bool,
}
impl<'de> Deserialize<'de> for ArtifactsV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            inventory: TypedRef<ArtifactPageV1>,
            count: Count,
            total_bytes: U64,
            finalized: Bool,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            inventory: fields.inventory,
            count: fields.count,
            total_bytes: fields.total_bytes,
            finalized: fields.finalized,
        })
    }
}
impl Address for ArtifactsV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ArtifactsV1";
}
impl sealed::Sealed for ArtifactsV1 {}
impl ReceiptRecord for ArtifactsV1 {}
impl Validate for ArtifactsV1 {
    fn validate(&self) -> Result<(), Error> {
        self.inventory.validate()?;
        self.count.validate()?;
        self.total_bytes.validate()?;
        self.finalized.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `MutationV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MutationV1 {
    pub campaigns: TypedRef<CampaignPageV1>,
    pub campaign_count: Count,
    pub distinct_mutants: Count,
    pub caught: Count,
    pub survived: Count,
    pub timed_out: Count,
    pub unviable: Count,
    pub equivalent: Count,
    pub excluded: Count,
    pub unmeasured: Count,
}
impl<'de> Deserialize<'de> for MutationV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            campaigns: TypedRef<CampaignPageV1>,
            campaign_count: Count,
            distinct_mutants: Count,
            caught: Count,
            survived: Count,
            timed_out: Count,
            unviable: Count,
            equivalent: Count,
            excluded: Count,
            unmeasured: Count,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            campaigns: fields.campaigns,
            campaign_count: fields.campaign_count,
            distinct_mutants: fields.distinct_mutants,
            caught: fields.caught,
            survived: fields.survived,
            timed_out: fields.timed_out,
            unviable: fields.unviable,
            equivalent: fields.equivalent,
            excluded: fields.excluded,
            unmeasured: fields.unmeasured,
        })
    }
}
impl Address for MutationV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:MutationV1";
}
impl sealed::Sealed for MutationV1 {}
impl ReceiptRecord for MutationV1 {}
impl Validate for MutationV1 {
    fn validate(&self) -> Result<(), Error> {
        self.campaigns.validate()?;
        self.campaign_count.validate()?;
        self.distinct_mutants.validate()?;
        self.caught.validate()?;
        self.survived.validate()?;
        self.timed_out.validate()?;
        self.unviable.validate()?;
        self.equivalent.validate()?;
        self.excluded.validate()?;
        self.unmeasured.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ReviewV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReviewV1 {
    pub reviewer: Text,
    pub effective_model: Maybe<Text>,
    pub action: Name,
    pub subject_sha256: Sha,
    pub shared_assumptions: TypedRef<AssumptionPageV1>,
    pub findings: TypedRef<FindingPageV1>,
    pub disposition: Name,
    pub residual_obligations: TypedRef<ObligationPageV1>,
    pub pre_fix_evidence: List<Ref>,
    pub post_fix_evidence: List<Ref>,
}
impl<'de> Deserialize<'de> for ReviewV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            reviewer: Text,
            effective_model: Maybe<Text>,
            action: Name,
            subject_sha256: Sha,
            shared_assumptions: TypedRef<AssumptionPageV1>,
            findings: TypedRef<FindingPageV1>,
            disposition: Name,
            residual_obligations: TypedRef<ObligationPageV1>,
            pre_fix_evidence: List<Ref>,
            post_fix_evidence: List<Ref>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            reviewer: fields.reviewer,
            effective_model: fields.effective_model,
            action: fields.action,
            subject_sha256: fields.subject_sha256,
            shared_assumptions: fields.shared_assumptions,
            findings: fields.findings,
            disposition: fields.disposition,
            residual_obligations: fields.residual_obligations,
            pre_fix_evidence: fields.pre_fix_evidence,
            post_fix_evidence: fields.post_fix_evidence,
        })
    }
}
impl Address for ReviewV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ReviewV1";
}
impl sealed::Sealed for ReviewV1 {}
impl ReceiptRecord for ReviewV1 {}
impl Validate for ReviewV1 {
    fn validate(&self) -> Result<(), Error> {
        self.reviewer.validate()?;
        self.effective_model.validate()?;
        self.action.validate()?;
        self.subject_sha256.validate()?;
        self.shared_assumptions.validate()?;
        self.findings.validate()?;
        self.disposition.validate()?;
        self.residual_obligations.validate()?;
        self.pre_fix_evidence.validate()?;
        self.post_fix_evidence.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `VerdictV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VerdictV1 {
    pub state: VerdictV1State,
    pub oracle_result: TypedRef<OracleResultV1>,
    pub intended_detector: Name,
    pub benign_pair: Maybe<Id>,
    pub reasons: List<Text>,
}
impl<'de> Deserialize<'de> for VerdictV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            state: VerdictV1State,
            oracle_result: TypedRef<OracleResultV1>,
            intended_detector: Name,
            benign_pair: Maybe<Id>,
            reasons: List<Text>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            state: fields.state,
            oracle_result: fields.oracle_result,
            intended_detector: fields.intended_detector,
            benign_pair: fields.benign_pair,
            reasons: fields.reasons,
        })
    }
}
impl Address for VerdictV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:VerdictV1";
}
impl sealed::Sealed for VerdictV1 {}
impl ReceiptRecord for VerdictV1 {}
impl Validate for VerdictV1 {
    fn validate(&self) -> Result<(), Error> {
        self.state.validate()?;
        self.oracle_result.validate()?;
        self.intended_detector.validate()?;
        self.benign_pair.validate()?;
        self.reasons.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `AvailabilityV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AvailabilityV1 {
    pub observed_unix_ms: U64,
    pub state: AvailabilityV1State,
    pub missing_objects: TypedRef<MissingObjectPageV1>,
    pub retention_policy: AvailabilityV1RetentionPolicy,
}
impl<'de> Deserialize<'de> for AvailabilityV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            observed_unix_ms: U64,
            state: AvailabilityV1State,
            missing_objects: TypedRef<MissingObjectPageV1>,
            retention_policy: AvailabilityV1RetentionPolicy,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            observed_unix_ms: fields.observed_unix_ms,
            state: fields.state,
            missing_objects: fields.missing_objects,
            retention_policy: fields.retention_policy,
        })
    }
}
impl Address for AvailabilityV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:AvailabilityV1";
}
impl sealed::Sealed for AvailabilityV1 {}
impl ReceiptRecord for AvailabilityV1 {}
impl Validate for AvailabilityV1 {
    fn validate(&self) -> Result<(), Error> {
        self.observed_unix_ms.validate()?;
        self.state.validate()?;
        self.missing_objects.validate()?;
        self.retention_policy.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ReceiptV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReceiptV1 {
    pub protocol: ReceiptV1Protocol,
    pub version: Count,
    pub serialization: ReceiptV1Serialization,
    pub schema_sha256: Sha,
    pub identity: IdentityV1,
    pub subjects: SubjectsV1,
    pub invocation: InvocationV1,
    pub observations: ObservationsV1,
    pub cases: CasesV1,
    pub diagnostics: DiagnosticsV1,
    pub artifacts: ArtifactsV1,
    pub mutation: MutationV1,
    pub review: Maybe<ReviewV1>,
    pub verdict: VerdictV1,
    pub availability: AvailabilityV1,
}
impl<'de> Deserialize<'de> for ReceiptV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            protocol: ReceiptV1Protocol,
            version: Count,
            serialization: ReceiptV1Serialization,
            schema_sha256: Sha,
            identity: IdentityV1,
            subjects: SubjectsV1,
            invocation: InvocationV1,
            observations: ObservationsV1,
            cases: CasesV1,
            diagnostics: DiagnosticsV1,
            artifacts: ArtifactsV1,
            mutation: MutationV1,
            review: Maybe<ReviewV1>,
            verdict: VerdictV1,
            availability: AvailabilityV1,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            protocol: fields.protocol,
            version: fields.version,
            serialization: fields.serialization,
            schema_sha256: fields.schema_sha256,
            identity: fields.identity,
            subjects: fields.subjects,
            invocation: fields.invocation,
            observations: fields.observations,
            cases: fields.cases,
            diagnostics: fields.diagnostics,
            artifacts: fields.artifacts,
            mutation: fields.mutation,
            review: fields.review,
            verdict: fields.verdict,
            availability: fields.availability,
        })
    }
}
impl Address for ReceiptV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ReceiptV1";
}
impl sealed::Sealed for ReceiptV1 {}
impl ReceiptRecord for ReceiptV1 {}
impl Validate for ReceiptV1 {
    fn validate(&self) -> Result<(), Error> {
        self.protocol.validate()?;
        self.version.validate()?;
        self.serialization.validate()?;
        self.schema_sha256.validate()?;
        self.identity.validate()?;
        self.subjects.validate()?;
        self.invocation.validate()?;
        self.observations.validate()?;
        self.cases.validate()?;
        self.diagnostics.validate()?;
        self.artifacts.validate()?;
        self.mutation.validate()?;
        self.review.validate()?;
        self.verdict.validate()?;
        self.availability.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ExpectedProducerV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExpectedProducerV1 {
    pub status: ExpectedProducerV1Status,
    pub exit_code: Maybe<Count>,
    pub signal: Maybe<Count>,
}
impl<'de> Deserialize<'de> for ExpectedProducerV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            status: ExpectedProducerV1Status,
            exit_code: Maybe<Count>,
            signal: Maybe<Count>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            status: fields.status,
            exit_code: fields.exit_code,
            signal: fields.signal,
        })
    }
}
impl Address for ExpectedProducerV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ExpectedProducerV1";
}
impl sealed::Sealed for ExpectedProducerV1 {}
impl ReceiptRecord for ExpectedProducerV1 {}
impl Validate for ExpectedProducerV1 {
    fn validate(&self) -> Result<(), Error> {
        self.status.validate()?;
        self.exit_code.validate()?;
        self.signal.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `CaseV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CaseV1 {
    pub case_id: Name,
    pub primary_module_id: Name,
    pub criterion_ids: List<Name>,
    pub fixture_sha256: Sha,
    pub oracle_id: Name,
    pub expected: TypedRef<ExpectationV1>,
    pub mandatory: Bool,
    pub excluded: Bool,
    pub selected: Bool,
    pub executed: Bool,
    pub outcome: CaseV1Outcome,
    pub producer_exit_or_signal: ProducerV1,
    pub detector_id: Name,
    pub benign_pair_id: Maybe<Id>,
    pub raw_evidence_refs: List<Ref>,
    pub reason: Text,
}
impl<'de> Deserialize<'de> for CaseV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            case_id: Name,
            primary_module_id: Name,
            criterion_ids: List<Name>,
            fixture_sha256: Sha,
            oracle_id: Name,
            expected: TypedRef<ExpectationV1>,
            mandatory: Bool,
            excluded: Bool,
            selected: Bool,
            executed: Bool,
            outcome: CaseV1Outcome,
            producer_exit_or_signal: ProducerV1,
            detector_id: Name,
            benign_pair_id: Maybe<Id>,
            raw_evidence_refs: List<Ref>,
            reason: Text,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            case_id: fields.case_id,
            primary_module_id: fields.primary_module_id,
            criterion_ids: fields.criterion_ids,
            fixture_sha256: fields.fixture_sha256,
            oracle_id: fields.oracle_id,
            expected: fields.expected,
            mandatory: fields.mandatory,
            excluded: fields.excluded,
            selected: fields.selected,
            executed: fields.executed,
            outcome: fields.outcome,
            producer_exit_or_signal: fields.producer_exit_or_signal,
            detector_id: fields.detector_id,
            benign_pair_id: fields.benign_pair_id,
            raw_evidence_refs: fields.raw_evidence_refs,
            reason: fields.reason,
        })
    }
}
impl Address for CaseV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:CaseV1";
}
impl sealed::Sealed for CaseV1 {}
impl ReceiptRecord for CaseV1 {}
impl Validate for CaseV1 {
    fn validate(&self) -> Result<(), Error> {
        self.case_id.validate()?;
        self.primary_module_id.validate()?;
        self.criterion_ids.validate()?;
        self.fixture_sha256.validate()?;
        self.oracle_id.validate()?;
        self.expected.validate()?;
        self.mandatory.validate()?;
        self.excluded.validate()?;
        self.selected.validate()?;
        self.executed.validate()?;
        self.outcome.validate()?;
        self.producer_exit_or_signal.validate()?;
        self.detector_id.validate()?;
        self.benign_pair_id.validate()?;
        self.raw_evidence_refs.validate()?;
        self.reason.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `SubjectV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SubjectV1 {
    pub subject_id: Id,
    pub files: TypedRef<SubjectFilePageV1>,
    pub tree_sha256: Sha,
    pub dirty_patch: Maybe<Payload>,
}
impl<'de> Deserialize<'de> for SubjectV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            subject_id: Id,
            files: TypedRef<SubjectFilePageV1>,
            tree_sha256: Sha,
            dirty_patch: Maybe<Payload>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            subject_id: fields.subject_id,
            files: fields.files,
            tree_sha256: fields.tree_sha256,
            dirty_patch: fields.dirty_patch,
        })
    }
}
impl Address for SubjectV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:SubjectV1";
}
impl sealed::Sealed for SubjectV1 {}
impl ReceiptRecord for SubjectV1 {}
impl Validate for SubjectV1 {
    fn validate(&self) -> Result<(), Error> {
        self.subject_id.validate()?;
        self.files.validate()?;
        self.tree_sha256.validate()?;
        self.dirty_patch.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `SubjectFileV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SubjectFileV1 {
    pub path: RelPath,
    pub kind: SubjectFileV1Kind,
    pub content: Maybe<Payload>,
    pub executable: Bool,
    pub link_target: Maybe<Text>,
    pub origin: SubjectFileV1Origin,
    pub exclusion_reason: Maybe<Text>,
}
impl<'de> Deserialize<'de> for SubjectFileV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            path: RelPath,
            kind: SubjectFileV1Kind,
            content: Maybe<Payload>,
            executable: Bool,
            link_target: Maybe<Text>,
            origin: SubjectFileV1Origin,
            exclusion_reason: Maybe<Text>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            path: fields.path,
            kind: fields.kind,
            content: fields.content,
            executable: fields.executable,
            link_target: fields.link_target,
            origin: fields.origin,
            exclusion_reason: fields.exclusion_reason,
        })
    }
}
impl Address for SubjectFileV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:SubjectFileV1";
}
impl sealed::Sealed for SubjectFileV1 {}
impl ReceiptRecord for SubjectFileV1 {}
impl Validate for SubjectFileV1 {
    fn validate(&self) -> Result<(), Error> {
        self.path.validate()?;
        self.kind.validate()?;
        self.content.validate()?;
        self.executable.validate()?;
        self.link_target.validate()?;
        self.origin.validate()?;
        self.exclusion_reason.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `LockV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LockV1 {
    pub lock_id: Name,
    pub ecosystem: Name,
    pub path: RelPath,
    pub content: Payload,
    pub dependencies: TypedRef<DependencyPageV1>,
}
impl<'de> Deserialize<'de> for LockV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            lock_id: Name,
            ecosystem: Name,
            path: RelPath,
            content: Payload,
            dependencies: TypedRef<DependencyPageV1>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            lock_id: fields.lock_id,
            ecosystem: fields.ecosystem,
            path: fields.path,
            content: fields.content,
            dependencies: fields.dependencies,
        })
    }
}
impl Address for LockV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:LockV1";
}
impl sealed::Sealed for LockV1 {}
impl ReceiptRecord for LockV1 {}
impl Validate for LockV1 {
    fn validate(&self) -> Result<(), Error> {
        self.lock_id.validate()?;
        self.ecosystem.validate()?;
        self.path.validate()?;
        self.content.validate()?;
        self.dependencies.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `DependencyV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyV1 {
    pub dependency_id: Name,
    pub name: Name,
    pub version: Name,
    pub source: Text,
    pub checksum: Maybe<Sha>,
    pub locked_by: Payload,
}
impl<'de> Deserialize<'de> for DependencyV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            dependency_id: Name,
            name: Name,
            version: Name,
            source: Text,
            checksum: Maybe<Sha>,
            locked_by: Payload,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            dependency_id: fields.dependency_id,
            name: fields.name,
            version: fields.version,
            source: fields.source,
            checksum: fields.checksum,
            locked_by: fields.locked_by,
        })
    }
}
impl Address for DependencyV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:DependencyV1";
}
impl sealed::Sealed for DependencyV1 {}
impl ReceiptRecord for DependencyV1 {}
impl Validate for DependencyV1 {
    fn validate(&self) -> Result<(), Error> {
        self.dependency_id.validate()?;
        self.name.validate()?;
        self.version.validate()?;
        self.source.validate()?;
        self.checksum.validate()?;
        self.locked_by.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ToolV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ToolV1 {
    pub tool_id: Name,
    pub executable: Payload,
    pub executable_path: Text,
    pub version: Text,
    pub version_output: Payload,
    pub target: Name,
}
impl<'de> Deserialize<'de> for ToolV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            tool_id: Name,
            executable: Payload,
            executable_path: Text,
            version: Text,
            version_output: Payload,
            target: Name,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            tool_id: fields.tool_id,
            executable: fields.executable,
            executable_path: fields.executable_path,
            version: fields.version,
            version_output: fields.version_output,
            target: fields.target,
        })
    }
}
impl Address for ToolV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ToolV1";
}
impl sealed::Sealed for ToolV1 {}
impl ReceiptRecord for ToolV1 {}
impl Validate for ToolV1 {
    fn validate(&self) -> Result<(), Error> {
        self.tool_id.validate()?;
        self.executable.validate()?;
        self.executable_path.validate()?;
        self.version.validate()?;
        self.version_output.validate()?;
        self.target.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `BuildProfileV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BuildProfileV1 {
    pub target: Name,
    pub features: List<Name>,
    pub default_features: Bool,
    pub build_profile: Name,
    pub language_flags: TypedRef<LanguageFlagsPageV1>,
}
impl<'de> Deserialize<'de> for BuildProfileV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            target: Name,
            features: List<Name>,
            default_features: Bool,
            build_profile: Name,
            language_flags: TypedRef<LanguageFlagsPageV1>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            target: fields.target,
            features: fields.features,
            default_features: fields.default_features,
            build_profile: fields.build_profile,
            language_flags: fields.language_flags,
        })
    }
}
impl Address for BuildProfileV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:BuildProfileV1";
}
impl sealed::Sealed for BuildProfileV1 {}
impl ReceiptRecord for BuildProfileV1 {}
impl Validate for BuildProfileV1 {
    fn validate(&self) -> Result<(), Error> {
        self.target.validate()?;
        self.features.validate()?;
        self.default_features.validate()?;
        self.build_profile.validate()?;
        self.language_flags.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `LanguageFlagsV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LanguageFlagsV1 {
    pub language: Name,
    pub argv: List<Text>,
}
impl<'de> Deserialize<'de> for LanguageFlagsV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            language: Name,
            argv: List<Text>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            language: fields.language,
            argv: fields.argv,
        })
    }
}
impl Address for LanguageFlagsV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:LanguageFlagsV1";
}
impl sealed::Sealed for LanguageFlagsV1 {}
impl ReceiptRecord for LanguageFlagsV1 {}
impl Validate for LanguageFlagsV1 {
    fn validate(&self) -> Result<(), Error> {
        self.language.validate()?;
        self.argv.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `StandardV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StandardV1 {
    pub standard_id: Name,
    pub revision: Name,
    pub document: Payload,
}
impl<'de> Deserialize<'de> for StandardV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            standard_id: Name,
            revision: Name,
            document: Payload,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            standard_id: fields.standard_id,
            revision: fields.revision,
            document: fields.document,
        })
    }
}
impl Address for StandardV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:StandardV1";
}
impl sealed::Sealed for StandardV1 {}
impl ReceiptRecord for StandardV1 {}
impl Validate for StandardV1 {
    fn validate(&self) -> Result<(), Error> {
        self.standard_id.validate()?;
        self.revision.validate()?;
        self.document.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `EnvironmentV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnvironmentV1 {
    pub name: Name,
    pub value: Maybe<Text>,
    pub secret_handle: Maybe<Id>,
}
impl<'de> Deserialize<'de> for EnvironmentV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            name: Name,
            value: Maybe<Text>,
            secret_handle: Maybe<Id>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            name: fields.name,
            value: fields.value,
            secret_handle: fields.secret_handle,
        })
    }
}
impl Address for EnvironmentV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:EnvironmentV1";
}
impl sealed::Sealed for EnvironmentV1 {}
impl ReceiptRecord for EnvironmentV1 {}
impl Validate for EnvironmentV1 {
    fn validate(&self) -> Result<(), Error> {
        self.name.validate()?;
        self.value.validate()?;
        self.secret_handle.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `GrantV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GrantV1 {
    pub grant_id: Id,
    pub scope_sha256: Sha,
    pub issuer_id: Name,
    pub grant: Payload,
}
impl<'de> Deserialize<'de> for GrantV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            grant_id: Id,
            scope_sha256: Sha,
            issuer_id: Name,
            grant: Payload,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            grant_id: fields.grant_id,
            scope_sha256: fields.scope_sha256,
            issuer_id: fields.issuer_id,
            grant: fields.grant,
        })
    }
}
impl Address for GrantV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:GrantV1";
}
impl sealed::Sealed for GrantV1 {}
impl ReceiptRecord for GrantV1 {}
impl Validate for GrantV1 {
    fn validate(&self) -> Result<(), Error> {
        self.grant_id.validate()?;
        self.scope_sha256.validate()?;
        self.issuer_id.validate()?;
        self.grant.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ExpectationV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExpectationV1 {
    pub oracle_id: Name,
    pub oracle_class: ExpectationV1OracleClass,
    pub specification: Payload,
    pub expected_producer: ExpectedProducerV1,
    pub expected_oracle: ExpectationV1ExpectedOracle,
    pub intended_detector: Name,
    pub shared_assumptions: TypedRef<AssumptionPageV1>,
}
impl<'de> Deserialize<'de> for ExpectationV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            oracle_id: Name,
            oracle_class: ExpectationV1OracleClass,
            specification: Payload,
            expected_producer: ExpectedProducerV1,
            expected_oracle: ExpectationV1ExpectedOracle,
            intended_detector: Name,
            shared_assumptions: TypedRef<AssumptionPageV1>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            oracle_id: fields.oracle_id,
            oracle_class: fields.oracle_class,
            specification: fields.specification,
            expected_producer: fields.expected_producer,
            expected_oracle: fields.expected_oracle,
            intended_detector: fields.intended_detector,
            shared_assumptions: fields.shared_assumptions,
        })
    }
}
impl Address for ExpectationV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ExpectationV1";
}
impl sealed::Sealed for ExpectationV1 {}
impl ReceiptRecord for ExpectationV1 {}
impl Validate for ExpectationV1 {
    fn validate(&self) -> Result<(), Error> {
        self.oracle_id.validate()?;
        self.oracle_class.validate()?;
        self.specification.validate()?;
        self.expected_producer.validate()?;
        self.expected_oracle.validate()?;
        self.intended_detector.validate()?;
        self.shared_assumptions.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `LimitsV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LimitsV1 {
    pub wall_ms: U64,
    pub memory_bytes: U64,
    pub memory_swap_bytes: U64,
    pub scratch_bytes: U64,
    pub stdout_bytes: U64,
    pub stderr_bytes: U64,
    pub artifact_bytes: U64,
    pub external_requests: U64,
    pub external_cost_microunits: U64,
    pub term_grace_ms: U64,
    pub cleanup_deadline_ms: U64,
    pub cpu_quota_percent: Count,
    pub tasks_max: Count,
    pub compiler_jobs: Count,
    pub julia_threads: Count,
    pub blas_threads: Count,
    pub currency: Maybe<Name>,
}
impl<'de> Deserialize<'de> for LimitsV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            wall_ms: U64,
            memory_bytes: U64,
            memory_swap_bytes: U64,
            scratch_bytes: U64,
            stdout_bytes: U64,
            stderr_bytes: U64,
            artifact_bytes: U64,
            external_requests: U64,
            external_cost_microunits: U64,
            term_grace_ms: U64,
            cleanup_deadline_ms: U64,
            cpu_quota_percent: Count,
            tasks_max: Count,
            compiler_jobs: Count,
            julia_threads: Count,
            blas_threads: Count,
            currency: Maybe<Name>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            wall_ms: fields.wall_ms,
            memory_bytes: fields.memory_bytes,
            memory_swap_bytes: fields.memory_swap_bytes,
            scratch_bytes: fields.scratch_bytes,
            stdout_bytes: fields.stdout_bytes,
            stderr_bytes: fields.stderr_bytes,
            artifact_bytes: fields.artifact_bytes,
            external_requests: fields.external_requests,
            external_cost_microunits: fields.external_cost_microunits,
            term_grace_ms: fields.term_grace_ms,
            cleanup_deadline_ms: fields.cleanup_deadline_ms,
            cpu_quota_percent: fields.cpu_quota_percent,
            tasks_max: fields.tasks_max,
            compiler_jobs: fields.compiler_jobs,
            julia_threads: fields.julia_threads,
            blas_threads: fields.blas_threads,
            currency: fields.currency,
        })
    }
}
impl Address for LimitsV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:LimitsV1";
}
impl sealed::Sealed for LimitsV1 {}
impl ReceiptRecord for LimitsV1 {}
impl Validate for LimitsV1 {
    fn validate(&self) -> Result<(), Error> {
        self.wall_ms.validate()?;
        self.memory_bytes.validate()?;
        self.memory_swap_bytes.validate()?;
        self.scratch_bytes.validate()?;
        self.stdout_bytes.validate()?;
        self.stderr_bytes.validate()?;
        self.artifact_bytes.validate()?;
        self.external_requests.validate()?;
        self.external_cost_microunits.validate()?;
        self.term_grace_ms.validate()?;
        self.cleanup_deadline_ms.validate()?;
        self.cpu_quota_percent.validate()?;
        self.tasks_max.validate()?;
        self.compiler_jobs.validate()?;
        self.julia_threads.validate()?;
        self.blas_threads.validate()?;
        self.currency.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `EffectV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectV1 {
    pub effect_id: Name,
    pub grant_id: Id,
    pub owner_id: Name,
    pub scope: Text,
    pub specification: Payload,
}
impl<'de> Deserialize<'de> for EffectV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            effect_id: Name,
            grant_id: Id,
            owner_id: Name,
            scope: Text,
            specification: Payload,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            effect_id: fields.effect_id,
            grant_id: fields.grant_id,
            owner_id: fields.owner_id,
            scope: fields.scope,
            specification: fields.specification,
        })
    }
}
impl Address for EffectV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:EffectV1";
}
impl sealed::Sealed for EffectV1 {}
impl ReceiptRecord for EffectV1 {}
impl Validate for EffectV1 {
    fn validate(&self) -> Result<(), Error> {
        self.effect_id.validate()?;
        self.grant_id.validate()?;
        self.owner_id.validate()?;
        self.scope.validate()?;
        self.specification.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `CleanupContractV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CleanupContractV1 {
    pub owner_id: Name,
    pub term_grace_ms: U64,
    pub deadline_ms: U64,
    pub require_empty_descendants: Bool,
    pub obligations: TypedRef<ObligationPageV1>,
    pub readback_specification: Payload,
}
impl<'de> Deserialize<'de> for CleanupContractV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            owner_id: Name,
            term_grace_ms: U64,
            deadline_ms: U64,
            require_empty_descendants: Bool,
            obligations: TypedRef<ObligationPageV1>,
            readback_specification: Payload,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            owner_id: fields.owner_id,
            term_grace_ms: fields.term_grace_ms,
            deadline_ms: fields.deadline_ms,
            require_empty_descendants: fields.require_empty_descendants,
            obligations: fields.obligations,
            readback_specification: fields.readback_specification,
        })
    }
}
impl Address for CleanupContractV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:CleanupContractV1";
}
impl sealed::Sealed for CleanupContractV1 {}
impl ReceiptRecord for CleanupContractV1 {}
impl Validate for CleanupContractV1 {
    fn validate(&self) -> Result<(), Error> {
        self.owner_id.validate()?;
        self.term_grace_ms.validate()?;
        self.deadline_ms.validate()?;
        self.require_empty_descendants.validate()?;
        self.obligations.validate()?;
        self.readback_specification.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `HostV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostV1 {
    pub os: Name,
    pub release: Name,
    pub architecture: Name,
    pub kernel: Text,
    pub boot_id: Name,
    pub logical_cpus: Count,
    pub memory_bytes: U64,
    pub facts: Payload,
}
impl<'de> Deserialize<'de> for HostV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            os: Name,
            release: Name,
            architecture: Name,
            kernel: Text,
            boot_id: Name,
            logical_cpus: Count,
            memory_bytes: U64,
            facts: Payload,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            os: fields.os,
            release: fields.release,
            architecture: fields.architecture,
            kernel: fields.kernel,
            boot_id: fields.boot_id,
            logical_cpus: fields.logical_cpus,
            memory_bytes: fields.memory_bytes,
            facts: fields.facts,
        })
    }
}
impl Address for HostV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:HostV1";
}
impl sealed::Sealed for HostV1 {}
impl ReceiptRecord for HostV1 {}
impl Validate for HostV1 {
    fn validate(&self) -> Result<(), Error> {
        self.os.validate()?;
        self.release.validate()?;
        self.architecture.validate()?;
        self.kernel.validate()?;
        self.boot_id.validate()?;
        self.logical_cpus.validate()?;
        self.memory_bytes.validate()?;
        self.facts.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ResourceV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResourceV1 {
    pub metric: Name,
    pub unit: Name,
    pub value: Maybe<U64>,
    pub limit: Maybe<U64>,
    pub limit_event: Bool,
    pub evidence: Ref,
}
impl<'de> Deserialize<'de> for ResourceV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            metric: Name,
            unit: Name,
            value: Maybe<U64>,
            limit: Maybe<U64>,
            limit_event: Bool,
            evidence: Ref,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            metric: fields.metric,
            unit: fields.unit,
            value: fields.value,
            limit: fields.limit,
            limit_event: fields.limit_event,
            evidence: fields.evidence,
        })
    }
}
impl Address for ResourceV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ResourceV1";
}
impl sealed::Sealed for ResourceV1 {}
impl ReceiptRecord for ResourceV1 {}
impl Validate for ResourceV1 {
    fn validate(&self) -> Result<(), Error> {
        self.metric.validate()?;
        self.unit.validate()?;
        self.value.validate()?;
        self.limit.validate()?;
        self.limit_event.validate()?;
        self.evidence.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ObligationV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObligationV1 {
    pub obligation_id: Id,
    pub owner_id: Name,
    pub scope: Text,
    pub material: Bool,
    pub state: ObligationV1State,
    pub evidence: List<Ref>,
    pub reason: Text,
}
impl<'de> Deserialize<'de> for ObligationV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            obligation_id: Id,
            owner_id: Name,
            scope: Text,
            material: Bool,
            state: ObligationV1State,
            evidence: List<Ref>,
            reason: Text,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            obligation_id: fields.obligation_id,
            owner_id: fields.owner_id,
            scope: fields.scope,
            material: fields.material,
            state: fields.state,
            evidence: fields.evidence,
            reason: fields.reason,
        })
    }
}
impl Address for ObligationV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ObligationV1";
}
impl sealed::Sealed for ObligationV1 {}
impl ReceiptRecord for ObligationV1 {}
impl Validate for ObligationV1 {
    fn validate(&self) -> Result<(), Error> {
        self.obligation_id.validate()?;
        self.owner_id.validate()?;
        self.scope.validate()?;
        self.material.validate()?;
        self.state.validate()?;
        self.evidence.validate()?;
        self.reason.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `DiagnosticV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticV1 {
    pub tool_id: Name,
    pub baseline: Bool,
    pub warning_count: Count,
    pub error_count: Count,
    pub stdout: Payload,
    pub stderr: Payload,
    pub stdout_truncated: Bool,
    pub stderr_truncated: Bool,
    pub producer: ProducerV1,
}
impl<'de> Deserialize<'de> for DiagnosticV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            tool_id: Name,
            baseline: Bool,
            warning_count: Count,
            error_count: Count,
            stdout: Payload,
            stderr: Payload,
            stdout_truncated: Bool,
            stderr_truncated: Bool,
            producer: ProducerV1,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            tool_id: fields.tool_id,
            baseline: fields.baseline,
            warning_count: fields.warning_count,
            error_count: fields.error_count,
            stdout: fields.stdout,
            stderr: fields.stderr,
            stdout_truncated: fields.stdout_truncated,
            stderr_truncated: fields.stderr_truncated,
            producer: fields.producer,
        })
    }
}
impl Address for DiagnosticV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:DiagnosticV1";
}
impl sealed::Sealed for DiagnosticV1 {}
impl ReceiptRecord for DiagnosticV1 {}
impl Validate for DiagnosticV1 {
    fn validate(&self) -> Result<(), Error> {
        self.tool_id.validate()?;
        self.baseline.validate()?;
        self.warning_count.validate()?;
        self.error_count.validate()?;
        self.stdout.validate()?;
        self.stderr.validate()?;
        self.stdout_truncated.validate()?;
        self.stderr_truncated.validate()?;
        self.producer.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ArtifactV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArtifactV1 {
    pub object: Ref,
    pub role: Name,
    pub required: Bool,
    pub truncated: Bool,
    pub availability: ArtifactV1Availability,
    pub reason: Text,
}
impl<'de> Deserialize<'de> for ArtifactV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            object: Ref,
            role: Name,
            required: Bool,
            truncated: Bool,
            availability: ArtifactV1Availability,
            reason: Text,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            object: fields.object,
            role: fields.role,
            required: fields.required,
            truncated: fields.truncated,
            availability: fields.availability,
            reason: fields.reason,
        })
    }
}
impl Address for ArtifactV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ArtifactV1";
}
impl sealed::Sealed for ArtifactV1 {}
impl ReceiptRecord for ArtifactV1 {}
impl Validate for ArtifactV1 {
    fn validate(&self) -> Result<(), Error> {
        self.object.validate()?;
        self.role.validate()?;
        self.required.validate()?;
        self.truncated.validate()?;
        self.availability.validate()?;
        self.reason.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `CampaignV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CampaignV1 {
    pub campaign_id: Name,
    pub language: CampaignV1Language,
    pub family_id: Name,
    pub tool: Payload,
    pub config: Payload,
    pub baseline_receipt: TypedRef<ReceiptV1>,
    pub mutants: TypedRef<MutantPageV1>,
    pub planned_mutants: Count,
}
impl<'de> Deserialize<'de> for CampaignV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            campaign_id: Name,
            language: CampaignV1Language,
            family_id: Name,
            tool: Payload,
            config: Payload,
            baseline_receipt: TypedRef<ReceiptV1>,
            mutants: TypedRef<MutantPageV1>,
            planned_mutants: Count,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            campaign_id: fields.campaign_id,
            language: fields.language,
            family_id: fields.family_id,
            tool: fields.tool,
            config: fields.config,
            baseline_receipt: fields.baseline_receipt,
            mutants: fields.mutants,
            planned_mutants: fields.planned_mutants,
        })
    }
}
impl Address for CampaignV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:CampaignV1";
}
impl sealed::Sealed for CampaignV1 {}
impl ReceiptRecord for CampaignV1 {}
impl Validate for CampaignV1 {
    fn validate(&self) -> Result<(), Error> {
        self.campaign_id.validate()?;
        self.language.validate()?;
        self.family_id.validate()?;
        self.tool.validate()?;
        self.config.validate()?;
        self.baseline_receipt.validate()?;
        self.mutants.validate()?;
        self.planned_mutants.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `MutantV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MutantV1 {
    pub mutant_id: Name,
    pub campaign_id: Name,
    pub baseline_subject_sha256: Sha,
    pub diff: Payload,
    pub expected_detector: Name,
    pub observed_detector: Maybe<Name>,
    pub outcome: MutantV1Outcome,
    pub executed: Bool,
    pub reason: Text,
    pub raw_evidence_refs: List<Ref>,
    pub review_ref: Maybe<TypedRef<ReviewV1>>,
}
impl<'de> Deserialize<'de> for MutantV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            mutant_id: Name,
            campaign_id: Name,
            baseline_subject_sha256: Sha,
            diff: Payload,
            expected_detector: Name,
            observed_detector: Maybe<Name>,
            outcome: MutantV1Outcome,
            executed: Bool,
            reason: Text,
            raw_evidence_refs: List<Ref>,
            review_ref: Maybe<TypedRef<ReviewV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            mutant_id: fields.mutant_id,
            campaign_id: fields.campaign_id,
            baseline_subject_sha256: fields.baseline_subject_sha256,
            diff: fields.diff,
            expected_detector: fields.expected_detector,
            observed_detector: fields.observed_detector,
            outcome: fields.outcome,
            executed: fields.executed,
            reason: fields.reason,
            raw_evidence_refs: fields.raw_evidence_refs,
            review_ref: fields.review_ref,
        })
    }
}
impl Address for MutantV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:MutantV1";
}
impl sealed::Sealed for MutantV1 {}
impl ReceiptRecord for MutantV1 {}
impl Validate for MutantV1 {
    fn validate(&self) -> Result<(), Error> {
        self.mutant_id.validate()?;
        self.campaign_id.validate()?;
        self.baseline_subject_sha256.validate()?;
        self.diff.validate()?;
        self.expected_detector.validate()?;
        self.observed_detector.validate()?;
        self.outcome.validate()?;
        self.executed.validate()?;
        self.reason.validate()?;
        self.raw_evidence_refs.validate()?;
        self.review_ref.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `AssumptionV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AssumptionV1 {
    pub assumption_id: Name,
    pub shared_with: List<Name>,
    pub statement: Reason,
    pub evidence: List<Ref>,
}
impl<'de> Deserialize<'de> for AssumptionV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            assumption_id: Name,
            shared_with: List<Name>,
            statement: Reason,
            evidence: List<Ref>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            assumption_id: fields.assumption_id,
            shared_with: fields.shared_with,
            statement: fields.statement,
            evidence: fields.evidence,
        })
    }
}
impl Address for AssumptionV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:AssumptionV1";
}
impl sealed::Sealed for AssumptionV1 {}
impl ReceiptRecord for AssumptionV1 {}
impl Validate for AssumptionV1 {
    fn validate(&self) -> Result<(), Error> {
        self.assumption_id.validate()?;
        self.shared_with.validate()?;
        self.statement.validate()?;
        self.evidence.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `FindingV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FindingV1 {
    pub finding_id: Name,
    pub owner_id: Name,
    pub scope: Text,
    pub material: Bool,
    pub disposition: FindingV1Disposition,
    pub rationale: Reason,
    pub evidence: List<Ref>,
    pub pre_fix_evidence: List<Ref>,
    pub post_fix_evidence: List<Ref>,
}
impl<'de> Deserialize<'de> for FindingV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            finding_id: Name,
            owner_id: Name,
            scope: Text,
            material: Bool,
            disposition: FindingV1Disposition,
            rationale: Reason,
            evidence: List<Ref>,
            pre_fix_evidence: List<Ref>,
            post_fix_evidence: List<Ref>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            finding_id: fields.finding_id,
            owner_id: fields.owner_id,
            scope: fields.scope,
            material: fields.material,
            disposition: fields.disposition,
            rationale: fields.rationale,
            evidence: fields.evidence,
            pre_fix_evidence: fields.pre_fix_evidence,
            post_fix_evidence: fields.post_fix_evidence,
        })
    }
}
impl Address for FindingV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:FindingV1";
}
impl sealed::Sealed for FindingV1 {}
impl ReceiptRecord for FindingV1 {}
impl Validate for FindingV1 {
    fn validate(&self) -> Result<(), Error> {
        self.finding_id.validate()?;
        self.owner_id.validate()?;
        self.scope.validate()?;
        self.material.validate()?;
        self.disposition.validate()?;
        self.rationale.validate()?;
        self.evidence.validate()?;
        self.pre_fix_evidence.validate()?;
        self.post_fix_evidence.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `OracleResultV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OracleResultV1 {
    pub oracle_id: Name,
    pub expected: TypedRef<ExpectationV1>,
    pub result: OracleResultV1Result,
    pub detector_id: Maybe<Name>,
    pub raw_evidence_refs: List<Ref>,
    pub reason: Text,
}
impl<'de> Deserialize<'de> for OracleResultV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            oracle_id: Name,
            expected: TypedRef<ExpectationV1>,
            result: OracleResultV1Result,
            detector_id: Maybe<Name>,
            raw_evidence_refs: List<Ref>,
            reason: Text,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            oracle_id: fields.oracle_id,
            expected: fields.expected,
            result: fields.result,
            detector_id: fields.detector_id,
            raw_evidence_refs: fields.raw_evidence_refs,
            reason: fields.reason,
        })
    }
}
impl Address for OracleResultV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:OracleResultV1";
}
impl sealed::Sealed for OracleResultV1 {}
impl ReceiptRecord for OracleResultV1 {}
impl Validate for OracleResultV1 {
    fn validate(&self) -> Result<(), Error> {
        self.oracle_id.validate()?;
        self.expected.validate()?;
        self.result.validate()?;
        self.detector_id.validate()?;
        self.raw_evidence_refs.validate()?;
        self.reason.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `MissingObjectV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MissingObjectV1 {
    pub artifact_id: Id,
    pub expected_sha256: Sha,
    pub expected_byte_length: Count,
    pub expected_schema_id: Name,
    pub reason: Reason,
}
impl<'de> Deserialize<'de> for MissingObjectV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            artifact_id: Id,
            expected_sha256: Sha,
            expected_byte_length: Count,
            expected_schema_id: Name,
            reason: Reason,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            artifact_id: fields.artifact_id,
            expected_sha256: fields.expected_sha256,
            expected_byte_length: fields.expected_byte_length,
            expected_schema_id: fields.expected_schema_id,
            reason: fields.reason,
        })
    }
}
impl Address for MissingObjectV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:MissingObjectV1";
}
impl sealed::Sealed for MissingObjectV1 {}
impl ReceiptRecord for MissingObjectV1 {}
impl Validate for MissingObjectV1 {
    fn validate(&self) -> Result<(), Error> {
        self.artifact_id.validate()?;
        self.expected_sha256.validate()?;
        self.expected_byte_length.validate()?;
        self.expected_schema_id.validate()?;
        self.reason.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `CasePageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CasePageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<CaseV1>,
    pub next: Maybe<TypedRef<CasePageV1>>,
}
impl<'de> Deserialize<'de> for CasePageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<CaseV1>,
            next: Maybe<TypedRef<CasePageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for CasePageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:CasePageV1";
}
impl sealed::Sealed for CasePageV1 {}
impl ReceiptRecord for CasePageV1 {}
impl Validate for CasePageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `SubjectFilePageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SubjectFilePageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<SubjectFileV1>,
    pub next: Maybe<TypedRef<SubjectFilePageV1>>,
}
impl<'de> Deserialize<'de> for SubjectFilePageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<SubjectFileV1>,
            next: Maybe<TypedRef<SubjectFilePageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for SubjectFilePageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:SubjectFilePageV1";
}
impl sealed::Sealed for SubjectFilePageV1 {}
impl ReceiptRecord for SubjectFilePageV1 {}
impl Validate for SubjectFilePageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `LockPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LockPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<LockV1>,
    pub next: Maybe<TypedRef<LockPageV1>>,
}
impl<'de> Deserialize<'de> for LockPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<LockV1>,
            next: Maybe<TypedRef<LockPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for LockPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:LockPageV1";
}
impl sealed::Sealed for LockPageV1 {}
impl ReceiptRecord for LockPageV1 {}
impl Validate for LockPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `DependencyPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<DependencyV1>,
    pub next: Maybe<TypedRef<DependencyPageV1>>,
}
impl<'de> Deserialize<'de> for DependencyPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<DependencyV1>,
            next: Maybe<TypedRef<DependencyPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for DependencyPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:DependencyPageV1";
}
impl sealed::Sealed for DependencyPageV1 {}
impl ReceiptRecord for DependencyPageV1 {}
impl Validate for DependencyPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ToolPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ToolPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<ToolV1>,
    pub next: Maybe<TypedRef<ToolPageV1>>,
}
impl<'de> Deserialize<'de> for ToolPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<ToolV1>,
            next: Maybe<TypedRef<ToolPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for ToolPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ToolPageV1";
}
impl sealed::Sealed for ToolPageV1 {}
impl ReceiptRecord for ToolPageV1 {}
impl Validate for ToolPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `LanguageFlagsPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LanguageFlagsPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<LanguageFlagsV1>,
    pub next: Maybe<TypedRef<LanguageFlagsPageV1>>,
}
impl<'de> Deserialize<'de> for LanguageFlagsPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<LanguageFlagsV1>,
            next: Maybe<TypedRef<LanguageFlagsPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for LanguageFlagsPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:LanguageFlagsPageV1";
}
impl sealed::Sealed for LanguageFlagsPageV1 {}
impl ReceiptRecord for LanguageFlagsPageV1 {}
impl Validate for LanguageFlagsPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `StandardPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StandardPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<StandardV1>,
    pub next: Maybe<TypedRef<StandardPageV1>>,
}
impl<'de> Deserialize<'de> for StandardPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<StandardV1>,
            next: Maybe<TypedRef<StandardPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for StandardPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:StandardPageV1";
}
impl sealed::Sealed for StandardPageV1 {}
impl ReceiptRecord for StandardPageV1 {}
impl Validate for StandardPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `EnvironmentPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnvironmentPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<EnvironmentV1>,
    pub next: Maybe<TypedRef<EnvironmentPageV1>>,
}
impl<'de> Deserialize<'de> for EnvironmentPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<EnvironmentV1>,
            next: Maybe<TypedRef<EnvironmentPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for EnvironmentPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:EnvironmentPageV1";
}
impl sealed::Sealed for EnvironmentPageV1 {}
impl ReceiptRecord for EnvironmentPageV1 {}
impl Validate for EnvironmentPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `GrantPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GrantPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<GrantV1>,
    pub next: Maybe<TypedRef<GrantPageV1>>,
}
impl<'de> Deserialize<'de> for GrantPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<GrantV1>,
            next: Maybe<TypedRef<GrantPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for GrantPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:GrantPageV1";
}
impl sealed::Sealed for GrantPageV1 {}
impl ReceiptRecord for GrantPageV1 {}
impl Validate for GrantPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `EffectPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<EffectV1>,
    pub next: Maybe<TypedRef<EffectPageV1>>,
}
impl<'de> Deserialize<'de> for EffectPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<EffectV1>,
            next: Maybe<TypedRef<EffectPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for EffectPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:EffectPageV1";
}
impl sealed::Sealed for EffectPageV1 {}
impl ReceiptRecord for EffectPageV1 {}
impl Validate for EffectPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ResourcePageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResourcePageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<ResourceV1>,
    pub next: Maybe<TypedRef<ResourcePageV1>>,
}
impl<'de> Deserialize<'de> for ResourcePageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<ResourceV1>,
            next: Maybe<TypedRef<ResourcePageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for ResourcePageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ResourcePageV1";
}
impl sealed::Sealed for ResourcePageV1 {}
impl ReceiptRecord for ResourcePageV1 {}
impl Validate for ResourcePageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ObligationPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObligationPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<ObligationV1>,
    pub next: Maybe<TypedRef<ObligationPageV1>>,
}
impl<'de> Deserialize<'de> for ObligationPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<ObligationV1>,
            next: Maybe<TypedRef<ObligationPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for ObligationPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ObligationPageV1";
}
impl sealed::Sealed for ObligationPageV1 {}
impl ReceiptRecord for ObligationPageV1 {}
impl Validate for ObligationPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `DiagnosticPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<DiagnosticV1>,
    pub next: Maybe<TypedRef<DiagnosticPageV1>>,
}
impl<'de> Deserialize<'de> for DiagnosticPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<DiagnosticV1>,
            next: Maybe<TypedRef<DiagnosticPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for DiagnosticPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:DiagnosticPageV1";
}
impl sealed::Sealed for DiagnosticPageV1 {}
impl ReceiptRecord for DiagnosticPageV1 {}
impl Validate for DiagnosticPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ArtifactPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArtifactPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<ArtifactV1>,
    pub next: Maybe<TypedRef<ArtifactPageV1>>,
}
impl<'de> Deserialize<'de> for ArtifactPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<ArtifactV1>,
            next: Maybe<TypedRef<ArtifactPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for ArtifactPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ArtifactPageV1";
}
impl sealed::Sealed for ArtifactPageV1 {}
impl ReceiptRecord for ArtifactPageV1 {}
impl Validate for ArtifactPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `CampaignPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CampaignPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<CampaignV1>,
    pub next: Maybe<TypedRef<CampaignPageV1>>,
}
impl<'de> Deserialize<'de> for CampaignPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<CampaignV1>,
            next: Maybe<TypedRef<CampaignPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for CampaignPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:CampaignPageV1";
}
impl sealed::Sealed for CampaignPageV1 {}
impl ReceiptRecord for CampaignPageV1 {}
impl Validate for CampaignPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `MutantPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MutantPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<MutantV1>,
    pub next: Maybe<TypedRef<MutantPageV1>>,
}
impl<'de> Deserialize<'de> for MutantPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<MutantV1>,
            next: Maybe<TypedRef<MutantPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for MutantPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:MutantPageV1";
}
impl sealed::Sealed for MutantPageV1 {}
impl ReceiptRecord for MutantPageV1 {}
impl Validate for MutantPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `AssumptionPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AssumptionPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<AssumptionV1>,
    pub next: Maybe<TypedRef<AssumptionPageV1>>,
}
impl<'de> Deserialize<'de> for AssumptionPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<AssumptionV1>,
            next: Maybe<TypedRef<AssumptionPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for AssumptionPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:AssumptionPageV1";
}
impl sealed::Sealed for AssumptionPageV1 {}
impl ReceiptRecord for AssumptionPageV1 {}
impl Validate for AssumptionPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `FindingPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FindingPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<FindingV1>,
    pub next: Maybe<TypedRef<FindingPageV1>>,
}
impl<'de> Deserialize<'de> for FindingPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<FindingV1>,
            next: Maybe<TypedRef<FindingPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for FindingPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:FindingPageV1";
}
impl sealed::Sealed for FindingPageV1 {}
impl ReceiptRecord for FindingPageV1 {}
impl Validate for FindingPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `MissingObjectPageV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MissingObjectPageV1 {
    pub page_index: Count,
    pub page_count: Count,
    pub row_count: Count,
    pub total_rows: Count,
    pub rows: List<MissingObjectV1>,
    pub next: Maybe<TypedRef<MissingObjectPageV1>>,
}
impl<'de> Deserialize<'de> for MissingObjectPageV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            page_index: Count,
            page_count: Count,
            row_count: Count,
            total_rows: Count,
            rows: List<MissingObjectV1>,
            next: Maybe<TypedRef<MissingObjectPageV1>>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            page_index: fields.page_index,
            page_count: fields.page_count,
            row_count: fields.row_count,
            total_rows: fields.total_rows,
            rows: fields.rows,
            next: fields.next,
        })
    }
}
impl Address for MissingObjectPageV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:MissingObjectPageV1";
}
impl sealed::Sealed for MissingObjectPageV1 {}
impl ReceiptRecord for MissingObjectPageV1 {}
impl Validate for MissingObjectPageV1 {
    fn validate(&self) -> Result<(), Error> {
        self.page_index.validate()?;
        self.page_count.validate()?;
        self.row_count.validate()?;
        self.total_rows.validate()?;
        self.rows.validate()?;
        self.next.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `ReviewReceiptV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReviewReceiptV1 {
    pub protocol: ReviewReceiptV1Protocol,
    pub version: Count,
    pub receipt: TypedRef<ReceiptV1>,
    pub review: ReviewV1,
}
impl<'de> Deserialize<'de> for ReviewReceiptV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            protocol: ReviewReceiptV1Protocol,
            version: Count,
            receipt: TypedRef<ReceiptV1>,
            review: ReviewV1,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            protocol: fields.protocol,
            version: fields.version,
            receipt: fields.receipt,
            review: fields.review,
        })
    }
}
impl Address for ReviewReceiptV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:ReviewReceiptV1";
}
impl sealed::Sealed for ReviewReceiptV1 {}
impl ReceiptRecord for ReviewReceiptV1 {}
impl Validate for ReviewReceiptV1 {
    fn validate(&self) -> Result<(), Error> {
        self.protocol.validate()?;
        self.version.validate()?;
        self.receipt.validate()?;
        self.review.validate()?;
        self.check()
    }
}

#[doc = "Closed RC04 `AvailabilityReceiptV1` record. Every listed field is required."]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AvailabilityReceiptV1 {
    pub protocol: AvailabilityReceiptV1Protocol,
    pub version: Count,
    pub receipt: TypedRef<ReceiptV1>,
    pub availability: AvailabilityV1,
}
impl<'de> Deserialize<'de> for AvailabilityReceiptV1 {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            protocol: AvailabilityReceiptV1Protocol,
            version: Count,
            receipt: TypedRef<ReceiptV1>,
            availability: AvailabilityV1,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            protocol: fields.protocol,
            version: fields.version,
            receipt: fields.receipt,
            availability: fields.availability,
        })
    }
}
impl Address for AvailabilityReceiptV1 {
    const SCHEMA_ID: &'static str = "hee3.receipt/1:AvailabilityReceiptV1";
}
impl sealed::Sealed for AvailabilityReceiptV1 {}
impl ReceiptRecord for AvailabilityReceiptV1 {}
impl Validate for AvailabilityReceiptV1 {
    fn validate(&self) -> Result<(), Error> {
        self.protocol.validate()?;
        self.version.validate()?;
        self.receipt.validate()?;
        self.availability.validate()?;
        self.check()
    }
}

/// Exactly the 61 concrete JSON schema addresses; raw is separate.
pub const SCHEMA_IDS: &[&str] = &[
    "hee3.receipt/1:IdentityV1",
    "hee3.receipt/1:SubjectsV1",
    "hee3.receipt/1:InvocationV1",
    "hee3.receipt/1:ObservationsV1",
    "hee3.receipt/1:ProducerV1",
    "hee3.receipt/1:CasesV1",
    "hee3.receipt/1:DiagnosticsV1",
    "hee3.receipt/1:ArtifactsV1",
    "hee3.receipt/1:MutationV1",
    "hee3.receipt/1:ReviewV1",
    "hee3.receipt/1:VerdictV1",
    "hee3.receipt/1:AvailabilityV1",
    "hee3.receipt/1:ReceiptV1",
    "hee3.receipt/1:ExpectedProducerV1",
    "hee3.receipt/1:CaseV1",
    "hee3.receipt/1:SubjectV1",
    "hee3.receipt/1:SubjectFileV1",
    "hee3.receipt/1:LockV1",
    "hee3.receipt/1:DependencyV1",
    "hee3.receipt/1:ToolV1",
    "hee3.receipt/1:BuildProfileV1",
    "hee3.receipt/1:LanguageFlagsV1",
    "hee3.receipt/1:StandardV1",
    "hee3.receipt/1:EnvironmentV1",
    "hee3.receipt/1:GrantV1",
    "hee3.receipt/1:ExpectationV1",
    "hee3.receipt/1:LimitsV1",
    "hee3.receipt/1:EffectV1",
    "hee3.receipt/1:CleanupContractV1",
    "hee3.receipt/1:HostV1",
    "hee3.receipt/1:ResourceV1",
    "hee3.receipt/1:ObligationV1",
    "hee3.receipt/1:DiagnosticV1",
    "hee3.receipt/1:ArtifactV1",
    "hee3.receipt/1:CampaignV1",
    "hee3.receipt/1:MutantV1",
    "hee3.receipt/1:AssumptionV1",
    "hee3.receipt/1:FindingV1",
    "hee3.receipt/1:OracleResultV1",
    "hee3.receipt/1:MissingObjectV1",
    "hee3.receipt/1:CasePageV1",
    "hee3.receipt/1:SubjectFilePageV1",
    "hee3.receipt/1:LockPageV1",
    "hee3.receipt/1:DependencyPageV1",
    "hee3.receipt/1:ToolPageV1",
    "hee3.receipt/1:LanguageFlagsPageV1",
    "hee3.receipt/1:StandardPageV1",
    "hee3.receipt/1:EnvironmentPageV1",
    "hee3.receipt/1:GrantPageV1",
    "hee3.receipt/1:EffectPageV1",
    "hee3.receipt/1:ResourcePageV1",
    "hee3.receipt/1:ObligationPageV1",
    "hee3.receipt/1:DiagnosticPageV1",
    "hee3.receipt/1:ArtifactPageV1",
    "hee3.receipt/1:CampaignPageV1",
    "hee3.receipt/1:MutantPageV1",
    "hee3.receipt/1:AssumptionPageV1",
    "hee3.receipt/1:FindingPageV1",
    "hee3.receipt/1:MissingObjectPageV1",
    "hee3.receipt/1:ReviewReceiptV1",
    "hee3.receipt/1:AvailabilityReceiptV1",
];

/// A decoded record selected by its exact external schema metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Record {
    IdentityV1(Box<IdentityV1>),
    SubjectsV1(Box<SubjectsV1>),
    InvocationV1(Box<InvocationV1>),
    ObservationsV1(Box<ObservationsV1>),
    ProducerV1(Box<ProducerV1>),
    CasesV1(Box<CasesV1>),
    DiagnosticsV1(Box<DiagnosticsV1>),
    ArtifactsV1(Box<ArtifactsV1>),
    MutationV1(Box<MutationV1>),
    ReviewV1(Box<ReviewV1>),
    VerdictV1(Box<VerdictV1>),
    AvailabilityV1(Box<AvailabilityV1>),
    ReceiptV1(Box<ReceiptV1>),
    ExpectedProducerV1(Box<ExpectedProducerV1>),
    CaseV1(Box<CaseV1>),
    SubjectV1(Box<SubjectV1>),
    SubjectFileV1(Box<SubjectFileV1>),
    LockV1(Box<LockV1>),
    DependencyV1(Box<DependencyV1>),
    ToolV1(Box<ToolV1>),
    BuildProfileV1(Box<BuildProfileV1>),
    LanguageFlagsV1(Box<LanguageFlagsV1>),
    StandardV1(Box<StandardV1>),
    EnvironmentV1(Box<EnvironmentV1>),
    GrantV1(Box<GrantV1>),
    ExpectationV1(Box<ExpectationV1>),
    LimitsV1(Box<LimitsV1>),
    EffectV1(Box<EffectV1>),
    CleanupContractV1(Box<CleanupContractV1>),
    HostV1(Box<HostV1>),
    ResourceV1(Box<ResourceV1>),
    ObligationV1(Box<ObligationV1>),
    DiagnosticV1(Box<DiagnosticV1>),
    ArtifactV1(Box<ArtifactV1>),
    CampaignV1(Box<CampaignV1>),
    MutantV1(Box<MutantV1>),
    AssumptionV1(Box<AssumptionV1>),
    FindingV1(Box<FindingV1>),
    OracleResultV1(Box<OracleResultV1>),
    MissingObjectV1(Box<MissingObjectV1>),
    CasePageV1(Box<CasePageV1>),
    SubjectFilePageV1(Box<SubjectFilePageV1>),
    LockPageV1(Box<LockPageV1>),
    DependencyPageV1(Box<DependencyPageV1>),
    ToolPageV1(Box<ToolPageV1>),
    LanguageFlagsPageV1(Box<LanguageFlagsPageV1>),
    StandardPageV1(Box<StandardPageV1>),
    EnvironmentPageV1(Box<EnvironmentPageV1>),
    GrantPageV1(Box<GrantPageV1>),
    EffectPageV1(Box<EffectPageV1>),
    ResourcePageV1(Box<ResourcePageV1>),
    ObligationPageV1(Box<ObligationPageV1>),
    DiagnosticPageV1(Box<DiagnosticPageV1>),
    ArtifactPageV1(Box<ArtifactPageV1>),
    CampaignPageV1(Box<CampaignPageV1>),
    MutantPageV1(Box<MutantPageV1>),
    AssumptionPageV1(Box<AssumptionPageV1>),
    FindingPageV1(Box<FindingPageV1>),
    MissingObjectPageV1(Box<MissingObjectPageV1>),
    ReviewReceiptV1(Box<ReviewReceiptV1>),
    AvailabilityReceiptV1(Box<AvailabilityReceiptV1>),
}
impl Record {
    /// Exact selected schema, never inferred from overlapping field shapes.
    #[must_use]
    pub fn schema_id(&self) -> &'static str {
        match self {
            Self::IdentityV1(_) => IdentityV1::SCHEMA_ID,
            Self::SubjectsV1(_) => SubjectsV1::SCHEMA_ID,
            Self::InvocationV1(_) => InvocationV1::SCHEMA_ID,
            Self::ObservationsV1(_) => ObservationsV1::SCHEMA_ID,
            Self::ProducerV1(_) => ProducerV1::SCHEMA_ID,
            Self::CasesV1(_) => CasesV1::SCHEMA_ID,
            Self::DiagnosticsV1(_) => DiagnosticsV1::SCHEMA_ID,
            Self::ArtifactsV1(_) => ArtifactsV1::SCHEMA_ID,
            Self::MutationV1(_) => MutationV1::SCHEMA_ID,
            Self::ReviewV1(_) => ReviewV1::SCHEMA_ID,
            Self::VerdictV1(_) => VerdictV1::SCHEMA_ID,
            Self::AvailabilityV1(_) => AvailabilityV1::SCHEMA_ID,
            Self::ReceiptV1(_) => ReceiptV1::SCHEMA_ID,
            Self::ExpectedProducerV1(_) => ExpectedProducerV1::SCHEMA_ID,
            Self::CaseV1(_) => CaseV1::SCHEMA_ID,
            Self::SubjectV1(_) => SubjectV1::SCHEMA_ID,
            Self::SubjectFileV1(_) => SubjectFileV1::SCHEMA_ID,
            Self::LockV1(_) => LockV1::SCHEMA_ID,
            Self::DependencyV1(_) => DependencyV1::SCHEMA_ID,
            Self::ToolV1(_) => ToolV1::SCHEMA_ID,
            Self::BuildProfileV1(_) => BuildProfileV1::SCHEMA_ID,
            Self::LanguageFlagsV1(_) => LanguageFlagsV1::SCHEMA_ID,
            Self::StandardV1(_) => StandardV1::SCHEMA_ID,
            Self::EnvironmentV1(_) => EnvironmentV1::SCHEMA_ID,
            Self::GrantV1(_) => GrantV1::SCHEMA_ID,
            Self::ExpectationV1(_) => ExpectationV1::SCHEMA_ID,
            Self::LimitsV1(_) => LimitsV1::SCHEMA_ID,
            Self::EffectV1(_) => EffectV1::SCHEMA_ID,
            Self::CleanupContractV1(_) => CleanupContractV1::SCHEMA_ID,
            Self::HostV1(_) => HostV1::SCHEMA_ID,
            Self::ResourceV1(_) => ResourceV1::SCHEMA_ID,
            Self::ObligationV1(_) => ObligationV1::SCHEMA_ID,
            Self::DiagnosticV1(_) => DiagnosticV1::SCHEMA_ID,
            Self::ArtifactV1(_) => ArtifactV1::SCHEMA_ID,
            Self::CampaignV1(_) => CampaignV1::SCHEMA_ID,
            Self::MutantV1(_) => MutantV1::SCHEMA_ID,
            Self::AssumptionV1(_) => AssumptionV1::SCHEMA_ID,
            Self::FindingV1(_) => FindingV1::SCHEMA_ID,
            Self::OracleResultV1(_) => OracleResultV1::SCHEMA_ID,
            Self::MissingObjectV1(_) => MissingObjectV1::SCHEMA_ID,
            Self::CasePageV1(_) => CasePageV1::SCHEMA_ID,
            Self::SubjectFilePageV1(_) => SubjectFilePageV1::SCHEMA_ID,
            Self::LockPageV1(_) => LockPageV1::SCHEMA_ID,
            Self::DependencyPageV1(_) => DependencyPageV1::SCHEMA_ID,
            Self::ToolPageV1(_) => ToolPageV1::SCHEMA_ID,
            Self::LanguageFlagsPageV1(_) => LanguageFlagsPageV1::SCHEMA_ID,
            Self::StandardPageV1(_) => StandardPageV1::SCHEMA_ID,
            Self::EnvironmentPageV1(_) => EnvironmentPageV1::SCHEMA_ID,
            Self::GrantPageV1(_) => GrantPageV1::SCHEMA_ID,
            Self::EffectPageV1(_) => EffectPageV1::SCHEMA_ID,
            Self::ResourcePageV1(_) => ResourcePageV1::SCHEMA_ID,
            Self::ObligationPageV1(_) => ObligationPageV1::SCHEMA_ID,
            Self::DiagnosticPageV1(_) => DiagnosticPageV1::SCHEMA_ID,
            Self::ArtifactPageV1(_) => ArtifactPageV1::SCHEMA_ID,
            Self::CampaignPageV1(_) => CampaignPageV1::SCHEMA_ID,
            Self::MutantPageV1(_) => MutantPageV1::SCHEMA_ID,
            Self::AssumptionPageV1(_) => AssumptionPageV1::SCHEMA_ID,
            Self::FindingPageV1(_) => FindingPageV1::SCHEMA_ID,
            Self::MissingObjectPageV1(_) => MissingObjectPageV1::SCHEMA_ID,
            Self::ReviewReceiptV1(_) => ReviewReceiptV1::SCHEMA_ID,
            Self::AvailabilityReceiptV1(_) => AvailabilityReceiptV1::SCHEMA_ID,
        }
    }
    /// Validate and serialize this one record.
    /// # Errors
    /// Refuses a local invariant or encoded byte bound.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        match self {
            Self::IdentityV1(value) => super::encode(value.as_ref()),
            Self::SubjectsV1(value) => super::encode(value.as_ref()),
            Self::InvocationV1(value) => super::encode(value.as_ref()),
            Self::ObservationsV1(value) => super::encode(value.as_ref()),
            Self::ProducerV1(value) => super::encode(value.as_ref()),
            Self::CasesV1(value) => super::encode(value.as_ref()),
            Self::DiagnosticsV1(value) => super::encode(value.as_ref()),
            Self::ArtifactsV1(value) => super::encode(value.as_ref()),
            Self::MutationV1(value) => super::encode(value.as_ref()),
            Self::ReviewV1(value) => super::encode(value.as_ref()),
            Self::VerdictV1(value) => super::encode(value.as_ref()),
            Self::AvailabilityV1(value) => super::encode(value.as_ref()),
            Self::ReceiptV1(value) => super::encode(value.as_ref()),
            Self::ExpectedProducerV1(value) => super::encode(value.as_ref()),
            Self::CaseV1(value) => super::encode(value.as_ref()),
            Self::SubjectV1(value) => super::encode(value.as_ref()),
            Self::SubjectFileV1(value) => super::encode(value.as_ref()),
            Self::LockV1(value) => super::encode(value.as_ref()),
            Self::DependencyV1(value) => super::encode(value.as_ref()),
            Self::ToolV1(value) => super::encode(value.as_ref()),
            Self::BuildProfileV1(value) => super::encode(value.as_ref()),
            Self::LanguageFlagsV1(value) => super::encode(value.as_ref()),
            Self::StandardV1(value) => super::encode(value.as_ref()),
            Self::EnvironmentV1(value) => super::encode(value.as_ref()),
            Self::GrantV1(value) => super::encode(value.as_ref()),
            Self::ExpectationV1(value) => super::encode(value.as_ref()),
            Self::LimitsV1(value) => super::encode(value.as_ref()),
            Self::EffectV1(value) => super::encode(value.as_ref()),
            Self::CleanupContractV1(value) => super::encode(value.as_ref()),
            Self::HostV1(value) => super::encode(value.as_ref()),
            Self::ResourceV1(value) => super::encode(value.as_ref()),
            Self::ObligationV1(value) => super::encode(value.as_ref()),
            Self::DiagnosticV1(value) => super::encode(value.as_ref()),
            Self::ArtifactV1(value) => super::encode(value.as_ref()),
            Self::CampaignV1(value) => super::encode(value.as_ref()),
            Self::MutantV1(value) => super::encode(value.as_ref()),
            Self::AssumptionV1(value) => super::encode(value.as_ref()),
            Self::FindingV1(value) => super::encode(value.as_ref()),
            Self::OracleResultV1(value) => super::encode(value.as_ref()),
            Self::MissingObjectV1(value) => super::encode(value.as_ref()),
            Self::CasePageV1(value) => super::encode(value.as_ref()),
            Self::SubjectFilePageV1(value) => super::encode(value.as_ref()),
            Self::LockPageV1(value) => super::encode(value.as_ref()),
            Self::DependencyPageV1(value) => super::encode(value.as_ref()),
            Self::ToolPageV1(value) => super::encode(value.as_ref()),
            Self::LanguageFlagsPageV1(value) => super::encode(value.as_ref()),
            Self::StandardPageV1(value) => super::encode(value.as_ref()),
            Self::EnvironmentPageV1(value) => super::encode(value.as_ref()),
            Self::GrantPageV1(value) => super::encode(value.as_ref()),
            Self::EffectPageV1(value) => super::encode(value.as_ref()),
            Self::ResourcePageV1(value) => super::encode(value.as_ref()),
            Self::ObligationPageV1(value) => super::encode(value.as_ref()),
            Self::DiagnosticPageV1(value) => super::encode(value.as_ref()),
            Self::ArtifactPageV1(value) => super::encode(value.as_ref()),
            Self::CampaignPageV1(value) => super::encode(value.as_ref()),
            Self::MutantPageV1(value) => super::encode(value.as_ref()),
            Self::AssumptionPageV1(value) => super::encode(value.as_ref()),
            Self::FindingPageV1(value) => super::encode(value.as_ref()),
            Self::MissingObjectPageV1(value) => super::encode(value.as_ref()),
            Self::ReviewReceiptV1(value) => super::encode(value.as_ref()),
            Self::AvailabilityReceiptV1(value) => super::encode(value.as_ref()),
        }
    }
}
pub(super) fn decode_named(schema_id: &str, bytes: &[u8]) -> Result<Record, Error> {
    match schema_id {
        IdentityV1::SCHEMA_ID => Ok(Record::IdentityV1(Box::new(super::decode(bytes)?))),
        SubjectsV1::SCHEMA_ID => Ok(Record::SubjectsV1(Box::new(super::decode(bytes)?))),
        InvocationV1::SCHEMA_ID => Ok(Record::InvocationV1(Box::new(super::decode(bytes)?))),
        ObservationsV1::SCHEMA_ID => Ok(Record::ObservationsV1(Box::new(super::decode(bytes)?))),
        ProducerV1::SCHEMA_ID => Ok(Record::ProducerV1(Box::new(super::decode(bytes)?))),
        CasesV1::SCHEMA_ID => Ok(Record::CasesV1(Box::new(super::decode(bytes)?))),
        DiagnosticsV1::SCHEMA_ID => Ok(Record::DiagnosticsV1(Box::new(super::decode(bytes)?))),
        ArtifactsV1::SCHEMA_ID => Ok(Record::ArtifactsV1(Box::new(super::decode(bytes)?))),
        MutationV1::SCHEMA_ID => Ok(Record::MutationV1(Box::new(super::decode(bytes)?))),
        ReviewV1::SCHEMA_ID => Ok(Record::ReviewV1(Box::new(super::decode(bytes)?))),
        VerdictV1::SCHEMA_ID => Ok(Record::VerdictV1(Box::new(super::decode(bytes)?))),
        AvailabilityV1::SCHEMA_ID => Ok(Record::AvailabilityV1(Box::new(super::decode(bytes)?))),
        ReceiptV1::SCHEMA_ID => Ok(Record::ReceiptV1(Box::new(super::decode(bytes)?))),
        ExpectedProducerV1::SCHEMA_ID => {
            Ok(Record::ExpectedProducerV1(Box::new(super::decode(bytes)?)))
        }
        CaseV1::SCHEMA_ID => Ok(Record::CaseV1(Box::new(super::decode(bytes)?))),
        SubjectV1::SCHEMA_ID => Ok(Record::SubjectV1(Box::new(super::decode(bytes)?))),
        SubjectFileV1::SCHEMA_ID => Ok(Record::SubjectFileV1(Box::new(super::decode(bytes)?))),
        LockV1::SCHEMA_ID => Ok(Record::LockV1(Box::new(super::decode(bytes)?))),
        DependencyV1::SCHEMA_ID => Ok(Record::DependencyV1(Box::new(super::decode(bytes)?))),
        ToolV1::SCHEMA_ID => Ok(Record::ToolV1(Box::new(super::decode(bytes)?))),
        BuildProfileV1::SCHEMA_ID => Ok(Record::BuildProfileV1(Box::new(super::decode(bytes)?))),
        LanguageFlagsV1::SCHEMA_ID => Ok(Record::LanguageFlagsV1(Box::new(super::decode(bytes)?))),
        StandardV1::SCHEMA_ID => Ok(Record::StandardV1(Box::new(super::decode(bytes)?))),
        EnvironmentV1::SCHEMA_ID => Ok(Record::EnvironmentV1(Box::new(super::decode(bytes)?))),
        GrantV1::SCHEMA_ID => Ok(Record::GrantV1(Box::new(super::decode(bytes)?))),
        ExpectationV1::SCHEMA_ID => Ok(Record::ExpectationV1(Box::new(super::decode(bytes)?))),
        LimitsV1::SCHEMA_ID => Ok(Record::LimitsV1(Box::new(super::decode(bytes)?))),
        EffectV1::SCHEMA_ID => Ok(Record::EffectV1(Box::new(super::decode(bytes)?))),
        CleanupContractV1::SCHEMA_ID => {
            Ok(Record::CleanupContractV1(Box::new(super::decode(bytes)?)))
        }
        HostV1::SCHEMA_ID => Ok(Record::HostV1(Box::new(super::decode(bytes)?))),
        ResourceV1::SCHEMA_ID => Ok(Record::ResourceV1(Box::new(super::decode(bytes)?))),
        ObligationV1::SCHEMA_ID => Ok(Record::ObligationV1(Box::new(super::decode(bytes)?))),
        DiagnosticV1::SCHEMA_ID => Ok(Record::DiagnosticV1(Box::new(super::decode(bytes)?))),
        ArtifactV1::SCHEMA_ID => Ok(Record::ArtifactV1(Box::new(super::decode(bytes)?))),
        CampaignV1::SCHEMA_ID => Ok(Record::CampaignV1(Box::new(super::decode(bytes)?))),
        MutantV1::SCHEMA_ID => Ok(Record::MutantV1(Box::new(super::decode(bytes)?))),
        AssumptionV1::SCHEMA_ID => Ok(Record::AssumptionV1(Box::new(super::decode(bytes)?))),
        FindingV1::SCHEMA_ID => Ok(Record::FindingV1(Box::new(super::decode(bytes)?))),
        OracleResultV1::SCHEMA_ID => Ok(Record::OracleResultV1(Box::new(super::decode(bytes)?))),
        MissingObjectV1::SCHEMA_ID => Ok(Record::MissingObjectV1(Box::new(super::decode(bytes)?))),
        CasePageV1::SCHEMA_ID => Ok(Record::CasePageV1(Box::new(super::decode(bytes)?))),
        SubjectFilePageV1::SCHEMA_ID => {
            Ok(Record::SubjectFilePageV1(Box::new(super::decode(bytes)?)))
        }
        LockPageV1::SCHEMA_ID => Ok(Record::LockPageV1(Box::new(super::decode(bytes)?))),
        DependencyPageV1::SCHEMA_ID => {
            Ok(Record::DependencyPageV1(Box::new(super::decode(bytes)?)))
        }
        ToolPageV1::SCHEMA_ID => Ok(Record::ToolPageV1(Box::new(super::decode(bytes)?))),
        LanguageFlagsPageV1::SCHEMA_ID => {
            Ok(Record::LanguageFlagsPageV1(Box::new(super::decode(bytes)?)))
        }
        StandardPageV1::SCHEMA_ID => Ok(Record::StandardPageV1(Box::new(super::decode(bytes)?))),
        EnvironmentPageV1::SCHEMA_ID => {
            Ok(Record::EnvironmentPageV1(Box::new(super::decode(bytes)?)))
        }
        GrantPageV1::SCHEMA_ID => Ok(Record::GrantPageV1(Box::new(super::decode(bytes)?))),
        EffectPageV1::SCHEMA_ID => Ok(Record::EffectPageV1(Box::new(super::decode(bytes)?))),
        ResourcePageV1::SCHEMA_ID => Ok(Record::ResourcePageV1(Box::new(super::decode(bytes)?))),
        ObligationPageV1::SCHEMA_ID => {
            Ok(Record::ObligationPageV1(Box::new(super::decode(bytes)?)))
        }
        DiagnosticPageV1::SCHEMA_ID => {
            Ok(Record::DiagnosticPageV1(Box::new(super::decode(bytes)?)))
        }
        ArtifactPageV1::SCHEMA_ID => Ok(Record::ArtifactPageV1(Box::new(super::decode(bytes)?))),
        CampaignPageV1::SCHEMA_ID => Ok(Record::CampaignPageV1(Box::new(super::decode(bytes)?))),
        MutantPageV1::SCHEMA_ID => Ok(Record::MutantPageV1(Box::new(super::decode(bytes)?))),
        AssumptionPageV1::SCHEMA_ID => {
            Ok(Record::AssumptionPageV1(Box::new(super::decode(bytes)?)))
        }
        FindingPageV1::SCHEMA_ID => Ok(Record::FindingPageV1(Box::new(super::decode(bytes)?))),
        MissingObjectPageV1::SCHEMA_ID => {
            Ok(Record::MissingObjectPageV1(Box::new(super::decode(bytes)?)))
        }
        ReviewReceiptV1::SCHEMA_ID => Ok(Record::ReviewReceiptV1(Box::new(super::decode(bytes)?))),
        AvailabilityReceiptV1::SCHEMA_ID => Ok(Record::AvailabilityReceiptV1(Box::new(
            super::decode(bytes)?,
        ))),
        _ => Err(Error::Schema),
    }
}
