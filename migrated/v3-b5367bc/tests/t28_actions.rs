//! T28 action-catalogue cases (`T28-AC-nn`).
//!
//! The first family is a **known-answer test whose answers come from an independent
//! source**: `corpus/PLAN_habitat_engine.json` declares the 21 actions, and these cases read
//! that file and compare the compiled catalogue against it entry by entry. Comparing the
//! table with itself would measure self-consistency, and self-consistency is exactly what a
//! projection bug preserves.
//!
//! The contract's required proof is *"complete 21-action owner projection; schema drift;
//! unknown/hidden action; denied effect; CLI/tool/UDS parity and literal input
//! preservation"*.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;

use habitat_engine::actions::{
    Action, CATALOGUE, CATALOGUE_REVISION, Caller, Catalogue, DECLARED_ACTIONS, Effect, MAX_PAGE,
    Owner, Refusal,
};
use serde_json::Value;

/// The HEE3-Control/1 receiver's cases: the catalogue served over the wire.
#[path = "t28_control.rs"]
mod control;

/// IPC01: the socket, its custody, the peer principal, the grant store and the binary.
#[path = "t28_socket.rs"]
mod socket;

/// The coordinator's start: the active generation, startup reconciliation, `health`.
#[path = "t28_coordinator.rs"]
mod coordinator;

/// The task owner: `task.submit` and `task.get` over a real ledger.
#[path = "t28_tasks.rs"]
mod tasks;

/// B07: `task.preview` composed over the declared recipes and the roster, writing nothing.
#[path = "t28_preview.rs"]
mod preview;

/// B09: `task.get`'s evidence views, read with their identity and checked now.
#[path = "t28_evidence.rs"]
mod evidence;

/// The offline native fixture shared with the t08 contract battery (B14a-4, A12).
#[path = "t08_rig.rs"]
mod t08_rig;

/// B14a-1c: the attempt lifecycle over the real ledger (`app::runtime`).
#[path = "t28_runtime.rs"]
mod runtime;

type Outcome = Result<(), Box<dyn Error>>;

/// The plan spine's own action list — the independent source these cases compare against.
fn declared() -> Result<Vec<Value>, Box<dyn Error>> {
    let raw = fs::read_to_string("corpus/PLAN_habitat_engine.json")?;
    let plan: Value = serde_json::from_str(&raw)?;
    Ok(plan["schematic_atlas"]["actions"]
        .as_array()
        .ok_or("schematic_atlas.actions is not an array")?
        .clone())
}

fn field<'a>(entry: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    entry[key]
        .as_str()
        .ok_or_else(|| format!("{key} missing").into())
}

/// A caller that can see everything and do everything.
fn omnipotent() -> Caller {
    let mut caller = Caller::new();
    for owner in Owner::ALL {
        caller = caller.seeing(owner);
    }
    for effect in Effect::ALL {
        caller = caller.granted(effect);
    }
    caller
}

// ---------------------------------------- the projection, against its declaring artefact

/// T28-AC-01 · the compiled catalogue has exactly as many entries as the plan spine declares.
#[test]
fn the_catalogue_size_matches_the_plan_spine() -> Outcome {
    let spine = declared()?;
    assert_eq!(spine.len(), DECLARED_ACTIONS);
    assert_eq!(CATALOGUE.len(), DECLARED_ACTIONS);
    assert_eq!(Catalogue::all().len(), DECLARED_ACTIONS);
    Ok(())
}

/// T28-AC-02 · every declared action id is present in the compiled catalogue, and the
/// catalogue declares no id the spine does not. The comparison is set-wise in **both**
/// directions: an include list can only see what it was given.
#[test]
fn the_action_id_sets_agree_in_both_directions() -> Outcome {
    let spine: Vec<String> = declared()?
        .iter()
        .map(|entry| field(entry, "id").map(str::to_owned))
        .collect::<Result<_, _>>()?;
    let compiled: Vec<String> = CATALOGUE.iter().map(|a| a.id.to_owned()).collect();
    let missing: Vec<&String> = spine.iter().filter(|id| !compiled.contains(id)).collect();
    let extra: Vec<&String> = compiled.iter().filter(|id| !spine.contains(id)).collect();
    assert!(
        missing.is_empty(),
        "declared but not projected: {missing:?}"
    );
    assert!(extra.is_empty(), "projected but not declared: {extra:?}");
    Ok(())
}

/// T28-AC-03 · for every action, owner, effect, version, CLI, tool and capability match the
/// spine exactly. This is the schema-drift proof: any field diverging is a named failure.
#[test]
fn every_projected_field_matches_the_spine() -> Outcome {
    let spine: BTreeMap<String, Value> = declared()?
        .into_iter()
        .map(|entry| Ok::<_, Box<dyn Error>>((field(&entry, "id")?.to_owned(), entry)))
        .collect::<Result<_, _>>()?;
    for action in CATALOGUE {
        let entry = spine
            .get(action.id)
            .ok_or_else(|| format!("{} is not declared", action.id))?;
        assert_eq!(
            action.owner.name(),
            field(entry, "owner")?,
            "{} owner",
            action.id
        );
        assert_eq!(
            action.effect.name(),
            field(entry, "effect")?,
            "{} effect",
            action.id
        );
        assert_eq!(
            action.version,
            field(entry, "version")?,
            "{} version",
            action.id
        );
        assert_eq!(action.cli, field(entry, "cli")?, "{} cli", action.id);
        assert_eq!(
            action.tool,
            entry["tool"].as_str(),
            "{} tool (the spine declares null for five actions)",
            action.id
        );
        assert_eq!(
            action.capability,
            field(entry, "capability")?,
            "{} capability",
            action.id
        );
    }
    Ok(())
}

/// T28-AC-04 · the catalogue preserves the spine's order, so a reader comparing the two by
/// eye is comparing the same sequence.
#[test]
fn the_catalogue_preserves_the_spine_order() -> Outcome {
    let spine: Vec<String> = declared()?
        .iter()
        .map(|entry| field(entry, "id").map(str::to_owned))
        .collect::<Result<_, _>>()?;
    let compiled: Vec<String> = CATALOGUE.iter().map(|a| a.id.to_owned()).collect();
    assert_eq!(compiled, spine);
    Ok(())
}

/// T28-AC-05 · every action id is unique; a duplicate would make `find` ambiguous.
#[test]
fn action_ids_are_unique() {
    let mut ids: Vec<&str> = CATALOGUE.iter().map(|a| a.id).collect();
    let before = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), before);
}

