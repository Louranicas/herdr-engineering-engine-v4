//! Independent T03 contract controls using a counted, non-I/O fixture owner.
//! No fixture grants tool authority or asserts process/workspace settlement.

use habitat_engine::contracts::{Generation, Sha256Digest, UuidV4};
use habitat_engine::worker::{
    Binding, CancelCommand, CancelReason, Candidate, Capabilities, Contract, ContractError,
    Envelope, Event, Feature, Finish, Identity, IdentityOrigin, Invocation, Phase, Request,
    Selection, Terminal, Usage, UsageForm, UsageScope, UsageStage, inference,
};

const TASK: &str = "123e4567-e89b-42d3-a456-000000000001";
const ATTEMPT: &str = "123e4567-e89b-42d3-a456-000000000002";
const INVOCATION: &str = "123e4567-e89b-42d3-a456-000000000003";
const NEIGHBOR: &str = "123e4567-e89b-42d3-a456-000000000004";
const RECIPE: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const WORKSPACE: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PROVIDER: &str = "fixture-provider";
const PROFILE: &str = "fixture-output/1";
const FEATURES: [Feature; 6] = [
    Feature::FinalOutput,
    Feature::Deltas,
    Feature::ToolProposals,
    Feature::Cancel,
    Feature::Usage,
    Feature::Identity,
];

#[derive(Clone, Copy)]
enum Class {
    OutputOnly,
    ToolCapable,
}

fn capabilities(class: Class) -> Capabilities {
    let features: Vec<_> = FEATURES
        .into_iter()
        .filter(|feature| matches!(class, Class::ToolCapable) || *feature != Feature::ToolProposals)
        .collect();
    Capabilities::new(&features)
}

fn invocation() -> Invocation<'static> {
    Invocation {
        binding: Binding {
            task: UuidV4::parse(TASK).unwrap(),
            attempt: UuidV4::parse(ATTEMPT).unwrap(),
            generation: "7".parse::<Generation>().unwrap(),
        },
        id: UuidV4::parse(INVOCATION).unwrap(),
    }
}

fn selection() -> Selection {
    Selection {
        provider: PROVIDER.into(),
        model: "fixture-model".into(),
        effort: Some("high".into()),
    }
}

fn request(required: &[Feature]) -> Request<'static> {
    Request {
        invocation: invocation(),
        recipe: Sha256Digest::parse(RECIPE).unwrap(),
        workspace: Sha256Digest::parse(WORKSPACE).unwrap(),
        adapter_profile: PROFILE.into(),
        selection: selection(),
        required: Capabilities::new(required),
        prompt: "Produce fixture output; this is not an execution grant.".into(),
    }
}

fn identity(origin: IdentityOrigin) -> Identity {
    Identity {
        selection: selection(),
        adapter_profile: PROFILE.into(),
        runtime_instance: "fixture-instance-1".into(),
        origin,
        provider_model: Some("provider-reported-model".into()),
        provider_revision: Some("provider-reported-revision".into()),
        raw: b"{ \"model\": \"provider-reported-model\", \"revision\": \"provider-reported-revision\" }\n".to_vec(),
    }
}

fn reported(total: u64, stage: UsageStage) -> Usage {
    Usage::Reported {
        provider: PROVIDER.into(),
        scope: UsageScope::Invocation,
        form: UsageForm::Cumulative,
        stage,
        input: None,
        output: None,
        total: Some(total),
        raw: format!("{{ \"vendor_total\": {total}, \"vendor_cost\": 1e-6 }}\n").into_bytes(),
    }
}

fn candidate(finish: Finish) -> Candidate {
    Candidate {
        text: "fixture answer".into(),
        has_tool_proposals: false,
        finish,
        identity: None,
        usage: Usage::Unknown,
        raw: b"{ \"output\": \"fixture answer\", \"vendor_finish\": \"opaque\" }\n".to_vec(),
    }
}

struct Owner {
    contract: Contract<'static>,
    dispatches: Vec<Request<'static>>,
    cancellations: Vec<CancelCommand<'static>>,
    phases: Vec<Phase>,
    sequence: u64,
}

impl Owner {
    fn new(required: &[Feature]) -> Self {
        Self {
            contract: Contract::new(request(required)).unwrap(),
            dispatches: Vec::new(),
            cancellations: Vec::new(),
            phases: vec![Phase::Prepared],
            sequence: 1,
        }
    }

    fn launch(&mut self, offered: Capabilities, time: u64) -> Result<(), ContractError> {
        let dispatched = self.contract.launch(offered, time)?.clone();
        self.dispatches.push(dispatched);
        self.phases.push(self.contract.phase());
        Ok(())
    }

