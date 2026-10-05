//! The VERIFY line grammar and `Brief::check_verify`: the refused table, the accepted table,
//! the deliberately-not-caught table and the dispatcher fixtures the grammar reproduces.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Write as _;

use hee4_contracts::{Brief, BriefField, Refusal, VerifyFault, VerifyLine};

/// A full brief whose VERIFY field is exactly `verify` (continuation lines included).
fn brief_with_verify(verify: &str) -> Brief {
    let mut s = String::from("UNIT U-1  AGENT x\n");
    for f in BriefField::ALL {
        let value = if f == BriefField::Verify {
            verify
        } else {
            "text"
        };
        writeln!(s, "{}: {value}", f.label()).unwrap();
    }
    let b = Brief::parse(&s).unwrap();
    assert_eq!(
        b.get(BriefField::Verify),
        verify.trim(),
        "VERIFY round-trips"
    );
    b
}

fn check(verify: &str) -> Result<String, Refusal> {
    brief_with_verify(verify).check_verify().map(str::to_owned)
}

const E2E_LIVE_LINE: &str = r#"sh: /usr/bin/curl -sS -m 60 --unix-socket "$HEE4_MODEL_SOCKET" http://model/api/generate -d '{"model":"qwen2.5:0.5b","prompt":"Say exactly: hee4 ok","stream":false}'"#;

#[test]
fn check_verify_table() {
    use VerifyFault::{Empty, NothingRuns, OnlyNoOps};
    let refused: [(&str, VerifyFault); 21] = [
        ("", Empty),
        ("   \n ", Empty),
        ("model: hi", NothingRuns { lines: 1 }),
        ("cargo test", NothingRuns { lines: 1 }),
        ("model: a\n`cargo test`", NothingRuns { lines: 2 }),
        ("sh: true", OnlyNoOps { lines: 1 }),
        ("sh: :", OnlyNoOps { lines: 1 }),
        ("sh: exit 0", OnlyNoOps { lines: 1 }),
        ("sh: exit", OnlyNoOps { lines: 1 }),
        ("sh:", OnlyNoOps { lines: 1 }),
        ("sh: /usr/bin/true", OnlyNoOps { lines: 1 }),
        ("sh: echo ok", OnlyNoOps { lines: 1 }),
        ("sh: echo \"$HEE4_MODEL_SOCKET\"", OnlyNoOps { lines: 1 }),
        ("/usr/bin/true", OnlyNoOps { lines: 1 }),
        ("/bin/true", OnlyNoOps { lines: 1 }),
        ("/bin/echo done", OnlyNoOps { lines: 1 }),
        ("- sh: true", OnlyNoOps { lines: 1 }),
        ("`sh: true`", OnlyNoOps { lines: 1 }),
        ("sh: true\nmodel: x\n/usr/bin/true", OnlyNoOps { lines: 2 }),
        ("sh: true\ncargo test", OnlyNoOps { lines: 1 }),
        ("sh: true\n\n  \n- /bin/echo", OnlyNoOps { lines: 2 }),
    ];
    for (text, cause) in refused {
        assert_eq!(
            check(text),
            Err(Refusal::VacuousVerify { cause }),
            "{text:?}"
        );
    }
    let accepted: [&str; 8] = [
        "/usr/bin/test -d /usr",
        "sh: test -d /usr",
        "sh: cat /etc/os-release",
        "sh: grep -q x f",
        "sh: true\n/usr/bin/test -d /usr",
        "model: hi\nsh: grep -q x f",
        "sh: echo ok | wc -c",
        E2E_LIVE_LINE,
    ];
    for text in accepted {
        assert_eq!(check(text).as_deref(), Ok(text.trim()), "{text:?}");
    }
}

#[test]
fn refusal_text_leads_with_verify() {
    assert_eq!(check("").unwrap_err().to_string(), "VERIFY is empty");
    assert_eq!(
        check("cargo test").unwrap_err().to_string(),
        "VERIFY runs nothing: 1 line(s), none is an absolute path or `sh:`"
    );
    assert_eq!(
        check("sh: true\n/bin/true").unwrap_err().to_string(),
        "VERIFY is vacuous: all 2 runnable line(s) are no-ops (true, :, exit 0, echo)"
    );
}

/// Admission reads the text, not the effect: each of these runs a real command and is a Pass
/// at admission on purpose. The list is `FLOW.md`, "What `check_verify` does not catch";
/// a change there changes this test.
#[test]
fn deliberately_not_caught() {
    let not_caught: [&str; 8] = [
        "sh: true && true",
        "sh: true; :",
        "sh: true # comment",
        "/bin/sh -c true",
        "sh: printf ok",
        "sh: cat /dev/null",
        "sh: exit 1",
        "/usr/bin/test -d /usr",
    ];
    for text in not_caught {
        assert_eq!(check(text).as_deref(), Ok(text), "{text:?}");
    }
}

#[test]
fn parse_all_matches_the_dispatcher_fixtures() {
    assert_eq!(
        VerifyLine::parse_all("sh: echo \"$HEE4_MODEL_SOCKET\" | wc -c\nmodel: hi\n"),
        vec![
            VerifyLine::Shell {
                command: "echo \"$HEE4_MODEL_SOCKET\" | wc -c".into()
            },
            VerifyLine::Unsupported {
                kind: "model".into()
            },
        ]
    );
    assert_eq!(
        VerifyLine::parse_all("/usr/bin/true\nmodel: say hi\n`cargo test`\n"),
        vec![
            VerifyLine::Exec {
                program: "/usr/bin/true".into(),
                args: vec![]
            },
            VerifyLine::Unsupported {
                kind: "model".into()
            },
            VerifyLine::Unsupported {
                kind: "cargo".into()
            },
        ]
    );
    assert_eq!(
        VerifyLine::parse_all("- - sh: x"),
        vec![VerifyLine::Shell {
            command: "x".into()
        }]
    );
    assert_eq!(VerifyLine::parse_all("   "), vec![]);
    assert_eq!(
        VerifyLine::parse_all("/usr/bin/test -d /usr"),
        vec![VerifyLine::Exec {
            program: "/usr/bin/test".into(),
            args: vec!["-d".into(), "/usr".into()]
        }]
    );
}

#[test]
fn runs_and_no_op_per_variant() {
    let shell = |c: &str| VerifyLine::Shell {
        command: c.to_owned(),
    };
    let unsupported = VerifyLine::Unsupported {
        kind: "model".into(),
    };
    assert!(!unsupported.runs());
    assert!(!unsupported.is_no_op());
    assert!(shell("true").runs());
    assert!(shell("true").is_no_op());
    assert!(shell("echo a > f").runs());
    assert!(!shell("echo a > f").is_no_op());
    assert!(!shell("echoes").is_no_op());
    let exec = VerifyLine::Exec {
        program: "/usr/bin/echo".into(),
        args: vec!["ok".into()],
    };
    assert!(exec.runs());
    assert!(exec.is_no_op());
}