/// T28-AC-06 · every CLI projection is unique, and every tool projection that exists is
/// unique, so a wrapper name resolves to exactly one owner.
///
/// Five actions have no tool projection at all. The first version of this case compared 21
/// tool names and found 17 distinct ones — because the table had written the literal string
/// `"None"` for each of the five. The absence had to be modelled, not spelled.
#[test]
fn cli_and_present_tool_projections_are_unique() {
    let mut cli: Vec<&str> = CATALOGUE.iter().map(|a| a.cli).collect();
    let before = cli.len();
    cli.sort_unstable();
    cli.dedup();
    assert_eq!(cli.len(), before, "cli projections collide");
    assert_eq!(before, DECLARED_ACTIONS);

    let mut tools: Vec<&str> = CATALOGUE.iter().filter_map(|a| a.tool).collect();
    let present = tools.len();
    tools.sort_unstable();
    tools.dedup();
    assert_eq!(tools.len(), present, "tool projections collide");
    assert_eq!(present, 16, "sixteen actions carry a tool projection");
    assert_eq!(
        CATALOGUE.iter().filter(|a| a.tool.is_none()).count(),
        5,
        "and five carry none"
    );
}

/// T28-AC-07 · every owner named in the catalogue is a declared owner, and every declared
/// owner is used — an unused owner would be a projection that lost an action.
#[test]
fn every_owner_is_declared_and_used() {
    let used: Vec<Owner> = Owner::ALL
        .into_iter()
        .filter(|owner| CATALOGUE.iter().any(|a| a.owner == *owner))
        .collect();
    assert_eq!(used.len(), Owner::ALL.len(), "an owner projects no action");
    for owner in Owner::ALL {
        assert!(!owner.name().is_empty());
        assert_eq!(owner.to_string(), owner.name());
    }
}

/// T28-AC-08 · every effect the spine uses is representable, and every representable effect
/// is used by at least one action.
#[test]
fn every_effect_is_declared_and_used() -> Outcome {
    let spine: Vec<String> = declared()?
        .iter()
        .map(|entry| field(entry, "effect").map(str::to_owned))
        .collect::<Result<_, _>>()?;
    for effect in Effect::ALL {
        assert_eq!(
            effect.to_string(),
            effect.name(),
            "Display for {}",
            effect.name()
        );
        assert!(
            spine.iter().any(|name| name == effect.name()),
            "{} is representable but never declared",
            effect.name()
        );
        assert!(
            CATALOGUE.iter().any(|a| a.effect == effect),
            "{} is declared but projects no action",
            effect.name()
        );
    }
    Ok(())
}

/// T28-AC-09 · exactly three effects are non-mutating, and they are the read family.
#[test]
fn only_the_read_family_is_non_mutating() {
    let readonly: Vec<&str> = Effect::ALL
        .into_iter()
        .filter(|effect| !effect.mutates())
        .map(Effect::name)
        .collect();
    assert_eq!(readonly, vec!["read", "read stream", "read-only planning"]);
}

/// T28-AC-10 · the catalogue revision is pinned.
#[test]
fn the_catalogue_revision_is_pinned() {
    assert_eq!(CATALOGUE_REVISION, 1);
    assert_eq!(DECLARED_ACTIONS, 21);
    assert_eq!(MAX_PAGE, 32);
}

// ---------------------------------------- visibility: seeing is not doing

/// T28-AC-11 · a caller that sees nothing lists nothing, even though 21 actions exist.
#[test]
fn a_caller_that_sees_nothing_lists_nothing() -> Outcome {
    let page = Catalogue::list(&Caller::new(), 0, MAX_PAGE)?;
    assert!(page.entries.is_empty());
    assert_eq!(page.next, None);
    assert_eq!(page.revision, CATALOGUE_REVISION);
    assert_eq!(Catalogue::all().len(), DECLARED_ACTIONS, "they still exist");
    Ok(())
}

/// T28-AC-12 · visibility is per owner: a caller seeing one owner lists exactly that owner's
/// actions and no others.
#[test]
fn visibility_is_per_owner() -> Outcome {
    let caller = Caller::new().seeing(Owner::Task);
    let page = Catalogue::list(&caller, 0, MAX_PAGE)?;
    assert!(!page.entries.is_empty());
    assert!(page.entries.iter().all(|a| a.owner == Owner::Task));
    let expected = CATALOGUE.iter().filter(|a| a.owner == Owner::Task).count();
    assert_eq!(page.entries.len(), expected);
    Ok(())
}

/// T28-AC-13 · a hidden action reports `UnknownAction`, as an undeclared one does: the error
/// must not tell a caller that something it may not see exists.
#[test]
fn a_hidden_action_is_reported_as_unknown() -> Outcome {
    let caller = Caller::new().seeing(Owner::Task);
    let hidden = CATALOGUE
        .iter()
        .find(|a| a.owner != Owner::Task)
        .ok_or("expected an action owned elsewhere")?;
    assert_eq!(
        Catalogue::inspect(&caller, hidden.id, hidden.version).map(|_| ()),
        Err(Refusal::UnknownAction),
        "visibility is not leaked by the error"
    );
    assert_eq!(
        Catalogue::inspect(&caller, "no.such.action", "1 (proposed)").map(|_| ()),
        Err(Refusal::UnknownAction),
        "and an action that truly does not exist is indistinguishable"
    );
    Ok(())
}

/// T28-AC-14 · a visible action inspects successfully and returns its whole entry.
#[test]
fn a_visible_action_inspects_to_its_whole_entry() -> Outcome {
    let caller = omnipotent();
    let expected = Catalogue::find("task.get")?;
    let seen = Catalogue::inspect(&caller, "task.get", expected.version)?;
    assert_eq!(seen, expected);
    assert_eq!(seen.owner, Owner::Task);
    assert_eq!(seen.effect, Effect::Read);
    Ok(())
}