    fn event(&mut self, event: Event, time: u64) -> Result<(), ContractError> {
        let result = self.contract.observe(
            Envelope {
                invocation: invocation(),
                sequence: self.sequence,
                event,
            },
            time,
        );
        if result.is_ok() {
            self.sequence += 1;
        }
        self.phases.push(self.contract.phase());
        result
    }

    fn cancel(&mut self, reason: CancelReason, time: u64) -> Result<(), ContractError> {
        if let Some(command) = self.contract.cancel(reason, time)? {
            self.cancellations.push(command);
        }
        Ok(())
    }
}

fn launched(required: &[Feature], class: Class) -> Owner {
    let mut owner = Owner::new(required);
    owner.launch(capabilities(class), 0).unwrap();
    owner
}

#[test]
fn output_only_launch_preserves_the_owned_request_and_dispatches_once() {
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    let sent = &owner.dispatches[0];
    assert_eq!(sent.selection, selection());
    assert_eq!(sent.adapter_profile, PROFILE);
    assert_eq!(sent.recipe.as_str(), RECIPE);
    assert_eq!(sent.workspace.as_str(), WORKSPACE);
    assert_eq!(sent.invocation.id.as_str(), INVOCATION);
    assert_eq!(sent.invocation.binding.task.as_str(), TASK);
    assert_eq!(sent.invocation.binding.attempt.as_str(), ATTEMPT);
    assert_eq!(
        owner.launch(capabilities(Class::ToolCapable), 1),
        Err(ContractError::State)
    );
    assert_eq!(owner.dispatches.len(), 1);
}

#[test]
fn tool_proposal_fixture_and_output_only_fixture_have_distinct_return_capabilities() {
    let mut proposed = candidate(Finish::Stop);
    proposed.has_tool_proposals = true;
    let mut capable = launched(
        &[Feature::FinalOutput, Feature::ToolProposals],
        Class::ToolCapable,
    );
    capable.event(Event::Final(proposed.clone()), 1).unwrap();
    capable
        .event(Event::Terminal(Terminal::Completed), 2)
        .unwrap();
    assert!(capable.contract.candidate().unwrap().has_tool_proposals);
    let mut output = launched(&[Feature::FinalOutput], Class::OutputOnly);
    assert_eq!(
        output.event(Event::Final(proposed.clone()), 1),
        Err(ContractError::Unsupported(Feature::ToolProposals))
    );
    assert_eq!(output.contract.phase(), Phase::Unknown);
    assert_eq!(
        inference::normalize(proposed, PROVIDER),
        Err(ContractError::Unsupported(Feature::ToolProposals))
    );
    assert_eq!(capable.dispatches.len(), 1);
    assert_eq!(output.dispatches.len(), 1);
}

#[test]
fn each_missing_required_feature_refuses_before_dispatch_without_fallback() {
    for missing in FEATURES {
        let mut owner = Owner::new(&[Feature::FinalOutput, missing]);
        let offered: Vec<_> = FEATURES
            .into_iter()
            .filter(|item| *item != missing)
            .collect();
        assert_eq!(
            owner.launch(Capabilities::new(&offered), 4),
            Err(ContractError::Unsupported(missing))
        );
        assert!(owner.dispatches.is_empty());
        assert_eq!(owner.contract.phase(), Phase::Prepared);
        owner.launch(capabilities(Class::ToolCapable), 4).unwrap();
        assert_eq!(owner.dispatches.len(), 1);
        assert_eq!(owner.dispatches[0].selection, selection());
        assert_eq!(owner.dispatches[0].adapter_profile, PROFILE);
    }
}

#[test]
fn invalid_requests_refuse_and_prompt_bound_counts_utf8_bytes() {
    let mut at_bound = request(&[Feature::FinalOutput]);
    at_bound.prompt = "é".repeat(131_072);
    assert!(Contract::new(at_bound.clone()).is_ok());
    at_bound.prompt.push('x');
    assert_eq!(
        Contract::new(at_bound).unwrap_err(),
        ContractError::InvalidRequest
    );
    for index in 0..5 {
        let mut invalid = request(&[Feature::FinalOutput]);
        match index {
            0 => invalid.prompt.clear(),
            1 => invalid.required = Capabilities::new(&[Feature::Cancel]),
            2 => invalid.selection.model = "model\nspoof".into(),
            3 => invalid.adapter_profile.clear(),
            _ => invalid.selection.effort = Some(String::new()),
        }
        assert_eq!(
            Contract::new(invalid).unwrap_err(),
            ContractError::InvalidRequest
        );
    }
}

