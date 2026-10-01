//! Local RC04 conditions only; reference graph and observed truth remain external.
use super::super::require;
use super::{
    Address, ArtifactPageV1, ArtifactV1, ArtifactsV1, AssumptionPageV1, AssumptionV1,
    AvailabilityReceiptV1, AvailabilityV1, AvailabilityV1State, BuildProfileV1, CampaignPageV1,
    CampaignV1, CasePageV1, CaseV1, CaseV1Outcome, CasesV1, CleanupContractV1, Count,
    DependencyPageV1, DependencyV1, DiagnosticPageV1, DiagnosticV1, DiagnosticsV1, EffectPageV1,
    EffectV1, EnvironmentPageV1, EnvironmentV1, Error, ExpectationV1, ExpectedProducerV1,
    ExpectedProducerV1Status, FindingPageV1, FindingV1, GrantPageV1, GrantV1, HostV1, IdentityV1,
    InvocationV1, LanguageFlagsPageV1, LanguageFlagsV1, LimitsV1, LockPageV1, LockV1, Maybe,
    MissingObjectPageV1, MissingObjectV1, MutantPageV1, MutantV1, MutantV1Outcome, MutationV1,
    ObligationPageV1, ObligationV1, ObservationsV1, ObservationsV1Cancellation,
    ObservationsV1Cleanup, OracleResultV1, ProducerV1, ProducerV1Status, ReceiptV1, ResourcePageV1,
    ResourceV1, ReviewReceiptV1, ReviewV1, StandardPageV1, StandardV1, SubjectFilePageV1,
    SubjectFileV1, SubjectFileV1Kind, SubjectFileV1Origin, SubjectV1, SubjectsV1, ToolPageV1,
    ToolV1, TypedRef, VerdictV1, VerdictV1State,
};

pub(super) trait LocalCheck {
    fn check(&self) -> Result<(), Error>;
}
macro_rules! field_only {
    ($($name:ident),+ $(,)?) => { $(impl LocalCheck for $name {
        fn check(&self) -> Result<(), Error> { Ok(()) }
    })+ };
}
field_only!(
    IdentityV1,
    SubjectsV1,
    InvocationV1,
    ObservationsV1,
    CasesV1,
    DiagnosticsV1,
    ArtifactsV1,
    MutationV1,
    ReviewV1,
    VerdictV1,
    AvailabilityV1,
    LockV1,
    DependencyV1,
    ToolV1,
    BuildProfileV1,
    LanguageFlagsV1,
    StandardV1,
    GrantV1,
    ExpectationV1,
    LimitsV1,
    EffectV1,
    CleanupContractV1,
    HostV1,
    ResourceV1,
    ObligationV1,
    DiagnosticV1,
    ArtifactV1,
    CampaignV1,
    AssumptionV1,
    FindingV1,
    OracleResultV1,
    MissingObjectV1
);

