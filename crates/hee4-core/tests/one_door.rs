//! Rung-2 door: no SQL outside `src/store/` (FLOW.md). A type cannot stop a module from
//! holding a `rusqlite::Connection` it was handed, so this is a census of the source. The walk
//! is recursive over `src/**`; the match is a token census (no regex), two families:
//!
//! - `IDENTS`: the crate name `rusqlite` as a whole identifier, anywhere (a `use`, a path, a
//!   type, a comment). A module cannot call a `Connection` method without naming the crate
//!   somewhere, so this closes the method family (`prepare_cached`, `query`, `query_one`,
//!   `raw_execute`, ...) instead of chasing names.
//! - `CALLS`: an identifier followed by optional spaces and `(`: the seven SQL verbs (so a
//!   planted `conn.execute(` is named by its verb) and `operate`, the one crate-private door
//!   that hands a `Transaction` to a closure: a closure needs no `rusqlite` token.

use std::error::Error;
use std::path::{Path, PathBuf};

const CALLS: [&str; 8] = [
    "execute",
    "execute_batch",
    "prepare",
    "query_row",
    "query_map",
    "pragma_update",
    "pragma",
    "operate",
];

const IDENTS: [&str; 1] = ["rusqlite"];

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
    for ident in IDENTS {
        let mut from = 0;
        while let Some(at) = text[from..].find(ident) {
            let start = from + at;
            let end = start + ident.len();
            from = end;
            if start > 0 && is_ident(bytes[start - 1]) {
                continue;
            }
            if bytes.get(end).is_some_and(|b| is_ident(*b)) {
                continue;
            }
            found.push(ident);
            break;
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
    assert_eq!(hits("store.operate(&key, bytes, |tx| f(tx))"), ["operate"]);
    assert_eq!(
        hits("store.apply(&task, Event::Accept)"),
        Vec::<&str>::new()
    );
    assert_eq!(hits("fn my_execute(x: u8)"), Vec::<&str>::new());
}

/// The identifier census: `rusqlite` is named wherever it appears as a whole token, so the
/// `Connection` methods `CALLS` does not list are caught by the crate name they need.
#[test]
fn the_census_catches_the_crate_name_and_the_methods_calls_does_not_list() {
    assert_eq!(
        hits(
            r#"let mut st = conn.prepare_cached("UPDATE tasks SET phase = 'x'")?; st.query([])?;"#
        ),
        Vec::<&str>::new(),
        "a method outside CALLS alone is not a verb hit (that is the gap the crate name closes)"
    );
    assert_eq!(
        hits("fn f(conn: &rusqlite::Connection) { conn.prepare_cached(\"x\")?.query([]) }"),
        ["rusqlite"]
    );
    assert_eq!(hits("use rusqlite;"), ["rusqlite"]);
    assert_eq!(hits("// a rusqlite comment"), ["rusqlite"]);
    assert_eq!(hits("rusqlite"), ["rusqlite"]);
    assert_eq!(hits("fn my_rusqlite() {}"), Vec::<&str>::new());
    assert_eq!(hits("let rusqlite2 = 1;"), Vec::<&str>::new());
    assert_eq!(hits("conn.query_one(\"x\", [], f)"), Vec::<&str>::new());
}

/// `src/probe.rs` (the only IO recovery consults) is walked by the census and holds no call
/// token and no crate name; the same matcher still names a planted `conn.query_row(`.
#[test]
fn probe_rs_is_walked_and_holds_no_sql_token() -> Result<(), Box<dyn Error>> {
    let src = Path::new(&manifest_dir()?).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files)?;
    let probe = src.join("probe.rs");
    assert!(files.contains(&probe), "probe.rs is walked: {files:?}");
    assert_eq!(
        hits(&std::fs::read_to_string(&probe)?),
        Vec::<&str>::new(),
        "probe.rs spells no census token as a call"
    );
    assert_eq!(
        hits("fn planted(conn: &Door) { conn.query_row(\"SELECT 1\", [], |r| r.get(0)) }"),
        ["query_row"],
        "the census still catches a planted query_row in a non-store file"
    );
    Ok(())
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
/// `recovery.rs` and one `conn.prepare_cached(..)` + `st.query(..)` (no `CALLS` verb) in
/// `backup.rs` names both as offenders, while `store/mod.rs` (which holds the real calls)
/// stays exempt.
#[test]
fn the_walk_names_planted_offenders_and_exempts_the_store_dir() -> Result<(), Box<dyn Error>> {
    let src = Path::new(&manifest_dir()?).join("src");
    let planted = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("one-door-planted-src");
    if planted.exists() {
        std::fs::remove_dir_all(&planted)?;
    }
    copy_tree(&src, &planted)?;
    let plants = [
        (
            "recovery.rs",
            "\nfn planted(conn: &rusqlite::Connection) { conn.execute(\"x\", []); }\n",
        ),
        (
            "backup.rs",
            "\nfn planted(conn: &rusqlite::Connection) -> rusqlite::Result<()> {\n    \
             let mut st = conn.prepare_cached(\"UPDATE tasks SET phase = 'accepted'\")?;\n    \
             let _rows = st.query([])?;\n    Ok(())\n}\n",
        ),
    ];
    for (file, plant) in plants {
        let path = planted.join(file);
        let mut text = std::fs::read_to_string(&path)?;
        text.push_str(plant);
        std::fs::write(&path, text)?;
    }
    let c = census(&planted)?;
    let mut names: Vec<&str> = c.offenders.iter().map(|(p, _)| p.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["backup.rs", "recovery.rs"], "{:?}", c.offenders);
    let calls = |file: &str| -> Vec<&'static str> {
        c.offenders
            .iter()
            .find(|(p, _)| p == file)
            .map(|(_, calls)| calls.clone())
            .unwrap_or_default()
    };
    assert_eq!(
        calls("backup.rs"),
        ["rusqlite"],
        "the prepare_cached/query plant is named by the crate name alone"
    );
    assert_eq!(calls("recovery.rs"), ["execute", "rusqlite"]);
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