#[test]
fn every_invocation_namespace_dimension_is_checked_and_refusal_is_permanent() {
    for dimension in 0..4 {
        let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
        let mut foreign = invocation();
        match dimension {
            0 => foreign.id = UuidV4::parse(NEIGHBOR).unwrap(),
            1 => foreign.binding.task = UuidV4::parse(NEIGHBOR).unwrap(),
            2 => foreign.binding.attempt = UuidV4::parse(NEIGHBOR).unwrap(),
            _ => foreign.binding.generation = "8".parse().unwrap(),
        }
        assert_eq!(
            owner.contract.observe(
                Envelope {
                    invocation: foreign,
                    sequence: 1,
                    event: Event::Activity
                },
                1
            ),
            Err(ContractError::Correlation)
        );
        assert_eq!(
            owner.event(Event::Final(candidate(Finish::Stop)), 2),
            Err(ContractError::State)
        );
        assert_eq!(owner.contract.phase(), Phase::Unknown);
        assert!(owner.contract.candidate().is_none());
    }
}

#[test]
fn acknowledgement_activity_final_and_terminal_have_distinct_trace_stages() {
    let mut owner = Owner::new(&[Feature::FinalOutput]);
    assert_eq!(owner.event(Event::Activity, 0), Err(ContractError::State));
    owner.launch(capabilities(Class::OutputOnly), 0).unwrap();
    owner.event(Event::Acknowledged, 1).unwrap();
    assert!(owner.contract.terminal().is_none());
    owner.event(Event::Activity, 2).unwrap();
    owner
        .event(Event::Final(candidate(Finish::Stop)), 3)
        .unwrap();
    assert!(owner.contract.terminal().is_none());
    owner
        .event(Event::Terminal(Terminal::Completed), 4)
        .unwrap();
    assert_eq!(
        owner.phases,
        [
            Phase::Prepared,
            Phase::Prepared,
            Phase::Dispatched,
            Phase::Dispatched,
            Phase::Active,
            Phase::Active,
            Phase::Terminal
        ]
    );
    assert_eq!(owner.dispatches.len(), 1);
}

#[test]
fn duplicate_and_gapped_event_sequences_cannot_advance() {
    for sequence in [0, 2] {
        let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
        assert_eq!(
            owner.contract.observe(
                Envelope {
                    invocation: invocation(),
                    sequence,
                    event: Event::Activity
                },
                1
            ),
            Err(ContractError::Sequence)
        );
        assert_eq!(owner.contract.phase(), Phase::Unknown);
    }
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    owner.event(Event::Activity, 1).unwrap();
    assert_eq!(
        owner.contract.observe(
            Envelope {
                invocation: invocation(),
                sequence: 1,
                event: Event::Activity
            },
            2
        ),
        Err(ContractError::Sequence)
    );
    assert_eq!(owner.contract.phase(), Phase::Unknown);
}

#[test]
fn lost_transport_after_dispatch_is_unknown_without_implicit_retry() {
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    owner.contract.transport_lost();
    assert_eq!(owner.contract.phase(), Phase::Unknown);
    assert_eq!(
        owner.launch(capabilities(Class::OutputOnly), 1),
        Err(ContractError::State)
    );
    assert_eq!(
        owner.cancel(CancelReason::Operator, 1),
        Err(ContractError::State)
    );
    assert_eq!(owner.dispatches.len(), 1);
    assert!(owner.cancellations.is_empty());
    assert!(owner.contract.terminal().is_none());
}

#[test]
fn final_output_is_authoritative_and_never_appended_to_streaming_deltas() {
    let mut owner = launched(&[Feature::FinalOutput, Feature::Deltas], Class::OutputOnly);
    owner.event(Event::Delta(vec![0xc3]), 1).unwrap();
    owner.event(Event::Delta(vec![0xa9]), 2).unwrap();
    assert!(owner.contract.candidate().is_none());
    let final_output = candidate(Finish::Stop);
    owner.event(Event::Final(final_output.clone()), 3).unwrap();
    assert_eq!(owner.contract.candidate(), Some(&final_output));
    assert_eq!(owner.contract.phase(), Phase::Active);
    owner
        .event(Event::Terminal(Terminal::Completed), 4)
        .unwrap();
}

#[test]
fn finish_reasons_map_to_distinct_terminal_dispositions() {
    for (finish, terminal) in [
        (Finish::Stop, Terminal::Completed),
        (Finish::Length, Terminal::Truncated),
        (Finish::Refusal, Terminal::Refused),
        (Finish::Error, Terminal::Failed),
        (Finish::Cancelled, Terminal::Cancelled),
    ] {
        let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
        owner.event(Event::Final(candidate(finish)), 1).unwrap();
        owner.event(Event::Terminal(terminal), 2).unwrap();
        assert_eq!(owner.contract.terminal(), Some(terminal));
        assert_eq!(owner.contract.candidate().unwrap().finish, finish);
    }
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    owner
        .event(Event::Final(candidate(Finish::Length)), 1)
        .unwrap();
    assert_eq!(
        owner.event(Event::Terminal(Terminal::Completed), 2),
        Err(ContractError::State)
    );
    assert_eq!(owner.contract.phase(), Phase::Unknown);
}

