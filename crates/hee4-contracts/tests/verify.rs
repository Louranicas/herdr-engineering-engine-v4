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
        "VERIFY is vacuous: all 2 runnable line(s) cannot fail (true, :, exit 0, echo, || true, ; exit 0)"
    );
}

/// Every line here runs, and its exit status cannot be non-zero: a listed no-op after
/// normalisation, a compound of listed no-ops, or a forced exit. `FLOW.md`, "What
/// `check_verify` refuses beyond the exact table"; a change there changes this test.
#[test]
fn cannot_fail_lines_are_refused() {
    let cannot_fail = [
        // Normalised no-ops (the six lines hee4-app pinned as admitted, 2026-10-05).
        "/usr/bin/env true",
        "sh: \"true\"",
        "sh: exit 0;",
        "sh: true;",
        "/usr/bin/../bin/true",
        "//usr/bin/true",
        // Forced exits.
        "sh: /usr/bin/false; exit 0",
        "sh: cargo test || true",
        "sh: cargo test || :",
        // Compounds of no-ops (FLOW.md listed these as not caught before this slice).
        "sh: true && true",
        "sh: true; :",
        "sh: true # comment",
        "/bin/sh -c true",
        // The same rules, one step further.
        "sh: 'true'",
        "sh: /usr/./bin//true",
        "sh: env true",
        "sh: sh -c 'cargo test || true'",
        "/usr/bin/env sh -c true",
        "sh: cargo test || exit 0",
        "sh: cargo test && cargo clippy || true",
        "sh: cargo test; true",
        "sh: cargo test; echo done",
        "sh: exit 0; cargo test",
        "sh: cargo test &",
        "sh: cargo test | true",
        // No pipefail: the dispatcher runs `/bin/sh -c <command>` (hee4-app dispatcher.rs).
        "sh: cargo test | tail -1",
        "sh: cargo test 2>&1 | head -n 5",
        // `|&` is a pipe (bash pipes stderr too), not `|` then `&`.
        "sh: cargo test |& tail -1",
        // A no-op whose only redirections cannot fail is still a no-op.
        "sh: cargo test || true 2>/dev/null",
        "sh: cargo test || : >/dev/null 2>&1",
        "sh: cargo test || echo failed >&2",
        "sh: cargo test || true &>/dev/null",
        "sh: true </dev/null",
        // `true` and `:` write nothing: a close or a dup onto fd 0 cannot make them fail.
        "sh: cargo test || true >&-",
        "sh: cargo test || : >&0",
        "sh: cargo test || echo failed >/dev/null",
        // Redirections apply in order, each to its own fd: `echo` stays a no-op while fd 1
        // ends open for writing, and `echo -n` with nothing to print writes nothing.
        "sh: cargo test || echo failed 2>&-",
        "sh: cargo test || echo failed <&-",
        "sh: cargo test || echo failed 0<&0",
        "sh: cargo test || echo failed >&- >/dev/null",
        "sh: cargo test || echo failed >&2 2>&-",
        "sh: cargo test || echo -n >&-",
        "sh: cargo test || echo -n \"\" >&0",
    ];
    for text in cannot_fail {
        assert_eq!(
            check(text),
            Err(Refusal::VacuousVerify {
                cause: VerifyFault::OnlyNoOps { lines: 1 }
            }),
            "{text:?}"
        );
    }
}

/// Admission reads the text, not the effect: each of these runs a real command whose status
/// can be non-zero, and is a Pass at admission on purpose. The list is `FLOW.md`, "What
/// `check_verify` does not catch"; a change there changes this test.
#[test]
fn deliberately_not_caught() {
    let not_caught = [
        "sh: printf ok",
        "sh: cat /dev/null",
        "sh: exit 1",
        "/usr/bin/test -d /usr",
        "sh: cargo test --workspace",
        "/usr/bin/env cargo test",
        // A command that can fail, then a no-op only on its success: the line can still fail.
        "sh: test -f out && true",
        // A masked command mid-line: the line's status is the last command's.
        "sh: cargo test || true; cargo clippy",
        // A bare `exit` keeps the status before it.
        "sh: cargo test; exit",
        // A built-in that can end the shell: `set -e` makes `false` fatal.
        "sh: set -e; false; true",
        "sh: cargo test || exit 1; true",
        // Text the reader does not follow is a line that may fail.
        "sh: (cargo test) || true",
        "sh: if cargo test; then :; fi",
        "sh: test -n \"$(cat f)\" || true",
        // A `#` inside a word is not a comment; a quoted operator is not an operator.
        "sh: echo a#b > f",
        "sh: grep -q 'a || true' f",
        // A redirection that can fail makes the no-op one that can fail: a file that may not
        // open, an fd that may be closed.
        "sh: cargo test || true > out",
        "sh: cargo test || true <&3",
        "sh: cargo test || true 2>",
        // `echo` writes to fd 1: a close or a dup onto fd 0 (read-only for a candidate) makes
        // the write fail (`echo_redirections_match_the_shell`).
        "sh: cargo test || echo failed >&-",
        "sh: cargo test || echo f >&0",
        // In order: fd 1 ends closed, or a dup copies an fd closed before it.
        "sh: cargo test || echo failed >/dev/null >&-",
        "sh: cargo test || echo failed 2>&- >&2",
        "sh: cargo test || true >&- 2>&1",
        // fd 1 opened read-only; quoted digits are an argument, not an fd.
        "sh: cargo test || echo failed 1</dev/null",
        "sh: cargo test || echo \"2\">&-",
    ];
    for text in not_caught {
        assert_eq!(check(text).as_deref(), Ok(text), "{text:?}");
    }
}

/// Admission agrees with the shell on the redirections that differ between `echo` and `true`,
/// and on their order:
/// each line's command runs through `/bin/sh -c` the way the host spawns a candidate (stdin
/// from `/dev/null`, stdout and stderr piped, `hee4-host/src/spawn.rs`), with `false` standing
/// in for `cargo test`. A line is admitted exactly when the measured status is non-zero.
#[test]
fn echo_redirections_match_the_shell() {
    use std::process::{Command, Stdio};
    let lines = [
        ("sh: cargo test || echo failed >&-", true),
        ("sh: cargo test || echo f >&0", true),
        ("sh: cargo test || true >&-", false),
        ("sh: cargo test || : >&0", false),
        ("sh: cargo test || echo failed >&2", false),
        ("sh: cargo test || echo failed >/dev/null", false),
        ("sh: cargo test || echo failed 2>&-", false),
        ("sh: cargo test || echo failed <&-", false),
        ("sh: cargo test || echo failed 0<&0", false),
        ("sh: cargo test || echo failed >&- >/dev/null", false),
        ("sh: cargo test || echo failed >&2 2>&-", false),
        ("sh: cargo test || echo -n >&-", false),
        ("sh: cargo test || echo -n \"\" >&0", false),
        ("sh: cargo test || echo failed >/dev/null >&-", true),
        ("sh: cargo test || echo failed 2>&- >&2", true),
        ("sh: cargo test || true >&- 2>&1", true),
        ("sh: cargo test || echo failed 1</dev/null", true),
        ("sh: cargo test || echo \"2\">&-", true),
        ("sh: cargo test || echo -n x >&-", true),
    ];
    for (line, can_fail) in lines {
        let script = line
            .strip_prefix("sh: cargo test")
            .map(|rest| format!("false{rest}"))
            .unwrap();
        let out = Command::new("/bin/sh")
            .args(["-c", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(
            !out.status.success(),
            can_fail,
            "{script:?}: {:?}",
            out.status
        );
        assert_eq!(check(line).is_ok(), can_fail, "{line:?}");
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
