//! The catalogue is the feature directory: its id set equals `gates/features/*.md` minus the
//! three non-action files, every mutating entry names a readback, the held id is held, exactly
//! two entries require a precondition, and the revision digest is stable.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use hee4_contracts::catalogue::{CATALOGUE, Owner, PreconditionRule, Scope, find, revision};
use hee4_contracts::{Sha256Hex, canonical_json};

/// The files under `gates/features` that are not actions.
const NOT_ACTIONS: [&str; 3] = ["README", "crash-restart", "multi-surface-journeys"];

fn feature_ids() -> BTreeSet<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gates/features");
    let mut ids = BTreeSet::new();
    for entry in std::fs::read_dir(&dir).expect("gates/features readable") {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        if let Some(stem) = name.strip_suffix(".md")
            && !NOT_ACTIONS.contains(&stem)
        {
            ids.insert(stem.to_owned());
        }
    }
    ids
}

fn catalogue_ids() -> Vec<&'static str> {
    CATALOGUE.iter().map(|a| a.id).collect()
}

#[test]
fn id_set_equals_the_feature_directory_and_is_unique() {
    let ids = catalogue_ids();
    let set: BTreeSet<String> = ids.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(set.len(), ids.len(), "duplicate id in CATALOGUE: {ids:?}");
    let files = feature_ids();
    assert!(!files.is_empty(), "no feature files found");
    assert_eq!(set, files, "CATALOGUE ids != gates/features basenames");
}

#[test]
fn find_round_trips_every_id_and_misses_an_unknown_one() {
    for action in &CATALOGUE {
        let found = find(action.id).expect("every catalogued id is found");
        assert_eq!(found.id, action.id);
        assert_eq!(found.version, 1);
        assert!(!found.purpose.is_empty(), "{} has no purpose", action.id);
    }
    assert!(find("task.explode").is_none());
    assert!(find("").is_none());
}

#[test]
fn mutating_entries_name_a_readback_and_reads_name_none() {
    for action in &CATALOGUE {
        if action.effect.mutates() {
            let readback = action
                .readback_action
                .unwrap_or_else(|| panic!("{} mutates without a readback", action.id));
            assert!(
                find(readback).is_some(),
                "{}: readback {readback} not catalogued",
                action.id
            );
            assert!(
                !find(readback).unwrap().effect.mutates(),
                "{}: readback mutates",
                action.id
            );
        } else {
            assert_eq!(
                action.readback_action, None,
                "{} is a read with a readback",
                action.id
            );
        }
    }
}

#[test]
fn judge_inspect_is_held_under_owner_judge() {
    let judge = find("judge.inspect").expect("catalogued");
    assert_eq!(judge.owner, Owner::Judge);
    assert_eq!(judge.scope, Scope::Held);
    assert!(Owner::Judge.is_held());
    assert!(Owner::Deploy.is_held());
    let held: Vec<&str> = CATALOGUE
        .iter()
        .filter(|a| a.owner.is_held())
        .map(|a| a.id)
        .collect();
    assert_eq!(held, ["judge.inspect"]);
    assert!(CATALOGUE.iter().all(|a| a.owner != Owner::Deploy));
}

#[test]
fn exactly_roster_disable_and_service_action_require_a_precondition() {
    let required: BTreeSet<(&str, &str)> = CATALOGUE
        .iter()
        .filter_map(|a| match a.precondition {
            PreconditionRule::Required(resource) => Some((a.id, resource)),
            PreconditionRule::None => None,
        })
        .collect();
    let want: BTreeSet<(&str, &str)> =
        [("roster.disable", "roster"), ("service.action", "service")]
            .into_iter()
            .collect();
    assert_eq!(required, want);
}

#[test]
fn the_skeleton_ids_are_v40() {
    let served = [
        "health",
        "task.submit",
        "task.get",
        "task.list",
        "task.cancel",
        "task.preview",
        "task.resolve",
        "events.subscribe",
    ];
    for id in served {
        assert_eq!(find(id).expect("catalogued").scope, Scope::V40, "{id}");
    }
}

#[test]
fn revision_is_stable_and_is_the_digest_of_the_canonical_json() {
    let a = revision();
    let b = revision();
    assert_eq!(a, b);
    let json = hee4_contracts::catalogue::to_json();
    let recomputed = Sha256Hex::digest(canonical_json(&json).as_bytes());
    assert_eq!(a, recomputed);
    assert_eq!(a.to_string().len(), 64);
    let entries = json.as_array().expect("array");
    assert_eq!(entries.len(), CATALOGUE.len());
    for (entry, action) in entries.iter().zip(CATALOGUE.iter()) {
        assert_eq!(entry["id"], action.id);
        assert_eq!(entry["effect"], action.effect.wire_name());
        assert_eq!(entry["owner"], action.owner.wire_name());
        assert_eq!(entry["scope"], action.scope.wire_name());
    }
}

#[test]
fn because_strings_are_pairwise_distinct_and_held_names_h8() {
    let strings = [
        Scope::V40.because(),
        Scope::V41.because(),
        Scope::V42.because(),
        Scope::Held.because(),
    ];
    let distinct: BTreeSet<&str> = strings.into_iter().collect();
    assert_eq!(distinct.len(), strings.len(), "{strings:?}");
    assert!(Scope::Held.because().contains("H-8"));
    assert!(Scope::V41.because().contains("v4.1"));
    assert!(Scope::V42.because().contains("v4.2"));
    assert_eq!(Scope::V40.because(), "scope v4.0: owner not composed");
}

#[test]
fn wire_names_match_the_serialize_spelling() {
    for action in &CATALOGUE {
        let effect = serde_json::to_value(action.effect).unwrap();
        assert_eq!(effect, action.effect.wire_name());
        let owner = serde_json::to_value(action.owner).unwrap();
        assert_eq!(owner, action.owner.wire_name());
        let scope = serde_json::to_value(action.scope).unwrap();
        assert_eq!(scope, action.scope.wire_name());
    }
}