#[test]
fn completed_or_truncated_requires_candidate_but_launch_failure_does_not() {
    for terminal in [Terminal::Completed, Terminal::Truncated] {
        let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
        assert_eq!(
            owner.event(Event::Terminal(terminal), 1),
            Err(ContractError::State)
        );
        assert!(owner.contract.terminal().is_none());
    }
    let mut failed = launched(&[Feature::FinalOutput], Class::OutputOnly);
    failed.event(Event::Terminal(Terminal::Failed), 1).unwrap();
    assert!(failed.contract.candidate().is_none());
    assert_eq!(failed.contract.terminal(), Some(Terminal::Failed));
}

#[test]
fn duplicate_final_and_late_activity_cannot_rewrite_retained_history() {
    let first = candidate(Finish::Stop);
    let mut duplicate = launched(&[Feature::FinalOutput], Class::OutputOnly);
    duplicate.event(Event::Final(first.clone()), 1).unwrap();
    let mut replacement = candidate(Finish::Error);
    replacement.text = "rewrite".into();
    assert_eq!(
        duplicate.event(Event::Final(replacement), 2),
        Err(ContractError::State)
    );
    assert_eq!(duplicate.contract.candidate(), Some(&first));
    let mut late = launched(&[Feature::FinalOutput], Class::OutputOnly);
    late.event(Event::Final(first.clone()), 1).unwrap();
    late.event(Event::Terminal(Terminal::Completed), 2).unwrap();
    assert_eq!(late.event(Event::Activity, 3), Err(ContractError::State));
    assert_eq!(late.contract.phase(), Phase::Unknown);
    assert_eq!(late.contract.terminal(), Some(Terminal::Completed));
    assert_eq!(late.contract.candidate(), Some(&first));
}

#[test]
fn runtime_identity_and_raw_origin_survive_the_common_result() {
    let mut owner = launched(
        &[Feature::FinalOutput, Feature::Identity],
        Class::OutputOnly,
    );
    let observed = identity(IdentityOrigin::RuntimeReadback);
    owner.event(Event::Identity(observed.clone()), 1).unwrap();
    owner
        .event(Event::Final(candidate(Finish::Stop)), 2)
        .unwrap();
    owner
        .event(Event::Terminal(Terminal::Completed), 3)
        .unwrap();
    assert_eq!(owner.contract.identity(), Some(&observed));
    assert_eq!(
        owner.contract.identity().unwrap().origin,
        IdentityOrigin::RuntimeReadback
    );
}

#[test]
fn request_echo_is_never_an_actual_identity_observation() {
    let echo = identity(IdentityOrigin::RequestEcho);
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    assert_eq!(
        owner.event(Event::Identity(echo.clone()), 1),
        Err(ContractError::Identity)
    );
    assert!(owner.contract.identity().is_none());
    let mut value = candidate(Finish::Stop);
    value.identity = Some(echo);
    assert_eq!(
        inference::normalize(value, PROVIDER),
        Err(ContractError::Identity)
    );
}

#[test]
fn actual_identity_mismatches_do_not_silently_change_recipe_or_runtime() {
    for dimension in 0..7 {
        let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
        let prior = identity(IdentityOrigin::RuntimeReadback);
        owner.event(Event::Identity(prior.clone()), 1).unwrap();
        let mut wrong = prior.clone();
        match dimension {
            0 => wrong.selection.provider = "other-provider".into(),
            1 => wrong.selection.model = "other-model".into(),
            2 => wrong.selection.effort = Some("low".into()),
            3 => wrong.adapter_profile = "other-profile".into(),
            4 => wrong.runtime_instance = "other-instance".into(),
            5 => wrong.provider_model = Some("other-provider-model".into()),
            _ => wrong.provider_revision = Some("other-revision".into()),
        }
        let mut value = candidate(Finish::Stop);
        value.identity = Some(wrong);
        if dimension == 0 {
            assert_eq!(
                inference::normalize(value.clone(), PROVIDER),
                Err(ContractError::Identity)
            );
        }
        assert_eq!(
            owner.event(Event::Final(value), 2),
            Err(ContractError::Identity)
        );
        assert_eq!(owner.contract.identity(), Some(&prior));
        assert!(owner.contract.candidate().is_none());
    }
}

#[test]
fn missing_optional_identity_stays_unknown_and_required_identity_refuses() {
    let value = candidate(Finish::Stop);
    assert!(
        inference::normalize(value.clone(), PROVIDER)
            .unwrap()
            .identity
            .is_none()
    );
    let mut optional = launched(&[Feature::FinalOutput], Class::OutputOnly);
    optional.event(Event::Final(value.clone()), 1).unwrap();
    optional
        .event(Event::Terminal(Terminal::Completed), 2)
        .unwrap();
    assert!(optional.contract.identity().is_none());
    let mut required = launched(
        &[Feature::FinalOutput, Feature::Identity],
        Class::OutputOnly,
    );
    required.event(Event::Final(value), 1).unwrap();
    assert_eq!(
        required.event(Event::Terminal(Terminal::Completed), 2),
        Err(ContractError::Identity)
    );
}

