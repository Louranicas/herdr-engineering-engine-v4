//! Rung 2 for a defect that recurred twice (brain/compile-time-paths-break-in-cached-exports.md):
//! no crate source or test may bake `CARGO_MANIFEST_DIR` at compile time. The gate reuses a
//! per-subject target dir across `git archive` exports, so a baked path names a deleted
//! directory and the test fails with `NotFound`. Read `std::env::var` at run time instead.

use std::error::Error;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.file_name().is_some_and(|n| n == "target") {
            continue;
        }
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

#[test]
fn no_crate_bakes_the_manifest_dir() -> Result<(), Box<dyn Error>> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let crates = manifest.parent().ok_or("no crates dir")?;
    let needle = concat!("env!(", "\"CARGO_MANIFEST_DIR\")");
    let mut files = Vec::new();
    rust_files(crates, &mut files)?;
    assert!(
        files.len() > 20,
        "the census looked at {} files",
        files.len()
    );
    let offenders: Vec<String> = files
        .iter()
        .filter(|p| std::fs::read_to_string(p).is_ok_and(|s| s.contains(needle)))
        .map(|p| p.display().to_string())
        .collect();
    assert!(
        offenders.is_empty(),
        "compile-time manifest path in: {offenders:?}"
    );
    Ok(())
}
