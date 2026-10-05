//! Rung-2 door: no SQL calls outside `src/store/` (FLOW.md). A type cannot stop a module
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

/// What one walk of `src` found: files seen, offenders outside `src/store/` (relative path and
/// the calls), and the files under `src/store/` that hold a call.
struct Census {
    files: usize,
    offenders: Vec<(String, Vec<&'static str>)>,
    door_files: Vec<String>,
}

fn census(src: &Path) -> Result<Census, Box<dyn Error>> {
    let mut files = Vec::new();
    rust_files(src, &mut files)?;
    let door = src.join("store");
    let mut out = Census {
        files: files.len(),
        offenders: Vec::new(),
        door_files: Vec::new(),
    };
    for path in &files {
        let found = hits(&std::fs::read_to_string(path)?);
        let rel = path.strip_prefix(src)?.display().to_string();
        if path.starts_with(&door) {
            if !found.is_empty() {
                out.door_files.push(rel);
            }
        } else if !found.is_empty() {
            out.offenders.push((rel, found));
        }
    }
    Ok(out)
}

#[test]
fn no_sql_outside_store_dir() -> Result<(), Box<dyn Error>> {
    let src = Path::new(&manifest_dir()?).join("src");
    let c = census(&src)?;
    assert!(c.files >= 4, "the census looked at {} files", c.files);
    assert!(
        !c.door_files.is_empty(),
        "at least one file under src/store/ holds a call: the door exists"
    );
    assert!(
        c.offenders.is_empty(),
        "SQL outside src/store/: {:?}",
        c.offenders
    );
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

fn copy_tree(from: &Path, to: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// The walk, not only the matcher: a copy of `src` with one `conn.execute(` planted in
/// `recovery.rs` and one in `backup.rs` names both as offenders, while `store/mod.rs` (which
/// holds the real calls) stays exempt.
#[test]
fn the_walk_names_planted_offenders_and_exempts_the_store_dir() -> Result<(), Box<dyn Error>> {
    let src = Path::new(&manifest_dir()?).join("src");
    let planted = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("one-door-planted-src");
    if planted.exists() {
        std::fs::remove_dir_all(&planted)?;
    }
    copy_tree(&src, &planted)?;
    for file in ["recovery.rs", "backup.rs"] {
        let path = planted.join(file);
        let mut text = std::fs::read_to_string(&path)?;
        text.push_str("\nfn planted(conn: &rusqlite::Connection) { conn.execute(\"x\", []); }\n");
        std::fs::write(&path, text)?;
    }
    let c = census(&planted)?;
    let mut names: Vec<&str> = c.offenders.iter().map(|(p, _)| p.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["backup.rs", "recovery.rs"], "{:?}", c.offenders);
    assert!(
        c.door_files.contains(&"store/mod.rs".to_owned()),
        "{:?}",
        c.door_files
    );
    assert!(
        !c.offenders.iter().any(|(p, _)| p.starts_with("store/")),
        "{:?}",
        c.offenders
    );
    Ok(())
}