#[test]
fn provider_self_report_is_preserved_but_cannot_satisfy_required_runtime_identity() {
    let observed = identity(IdentityOrigin::ProviderResponse);
    let mut optional = launched(&[Feature::FinalOutput], Class::OutputOnly);
    optional
        .event(Event::Identity(observed.clone()), 1)
        .unwrap();
    assert_eq!(optional.contract.identity(), Some(&observed));
    let mut required = launched(
        &[Feature::FinalOutput, Feature::Identity],
        Class::OutputOnly,
    );
    let early = required.event(Event::Identity(observed), 1);
    if early.is_ok() {
        required
            .event(Event::Final(candidate(Finish::Stop)), 2)
            .unwrap();
        assert!(
            required
                .event(Event::Terminal(Terminal::Completed), 3)
                .is_err()
        );
    } else {
        assert_eq!(early, Err(ContractError::Identity));
    }
    assert_eq!(required.contract.phase(), Phase::Unknown);
    assert!(required.contract.terminal().is_none());
}

#[test]
fn required_usage_needs_a_final_report_not_missing_or_provisional_data() {
    for usage in [Usage::Unknown, reported(12, UsageStage::Provisional)] {
        let mut owner = launched(&[Feature::FinalOutput, Feature::Usage], Class::OutputOnly);
        let mut value = candidate(Finish::Stop);
        value.usage = usage;
        owner.event(Event::Final(value), 1).unwrap();
        assert!(
            owner
                .event(Event::Terminal(Terminal::Completed), 2)
                .is_err()
        );
        assert!(owner.contract.terminal().is_none());
    }
    let mut benign = launched(&[Feature::FinalOutput, Feature::Usage], Class::OutputOnly);
    let mut value = candidate(Finish::Stop);
    value.usage = reported(12, UsageStage::Final);
    benign.event(Event::Final(value), 1).unwrap();
    benign
        .event(Event::Terminal(Terminal::Completed), 2)
        .unwrap();
}

#[test]
fn missing_and_explicit_zero_usage_remain_different() {
    let unknown = inference::normalize(candidate(Finish::Stop), PROVIDER).unwrap();
    let mut zero = candidate(Finish::Stop);
    zero.usage = reported(0, UsageStage::Final);
    let normalized = inference::normalize(zero.clone(), PROVIDER).unwrap();
    assert_eq!(normalized, zero);
    assert_ne!(normalized.usage, unknown.usage);
    assert_eq!(unknown.usage, Usage::Unknown);
}

#[test]
fn provider_specific_subset_arithmetic_is_preserved_without_a_universal_sum() {
    for (provider, total, raw) in [
        ("disjoint-provider", 18, b"{\"input\":10,\"output\":5,\"cacheRead\":3,\"reasoning\":2,\"total\":18}".as_slice()),
        ("subset-provider", 15, b"{\"prompt\":10,\"completion\":5,\"cachedPromptSubset\":3,\"reasoningSubset\":2,\"total\":15}".as_slice()),
    ] {
        let mut value = candidate(Finish::Stop);
        value.usage = Usage::Reported { provider: provider.into(), scope: UsageScope::Invocation,
            form: UsageForm::Cumulative, stage: UsageStage::Final, input: Some(10), output: Some(5),
            total: Some(total), raw: raw.to_vec() };
        assert_eq!(inference::normalize(value.clone(), provider).unwrap(), value);
    }
}

#[test]
fn snapshots_are_not_summed_and_delta_form_is_not_reinterpreted() {
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    owner
        .event(Event::Usage(reported(10, UsageStage::Provisional)), 1)
        .unwrap();
    let twelve = reported(12, UsageStage::Provisional);
    owner.event(Event::Usage(twelve.clone()), 2).unwrap();
    owner.event(Event::Usage(twelve.clone()), 3).unwrap();
    assert_eq!(owner.contract.usage(), &twelve);
    let mut delta = reported(2, UsageStage::Provisional);
    if let Usage::Reported { form, scope, .. } = &mut delta {
        *form = UsageForm::Delta;
        *scope = UsageScope::Session;
    }
    owner.event(Event::Usage(delta.clone()), 4).unwrap();
    assert_eq!(owner.contract.usage(), &delta);
}