/// The readback column against the plan spine's `retry_readback` sentences, which the catalogue
/// cannot see: an action that changes nothing is its own readback, and one that changes state
/// reads back through a read action -- the one its sentence names, when it names one. Only
/// `service.probe`'s sentence names none; the published result schema pins that row, and
/// `t28_control` compares the whole column with it.
#[test]
fn every_readback_action_is_the_read_its_spine_sentence_names() -> Outcome {
    let spine: BTreeMap<String, Value> = declared()?
        .into_iter()
        .map(|entry| Ok::<_, Box<dyn Error>>((field(&entry, "id")?.to_owned(), entry)))
        .collect::<Result<_, _>>()?;
    let mut routed = Vec::new();
    for action in CATALOGUE {
        let sentence = field(
            spine.get(action.id).ok_or("undeclared action")?,
            "retry_readback",
        )?;
        let named: Vec<&str> = CATALOGUE
            .iter()
            .map(|other| other.id)
            .filter(|&other| other != action.id && sentence.contains(other))
            .collect();
        match action.readback_action {
            None => assert!(
                !action.effect.mutates(),
                "{} changes state and names no readback",
                action.id
            ),
            Some(readback) => {
                assert!(action.effect.mutates(), "{} is its own readback", action.id);
                assert_eq!(
                    Catalogue::find(readback)?.effect,
                    Effect::Read,
                    "{} reads back through {readback}",
                    action.id
                );
                assert!(
                    named.is_empty() || named == [readback],
                    "{}: spine names {named:?}, catalogue {readback}",
                    action.id
                );
                routed.push((action.id, readback, named.len()));
            }
        }
    }
    // Pinned off the identity (F129): eight state-changing actions, seven named by the spine.
    assert_eq!(
        routed,
        [
            ("task.submit", "task.get", 1),
            ("task.cancel", "task.get", 1),
            ("task.resolve", "task.get", 1),
            ("roster.update", "roster.inspect", 1),
            ("roster.disable", "roster.inspect", 1),
            ("service.probe", "service.inspect", 0),
            ("service.action", "service.inspect", 1),
            ("analysis.request", "analysis.get", 1),
        ]
    );
    Ok(())
}

/// T28-AC-15 · an unknown version of a visible action is refused distinctly from an unknown
/// action, because the caller can act on the difference.
#[test]
fn an_unknown_version_is_refused_distinctly() {
    let caller = omnipotent();
    assert_eq!(
        Catalogue::inspect(&caller, "task.get", "2 (proposed)").map(|_| ()),
        Err(Refusal::UnknownVersion)
    );
}

/// T28-AC-16 · seeing an action does not grant it. A caller that can see every action but
/// holds no effect grant is refused at dispatch for every one of the 21.
#[test]
fn seeing_every_action_grants_none_of_them() -> Outcome {
    let mut caller = Caller::new();
    for owner in Owner::ALL {
        caller = caller.seeing(owner);
    }
    let page = Catalogue::list(&caller, 0, MAX_PAGE)?;
    assert_eq!(page.entries.len(), MAX_PAGE.min(DECLARED_ACTIONS));
    for action in CATALOGUE {
        assert_eq!(
            Catalogue::validate(&caller, action.id, action.version).map(|_| ()),
            Err(Refusal::UngrantedEffect),
            "{} was dispatchable without a grant",
            action.id
        );
    }
    Ok(())
}

/// T28-AC-17 · a grant without visibility is equally useless: the action is unknown.
#[test]
fn a_grant_without_visibility_dispatches_nothing() {
    let mut caller = Caller::new();
    for effect in Effect::ALL {
        caller = caller.granted(effect);
    }
    for action in CATALOGUE {
        assert_eq!(
            Catalogue::validate(&caller, action.id, action.version).map(|_| ()),
            Err(Refusal::UnknownAction),
            "{} dispatched without visibility",
            action.id
        );
    }
}

/// T28-AC-18 · a caller holding only read grants cannot dispatch any mutating action, and
/// can dispatch every read one it can see.
#[test]
fn read_grants_reach_exactly_the_read_actions() {
    let mut caller = Caller::new();
    for owner in Owner::ALL {
        caller = caller.seeing(owner);
    }
    for effect in Effect::ALL.into_iter().filter(|e| !e.mutates()) {
        caller = caller.granted(effect);
    }
    for action in CATALOGUE {
        let outcome = Catalogue::validate(&caller, action.id, action.version);
        assert_eq!(
            outcome.is_ok(),
            !action.effect.mutates(),
            "{} with effect {}",
            action.id,
            action.effect.name()
        );
    }
}

/// T28-AC-19 · the caller predicates agree with the refusals, so the two cannot drift.
#[test]
fn caller_predicates_agree_with_the_refusals() {
    let caller = Caller::new().seeing(Owner::Task).granted(Effect::Read);
    assert!(caller.sees(Owner::Task));
    assert!(!caller.sees(Owner::Roster));
    assert!(caller.holds(Effect::Read));
    assert!(!caller.holds(Effect::DurableAdmission));
    assert!(Catalogue::validate(&caller, "task.get", "1 (proposed)").is_ok());
    assert_eq!(
        Catalogue::validate(&caller, "task.submit", "1 (proposed)").map(|_| ()),
        Err(Refusal::UngrantedEffect)
    );
}

/// T28-AC-20 · repeating a `seeing` or `granted` is not two grants.
#[test]
fn repeated_grants_are_idempotent() -> Outcome {
    let caller = Caller::new()
        .seeing(Owner::Task)
        .seeing(Owner::Task)
        .granted(Effect::Read)
        .granted(Effect::Read);
    let page = Catalogue::list(&caller, 0, MAX_PAGE)?;
    let expected = CATALOGUE.iter().filter(|a| a.owner == Owner::Task).count();
    assert_eq!(page.entries.len(), expected);
    Ok(())
}

// ---------------------------------------- dispatch derives its owner

/// T28-AC-21 · a dispatch's owner comes from the catalogue, never from the request. This is
/// the whole of "wrappers must derive from the same owner".
#[test]
fn a_dispatch_owner_comes_from_the_catalogue() -> Outcome {
    let caller = omnipotent();
    for action in CATALOGUE {
        let dispatch = Catalogue::validate(&caller, action.id, action.version)?;
        assert_eq!(dispatch.owner(), action.owner, "{}", action.id);
        assert_eq!(dispatch.action(), action);
    }
    Ok(())
}

/// T28-AC-22 · the CLI, tool and API projections of one action all resolve to the same
/// owner, because none of them supplies one. This is CLI/tool/UDS parity.
#[test]
fn every_transport_projection_resolves_to_one_owner() -> Outcome {
    let caller = omnipotent();
    for action in CATALOGUE {
        let by_id = Catalogue::find(action.id)?;
        let dispatch = Catalogue::validate(&caller, action.id, action.version)?;
        // The CLI string and the tool name are descriptions of this same entry; there is no
        // API that turns either of them into a different owner.
        assert_eq!(by_id.cli, action.cli);
        assert_eq!(by_id.tool, action.tool);
        // An action without a tool projection still dispatches; the absence of a wrapper
        // name is not the absence of the action.
        assert_eq!(dispatch.owner(), by_id.owner);
    }
    Ok(())
}