impl LocalCheck for ProducerV1 {
    fn check(&self) -> Result<(), Error> {
        require(match self.status {
            ProducerV1Status::Exited => {
                self.exit_code.value.is_some() && self.signal.value.is_none()
            }
            ProducerV1Status::Signalled => {
                self.exit_code.value.is_none() && self.signal.value.is_some()
            }
            ProducerV1Status::NotStarted | ProducerV1Status::Unknown => {
                self.exit_code.value.is_none() && self.signal.value.is_none()
            }
        })
    }
}
impl LocalCheck for ExpectedProducerV1 {
    fn check(&self) -> Result<(), Error> {
        require(match self.status {
            ExpectedProducerV1Status::Exited => {
                self.exit_code.value.is_some() && self.signal.value.is_none()
            }
            ExpectedProducerV1Status::Signalled => {
                self.exit_code.value.is_none() && self.signal.value.is_some()
            }
        })
    }
}
impl LocalCheck for CaseV1 {
    fn check(&self) -> Result<(), Error> {
        require(!self.mandatory || (self.selected && !self.excluded))?;
        require(!self.executed || self.selected)?;
        require(
            !self.excluded
                || (!self.selected
                    && !self.executed
                    && self.outcome == CaseV1Outcome::Unmeasured
                    && !self.reason.as_str().is_empty()),
        )?;
        require(
            self.selected
                || (self.outcome == CaseV1Outcome::Unmeasured && !self.reason.as_str().is_empty()),
        )?;
        require(self.outcome == CaseV1Outcome::Passed || !self.reason.as_str().is_empty())
    }
}
impl LocalCheck for SubjectV1 {
    fn check(&self) -> Result<(), Error> {
        // RC04 defines the tree digest as the first typed file-page digest.
        require(self.tree_sha256 == self.files.as_ref().sha256)
    }
}
impl LocalCheck for SubjectFileV1 {
    fn check(&self) -> Result<(), Error> {
        require(!self.executable || self.kind == SubjectFileV1Kind::File)?;
        match self.kind {
            SubjectFileV1Kind::File => {
                require(self.content.value.is_some() && self.link_target.value.is_none())?;
            }
            SubjectFileV1Kind::Directory => {
                require(self.content.value.is_none() && self.link_target.value.is_none())?;
            }
            SubjectFileV1Kind::Symlink => require(self.link_target.value.is_some())?,
            SubjectFileV1Kind::Submodule | SubjectFileV1Kind::Other => (),
        }
        require(
            self.origin != SubjectFileV1Origin::Excluded
                || self
                    .exclusion_reason
                    .value
                    .as_ref()
                    .is_some_and(|reason| !reason.as_str().is_empty()),
        )
    }
}
impl LocalCheck for EnvironmentV1 {
    fn check(&self) -> Result<(), Error> {
        require(!self.name.as_str().contains(['\0', '=']))?;
        require(self.value.value.is_some() != self.secret_handle.value.is_some())
    }
}
impl LocalCheck for MutantV1 {
    fn check(&self) -> Result<(), Error> {
        use MutantV1Outcome::{
            Caught, Excluded, ReviewedEquivalent, Survived, Timeout, Unmeasured, Unviable,
        };
        if matches!(
            self.outcome,
            Caught | Survived | Timeout | ReviewedEquivalent
        ) {
            require(self.diff.as_ref().byte_length > 0)?;
        }
        if self.outcome == Caught {
            require(
                self.executed
                    && self.observed_detector.value.as_ref() == Some(&self.expected_detector),
            )?;
        }
        require(self.outcome != ReviewedEquivalent || self.review_ref.value.is_some())?;
        if !self.executed
            || matches!(
                self.outcome,
                Survived | ReviewedEquivalent | Excluded | Unviable | Unmeasured
            )
        {
            require(!self.reason.as_str().is_empty())?;
        }
        Ok(())
    }
}
impl LocalCheck for ReceiptV1 {
    fn check(&self) -> Result<(), Error> {
        require(self.version == 1 && self.review.value.is_none())?;
        if self.verdict.state != VerdictV1State::PassCandidate {
            return Ok(());
        }
        require(
            self.subjects.result_subject.value.is_some()
                && self.subjects.seed_to_result_patch.value.is_some(),
        )?;
        let producer = &self.observations.producer;
        require(
            producer.status == ProducerV1Status::Exited
                && !producer.timeout
                && producer.stdout.value.is_some()
                && producer.stderr.value.is_some(),
        )?;
        require(
            self.observations.cancellation == ObservationsV1Cancellation::NotRequested
                && self.observations.cleanup == ObservationsV1Cleanup::Settled,
        )?;
        require(self.cases.selected > 0 && self.cases.executed > 0)?;
        let diagnostics = &self.diagnostics;
        require(
            !diagnostics.stdout_truncated
                && !diagnostics.stderr_truncated
                && diagnostics.mismatch.value.is_none(),
        )?;
        require(
            !diagnostics.baseline
                || (diagnostics.warning_count == 0 && diagnostics.error_count == 0),
        )?;
        require(
            self.artifacts.finalized && self.availability.state == AvailabilityV1State::Complete,
        )
    }
}
impl LocalCheck for ReviewReceiptV1 {
    fn check(&self) -> Result<(), Error> {
        require(self.version == 1 && self.review.subject_sha256 == self.receipt.as_ref().sha256)
    }
}
impl LocalCheck for AvailabilityReceiptV1 {
    fn check(&self) -> Result<(), Error> {
        require(self.version == 1)
    }
}
fn page<T: Address>(
    index: Count,
    count: Count,
    rows: Count,
    total: Count,
    length: usize,
    next: &Maybe<TypedRef<T>>,
) -> Result<(), Error> {
    require(
        count > 0
            && index < count
            && rows <= 256
            && usize::try_from(rows).ok() == Some(length)
            && rows <= total,
    )?;
    let final_page = u64::from(index) + 1 == u64::from(count);
    require(next.value.is_none() == final_page)?;
    if final_page {
        require(
            next.unavailable_reason
                .as_ref()
                .is_some_and(|reason| reason.as_str() == "end_of_inventory"),
        )?;
    }
    if total == 0 {
        require(index == 0 && count == 1 && rows == 0)?;
    } else {
        require(rows > 0)?;
    }
    Ok(())
}
macro_rules! pages {
    ($($name:ident),+ $(,)?) => { $(impl LocalCheck for $name {
        fn check(&self) -> Result<(), Error> {
            page(self.page_index, self.page_count, self.row_count, self.total_rows, self.rows.as_slice().len(), &self.next)
        }
    })+ };
}
pages!(
    CasePageV1,
    SubjectFilePageV1,
    LockPageV1,
    DependencyPageV1,
    ToolPageV1,
    LanguageFlagsPageV1,
    StandardPageV1,
    EnvironmentPageV1,
    GrantPageV1,
    EffectPageV1,
    ResourcePageV1,
    ObligationPageV1,
    DiagnosticPageV1,
    ArtifactPageV1,
    CampaignPageV1,
    MutantPageV1,
    AssumptionPageV1,
    FindingPageV1,
    MissingObjectPageV1
);