#[test]
fn normalizer_preserves_candidate_identity_and_usage_bytes_without_reserialization() {
    let mut value = candidate(Finish::Length);
    value.text = "partial é🙂".into();
    let mut actual = identity(IdentityOrigin::ProviderResponse);
    actual.provider_model = None;
    actual.provider_revision = None;
    actual.raw = b"{ \"vendor_alias\" : \"opaque\", \"confidence\": 1e-3 }\r\n".to_vec();
    value.identity = Some(actual);
    value.raw = b"{ \"finish\" : \"length\", \"vendor_extra\": [1.00, 1e0] }\n".to_vec();
    value.usage = Usage::Reported {
        provider: PROVIDER.into(),
        scope: UsageScope::Unknown,
        form: UsageForm::Unknown,
        stage: UsageStage::Provisional,
        input: None,
        output: None,
        total: None,
        raw: b"{ \"opaqueUnits\": 0.000001, \"cache1h\": null }\n".to_vec(),
    };
    assert_eq!(
        inference::normalize(value.clone(), PROVIDER).unwrap(),
        value
    );
}

#[test]
fn usage_from_a_different_provider_refuses_without_replacing_known_usage() {
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    let known = reported(12, UsageStage::Provisional);
    owner.event(Event::Usage(known.clone()), 1).unwrap();
    let mut foreign = reported(12, UsageStage::Provisional);
    if let Usage::Reported { provider, .. } = &mut foreign {
        *provider = "other-provider".into();
    }
    assert_eq!(
        owner.event(Event::Usage(foreign.clone()), 2),
        Err(ContractError::Identity)
    );
    assert_eq!(owner.contract.usage(), &known);
    let mut value = candidate(Finish::Stop);
    value.usage = foreign;
    assert_eq!(
        inference::normalize(value, PROVIDER),
        Err(ContractError::Identity)
    );
}

#[test]
fn final_usage_cannot_regress_to_provisional_or_be_erased_by_a_missing_final_field() {
    let final_usage = Usage::Reported {
        provider: PROVIDER.into(),
        scope: UsageScope::Invocation,
        form: UsageForm::Cumulative,
        stage: UsageStage::Final,
        input: Some(7),
        output: Some(5),
        total: Some(12),
        raw: b"{ \"input\": 7, \"output\": 5, \"total\": 12 }\n".to_vec(),
    };
    for embedded in [false, true] {
        // Identical final reports are a benign neighbor through both entry paths.
        let mut benign = launched(&[Feature::FinalOutput], Class::OutputOnly);
        benign.event(Event::Usage(final_usage.clone()), 1).unwrap();
        let mut value = candidate(Finish::Stop);
        value.usage = final_usage.clone();
        let event = if embedded {
            Event::Final(value)
        } else {
            Event::Usage(final_usage.clone())
        };
        benign.event(event, 2).unwrap();
        assert_eq!(benign.contract.usage(), &final_usage);
        assert_ne!(benign.contract.phase(), Phase::Unknown);
        for change in [
            "provisional",
            "session",
            "unknown-scope",
            "delta",
            "unknown-form",
            "missing-input",
            "missing-output",
            "missing-total",
            "missing-usage",
        ] {
            let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
            owner.event(Event::Usage(final_usage.clone()), 1).unwrap();
            let mut incoming = final_usage.clone();
            if let Usage::Reported {
                scope,
                form,
                stage,
                input,
                output,
                total,
                ..
            } = &mut incoming
            {
                match change {
                    "provisional" => *stage = UsageStage::Provisional,
                    "session" => *scope = UsageScope::Session,
                    "unknown-scope" => *scope = UsageScope::Unknown,
                    "delta" => *form = UsageForm::Delta,
                    "unknown-form" => *form = UsageForm::Unknown,
                    "missing-input" => *input = None,
                    "missing-output" => *output = None,
                    "missing-total" => *total = None,
                    "missing-usage" => (),
                    _ => unreachable!(),
                }
            }
            if change == "missing-usage" {
                incoming = Usage::Unknown;
            }
            let mut value = candidate(Finish::Stop);
            value.usage = incoming.clone();
            let event = if embedded {
                Event::Final(value)
            } else {
                Event::Usage(incoming)
            };
            assert!(
                owner.event(event, 2).is_err(),
                "accepted {change}; embedded={embedded}"
            );
            assert_eq!(
                owner.contract.usage(),
                &final_usage,
                "lost prior final report: {change}"
            );
            assert_eq!(owner.contract.phase(), Phase::Unknown);
            assert!(owner.contract.candidate().is_none());
        }
    }
}