/// T28-AC-23 · the action id is preserved literally through validation — no normalisation,
/// no case folding, no trimming.
#[test]
fn the_action_id_is_preserved_literally() -> Outcome {
    let caller = omnipotent();
    let dispatch = Catalogue::validate(&caller, "task.submit", "1 (proposed)")?;
    assert_eq!(dispatch.action().id, "task.submit");
    for variant in ["Task.Submit", "task.submit ", " task.submit", "TASK.SUBMIT"] {
        assert_eq!(
            Catalogue::validate(&caller, variant, "1 (proposed)").map(|_| ()),
            Err(Refusal::UnknownAction),
            "{variant} was normalised into a match"
        );
    }
    Ok(())
}

/// T28-AC-24 · a version string is matched literally too.
#[test]
fn the_version_is_matched_literally() {
    let caller = omnipotent();
    for variant in ["1", "1 (Proposed)", "1(proposed)", " 1 (proposed)"] {
        assert_eq!(
            Catalogue::validate(&caller, "task.get", variant).map(|_| ()),
            Err(Refusal::UnknownVersion),
            "{variant} was normalised into a match"
        );
    }
}

// ---------------------------------------- paging

/// T28-AC-25 · paging walks the whole visible catalogue exactly once, with no repeats and no
/// gaps.
#[test]
fn paging_walks_the_catalogue_exactly_once() -> Outcome {
    let caller = omnipotent();
    let mut seen: Vec<String> = Vec::new();
    let mut from = 0;
    // The loop takes a budget. Without one, a mutation of the `next` arithmetic does not fail
    // this case, it hangs it — and mutation testing reports TIMEOUT rather than a kill, which
    // reads as a tooling problem instead of a test defect. The bound is stated with both
    // numbers so a breach says what it expected.
    let budget = DECLARED_ACTIONS + 1;
    let mut rounds = 0;
    loop {
        rounds += 1;
        assert!(
            rounds <= budget,
            "paging did not terminate: {rounds} rounds for {DECLARED_ACTIONS} actions"
        );
        let page = Catalogue::list(&caller, from, 4)?;
        seen.extend(page.entries.iter().map(|a| a.id.to_owned()));
        match page.next {
            Some(next) => from = next,
            None => break,
        }
    }
    let expected: Vec<String> = CATALOGUE.iter().map(|a| a.id.to_owned()).collect();
    assert_eq!(seen, expected);
    Ok(())
}

/// T28-AC-26 · a page wider than the bound is refused before any entry is copied.
#[test]
fn a_page_wider_than_the_bound_is_refused() -> Outcome {
    let caller = omnipotent();
    assert_eq!(
        Catalogue::list(&caller, 0, MAX_PAGE + 1).map(|_| ()),
        Err(Refusal::PageTooWide)
    );
    let ok = Catalogue::list(&caller, 0, MAX_PAGE)?;
    assert_eq!(ok.entries.len(), DECLARED_ACTIONS.min(MAX_PAGE));
    Ok(())
}

/// T28-AC-27 · a page token past the end is refused; a token exactly at the end is an empty
/// final page, not an error.
#[test]
fn the_paging_boundary_is_asserted_from_both_sides() -> Outcome {
    let caller = omnipotent();
    let end = Catalogue::list(&caller, DECLARED_ACTIONS, 4)?;
    assert!(end.entries.is_empty());
    assert_eq!(end.next, None);
    assert_eq!(
        Catalogue::list(&caller, DECLARED_ACTIONS + 1, 4).map(|_| ()),
        Err(Refusal::PageOutOfRange)
    );
    Ok(())
}

/// T28-AC-28 · a zero-width page returns nothing and resumes where it started, so a caller
/// polling with zero cannot silently skip an action.
#[test]
fn a_zero_width_page_does_not_advance() -> Outcome {
    let caller = omnipotent();
    let page = Catalogue::list(&caller, 3, 0)?;
    assert!(page.entries.is_empty());
    assert_eq!(page.next, Some(3));
    Ok(())
}

/// T28-AC-29 · paging is computed over the **visible** catalogue, so a restricted caller's
/// page tokens are dense rather than sparse.
#[test]
fn paging_is_dense_over_the_visible_catalogue() -> Outcome {
    let caller = Caller::new().seeing(Owner::Roster);
    let expected = CATALOGUE
        .iter()
        .filter(|a| a.owner == Owner::Roster)
        .count();
    let first = Catalogue::list(&caller, 0, 2)?;
    assert_eq!(first.entries.len(), 2.min(expected));
    assert_eq!(first.next, (expected > 2).then_some(2));
    Ok(())
}

/// Every refusal the module declares. [`declared_position`] is an exhaustive match, so a
/// variant added to `Refusal` without a row here does not compile.
const REFUSALS: [Refusal; 5] = [
    Refusal::UnknownAction,
    Refusal::UnknownVersion,
    Refusal::UngrantedEffect,
    Refusal::PageTooWide,
    Refusal::PageOutOfRange,
];

const fn declared_position(refusal: Refusal) -> usize {
    match refusal {
        Refusal::UnknownAction => 0,
        Refusal::UnknownVersion => 1,
        Refusal::UngrantedEffect => 2,
        Refusal::PageTooWide => 3,
        Refusal::PageOutOfRange => 4,
    }
}

/// T28-AC-30 · every refusal has a distinct name and none is a substring of another.
#[test]
fn refusal_names_are_distinct_and_non_overlapping() {
    let all = REFUSALS;
    for (i, a) in all.iter().enumerate() {
        assert!(!a.name().is_empty());
        assert_eq!(a.to_string(), a.name());
        for (j, b) in all.iter().enumerate() {
            if i != j {
                assert!(
                    !a.name().contains(b.name()),
                    "{} contains {}",
                    a.name(),
                    b.name()
                );
            }
        }
    }
}

