//! `hee4 doctor`: the rows `tools/doctor` reads, each `present` or `MISSING`, then a verdict.
//! It never opens the ledger itself (`Store::open` resets `recovery_complete` under a live
//! server); "ledger opens" is read through `health`.

use std::fmt::Write as _;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::Path;
use std::process::Command;

use hee4_host::model::OllamaClient;
use serde_json::json;

use crate::{dispatcher::MODEL_URL, socket, wire};

/// One row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Row name.
    pub name: &'static str,
    /// Present.
    pub present: bool,
    /// What was read.
    pub detail: String,
}

fn row(name: &'static str, present: bool, detail: String) -> Row {
    Row {
        name,
        present,
        detail,
    }
}

/// Run every row.
#[must_use]
pub fn rows(unit: &str, sock: &Path, repo: &Path) -> Vec<Row> {
    let active = Command::new("systemctl")
        .args(["--user", "is-active", unit])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    let unit_row = match active {
        Ok(s) => row("unit", s == "active", format!("{unit} {s}")),
        Err(e) => row("unit", false, format!("systemctl: {e}")),
    };
    let perms = sock
        .parent()
        .and_then(|d| std::fs::metadata(d).ok())
        .zip(std::fs::metadata(sock).ok());
    let sock_row = match perms {
        Some((d, s)) => {
            let (dm, sm) = (
                d.permissions().mode() & 0o777,
                s.permissions().mode() & 0o777,
            );
            let ok = dm == 0o700
                && sm == 0o600
                && s.file_type().is_socket()
                && Some(s.uid()) == crate::process_uid();
            row("socket", ok, format!("dir={dm:o} sock={sm:o}"))
        }
        None => row("socket", false, format!("{} absent", sock.display())),
    };
    let health = socket::request(sock, &wire::request("doctor", "health", None, json!({})));
    let ledger_row = match health {
        Ok(v) if v["body"]["recovery_complete"] == true => {
            row("ledger", true, "health recovery_complete=true".into())
        }
        Ok(v) => row("ledger", false, format!("health {v}")),
        Err(e) => row("ledger", false, format!("health: {e}")),
    };
    let model_row = match OllamaClient::new(MODEL_URL).tags() {
        Ok(tags) => row("model", true, format!("{MODEL_URL} tags={}", tags.len())),
        Err(e) => row("model", false, format!("{MODEL_URL}: {e}")),
    };
    let head = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default();
    let sha_row = row(
        "binary_sha",
        !head.is_empty() && head == crate::HEAD,
        format!(
            "binary={} HEAD={}",
            crate::head12(),
            head.get(..12).unwrap_or("unknown")
        ),
    );
    vec![unit_row, sock_row, ledger_row, model_row, sha_row]
}

/// Print the rows and the verdict line; `true` when every row is present.
#[must_use]
pub fn render(rows: &[Row]) -> (String, bool) {
    let mut out = String::new();
    for r in rows {
        let _ = writeln!(
            out,
            "doctor row={} {} {}",
            r.name,
            if r.present { "present" } else { "MISSING" },
            r.detail
        );
    }
    let k = rows.iter().filter(|r| r.present).count();
    let pass = k == rows.len();
    let _ = writeln!(
        out,
        "doctor verdict={} present={k}/{} head={}",
        if pass { "PASS" } else { "FAIL" },
        rows.len(),
        crate::head12()
    );
    (out, pass)
}