#[test]
fn optional_reported_features_still_require_selected_adapter_capabilities() {
    for event in [
        Event::Identity(identity(IdentityOrigin::RuntimeReadback)),
        Event::Usage(reported(1, UsageStage::Provisional)),
    ] {
        let feature = if matches!(&event, Event::Identity(_)) {
            Feature::Identity
        } else {
            Feature::Usage
        };
        let mut owner = Owner::new(&[Feature::FinalOutput]);
        owner
            .launch(Capabilities::new(&[Feature::FinalOutput]), 0)
            .unwrap();
        assert_eq!(
            owner.event(event, 1),
            Err(ContractError::Unsupported(feature))
        );
        assert_eq!(owner.contract.phase(), Phase::Unknown);
    }
    for feature in [Feature::Identity, Feature::Usage] {
        let mut owner = Owner::new(&[Feature::FinalOutput]);
        owner
            .launch(Capabilities::new(&[Feature::FinalOutput]), 0)
            .unwrap();
        let mut value = candidate(Finish::Stop);
        match feature {
            Feature::Identity => value.identity = Some(identity(IdentityOrigin::RuntimeReadback)),
            Feature::Usage => value.usage = reported(1, UsageStage::Final),
            _ => unreachable!(),
        }
        assert_eq!(
            owner.event(Event::Final(value), 1),
            Err(ContractError::Unsupported(feature))
        );
        assert!(owner.contract.candidate().is_none());
    }
}

#[test]
fn cancellation_is_one_intent_and_acknowledgement_does_not_imply_idle() {
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    owner.cancel(CancelReason::Operator, 1).unwrap();
    owner.cancel(CancelReason::Superseded, 2).unwrap();
    assert_eq!(owner.cancellations.len(), 1);
    assert_eq!(owner.cancellations[0].reason, CancelReason::Operator);
    assert_eq!(owner.cancellations[0].invocation.id.as_str(), INVOCATION);
    assert_eq!(
        owner.contract.cancellation().intent,
        Some(CancelReason::Operator)
    );
    assert!(owner.contract.cancellation().issued());
    owner.event(Event::CancelAcknowledged, 3).unwrap();
    assert!(owner.contract.cancellation().acknowledged);
    assert!(!owner.contract.cancellation().adapter_idle);
    assert!(owner.contract.terminal().is_none());
    owner.event(Event::AdapterIdle, 4).unwrap();
    assert!(owner.contract.cancellation().adapter_idle);
    assert!(owner.contract.terminal().is_none());
}

#[test]
fn unsupported_cancel_retains_intent_and_never_calls_the_fake_adapter() {
    let mut owner = Owner::new(&[Feature::FinalOutput]);
    owner
        .launch(Capabilities::new(&[Feature::FinalOutput]), 0)
        .unwrap();
    assert_eq!(
        owner.cancel(CancelReason::Operator, 1),
        Err(ContractError::Unsupported(Feature::Cancel))
    );
    assert!(owner.contract.cancellation().unsupported());
    assert!(!owner.contract.cancellation().issued());
    assert_eq!(
        owner.contract.cancellation().intent,
        Some(CancelReason::Operator)
    );
    owner.cancel(CancelReason::Operator, 2).unwrap();
    assert!(owner.cancellations.is_empty());
    assert_eq!(owner.contract.phase(), Phase::Dispatched);
}

#[test]
fn raced_completion_and_late_cancel_readbacks_preserve_the_candidate_disposition() {
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    owner.cancel(CancelReason::Operator, 1).unwrap();
    let final_output = candidate(Finish::Stop);
    owner.event(Event::Final(final_output.clone()), 2).unwrap();
    owner
        .event(Event::Terminal(Terminal::Completed), 3)
        .unwrap();
    owner.cancel(CancelReason::Operator, 4).unwrap();
    owner.event(Event::CancelAcknowledged, 5).unwrap();
    owner.event(Event::AdapterIdle, 6).unwrap();
    assert_eq!(owner.cancellations.len(), 1);
    assert_eq!(owner.contract.terminal(), Some(Terminal::Completed));
    assert_eq!(owner.contract.candidate(), Some(&final_output));
    assert_eq!(owner.contract.phase(), Phase::Terminal);
}

#[test]
fn idle_before_cancellation_cannot_satisfy_a_later_cancellation_readback() {
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    let idle = owner.event(Event::AdapterIdle, 1);
    if idle.is_err() {
        assert_eq!(idle, Err(ContractError::State));
        assert_eq!(owner.contract.phase(), Phase::Unknown);
        assert!(!owner.contract.cancellation().adapter_idle);
        return;
    }
    owner.event(Event::Activity, 2).unwrap();
    owner.cancel(CancelReason::Operator, 3).unwrap();
    assert!(!owner.contract.cancellation().adapter_idle);
    owner.event(Event::CancelAcknowledged, 4).unwrap();
    assert!(!owner.contract.cancellation().adapter_idle);
    owner.event(Event::AdapterIdle, 5).unwrap();
    assert!(owner.contract.cancellation().adapter_idle);
}