/// Completion standard G10 (dead paths removed): every declared refusal is one the public
/// catalogue operations can actually produce. The probe is a grid over the world, not one
/// hand-picked input per refusal: every caller shape (sees nothing; sees every owner without
/// grants; sees and holds everything), every declared action plus an undeclared identity, the
/// declared and an undeclared version, and page bounds on both sides of each edge. A variant
/// nothing constructs is absent from the produced set and fails here by name.
#[test]
fn every_declared_refusal_is_produced_by_a_catalogue_operation() {
    for (at, refusal) in REFUSALS.iter().enumerate() {
        assert_eq!(declared_position(*refusal), at, "{refusal:?} out of place");
    }
    let blind = Caller::new();
    let seeing = Owner::ALL.into_iter().fold(Caller::new(), Caller::seeing);
    let holding = Effect::ALL
        .into_iter()
        .fold(seeing.clone(), Caller::granted);
    let mut produced = std::collections::BTreeSet::new();
    let mut probes = 0_usize;
    let mut note = |result: Result<(), Refusal>| {
        probes += 1;
        if let Err(refusal) = result {
            produced.insert(declared_position(refusal));
        }
    };
    let ids: Vec<&str> = Catalogue::all()
        .iter()
        .map(|action| action.id)
        .chain(["no.such.action"])
        .collect();
    let past_end = Catalogue::all().len() + 1;
    for caller in [&blind, &seeing, &holding] {
        for id in &ids {
            let declared = Catalogue::find(id).map_or("1", |action| action.version);
            for version in [declared, "0 (never declared)"] {
                note(Catalogue::inspect(caller, id, version).map(|_| ()));
                note(Catalogue::validate(caller, id, version).map(|_| ()));
            }
            note(Catalogue::find(id).map(|_| ()));
            note(Catalogue::search(caller, None, Some(id), 1).map(|_| ()));
        }
        for from in [0, past_end] {
            for limit in [1, MAX_PAGE, MAX_PAGE + 1] {
                note(Catalogue::list(caller, from, limit).map(|_| ()));
            }
        }
    }
    let expected_probes = 3 * (ids.len() * 6 + 6);
    assert_eq!(probes, expected_probes, "the grid ran every probe");
    let missing: Vec<Refusal> = REFUSALS
        .into_iter()
        .filter(|refusal| !produced.contains(&declared_position(*refusal)))
        .collect();
    assert!(
        missing.is_empty(),
        "declared but never produced by any of {probes} probes: {missing:?}"
    );
}

/// T28-AC-31 · `find` ignores visibility and is the only read that does, so an operator tool
/// can describe an action a caller cannot see without that leaking into the caller's view.
#[test]
fn find_ignores_visibility() -> Outcome {
    let action: Action = Catalogue::find("service.action")?;
    assert_eq!(action.owner, Owner::Service);
    assert_eq!(action.effect, Effect::ManagedLifecycle);
    assert_eq!(
        Catalogue::find("no.such.action").map(|_| ()),
        Err(Refusal::UnknownAction)
    );
    Ok(())
}

/// T28-AC-32 · the mutating actions are exactly those the spine gives a mutating effect, so
/// the read/write split is the spine's and not this module's opinion.
#[test]
fn the_mutating_set_comes_from_the_spine() -> Outcome {
    let spine = declared()?;
    for entry in spine {
        let id = field(&entry, "id")?;
        let effect = field(&entry, "effect")?;
        let action = Catalogue::find(id)?;
        let readonly = matches!(effect, "read" | "read stream" | "read-only planning");
        assert_eq!(action.effect.mutates(), !readonly, "{id}");
    }
    Ok(())
}

// ---------------------------------------- per-owner projection

/// Assert one owner's slice of the catalogue against the spine: the ids it owns, and that
/// every one of them resolves back to it.
fn owner_slice(owner: Owner) -> Result<Vec<String>, Box<dyn Error>> {
    let spine = declared()?;
    let expected: Vec<String> = spine
        .iter()
        .filter(|entry| entry["owner"].as_str() == Some(owner.name()))
        .map(|entry| field(entry, "id").map(str::to_owned))
        .collect::<Result<_, _>>()?;
    let projected: Vec<String> = CATALOGUE
        .iter()
        .filter(|action| action.owner == owner)
        .map(|action| action.id.to_owned())
        .collect();
    assert_eq!(projected, expected, "{} slice", owner.name());
    for id in &projected {
        assert_eq!(Catalogue::find(id)?.owner, owner, "{id} resolves elsewhere");
    }
    Ok(projected)
}

/// T28-AC-33 · the `actions` owner projects exactly its declared actions.
#[test]
fn the_actions_owner_slice_matches() -> Outcome {
    assert_eq!(owner_slice(Owner::Actions)?.len(), 2);
    Ok(())
}

/// T28-AC-34 · the `task` owner projects exactly its declared actions.
#[test]
fn the_task_owner_slice_matches() -> Outcome {
    assert_eq!(owner_slice(Owner::Task)?.len(), 6);
    Ok(())
}

/// T28-AC-35 · the `cohort` owner projects exactly its declared actions.
#[test]
fn the_cohort_owner_slice_matches() -> Outcome {
    assert_eq!(owner_slice(Owner::Cohort)?.len(), 2);
    Ok(())
}

/// T28-AC-36 · the `roster` owner projects exactly its declared actions.
#[test]
fn the_roster_owner_slice_matches() -> Outcome {
    assert_eq!(owner_slice(Owner::Roster)?.len(), 4);
    Ok(())
}

/// T28-AC-37 · the `service` owner projects exactly its declared actions.
#[test]
fn the_service_owner_slice_matches() -> Outcome {
    assert_eq!(owner_slice(Owner::Service)?.len(), 3);
    Ok(())
}

/// T28-AC-38 · the `numerical` owner projects exactly its declared actions.
#[test]
fn the_numerical_owner_slice_matches() -> Outcome {
    assert_eq!(owner_slice(Owner::Numerical)?.len(), 2);
    Ok(())
}

/// T28-AC-39 · the `notify` owner projects exactly its declared actions.
#[test]
fn the_notify_owner_slice_matches() -> Outcome {
    assert_eq!(owner_slice(Owner::Notify)?.len(), 1);
    Ok(())
}

/// T28-AC-40 · the `app` owner projects exactly its declared actions.
#[test]
fn the_app_owner_slice_matches() -> Outcome {
    assert_eq!(owner_slice(Owner::App)?.len(), 1);
    Ok(())
}

/// T28-AC-41 · the per-owner slices partition the catalogue: every action belongs to exactly
/// one owner, and the eight slices sum to 21. A partition is what an include list is not.
#[test]
fn the_owner_slices_partition_the_catalogue() -> Outcome {
    let mut total = 0;
    for owner in Owner::ALL {
        total += owner_slice(owner)?.len();
    }
    assert_eq!(total, DECLARED_ACTIONS);
    Ok(())
}

// ---------------------------------------- effect families

