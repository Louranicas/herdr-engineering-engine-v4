//! Rung-2 door: no SQL calls outside `src/store.rs` (FLOW.md). A type cannot stop a module
//! from holding a `rusqlite::Connection` it was handed, so this is a census of the source.
//! The walk is recursive over `src/**`; the match is a token census (no regex): an identifier
//! from `CALLS`, not preceded by an identifier character, followed by optional spaces and `(`.

use std::error::Error;
use std::path::{Path, PathBuf};

const CALLS: [&str; 7] = [
    "execute",
    "execute_batch",
    "prepare",
    "query_row",
    "query_map",
    "pragma_update",
    "pragma",
];

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn hits(text: &str) -> Vec<&'static str> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    for call in CALLS {
        let mut from = 0;
        while let Some(at) = text[from..].find(call) {
            let start = from + at;
            let end = start + call.len();
            from = end;
            if start > 0 && is_ident(bytes[start - 1]) {
                continue;
            }
            let rest = &bytes[end..];
            let open = rest.iter().position(|b| *b != b' ');
            if open.is_some_and(|i| rest[i] == b'(') {
                found.push(call);
                break;
            }
        }
    }
    found
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The crate directory, read when the test runs: `env!` would bake the compile-time path into
/// a binary the gate reuses across `git archive` exports.
fn manifest_dir() -> Result<String, Box<dyn Error>> {
    std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| format!("CARGO_MANIFEST_DIR is not set (run under cargo test): {e}").into())
}

#[test]
fn no_sql_outside_store_rs() -> Result<(), Box<dyn Error>> {
    let src = Path::new(&manifest_dir()?).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files)?;
    let mut offenders = Vec::new();
    for path in &files {
        let found = hits(&std::fs::read_to_string(path)?);
        if path == &src.join("store.rs") {
            assert!(!found.is_empty(), "store.rs is the door");
        } else if !found.is_empty() {
            offenders.push(format!("{}: {found:?}", path.display()));
        }
    }
    assert!(
        files.len() >= 4,
        "the census looked at {} files",
        files.len()
    );
    assert!(offenders.is_empty(), "SQL outside store.rs: {offenders:?}");
    Ok(())
}

#[test]
fn the_census_catches_planted_calls() {
    assert_eq!(
        hits(r#"conn.execute("UPDATE tasks SET phase = 'accepted'", [])"#),
        ["execute"]
    );
    assert_eq!(hits("conn.query_row (\"x\", [], f)"), ["query_row"]);
    assert_eq!(hits("c.pragma_update(None, \"a\", 1)"), ["pragma_update"]);
    assert_eq!(hits("c.pragma  (x)"), ["pragma"]);
    assert_eq!(
        hits("store.apply(&task, Event::Accept)"),
        Vec::<&str>::new()
    );
    assert_eq!(hits("fn my_execute(x: u8)"), Vec::<&str>::new());
}
