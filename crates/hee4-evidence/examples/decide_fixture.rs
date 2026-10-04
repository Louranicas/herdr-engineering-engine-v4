//! One manual `decide` over the fixture diff: run deep-diff-forge, decide, seal, print.
//! `cargo run -p hee4-evidence --example decide_fixture`

use hee4_contracts::Sha256Hex;
use hee4_evidence::{Identities, Identity, Source, Subject, Why, ddf, decide, decide_and_seal};
use hee4_host::clock::SystemClock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let diff = include_bytes!("../tests/fixtures/verdict.diff");
    let subject = Subject {
        task_id: "T-fixture".parse()?,
        head_sha: "3c1db9a2ebaedd3a0c153285fc0fc3e75bab163a".parse()?,
        input_sha256: Sha256Hex::digest(diff),
    };
    let pin = |n: &str| -> Result<Identity, Box<dyn std::error::Error>> {
        Ok(Identity::Wired(Source {
            name: n.parse()?,
            digest: Sha256Hex::digest(n.as_bytes()),
        }))
    };
    let ids = Identities {
        collector: pin("collector")?,
        locks: pin("locks")?,
        standards: pin("standards")?,
    };
    let obs = ddf::observe(
        diff,
        &subject,
        &SystemClock,
        std::time::Duration::from_secs(60),
    )?;
    println!("observation: {}", serde_json::to_string(&obs)?);
    println!(
        "decide: {:?}",
        decide(&ids, std::slice::from_ref(&obs), &subject)
    );
    let r = decide_and_seal(
        Sha256Hex::GENESIS,
        "R-fixture".parse()?,
        &ids,
        std::slice::from_ref(&obs),
        &subject,
    )?;
    println!(
        "receipt: verdict={:?} observed={:?} hash_self={}",
        r.decision().verdict,
        r.observed(),
        r.hash_self()
    );
    let no_locks = Identities {
        locks: Identity::Unavailable(Why::NotWired),
        ..ids
    };
    println!(
        "decide (locks unavailable): {:?}",
        decide(&no_locks, &[obs], &subject)
    );
    Ok(())
}
