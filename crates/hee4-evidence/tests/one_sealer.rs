//! Census: the seal door. `Receipt::seal(` and `ReceiptBody {` may appear in the `src/` of exactly
//! two files, `hee4-evidence/src/decide.rs` (the one sealer, `decide_and_seal`) and
//! `hee4-contracts/src/receipt.rs` (the definition). A type cannot close this door today
//! because `ReceiptBody`'s fields are public in `hee4-contracts` (see the DC in `FLOW.md`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED: [&str; 2] = [
    "hee4-evidence/src/decide.rs",
    "hee4-contracts/src/receipt.rs",
];
const NEEDLES: [&str; 2] = ["Receipt::seal(", "ReceiptBody {"];

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn only_decide_and_the_receipt_definition_name_the_sealer() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut scanned = 0_usize;
    let mut offenders = Vec::new();
    for c in fs::read_dir(&crates).unwrap() {
        let src = c.unwrap().path().join("src");
        if !src.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        rs_files(&src, &mut files);
        for f in files {
            scanned += 1;
            let rel = f
                .strip_prefix(&crates)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if ALLOWED.contains(&rel.as_str()) {
                continue;
            }
            let text = fs::read_to_string(&f).unwrap();
            for n in NEEDLES {
                if text.contains(n) {
                    offenders.push(format!("{rel} contains `{n}`"));
                }
            }
        }
    }
    println!("MEASURED scanned {scanned} files under crates/*/src");
    assert!(scanned > 0, "the census looked at nothing");
    assert!(offenders.is_empty(), "seal door breached: {offenders:#?}");
}