/// T28-AC-42 · every action carrying a mutating effect requires a grant that a read-only
/// caller does not have, for each mutating effect in turn.
#[test]
fn each_mutating_effect_is_separately_gated() {
    let mut seeing = Caller::new();
    for owner in Owner::ALL {
        seeing = seeing.seeing(owner);
    }
    for effect in Effect::ALL.into_iter().filter(|e| e.mutates()) {
        let with = seeing.clone().granted(effect);
        for action in CATALOGUE.iter().filter(|a| a.effect == effect) {
            assert!(
                Catalogue::validate(&with, action.id, action.version).is_ok(),
                "{} refused with its own grant",
                action.id
            );
            assert_eq!(
                Catalogue::validate(&seeing, action.id, action.version).map(|_| ()),
                Err(Refusal::UngrantedEffect),
                "{} admitted without a grant",
                action.id
            );
        }
    }
}

/// T28-AC-43 · holding one effect grant does not admit an action with a different effect.
#[test]
fn one_grant_does_not_admit_another_effect() {
    let caller = Caller::new()
        .seeing(Owner::Roster)
        .granted(Effect::ConfigurationMutation);
    assert!(Catalogue::validate(&caller, "roster.update", "1 (proposed)").is_ok());
    assert_eq!(
        Catalogue::validate(&caller, "roster.list", "1 (proposed)").map(|_| ()),
        Err(Refusal::UngrantedEffect),
        "a mutation grant is not a read grant"
    );
}

/// T28-AC-44 · the read-stream effect is distinct from read, and a read grant does not admit
/// a stream.
#[test]
fn a_read_grant_does_not_admit_a_stream() {
    let caller = Caller::new().seeing(Owner::Notify).granted(Effect::Read);
    assert_eq!(
        Catalogue::validate(&caller, "events.subscribe", "1 (proposed)").map(|_| ()),
        Err(Refusal::UngrantedEffect)
    );
    let streaming = caller.granted(Effect::ReadStream);
    assert!(Catalogue::validate(&streaming, "events.subscribe", "1 (proposed)").is_ok());
}

/// T28-AC-45 · read-only planning is distinct from durable admission: previewing a task does
/// not grant submitting one.
#[test]
fn previewing_does_not_grant_submitting() {
    let caller = Caller::new()
        .seeing(Owner::Task)
        .granted(Effect::ReadOnlyPlanning);
    assert!(Catalogue::validate(&caller, "task.preview", "1 (proposed)").is_ok());
    assert_eq!(
        Catalogue::validate(&caller, "task.submit", "1 (proposed)").map(|_| ()),
        Err(Refusal::UngrantedEffect)
    );
}

/// T28-AC-46 · a bounded probe grant does not admit a managed lifecycle effect, which is the
/// difference between looking at a service and acting on it.
#[test]
fn probing_does_not_grant_acting() {
    let caller = Caller::new()
        .seeing(Owner::Service)
        .granted(Effect::BoundedProbe);
    assert!(Catalogue::validate(&caller, "service.probe", "1 (proposed)").is_ok());
    assert_eq!(
        Catalogue::validate(&caller, "service.action", "1 (proposed)").map(|_| ()),
        Err(Refusal::UngrantedEffect)
    );
}

/// T28-AC-47 · cancelling and resolving are separate grants: recording a disposition is not
/// the same authority as asking for a cancellation.
#[test]
fn cancelling_and_resolving_are_separate_grants() {
    let caller = Caller::new()
        .seeing(Owner::Task)
        .granted(Effect::CancelIntent);
    assert!(Catalogue::validate(&caller, "task.cancel", "1 (proposed)").is_ok());
    assert_eq!(
        Catalogue::validate(&caller, "task.resolve", "1 (proposed)").map(|_| ()),
        Err(Refusal::UngrantedEffect)
    );
}

// ---------------------------------------- capability text and inspection

/// T28-AC-48 · every action carries a non-empty capability sentence, so inspection always
/// tells a caller what it would need.
#[test]
fn every_action_states_the_capability_it_needs() {
    for action in CATALOGUE {
        assert!(
            !action.capability.is_empty(),
            "{} has no capability",
            action.id
        );
        assert!(!action.version.is_empty(), "{} has no version", action.id);
        assert!(!action.cli.is_empty(), "{} has no cli", action.id);
    }
}

/// T28-AC-49 · inspection returns the capability text unchanged from the spine, so a caller
/// reading it is reading the declaration and not a paraphrase.
#[test]
fn inspection_returns_the_spine_capability_verbatim() -> Outcome {
    let caller = omnipotent();
    for entry in declared()? {
        let id = field(&entry, "id")?;
        let seen = Catalogue::inspect(&caller, id, field(&entry, "version")?)?;
        assert_eq!(seen.capability, field(&entry, "capability")?, "{id}");
    }
    Ok(())
}

/// T28-AC-50 · every action is inspectable and dispatchable by an omnipotent caller, so no
/// entry is unreachable through the API that advertises it.
#[test]
fn every_declared_action_is_reachable() -> Outcome {
    let caller = omnipotent();
    let mut reached = 0;
    for action in CATALOGUE {
        Catalogue::inspect(&caller, action.id, action.version)?;
        Catalogue::validate(&caller, action.id, action.version)?;
        reached += 1;
    }
    assert_eq!(reached, DECLARED_ACTIONS);
    Ok(())
}

/// T28-AC-51 · the catalogue is a description: `all` exposes every entry regardless of any
/// caller, and no method on `Action` returns a capability, a token or a dispatch.
#[test]
fn the_catalogue_is_a_description_not_a_grant() -> Outcome {
    let entries = Catalogue::all();
    assert_eq!(entries.len(), DECLARED_ACTIONS);
    // An `Action` is data. The only producer of an authorised `Dispatch` is `validate`, and
    // it takes a `Caller`. This case exists to fail if that ever stops being true.
    let action = Catalogue::find("service.action")?;
    assert_eq!(action.owner, Owner::Service);
    assert_eq!(
        Catalogue::validate(&Caller::new(), action.id, action.version).map(|_| ()),
        Err(Refusal::UnknownAction)
    );
    Ok(())
}

/// T28-AC-52 · a page's entries are exactly the catalogue's entries in order, for a caller
/// that sees everything, so paging does not reorder or transform.
#[test]
fn a_full_page_is_the_catalogue_in_order() -> Outcome {
    let caller = omnipotent();
    let page = Catalogue::list(&caller, 0, MAX_PAGE)?;
    let expected: Vec<Action> = CATALOGUE.into_iter().take(MAX_PAGE).collect();
    assert_eq!(page.entries, expected);
    Ok(())
}

// ---------------------------------------------------------------------------------------
// T28 clause: "One action definition drives CLI/Unix schemas and one selected LLM-tool
// adapter", bounded by "generated projections cannot drift".
//
// The claim the adapter makes is TOTALITY -- every catalogue action is offered or omitted by
// name -- so every denominator below comes from `Catalogue::all()`, never from the projection
// being tested and never from a literal. A count taken from the projection itself is equal by
// construction and cannot see a dropped action.
// ---------------------------------------------------------------------------------------

