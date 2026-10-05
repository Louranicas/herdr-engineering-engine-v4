//! Receipt sealing, canonical JSON, and the chain break index under tampering.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use hee4_contracts::{
    BreakCause, ChainBreak, Decision, Reason, Receipt, ReceiptBody, Sha256Hex, Verdict,
    canonical_json, merkle_root,
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

/// The RFC 6962 reference leaves (raw bytes) and the Merkle Tree Hash of each prefix, pinned
/// against a python3 hashlib re-derivation (leaf `0x00`, node `0x01`, `k` the largest power of
/// two below `n`) on 2026-10-05.
#[test]
fn merkle_root_matches_rfc6962_vectors() {
    let leaves: [&[u8]; 8] = [
        b"",
        &[0x00],
        &[0x10],
        &[0x20, 0x21],
        &[0x30, 0x31],
        &[0x40, 0x41, 0x42, 0x43],
        &[0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57],
        &[
            0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d,
            0x6e, 0x6f,
        ],
    ];
    let pinned = [
        (
            0,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            1,
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d",
        ),
        (
            2,
            "fac54203e7cc696cf0dfcb42c92a1d9dbaf70ad9e621f4bd8d98662f00e3c125",
        ),
        (
            3,
            "aeb6bcfe274b70a14fb067a5e5578264db0fa9b51af5e0ba159158f329e06e77",
        ),
        (
            8,
            "5dc9da79a70659a9ad559cb701ded9a2ab9d823aad2f4960cfe370eff4604328",
        ),
    ];
    for (n, hex) in pinned {
        assert_eq!(merkle_root(&leaves[..n]).to_string(), hex, "MTH({n})");
    }
    assert_eq!(merkle_root(&[]), Sha256Hex::digest(b""));
}

#[test]
fn checkpoint_is_order_sensitive_and_equals_merkle_root_over_digests() {
    let c = chain(5);
    let hashes: Vec<Sha256Hex> = c.iter().map(Receipt::hash_self).collect();
    let root = Receipt::checkpoint(&hashes);
    let leaves: Vec<&[u8]> = hashes.iter().map(|h| h.as_bytes().as_slice()).collect();
    assert_eq!(root, merkle_root(&leaves));
    assert_eq!(Receipt::checkpoint(&hashes), root, "deterministic");
    let mut swapped = hashes.clone();
    swapped.swap(1, 3);
    assert_ne!(Receipt::checkpoint(&swapped), root, "order matters");
    assert_ne!(
        Receipt::checkpoint(&hashes[..4]),
        root,
        "a dropped tail changes the root"
    );
    assert_eq!(Receipt::checkpoint(&[]), Sha256Hex::digest(b""));
}