#[test]
fn unsolicited_or_duplicate_cancel_acknowledgement_is_a_protocol_refusal() {
    let mut unsolicited = launched(&[Feature::FinalOutput], Class::OutputOnly);
    assert_eq!(
        unsolicited.event(Event::CancelAcknowledged, 1),
        Err(ContractError::State)
    );
    assert!(!unsolicited.contract.cancellation().acknowledged);
    let mut duplicate = launched(&[Feature::FinalOutput], Class::OutputOnly);
    duplicate.cancel(CancelReason::Operator, 1).unwrap();
    duplicate.event(Event::CancelAcknowledged, 2).unwrap();
    assert_eq!(
        duplicate.event(Event::CancelAcknowledged, 3),
        Err(ContractError::State)
    );
    assert_eq!(duplicate.contract.phase(), Phase::Unknown);
    assert!(duplicate.contract.cancellation().acknowledged);
    assert_eq!(duplicate.cancellations.len(), 1);
}

#[test]
fn clocks_preserve_one_deadline_and_refusals_cannot_reopen_dispatch() {
    let mut reserve = Owner::new(&[Feature::FinalOutput]);
    assert_eq!(
        reserve.launch(capabilities(Class::OutputOnly), 900_000),
        Err(ContractError::Deadline)
    );
    assert_eq!(
        reserve.launch(capabilities(Class::OutputOnly), 0),
        Err(ContractError::ClockRewind)
    );
    assert!(reserve.dispatches.is_empty());
    let mut benign = Owner::new(&[Feature::FinalOutput]);
    benign
        .launch(capabilities(Class::OutputOnly), 899_999)
        .unwrap();
    benign.event(Event::Activity, 1_199_999).unwrap();
    assert_eq!(
        benign.event(Event::Activity, 1_200_000),
        Err(ContractError::Deadline)
    );
    assert_eq!(benign.contract.phase(), Phase::Unknown);
    let mut rewind = launched(&[Feature::FinalOutput], Class::OutputOnly);
    rewind.event(Event::Activity, 5).unwrap();
    assert_eq!(
        rewind.cancel(CancelReason::Operator, 4),
        Err(ContractError::ClockRewind)
    );
    assert_eq!(rewind.contract.phase(), Phase::Unknown);
    assert!(rewind.cancellations.is_empty());
}

#[test]
fn delta_stream_limit_is_cumulative_and_overflow_permanently_refuses() {
    let mut owner = launched(&[Feature::FinalOutput, Feature::Deltas], Class::OutputOnly);
    owner.event(Event::Delta(vec![b'x'; 4_194_304]), 1).unwrap();
    owner.event(Event::Delta(vec![b'y'; 4_194_304]), 2).unwrap();
    assert_eq!(owner.contract.phase(), Phase::Active);
    assert_eq!(
        owner.event(Event::Delta(vec![b'z']), 3),
        Err(ContractError::Bound)
    );
    assert_eq!(
        owner.event(Event::Final(candidate(Finish::Stop)), 4),
        Err(ContractError::State)
    );
    assert!(owner.contract.candidate().is_none());
}

#[test]
fn record_count_boundary_is_enforced_even_for_tiny_valid_activity() {
    let mut owner = launched(&[Feature::FinalOutput], Class::OutputOnly);
    for _ in 0..65_536 {
        owner.event(Event::Activity, 1).unwrap();
    }
    assert_eq!(owner.event(Event::Activity, 1), Err(ContractError::Bound));
    assert_eq!(owner.contract.phase(), Phase::Unknown);
}

#[test]
fn normalization_checks_each_byte_bound_without_truncation_or_utf8_character_counting() {
    let mut at_bound = candidate(Finish::Stop);
    at_bound.text = "é".repeat(524_288);
    at_bound.raw = vec![b' '; 1_048_576];
    assert_eq!(
        inference::normalize(at_bound.clone(), PROVIDER).unwrap(),
        at_bound
    );
    for field in 0..4 {
        let mut value = candidate(Finish::Stop);
        match field {
            0 => value.text = "é".repeat(524_289),
            1 => value.raw = vec![b'x'; 1_048_577],
            2 => {
                let mut observed = identity(IdentityOrigin::ProviderResponse);
                observed.raw = vec![b'x'; 65_537];
                value.identity = Some(observed);
            }
            _ => {
                let mut usage = reported(0, UsageStage::Final);
                if let Usage::Reported { raw, .. } = &mut usage {
                    *raw = vec![b'x'; 65_537];
                }
                value.usage = usage;
            }
        }
        assert_eq!(
            inference::normalize(value, PROVIDER),
            Err(ContractError::Bound)
        );
    }
    let mut empty = candidate(Finish::Stop);
    empty.raw.clear();
    assert_eq!(
        inference::normalize(empty, PROVIDER),
        Err(ContractError::Bound)
    );
}
