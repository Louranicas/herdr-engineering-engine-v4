//! Receipt sealing, canonical JSON, and the chain break index under tampering.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use hee4_contracts::{
    BreakCause, ChainBreak, Decision, Reason, Receipt, ReceiptBody, Sha256Hex, Verdict,
    canonical_json,
};

fn body(i: usize, verdict: Verdict) -> ReceiptBody {
    ReceiptBody {
        id: format!("r-{i}").parse().unwrap(),
        task_id: "task-7".parse().unwrap(),
        decision: Decision { verdict },
        observed: vec![
            format!("obs-{i}a").parse().unwrap(),
            "obs-x".parse().unwrap(),
        ],
    }
}

fn chain(n: usize) -> Vec<Receipt> {
    let verdicts = [
        Verdict::Pass,
        Verdict::Fail,
        Verdict::Refused(Reason::Timeout),
    ];
    let mut out: Vec<Receipt> = Vec::new();
    for i in 0..n {
        let prev = out.last().map_or(Sha256Hex::GENESIS, Receipt::hash_self);
        out.push(Receipt::seal(prev, body(i, verdicts[i % 3])));
    }
    out
}

#[test]
fn canonical_json_sorts_keys_and_drops_whitespace() {
    let v: serde_json::Value =
        serde_json::from_str(r#"{ "b": [1, {"z": 0, "a": "x"}], "a": null }"#).unwrap();
    assert_eq!(canonical_json(&v), r#"{"a":null,"b":[1,{"a":"x","z":0}]}"#);
}

#[test]
fn seal_is_sha256_of_the_canonical_body() {
    let r = Receipt::seal(
        Sha256Hex::GENESIS,
        body(0, Verdict::Refused(Reason::Invalid)),
    );
    let expected = format!(
        r#"{{"decision":{{"verdict":{{"refused":"invalid"}}}},"hash_prev":"{}","id":"r-0","observed":["obs-0a","obs-x"],"task_id":"task-7"}}"#,
        "0".repeat(64)
    );
    assert_eq!(r.hash_self(), Sha256Hex::digest(expected.as_bytes()));
}

#[test]
fn an_untouched_chain_verifies_and_survives_json() {
    let c = chain(6);
    assert_eq!(Receipt::verify_chain(&c), Ok(()));
    let text = serde_json::to_string(&c).unwrap();
    let back: Vec<Receipt> = serde_json::from_str(&text).unwrap();
    assert_eq!(Receipt::verify_chain(&back), Ok(()));
}

/// Replace one byte with a different byte of the same class, so most edits still parse.
fn twin(b: u8) -> u8 {
    match b {
        b'0'..=b'8' | b'a'..=b'y' | b'A'..=b'Y' => b + 1,
        b'9' => b'0',
        b'z' => b'a',
        b'Z' => b'A',
        _ => b,
    }
}

#[test]
fn tampering_one_byte_breaks_the_chain_at_that_receipt() {
    let c = chain(5);
    let (mut tried, mut parsed) = (0, 0);
    for k in 0..c.len() {
        let original = serde_json::to_vec(&c[k]).unwrap();
        for pos in 0..original.len() {
            let mut bytes = original.clone();
            let t = twin(bytes[pos]);
            if t == bytes[pos] {
                continue;
            }
            bytes[pos] = t;
            tried += 1;
            let Ok(tampered) = serde_json::from_slice::<Receipt>(&bytes) else {
                continue; // the parse door refused it: also a detection
            };
            parsed += 1;
            let mut copy = c.clone();
            copy[k] = tampered;
            let err = Receipt::verify_chain(&copy).expect_err("tamper must break");
            assert_eq!(err.index, k, "byte {pos} of receipt {k}");
        }
    }
    println!("tampered={tried} parsed_and_caught={parsed}");
    assert!(
        parsed > 100,
        "the sweep must exercise the chain, not only the parser"
    );
}

#[test]
fn a_reordered_chain_breaks_at_the_first_wrong_link() {
    let mut c = chain(4);
    c.swap(1, 2);
    assert_eq!(
        Receipt::verify_chain(&c),
        Err(ChainBreak {
            index: 1,
            cause: BreakCause::Link
        })
    );
}
