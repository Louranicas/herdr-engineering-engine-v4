//! The lattice, by table: identities, binding, tier, outcome, budget, evidence.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use hee4_contracts::{
    Evidence, Observation, Outcome, Reason, RefusalText, Sha256Hex, ToolId, Verdict,
};
use hee4_evidence::{Identities, Identity, Source, Subject, Why, decide};

const HEAD: &str = "1111111111111111111111111111111111111111";
const OTHER_HEAD: &str = "2222222222222222222222222222222222222222";

fn subject() -> Subject {
    Subject {
        task_id: "T-1".parse().unwrap(),
        head_sha: HEAD.parse().unwrap(),
        input_sha256: Sha256Hex::digest(b"diff"),
    }
}

fn wired(name: &str) -> Identity {
    Identity::Wired(Source {
        name: name.parse().unwrap(),
        digest: Sha256Hex::digest(name.as_bytes()),
    })
}

fn all_wired() -> Identities {
    Identities {
        collector: wired("collector"),
        locks: wired("locks"),
        standards: wired("standards"),
    }
}

#[derive(Clone, Copy, Debug)]
enum Tier {
    T0,
    Adv,
}
#[derive(Clone, Copy, Debug)]
enum Bind {
    Bound,
    HeadOff,
    InputOff,
}

fn obs(tier: Tier, outcome: Outcome, bind: Bind) -> Observation {
    Observation {
        source: "src".parse().unwrap(),
        input_sha256: Sha256Hex::digest(match bind {
            Bind::InputOff => b"other".as_slice(),
            _ => b"diff".as_slice(),
        }),
        tool: ToolId {
            name: "tool".parse().unwrap(),
            version: "1".parse().unwrap(),
        },
        head_sha: match bind {
            Bind::HeadOff => OTHER_HEAD,
            _ => HEAD,
        }
        .parse()
        .unwrap(),
        outcome,
        evidence: vec![Evidence {
            label: "e".parse().unwrap(),
            sha256: Sha256Hex::digest(b"e"),
        }],
        advisory: matches!(tier, Tier::Adv),
        elapsed_ms: 1,
        budget_ms: 10,
    }
}

fn v(ids: &Identities, o: &[Observation]) -> Verdict {
    decide(ids, o, &subject()).verdict
}

const R: fn(Reason) -> Verdict = Verdict::Refused;

#[test]
fn absent_identity_is_refused_never_pass() {
    let pass = [obs(Tier::T0, Outcome::Pass, Bind::Bound)];
    for which in 0..3 {
        let mut ids = all_wired();
        let slot = [&mut ids.collector, &mut ids.locks, &mut ids.standards];
        *slot.into_iter().nth(which).unwrap() = Identity::Unavailable(Why::NotWired);
        assert_eq!(v(&ids, &pass), R(Reason::Error), "identity {which}");
    }
    let mut ids = all_wired();
    ids.standards = Identity::Unavailable(Why::Unreadable {
        what: "STANDARDS.md".into(),
    });
    assert_eq!(v(&ids, &pass), R(Reason::Error));
    assert_eq!(v(&all_wired(), &pass), Verdict::Pass);
}

#[test]
fn looking_at_nothing_is_refused() {
    assert_eq!(v(&all_wired(), &[]), R(Reason::Invalid));
    assert_eq!(
        v(&all_wired(), &[obs(Tier::Adv, Outcome::Pass, Bind::Bound)]),
        R(Reason::Invalid)
    );
    let mut empty = obs(Tier::T0, Outcome::Pass, Bind::Bound);
    empty.evidence.clear();
    assert_eq!(v(&all_wired(), &[empty]), R(Reason::Invalid));
}

#[test]
fn over_budget_tier0_is_timeout() {
    let mut slow = obs(Tier::T0, Outcome::Pass, Bind::Bound);
    slow.elapsed_ms = 11;
    assert_eq!(v(&all_wired(), &[slow]), R(Reason::Timeout));
}

