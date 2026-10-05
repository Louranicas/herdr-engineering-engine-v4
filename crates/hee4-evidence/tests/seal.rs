//! Decision and observations sealed together, canonically.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use hee4_contracts::{Evidence, Observation, Outcome, Receipt, Sha256Hex, ToolId, Verdict};
use hee4_evidence::{Identities, Identity, Source, Subject, decide_and_seal, observation_id};

const HEAD: &str = "1111111111111111111111111111111111111111";

fn subject() -> Subject {
    Subject {
        task_id: "T-1".parse().unwrap(),
        head_sha: HEAD.parse().unwrap(),
        input_sha256: Sha256Hex::digest(b"diff"),
    }
}

fn ids() -> Identities {
    let w = |n: &str| {
        Identity::Wired(Source {
            name: n.parse().unwrap(),
            digest: Sha256Hex::digest(n.as_bytes()),
        })
    };
    Identities {
        collector: w("collector"),
        locks: w("locks"),
        standards: w("standards"),
    }
}

fn obs(source: &str, outcome: Outcome) -> Observation {
    Observation {
        source: source.parse().unwrap(),
        input_sha256: Sha256Hex::digest(b"diff"),
        tool: ToolId {
            name: "tool".parse().unwrap(),
            version: "1".parse().unwrap(),
        },
        head_sha: HEAD.parse().unwrap(),
        outcome,
        evidence: vec![Evidence {
            label: "e".parse().unwrap(),
            sha256: Sha256Hex::digest(b"e"),
        }],
        advisory: false,
        elapsed_ms: 1,
        budget_ms: 10,
    }
}

fn seal(o: &[Observation]) -> Receipt {
    decide_and_seal(
        Sha256Hex::GENESIS,
        "R-1".parse().unwrap(),
        &ids(),
        o,
        &subject(),
    )
    .unwrap()
}

#[test]
fn order_does_not_change_the_seal_and_the_set_does() {
    let (a, b, c) = (
        obs("a", Outcome::Pass),
        obs("b", Outcome::Pass),
        obs("c", Outcome::Pass),
    );
    let abc = seal(&[a.clone(), b.clone(), c.clone()]);
    let cab = seal(&[c.clone(), a.clone(), b.clone()]);
    assert_eq!(abc.hash_self(), cab.hash_self());
    assert_eq!(abc.observed(), cab.observed());
    assert_eq!(abc.decision().verdict, Verdict::Pass);
    let ab = seal(&[a.clone(), b.clone()]);
    assert_ne!(ab.hash_self(), abc.hash_self());
    let changed = seal(&[a.clone(), b.clone(), obs("c", Outcome::Fail)]);
    assert_ne!(changed.hash_self(), abc.hash_self());
    assert_eq!(changed.decision().verdict, Verdict::Fail);
    Receipt::verify_chain(&[abc]).unwrap();
}

#[test]
fn the_seal_names_exactly_the_observations_read() {
    let (a, b) = (obs("a", Outcome::Pass), obs("b", Outcome::Fail));
    let r = seal(&[b.clone(), a.clone()]);
    let task = r.task_id().clone();
    let mut want = vec![
        observation_id(&task, &a).unwrap(),
        observation_id(&task, &b).unwrap(),
    ];
    want.sort_by(|x, y| x.as_str().cmp(y.as_str()));
    assert_eq!(r.observed(), want.as_slice());
    assert!(r.observed()[0].as_str().starts_with("obs-"));
    assert_eq!(r.observed()[0].as_str().len(), 68);
}

#[test]
fn the_same_observation_for_two_tasks_has_two_ids() {
    let o = obs("a", Outcome::Pass);
    let t1: hee4_contracts::TaskId = "t-1".parse().unwrap();
    let t2: hee4_contracts::TaskId = "t-2".parse().unwrap();
    assert_ne!(
        observation_id(&t1, &o).unwrap(),
        observation_id(&t2, &o).unwrap(),
        "the ledger binds an observation to its task; identical content across tasks must not collide"
    );
    assert_eq!(
        observation_id(&t1, &o).unwrap(),
        observation_id(&t1, &o).unwrap()
    );
}
