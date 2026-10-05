//! `hee4 doctor`: the rows `tools/doctor` reads, each `present` or `MISSING`, then a verdict.
//! It never opens the ledger itself (`Store::open` resets `recovery_complete` under a live
//! server); "ledger opens" is read through `health`.

use std::fmt::Write as _;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::Path;
use std::process::Command;

use hee4_contracts::Budgets;
use hee4_host::model::OllamaClient;
use serde_json::{Value, json};

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
    let ledger_row = match &health {
        Ok(v) if v["body"]["recovery_complete"] == true => {
            row("ledger", true, "health recovery_complete=true".into())
        }
        Ok(v) => row("ledger", false, format!("health {v}")),
        Err(e) => row("ledger", false, format!("health: {e}")),
    };
    let budgets_row = match &health {
        Ok(v) => budgets_row(&v["body"]["budgets"]),
        Err(e) => row("budgets", false, format!("health: {e}")),
    };
    // The CLI reads no budgets file: the contracts' default tags timeout.
    let model_row =
        match OllamaClient::new(MODEL_URL).tags_within(Budgets::DEFAULT.model.tags_timeout()) {
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
    vec![
        unit_row,
        sock_row,
        ledger_row,
        budgets_row,
        model_row,
        sha_row,
    ]
}

/// `budgets` is present iff `health`'s `body.budgets` re-parses through [`Budgets::parse`]
/// (validation and rendering stay the contracts') AND serialises back to the same JSON: a
/// served value always names every field, so `{}` or a partial object (which `parse` would fill
/// with defaults) is not what the engine serves and is `MISSING`. The detail is
/// [`Budgets::render`].
fn budgets_row(budgets: &Value) -> Row {
    if budgets.is_null() {
        return row("budgets", false, "health carries no budgets".into());
    }
    match Budgets::parse(&budgets.to_string()) {
        Ok(b) => match serde_json::to_value(b) {
            Ok(full) if full == *budgets => row("budgets", true, b.render()),
            Ok(full) => row(
                "budgets",
                false,
                format!("health budgets are not the full served value: {budgets} != {full}"),
            ),
            Err(e) => row("budgets", false, e.to_string()),
        },
        Err(e) => row("budgets", false, e.to_string()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_budgets_row_re_parses_through_the_contracts() -> Result<(), serde_json::Error> {
        let present = budgets_row(&serde_json::to_value(Budgets::DEFAULT)?);
        assert_eq!(present, row("budgets", true, Budgets::DEFAULT.render()));
        let zero = budgets_row(&json!({"door": {"max_body_bytes": 0}}));
        assert!(!zero.present, "{zero:?}");
        assert!(zero.detail.contains("door.max_body_bytes"), "{zero:?}");
        assert!(!budgets_row(&Value::Null).present);
        // `parse` fills these with defaults; the engine never serves them.
        for partial in [
            json!({}),
            json!({"socket": {"max_connections": 2}}),
            json!({"socket": {}, "stream": {}}),
        ] {
            let r = budgets_row(&partial);
            assert!(!r.present, "{partial} -> {r:?}");
            assert!(r.detail.contains("not the full served value"), "{r:?}");
        }
        for wrong in [json!({"bogus": {}}), json!({"socket": 5}), json!([1])] {
            assert!(!budgets_row(&wrong).present, "{wrong}");
        }
        Ok(())
    }
}