/// Every single observation in the 2 x 4 x 3 domain, alone and next to a bound tier-0 pass.
#[test]
fn single_and_paired_table() {
    use Bind::{Bound, HeadOff, InputOff};
    use Outcome::{Error, Fail, Pass};
    use Tier::{Adv, T0};
    let refused = || Outcome::Refused {
        reason: "refused: 0 files".parse::<RefusalText>().unwrap(),
    };
    let u = R(Reason::Unreconciled);
    #[rustfmt::skip]
    let table: [(Tier, Outcome, Bind, Verdict, Verdict); 24] = [
        // tier, outcome, binding,   alone,              with a bound tier-0 Pass
        (T0,  Pass,  Bound,    Verdict::Pass,      Verdict::Pass),
        (T0,  Fail,  Bound,    Verdict::Fail,      Verdict::Fail),
        (T0,  Error, Bound,    R(Reason::Error),   R(Reason::Error)),
        (T0,  refused(), Bound, R(Reason::Invalid), R(Reason::Invalid)),
        (T0,  refused(), HeadOff, u, u),
        (T0,  refused(), InputOff, u, u),
        (T0,  Pass,  HeadOff,  u, u),
        (T0,  Fail,  HeadOff,  u, u),
        (T0,  Error, HeadOff,  u, u),
        (T0,  Pass,  InputOff, u, u),
        (T0,  Fail,  InputOff, u, u),
        (T0,  Error, InputOff, u, u),
        (Adv, Pass,  Bound,    R(Reason::Invalid), Verdict::Pass),
        (Adv, Fail,  Bound,    R(Reason::Invalid), Verdict::Pass),
        (Adv, Error, Bound,    R(Reason::Invalid), Verdict::Pass),
        (Adv, refused(), Bound, R(Reason::Invalid), Verdict::Pass),
        (Adv, refused(), HeadOff, u, u),
        (Adv, refused(), InputOff, u, u),
        (Adv, Pass,  HeadOff,  u, u),
        (Adv, Fail,  HeadOff,  u, u),
        (Adv, Error, HeadOff,  u, u),
        (Adv, Pass,  InputOff, u, u),
        (Adv, Fail,  InputOff, u, u),
        (Adv, Error, InputOff, u, u),
    ];
    let base = obs(T0, Pass, Bound);
    for (tier, outcome, bind, alone, paired) in table {
        let o = obs(tier, outcome.clone(), bind);
        assert_eq!(
            v(&all_wired(), std::slice::from_ref(&o)),
            alone,
            "{tier:?} {outcome:?} {bind:?} alone"
        );
        assert_eq!(
            v(&all_wired(), &[base.clone(), o]),
            paired,
            "{tier:?} {outcome:?} {bind:?} paired"
        );
    }
}

fn domain() -> Vec<Observation> {
    let mut d = Vec::new();
    for tier in [Tier::T0, Tier::Adv] {
        for outcome in [
            Outcome::Pass,
            Outcome::Fail,
            Outcome::Error,
            Outcome::Refused {
                reason: "refused: 0 files".parse().unwrap(),
            },
        ] {
            for bind in [Bind::Bound, Bind::HeadOff, Bind::InputOff] {
                d.push(obs(tier, outcome.clone(), bind));
            }
        }
    }
    d
}

fn rank(x: Verdict) -> u8 {
    match x {
        Verdict::Pass => 0,
        Verdict::Fail => 1,
        Verdict::Refused(_) => 2,
    }
}

/// Over every ordered triple of the domain (24^3 = 13824 sets): order does not matter, an
/// advisory never moves a verdict except to `Refused(Unreconciled)`, a Pass needs a bound
/// tier-0 Pass, and adding a tier-0 observation never lowers a verdict that already had one.
#[test]
fn exhaustive_laws_over_triples() {
    let d = domain();
    let ids = all_wired();
    let mut cases = 0u32;
    for a in &d {
        for b in &d {
            for c in &d {
                let set = [a.clone(), b.clone(), c.clone()];
                let got = v(&ids, &set);
                let rev = [c.clone(), b.clone(), a.clone()];
                assert_eq!(got, v(&ids, &rev));
                let t0: Vec<_> = set.iter().filter(|o| !o.advisory).cloned().collect();
                if c.advisory {
                    let without = v(&ids, &set[..2]);
                    assert!(
                        got == without || got == R(Reason::Unreconciled),
                        "{got:?} vs {without:?}"
                    );
                } else if set[..2].iter().any(|o| !o.advisory) {
                    assert!(rank(got) >= rank(v(&ids, &set[..2])));
                }
                if got == Verdict::Pass {
                    assert_ne!(t0.len(), 0, "Pass with no tier-0");
                    assert!(t0.iter().all(|o| o.outcome == Outcome::Pass));
                    assert_eq!(v(&ids, &t0), Verdict::Pass, "advisories raised to Pass");
                }
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 13_824);
}
