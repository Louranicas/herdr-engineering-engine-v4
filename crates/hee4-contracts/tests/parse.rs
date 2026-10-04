//! Brief, hex newtypes and observation parsing at the boundary.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Write as _;

use hee4_contracts::{
    Brief, BriefField, GitSha, HexFault, HexKind, Observation, Refusal, Sha256Hex, TaskId,
    TokenFault, TokenKind,
};

fn brief_text(skip: Option<BriefField>, restatement: &str) -> String {
    let mut s = String::from("UNIT U-1  AGENT x\n");
    for f in BriefField::ALL {
        if Some(f) == skip {
            continue;
        }
        let value = if f == BriefField::Restatement {
            restatement
        } else {
            "text"
        };
        writeln!(s, "{}: {value}\n  continued {}", f.label(), f.label().len()).unwrap();
    }
    s
}

#[test]
fn a_full_brief_parses_and_keeps_continuations() {
    let b = Brief::parse(&brief_text(None, "I will build the contracts")).unwrap();
    assert_eq!(b.get(BriefField::Goal), "text\n  continued 4");
    assert_eq!(
        b.check_restatement(),
        Ok("I will build the contracts\n  continued 11")
    );
}

#[test]
fn each_missing_field_is_refused_by_name() {
    for f in BriefField::ALL {
        assert_eq!(
            Brief::parse(&brief_text(Some(f), "r")),
            Err(Refusal::MissingBriefField { field: f }),
            "{}",
            f.label()
        );
    }
}

#[test]
fn an_empty_restatement_parses_but_fails_the_check() {
    let text = brief_text(None, "").replace("RESTATEMENT: \n  continued 11\n", "RESTATEMENT:\n");
    let b = Brief::parse(&text).unwrap();
    assert_eq!(b.check_restatement(), Err(Refusal::EmptyRestatement));
    let blank = brief_text(None, "   ").replace("  continued 11", "   ");
    assert_eq!(
        Brief::parse(&blank).unwrap().check_restatement(),
        Err(Refusal::EmptyRestatement)
    );
}

#[test]
fn a_duplicate_field_is_refused() {
    let text = format!("{}GOAL: again\n", brief_text(None, "r"));
    assert_eq!(
        Brief::parse(&text),
        Err(Refusal::DuplicateBriefField {
            field: BriefField::Goal
        })
    );
}

#[test]
fn sha256hex_parses_only_lower_hex_of_length_64() {
    let good = "ab".repeat(32);
    assert_eq!(good.parse::<Sha256Hex>().unwrap().to_string(), good);
    assert_eq!(
        Sha256Hex::digest(b"").to_string(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        "ab".repeat(31).parse::<Sha256Hex>(),
        Err(Refusal::MalformedHex {
            kind: HexKind::Sha256,
            fault: HexFault::Length { found: 62 }
        })
    );
    assert_eq!(
        format!("{}G", "a".repeat(63)).parse::<Sha256Hex>(),
        Err(Refusal::MalformedHex {
            kind: HexKind::Sha256,
            fault: HexFault::NotLowerHex { at: 63 }
        })
    );
    assert!("AB".repeat(32).parse::<Sha256Hex>().is_err());
}

#[test]
fn gitsha_takes_40_or_64() {
    assert!("a".repeat(40).parse::<GitSha>().is_ok());
    assert!("a".repeat(64).parse::<GitSha>().is_ok());
    assert_eq!(
        "a".repeat(12).parse::<GitSha>(),
        Err(Refusal::MalformedHex {
            kind: HexKind::GitSha,
            fault: HexFault::Length { found: 12 }
        })
    );
}

#[test]
fn tokens_refuse_empty_and_whitespace() {
    assert_eq!(
        "".parse::<TaskId>(),
        Err(Refusal::MalformedToken {
            kind: TokenKind::TaskId,
            fault: TokenFault::Empty
        })
    );
    assert_eq!(
        "a b".parse::<TaskId>(),
        Err(Refusal::MalformedToken {
            kind: TokenKind::TaskId,
            fault: TokenFault::Forbidden { at: 1 }
        })
    );
}

fn observation_json(input: &str) -> String {
    format!(
        r#"{{"source":"cargo-test","input_sha256":"{input}","tool":{{"name":"cargo","version":"1.99.0"}},
        "head_sha":"{}","outcome":"pass","evidence":[{{"label":"log","sha256":"{}"}}],
        "advisory":false,"elapsed_ms":120,"budget_ms":60000}}"#,
        "c".repeat(40),
        "d".repeat(64)
    )
}

#[test]
fn an_observation_parses_and_a_bad_digest_is_refused_at_the_boundary() {
    let o: Observation = serde_json::from_str(&observation_json(&"e".repeat(64))).unwrap();
    assert_eq!(o.tool.version.as_str(), "1.99.0");
    let back = serde_json::to_string(&o).unwrap();
    assert_eq!(serde_json::from_str::<Observation>(&back).unwrap(), o);
    let err = serde_json::from_str::<Observation>(&observation_json("not-a-digest")).unwrap_err();
    assert!(err.to_string().contains("Sha256"), "{err}");
}
