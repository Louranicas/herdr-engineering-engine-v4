//! Rung-2 door: no SQL writes outside `src/store.rs` (FLOW.md). A type cannot stop a module
//! from holding a `rusqlite::Connection` it was handed, so this is a census of the source.

use std::error::Error;
use std::path::Path;

const WRITE_CALLS: [&str; 3] = ["execute(", "execute_batch(", "prepare("];

fn hits(text: &str) -> Vec<&'static str> {
    WRITE_CALLS
        .iter()
        .copied()
        .filter(|c| text.contains(c))
        .collect()
}

#[test]
fn no_sql_writes_outside_store_rs() -> Result<(), Box<dyn Error>> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut scanned = 0;
    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(&src)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        scanned += 1;
        if path.file_name().is_some_and(|n| n == "store.rs") {
            assert!(
                !hits(&std::fs::read_to_string(&path)?).is_empty(),
                "store.rs is the door"
            );
            continue;
        }
        let found = hits(&std::fs::read_to_string(&path)?);
        if !found.is_empty() {
            offenders.push(format!("{}: {found:?}", path.display()));
        }
    }
    assert!(scanned >= 4, "the census looked at {scanned} files");
    assert!(offenders.is_empty(), "SQL outside store.rs: {offenders:?}");
    Ok(())
}

#[test]
fn the_census_catches_a_planted_write() {
    assert_eq!(
        hits(r#"conn.execute("UPDATE tasks SET phase = 'accepted'", [])"#),
        ["execute("]
    );
    assert_eq!(
        hits("store.apply(&task, Event::Accept)"),
        Vec::<&str>::new()
    );
}