use habitat_engine::worker::tools::{
    MAX_TOOL_NAME, Omitted, Refusal as ToolRefusal, ToolDefinition, distinct_tool_names, project,
};

#[test]
fn every_catalogue_action_is_either_offered_or_omitted_by_name() -> Outcome {
    let projection = project(&omnipotent())?;
    assert!(projection.accounts_for_every_action());
    assert_eq!(
        projection.tools().len() + projection.omissions().len(),
        Catalogue::all().len(),
        "the projection must account for every action the catalogue declares"
    );
    Ok(())
}

#[test]
fn the_five_actions_without_a_tool_name_are_omitted_and_named() -> Outcome {
    // The count is derived from the catalogue, not typed: an action gaining a tool name must
    // move this number by itself rather than making a literal stale.
    let expected: Vec<&str> = CATALOGUE
        .iter()
        .filter(|action| action.tool.is_none())
        .map(|action| action.id)
        .collect();
    let projection = project(&omnipotent())?;
    let omitted: Vec<&str> = projection
        .omissions()
        .iter()
        .filter(|row| row.reason() == Omitted::NoToolName)
        .map(|row| row.action().id)
        .collect();
    assert_eq!(omitted, expected);
    // Off the identity element: the spine declares five such actions, and a projection that
    // silently dropped them would show an empty list here and still look coherent.
    assert_eq!(expected.len(), 5);
    Ok(())
}

#[test]
fn a_projected_tool_carries_the_actions_own_owner_and_effect() -> Outcome {
    // The no-widening claim, asserted over every tool rather than one example.
    let projection = project(&omnipotent())?;
    for tool in projection.tools() {
        let action = tool.action();
        assert_eq!(tool.owner(), action.owner, "{}", action.id);
        assert_eq!(tool.effect(), action.effect, "{}", action.id);
        assert_eq!(tool.mutates(), action.effect.mutates(), "{}", action.id);
        assert_eq!(tool.capability(), action.capability, "{}", action.id);
    }
    Ok(())
}

#[test]
fn a_tool_name_is_the_declared_one_never_derived_from_the_id() -> Outcome {
    // A name built from the id would agree today and diverge the first time an id changed
    // shape. Asserted against the spine's own field, for every tool.
    let projection = project(&omnipotent())?;
    for tool in projection.tools() {
        assert_eq!(
            Some(tool.name()),
            tool.action().tool,
            "{}",
            tool.action().id
        );
    }
    Ok(())
}

#[test]
fn every_tool_name_is_distinct() -> Outcome {
    let projection = project(&omnipotent())?;
    let mut names: Vec<&str> = projection.tools().iter().map(|tool| tool.name()).collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), total, "two actions project to one tool name");
    Ok(())
}

#[test]
fn a_caller_that_cannot_see_an_owner_is_not_offered_its_tools() -> Outcome {
    // The filter narrows and only narrows: the omission is recorded with its ground, so a
    // short projection cannot be mistaken for the whole catalogue.
    let mut caller = Caller::new();
    for effect in Effect::ALL {
        caller = caller.granted(effect);
    }
    caller = caller.seeing(Owner::Actions);
    let projection = project(&caller)?;
    assert!(projection.accounts_for_every_action());
    for tool in projection.tools() {
        assert_eq!(tool.owner(), Owner::Actions);
    }
    assert!(
        projection
            .omissions()
            .iter()
            .any(|row| row.reason() == Omitted::OwnerNotVisible),
        "an unseen owner must be recorded, not dropped"
    );
    Ok(())
}

#[test]
fn a_caller_holding_no_effects_is_offered_nothing_and_told_why() -> Outcome {
    let mut caller = Caller::new();
    for owner in Owner::ALL {
        caller = caller.seeing(owner);
    }
    let projection = project(&caller)?;
    assert!(projection.tools().is_empty());
    assert!(projection.accounts_for_every_action());
    // Every omission has a ground; none is unexplained.
    assert_eq!(
        projection.omissions().len(),
        Catalogue::all().len(),
        "with no effects held, every action must be accounted for as an omission"
    );
    Ok(())
}

#[test]
fn a_read_only_caller_is_offered_only_non_mutating_tools() -> Outcome {
    let mut caller = Caller::new();
    for owner in Owner::ALL {
        caller = caller.seeing(owner);
    }
    for effect in Effect::ALL.into_iter().filter(|e| !e.mutates()) {
        caller = caller.granted(effect);
    }
    let projection = project(&caller)?;
    assert!(
        !projection.tools().is_empty(),
        "read tools must still be offered"
    );
    for tool in projection.tools() {
        assert!(
            !tool.mutates(),
            "{} mutates and was offered",
            tool.action().id
        );
    }
    Ok(())
}

#[test]
fn an_action_with_no_tool_name_projects_to_nothing_rather_than_a_placeholder() -> Outcome {
    let bare = CATALOGUE
        .iter()
        .find(|action| action.tool.is_none())
        .ok_or("the spine declares no tool-less action")?;
    assert_eq!(ToolDefinition::of(*bare)?, None);
    Ok(())
}

#[test]
fn a_malformed_declared_tool_name_is_refused() -> Outcome {
    // These cannot arise from the compiled catalogue, so the case builds the Action value
    // directly: the refusal exists for the day the spine declares such a name, and a rule
    // that cannot be exercised is a rule nothing pins.
    let base = *CATALOGUE.first().ok_or("empty catalogue")?;
    let long = Box::leak(vec![b'a'; MAX_TOOL_NAME + 1].into_boxed_slice());
    let long = core::str::from_utf8(long)?;
    for (name, expected) in [
        ("", ToolRefusal::MalformedToolName),
        (long, ToolRefusal::MalformedToolName),
        ("Habitat_Tools", ToolRefusal::UnexpectedToolNameShape),
        ("habitat-tools", ToolRefusal::UnexpectedToolNameShape),
        ("habitat tools", ToolRefusal::UnexpectedToolNameShape),
    ] {
        let action = Action {
            tool: Some(name),
            ..base
        };
        assert_eq!(ToolDefinition::of(action), Err(expected), "{name:?}");
    }
    // The boundary from the other side, or only the refusing half is checked.
    let at_bound = Box::leak(vec![b'a'; MAX_TOOL_NAME].into_boxed_slice());
    let at_bound = core::str::from_utf8(at_bound)?;
    let action = Action {
        tool: Some(at_bound),
        ..base
    };
    assert!(ToolDefinition::of(action)?.is_some());
    Ok(())
}

