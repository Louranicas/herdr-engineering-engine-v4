//! Rung-2 census behind a rung-1 door: a spawn plan is built only by `spawn::plan` under a
//! `Permit`. The plan's fields are private (rung 1), so a struct literal outside
//! `crates/hee4-host/src/spawn.rs` cannot compile. The privacy itself is held by the
//! `compile_fail` doctests on the type: one `E0451` literal, plus one `E0616` write per field,
//! so making any single field `pub` fails its own doctest. This census only counts literals:
//! it names one in any `.rs` file under `crates/` outside the door, which catches a literal
//! even when every field is re-exposed, but it does not see a field write. The match is a
//! token census (no regex): the type name as a whole identifier followed by optional
//! whitespace and `{`. The needle is assembled at run time, so this file holds no literal of
//! its own.

use std::error::Error;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The type name; the census appends the `{` check itself.
const TYPE: &str = "SpawnPlan";

/// The one file allowed to spell a plan literal, relative to the `crates/` directory.
const DOOR: &str = "hee4-host/src/spawn.rs";

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Whether the path ending at `start` sits in a type position whose `{` opens a body, not a
/// literal: a return type (`-> ..::T {`) or an impl header (`impl T {`, `impl X for T {`).
fn type_position(bytes: &[u8], start: usize) -> bool {
    let mut i = start;
    while i > 0 && (is_ident(bytes[i - 1]) || bytes[i - 1] == b':') {
        i -= 1;
    }
    let before = std::str::from_utf8(&bytes[..i])
        .unwrap_or_default()
        .trim_end();
    let word = |w: &str| {
        before
            .strip_suffix(w)
            .is_some_and(|rest| rest.as_bytes().last().is_none_or(|b| !is_ident(*b)))
    };
    before.ends_with("->") || word("impl") || word("for")
}

/// How many plan literals `text` spells: the type name as a whole identifier, then optional
/// whitespace, then `{`, outside a return type or impl header.
fn literals(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut from = 0;
    while let Some(at) = text[from..].find(TYPE) {
        let start = from + at;
        let end = start + TYPE.len();
        from = end;
        if start > 0 && is_ident(bytes[start - 1]) {
            continue;
        }
        let rest = &bytes[end..];
        let next = rest.iter().position(|b| !b.is_ascii_whitespace());
        if next.is_some_and(|i| rest[i] == b'{') && !type_position(bytes, start) {
            count += 1;
        }
    }
    count
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The `crates/` directory, read when the test runs: `env!` would bake the compile-time path
/// into a binary the gate reuses across `git archive` exports.
fn crates_dir() -> Result<PathBuf, Box<dyn Error>> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| format!("CARGO_MANIFEST_DIR is not set (run under cargo test): {e}"))?;
    Path::new(&manifest)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("{manifest} has no parent").into())
}

/// What one walk found: every file seen (relative), offenders outside [`DOOR`] with their
/// literal count, and the door's own count.
struct Census {
    files: Vec<String>,
    offenders: Vec<(String, usize)>,
    door_literals: usize,
}

fn census(crates: &Path) -> Result<Census, Box<dyn Error>> {
    let mut paths = Vec::new();
    rust_files(crates, &mut paths)?;
    let mut out = Census {
        files: Vec::new(),
        offenders: Vec::new(),
        door_literals: 0,
    };
    for path in &paths {
        let n = literals(&std::fs::read_to_string(path)?);
        let rel = path.strip_prefix(crates)?.display().to_string();
        if rel == DOOR {
            out.door_literals = n;
        } else if n > 0 {
            out.offenders.push((rel.clone(), n));
        }
        out.files.push(rel);
    }
    Ok(out)
}

#[test]
fn no_plan_literal_outside_the_spawn_door() -> Result<(), Box<dyn Error>> {
    let c = census(&crates_dir()?)?;
    for walked in [DOOR, "hee4-worker/src/namespace.rs", "hee4-app/src/main.rs"] {
        assert!(
            c.files.iter().any(|f| f == walked),
            "{walked} is walked: {:?}",
            c.files
        );
    }
    assert!(
        c.door_literals > 0,
        "the door itself builds the plan: {DOOR} holds a literal"
    );
    assert!(
        c.offenders.is_empty(),
        "plan literal outside {DOOR}: {:?}",
        c.offenders
    );
    Ok(())
}

#[test]
fn the_census_catches_planted_literals() {
    assert_eq!(literals(&format!("let p = {TYPE} {{ permit: x }};")), 1);
    assert_eq!(literals(&format!("hee4_host::spawn::{TYPE}{{ argv }}")), 1);
    assert_eq!(literals(&format!("let p = {TYPE}\n    {{ argv }};")), 1);
    assert_eq!(literals(&format!("fn f(p: {TYPE}) {{}}")), 0);
    assert_eq!(literals(&format!("let a = p.argv(); // {TYPE} getter")), 0);
    assert_eq!(literals(&format!("struct My{TYPE} {{ x: u8 }}")), 0);
    assert_eq!(literals(&format!("fn f() -> spawn::{TYPE} {{ g() }}")), 0);
    assert_eq!(literals(&format!("impl Tr for {TYPE} {{}}")), 0);
    assert_eq!(
        literals(&format!("fn f() -> {TYPE} {{ {TYPE} {{ argv }} }}")),
        1
    );
    assert_eq!(literals(&format!("let before = {TYPE} {{ argv }};")), 1);
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            if entry.file_name() == "target" {
                continue;
            }
            copy_tree(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// The walk, not only the matcher: a copy of `crates/` with one literal planted in the
/// worker's `native.rs` (the shape the wave-1 refuter named) names that file, and only that
/// file, while the door stays exempt.
#[test]
fn the_walk_names_a_planted_literal_and_exempts_the_door() -> Result<(), Box<dyn Error>> {
    let planted = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("spawn-plan-door-crates");
    if planted.exists() {
        std::fs::remove_dir_all(&planted)?;
    }
    copy_tree(&crates_dir()?, &planted)?;
    let victim = planted.join("hee4-worker/src/native.rs");
    let mut text = std::fs::read_to_string(&victim)?;
    write!(
        text,
        "\nfn bypass() -> hee4_host::spawn::{TYPE} {{\n    {TYPE} {{ permit: PermitId(0), \
         receipt: r, program: p, argv: vec![], timeout: t }}\n}}\n"
    )?;
    std::fs::write(&victim, text)?;
    let c = census(&planted)?;
    assert_eq!(
        c.offenders,
        [("hee4-worker/src/native.rs".to_owned(), 1)],
        "the literal is named; the return type is not"
    );
    assert!(c.door_literals > 0);
    Ok(())
}
