//! Bake the build commit into the binary: `HEE4_HEAD` (40 hex digits, or `unknown`).

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn env_head() -> Option<String> {
    let v = std::env::var("HEE4_HEAD").ok()?;
    (v.len() == 40 && v.bytes().all(|b| b.is_ascii_hexdigit())).then_some(v)
}

fn main() {
    println!("cargo:rerun-if-env-changed=HEE4_HEAD");
    let head = env_head()
        .or_else(|| git(&["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=HEE4_HEAD={head}");
    if let Some(dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={dir}/HEAD");
        println!("cargo:rerun-if-changed={dir}/logs/HEAD");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