#[test]
fn every_tool_refusal_renders_its_own_diagnostic() {
    // Whole values, one per variant: `contains` or a non-empty check would pass a renderer
    // that returned one fixed string, and the mutation run over tools.rs found exactly that
    // gap -- `name` replaced by "xyzzy" and `fmt` replaced by an empty write both survived.
    let rendered: Vec<(String, &str)> = [
        ToolRefusal::DuplicateToolName,
        ToolRefusal::MalformedToolName,
        ToolRefusal::UnexpectedToolNameShape,
    ]
    .into_iter()
    .map(|refusal| (refusal.to_string(), refusal.name()))
    .collect();
    assert_eq!(
        rendered,
        [
            (
                "two actions project to one tool name".to_owned(),
                "two actions project to one tool name"
            ),
            (
                "tool name is empty or over the length bound".to_owned(),
                "tool name is empty or over the length bound"
            ),
            (
                "tool name is not lowercase with underscores".to_owned(),
                "tool name is not lowercase with underscores"
            ),
        ]
    );
}

#[test]
fn every_omission_ground_has_a_distinct_name() {
    let names: Vec<&str> = [
        Omitted::NoToolName,
        Omitted::OwnerNotVisible,
        Omitted::EffectNotHeld,
    ]
    .into_iter()
    .map(Omitted::name)
    .collect();
    assert_eq!(
        names,
        ["no-tool-name", "owner-not-visible", "effect-not-held"]
    );
    let unique: std::collections::BTreeSet<&str> = names.iter().copied().collect();
    assert_eq!(unique.len(), names.len());
}

#[test]
fn the_tool_projection_and_the_cli_projection_describe_the_same_actions() -> Outcome {
    // The clause is "ONE action definition drives CLI/Unix schemas and one LLM-tool adapter".
    // Every action has a CLI projection; sixteen also have a tool. The two never disagree
    // about which action they describe, because both are fields on the same row.
    let projection = project(&omnipotent())?;
    for tool in projection.tools() {
        let action = tool.action();
        assert!(
            !action.cli.is_empty(),
            "{} has a tool but no CLI form",
            action.id
        );
        assert!(
            action.cli.contains(action.id) || action.cli.starts_with("habitat-engine"),
            "{}: the CLI form does not name its action",
            action.id
        );
    }
    Ok(())
}

/// Review D6: tool-name uniqueness is checked over the whole catalogue, before any caller
/// filtering. It was checked per caller against only the tools that caller could see, so a
/// duplicate whose first holder was hidden was never refused. A named action given twice is
/// refused; the live catalogue passes; an action with no tool name cannot collide.
#[test]
fn tool_name_uniqueness_is_a_property_of_the_catalogue() -> Result<(), Box<dyn Error>> {
    let named = *Catalogue::all()
        .iter()
        .find(|action| action.tool.is_some())
        .ok_or("the catalogue names no tool")?;
    assert_eq!(
        distinct_tool_names(&[named, named]),
        Err(ToolRefusal::DuplicateToolName)
    );
    assert_eq!(distinct_tool_names(Catalogue::all()), Ok(()));
    if let Some(unnamed) = Catalogue::all().iter().find(|action| action.tool.is_none()) {
        assert_eq!(distinct_tool_names(&[*unnamed, *unnamed]), Ok(()));
    }
    Ok(())
}

/// Review N2: the Rust catalogue and the control-v1 schema describe the same actions, and one
/// test now ties them. The wire classes the Rust effects project to are exactly the schema's
/// `ActionEffectV1` enum (every class reached, none invented), every declared action has a
/// request definition, and each action's wire version is that definition's `action_version`.
#[test]
fn the_catalogue_projects_exactly_onto_the_control_schema() -> Result<(), Box<dyn Error>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas/actions/control-v1.schema.json");
    let schema: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
    let defs = &schema["$defs"];
    let wire: std::collections::BTreeSet<&str> = defs["ActionEffectV1"]["enum"]
        .as_array()
        .ok_or("ActionEffectV1 has no enum")?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    let projected: std::collections::BTreeSet<&str> = Effect::ALL
        .iter()
        .map(|effect| effect.wire_class())
        .collect();
    assert_eq!(
        projected, wire,
        "the effect projection and the wire vocabulary differ"
    );
    for action in Catalogue::all() {
        let name = format!("Request_{}", action.id.replace('.', "_"));
        let declared = defs[&name]["properties"]["action_version"]["const"].as_u64();
        assert!(
            declared.is_some(),
            "{} has no request definition",
            action.id
        );
        assert_eq!(
            action.wire_version().map(u64::from),
            declared,
            "{}: spine version {:?}",
            action.id,
            action.version
        );
    }
    Ok(())
}

/// ACT-G02 · the reverse pin: the control-v1 schema's `ActionId` enum names exactly the
/// catalogue's actions, each once. The projection test above walks the catalogue into the
/// schema, so an identity the schema admits that no Rust action serves passed it; this one
/// walks the schema back. Compared as sorted lists so a duplicate enum entry is also caught.
#[test]
fn the_schema_action_ids_are_exactly_the_catalogue() -> Outcome {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas/actions/control-v1.schema.json");
    let schema: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
    let enumerated = schema["$defs"]["ActionId"]["enum"]
        .as_array()
        .ok_or("ActionId has no enum")?;
    let mut wire = enumerated
        .iter()
        .map(|id| id.as_str().ok_or("ActionId entry is not a string"))
        .collect::<Result<Vec<&str>, _>>()?;
    let mut declared: Vec<&str> = Catalogue::all().iter().map(|action| action.id).collect();
    wire.sort_unstable();
    declared.sort_unstable();
    assert_eq!(wire, declared, "schema ActionId enum vs Catalogue::all()");
    Ok(())
}

/// The wire version is read, not assumed: every live action is version 1, so a projection that
/// answered 1 regardless would pass the schema tie above. Pinned off that value (F129).
#[test]
fn the_wire_version_is_parsed_from_the_spelling() {
    let base = Catalogue::all()[0];
    for (spelling, wire) in [
        ("2 (proposed)", Some(2)),
        ("12", Some(12)),
        ("1 (proposed)", Some(1)),
        ("(proposed)", None),
        ("v1", None),
    ] {
        let action = Action {
            version: spelling,
            ..base
        };
        assert_eq!(action.wire_version(), wire, "{spelling:?}");
    }
}
