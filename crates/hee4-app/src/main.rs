//! `hee4`: `serve`, `doctor`, `--version`, and the one-frame client. The five positional verbs
//! (`health`, `task.list`, `task.get ID`, `task.cancel ID --key K`, `task.submit --brief-file F
//! --key K`) keep their forms; every catalogued action is reachable as `hee4 <action> [--key K]
//! [--body JSON | --body-file F] [--precondition JSON]`. An action the catalogue does not carry
//! is exit 2 with the catalogue's ids.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use hee4_app::{ServeArgs, catalogue12, dispatcher, doctor, head12, socket, wire};
use hee4_contracts::catalogue::{self, CATALOGUE};
use hee4_core::Store;
use hee4_core::receipts::VerifyError;
use serde_json::{Value, json};

const USAGE: &str = "usage: hee4 --version | serve --socket P --ledger P --work D [--budgets F] [--backups D] | restore --into D ID [--backups D] | doctor [--unit U] [--socket P] [--repo D] | verify-ledger --ledger P
       | health | task.list | task.get ID | task.cancel ID --key K | task.submit --brief-file F --key K   [--socket P]
       | <action> [--key K] [--body JSON | --body-file F] [--precondition JSON] [--socket P]   (any catalogued action)";

fn default_socket() -> PathBuf {
    let rt =
        std::env::var_os("XDG_RUNTIME_DIR").map_or_else(|| PathBuf::from("/tmp"), PathBuf::from);
    rt.join("hee4/control.sock")
}

/// `--name value` from `args`.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn positional(args: &[String]) -> Option<String> {
    args.get(1).filter(|a| !a.starts_with("--")).cloned()
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("hee4: {msg}\n{USAGE}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(verb) = args.first().map(String::as_str) else {
        return fail("no verb");
    };
    let sock = flag(&args, "--socket").map_or_else(default_socket, PathBuf::from);
    match verb {
        "--version" | "version" => {
            println!(
                "hee4 {} {} catalogue={}",
                hee4_app::VERSION,
                head12(),
                catalogue12()
            );
            ExitCode::SUCCESS
        }
        "serve" => {
            let serve = match serve_args(
                &args,
                std::env::var_os("HEE4_REQUIRE_BACKUPS"),
                std::env::var_os("HEE4_BUDGETS"),
            ) {
                Ok(serve) => serve,
                Err(e) => return fail(&format!("serve: {e}")),
            };
            match hee4_app::serve(&serve, &dispatcher::Config::from_env()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("hee4 serve refused: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "doctor" => {
            let unit = flag(&args, "--unit").unwrap_or_else(|| "hee4.service".into());
            let repo = flag(&args, "--repo").map_or_else(|| PathBuf::from("."), PathBuf::from);
            let (text, pass) = doctor::render(&doctor::rows(&unit, &sock, &repo));
            print!("{text}");
            if pass {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        "restore" => {
            let Some(into) = flag(&args, "--into") else {
                return fail("restore needs --into DIR and a backup ID");
            };
            let Some(id) = restore_id(&args) else {
                return fail("restore needs a backup ID");
            };
            let root =
                flag(&args, "--backups").map_or_else(|| PathBuf::from(BACKUP_ROOT), PathBuf::from);
            restore(&root, &id, Path::new(&into))
        }
        "verify-ledger" => {
            let Some(ledger) = flag(&args, "--ledger") else {
                return fail("verify-ledger needs --ledger");
            };
            verify_ledger(Path::new(&ledger))
        }
        _ => client(verb, &args, &sock),
    }
}

/// The flags `hee4 serve` takes, each once and each with one value.
const SERVE_FLAGS: [&str; 5] = ["--socket", "--ledger", "--work", "--budgets", "--backups"];

/// Why `hee4 serve`'s argv or environment was refused at the boundary (exit 2, nothing opened).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum ServeFlagError {
    /// A flag with no value: last on the line, or followed by another flag or an empty word.
    #[error("{0} needs a value")]
    MissingValue(&'static str),
    /// A flag given twice: which one wins would be a guess.
    #[error("{0} is given twice")]
    Repeated(&'static str),
    /// A word that is not one of [`SERVE_FLAGS`] where a flag was due.
    #[error(
        "{0:?} is not a serve flag (expected one of --socket, --ledger, --work, --budgets, --backups)"
    )]
    Unknown(String),
    /// `--ledger` or `--work` is absent.
    #[error("{0} is required")]
    Required(&'static str),
    /// `--backups` is relative: the unit's working directory would choose the root.
    #[error("--backups must be an absolute path, not {0:?}")]
    RelativeBackups(PathBuf),
    /// `HEE4_REQUIRE_BACKUPS` is set to something other than `1` or `0`.
    #[error("HEE4_REQUIRE_BACKUPS must be 1 or 0, not {0:?}")]
    RequireBackups(std::ffi::OsString),
}

/// `hee4 serve`'s argv (`args[0]` is `serve`) and environment, parsed once: every word after
/// the verb is a [`SERVE_FLAGS`] flag followed by its value, each flag at most once; `--ledger`
/// and `--work` are required, `--socket` defaults to `$XDG_RUNTIME_DIR/hee4/control.sock`,
/// `--backups` is absolute, `--budgets` else `HEE4_BUDGETS` names the budgets file, and
/// `HEE4_REQUIRE_BACKUPS` is unset, `0` or `1`.
fn serve_args(
    args: &[String],
    require_backups: Option<std::ffi::OsString>,
    budgets_env: Option<std::ffi::OsString>,
) -> Result<ServeArgs, ServeFlagError> {
    let mut values: [Option<&String>; SERVE_FLAGS.len()] = [None; SERVE_FLAGS.len()];
    let mut words = args.iter().skip(1);
    while let Some(word) = words.next() {
        let Some(i) = SERVE_FLAGS.iter().position(|f| f == word) else {
            return Err(ServeFlagError::Unknown(word.clone()));
        };
        let name = SERVE_FLAGS[i];
        let value = words
            .next()
            .filter(|v| !v.is_empty() && !v.starts_with("--"))
            .ok_or(ServeFlagError::MissingValue(name))?;
        if values[i].replace(value).is_some() {
            return Err(ServeFlagError::Repeated(name));
        }
    }
    let [socket, ledger, work, budgets, backups] = values.map(|v| v.map(PathBuf::from));
    let require_backups = match require_backups {
        None => false,
        Some(v) if v == "1" => true,
        Some(v) if v == "0" => false,
        Some(v) => return Err(ServeFlagError::RequireBackups(v)),
    };
    if let Some(b) = backups.as_ref().filter(|b| !b.is_absolute()) {
        return Err(ServeFlagError::RelativeBackups(b.clone()));
    }
    Ok(ServeArgs {
        socket: socket.unwrap_or_else(default_socket),
        ledger: ledger.ok_or(ServeFlagError::Required("--ledger"))?,
        work: work.ok_or(ServeFlagError::Required("--work"))?,
        // The one place a budgets path is named: the flag, else the unit's env.
        budgets: budgets.or_else(|| budgets_env.map(PathBuf::from)),
        backups,
        require_backups,
    })
}

/// `hee4 restore`'s default backup root (the unit's `--backups`).
const BACKUP_ROOT: &str = "/mnt/storage-10tb/hee4-backups";

/// The restore's one positional: the first argument after the verb that is neither a flag nor
/// a flag's value.
fn restore_id(args: &[String]) -> Option<String> {
    let mut rest = args.iter().skip(1);
    while let Some(a) = rest.next() {
        if a.starts_with("--") {
            rest.next();
        } else {
            return Some(a.clone());
        }
    }
    None
}

/// K1's `restore` of `<root>/<id>` into `into`: one `restore ... verdict=` line printed and
/// appended to `<root>/restore.log`; exit 0 only on PASS. An id that names no backup directory
/// (or is not one path component) is `reason=not_found`; a K1 refusal is its name.
fn restore(root: &Path, id: &str, into: &Path) -> ExitCode {
    let started = std::time::Instant::now();
    let one_component = !id.is_empty() && id != "." && id != ".." && !id.contains('/');
    let dir = root.join(id);
    let (line, pass) = if one_component && dir.is_dir() {
        match hee4_core::backup::restore(&dir, into) {
            Ok(r) => (
                format!(
                    "restore backup={} ledger={} objects={}/{} rto_s={:.3} verdict=PASS",
                    r.backup_id,
                    r.ledger_sha256.get(..12).unwrap_or(&r.ledger_sha256),
                    r.objects_n,
                    r.objects_total,
                    Duration::from_millis(r.rto_ms).as_secs_f64()
                ),
                true,
            ),
            Err(e) => {
                eprintln!("hee4 restore: {e}");
                (
                    restore_fail(id, started, dispatcher::backup_error_name(&e)),
                    false,
                )
            }
        }
    } else {
        (restore_fail(id, started, "not_found"), false)
    };
    println!("{line}");
    let log = root.join("restore.log");
    let appended = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .and_then(|mut f| std::io::Write::write_all(&mut f, format!("{line}\n").as_bytes()));
    if let Err(e) = appended {
        eprintln!("hee4 restore: cannot append {}: {e}", log.display());
        return ExitCode::from(1);
    }
    if pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn restore_fail(id: &str, started: std::time::Instant, reason: &str) -> String {
    format!(
        "restore backup={id} ledger=none objects=none rto_s={:.3} verdict=FAIL reason={reason}",
        started.elapsed().as_secs_f64()
    )
}

/// The offline verifier (no socket, no engine): open the ledger file read-only and re-derive
/// every link, seal and checkpoint root. One line on stdout; exit 0 on PASS, 1 on a break, 2
/// when the file cannot be opened or read.
fn verify_ledger(ledger: &Path) -> ExitCode {
    let store = match Store::open_read_only(ledger) {
        Ok(store) => store,
        Err(e) => {
            return fail(&format!(
                "verify-ledger cannot open {}: {e}",
                ledger.display()
            ));
        }
    };
    match store.verify_ledger() {
        Ok(r) => {
            let root = r.root.to_string();
            println!(
                "verify-ledger receipts={} tasks={} checkpoints={} root={} verdict=PASS",
                r.receipts,
                r.tasks,
                r.checkpoints,
                root.get(..12).unwrap_or(&root)
            );
            ExitCode::SUCCESS
        }
        Err(VerifyError::Fault(f)) => {
            println!(
                "verify-ledger receipts=unmeasured checkpoints=unmeasured break receipt={} seq={} cause={} verdict=FAIL",
                f.receipt
                    .as_ref()
                    .map_or_else(|| "none".to_owned(), ToString::to_string),
                f.seq,
                f.cause
            );
            ExitCode::from(1)
        }
        Err(VerifyError::Store(e)) => fail(&format!(
            "verify-ledger cannot read {}: {e}",
            ledger.display()
        )),
    }
}

/// `--body JSON` or `--body-file F`, parsed; `None` when neither is given.
fn body_flag(args: &[String]) -> Result<Option<Value>, String> {
    let text = match (flag(args, "--body"), flag(args, "--body-file")) {
        (Some(_), Some(_)) => return Err("--body or --body-file, not both".into()),
        (Some(text), None) => text,
        (None, Some(path)) => {
            std::fs::read_to_string(&path).map_err(|e| format!("body file: {e}"))?
        }
        (None, None) => return Ok(None),
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(v @ Value::Object(_)) => Ok(Some(v)),
        Ok(_) => Err("body must be a JSON object".into()),
        Err(e) => Err(format!("body: {e}")),
    }
}

fn client(verb: &str, args: &[String], sock: &std::path::Path) -> ExitCode {
    if catalogue::find(verb).is_none() {
        let ids: Vec<&str> = CATALOGUE.iter().map(|a| a.id).collect();
        return fail(&format!(
            "unknown action {verb}; the catalogue: {}",
            ids.join(" ")
        ));
    }
    let key = flag(args, "--key");
    let given = match body_flag(args) {
        Ok(b) => b,
        Err(e) => return fail(&e),
    };
    // The positional forms, byte-compatible with the skeleton's client.
    let body = match verb {
        "task.get" | "task.cancel" => match (positional(args), given) {
            (Some(id), _) => json!({ "task_id": id }),
            (None, Some(b)) => b,
            (None, None) => return fail("task id required"),
        },
        "task.submit" => match (
            flag(args, "--brief-file").map(std::fs::read_to_string),
            given,
        ) {
            (Some(Ok(brief)), _) => json!({ "brief": brief }),
            (Some(Err(e)), _) => return fail(&format!("brief file: {e}")),
            (None, Some(b)) => b,
            (None, None) => return fail("--brief-file required"),
        },
        _ => given.unwrap_or_else(|| json!({})),
    };
    let precondition = match flag(args, "--precondition").map(|p| serde_json::from_str::<Value>(&p))
    {
        None => None,
        Some(Ok(v)) => Some(v),
        Some(Err(e)) => return fail(&format!("precondition: {e}")),
    };
    let id = format!("cli-{}", std::process::id());
    let frame = wire::request_with(&id, verb, key.as_deref(), body, precondition);
    let reply = match socket::request(sock, &frame) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("hee4: cannot reach {}: {e}", sock.display());
            return ExitCode::from(3);
        }
    };
    if verb == "health" && reply["kind"] == "result" {
        let b = &reply["body"];
        let complete = b["recovery_complete"] == Value::Bool(true);
        println!(
            "ready={} recovery={} database=ready socket=owned head={} uptime_s={}",
            b["ok"],
            if complete { "complete" } else { "incomplete" },
            b["head_sha"]
                .as_str()
                .and_then(|h| h.get(..12))
                .unwrap_or("unknown"),
            b["uptime_s"]
        );
    }
    println!("{reply}");
    if reply["kind"] == "result" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::{ServeFlagError, serve_args};
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn argv(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    fn parse(line: &str) -> Result<hee4_app::ServeArgs, ServeFlagError> {
        serve_args(&argv(line), None, None)
    }

    const BASE: &str = "serve --socket /r/s --ledger /l --work /w";

    #[test]
    fn a_flag_without_its_value_is_refused_by_name() {
        assert_eq!(
            parse(&format!("{BASE} --budgets")).err(),
            Some(ServeFlagError::MissingValue("--budgets"))
        );
        assert_eq!(
            parse(&format!("{BASE} --budgets --backups /b")).err(),
            Some(ServeFlagError::MissingValue("--budgets"))
        );
        assert_eq!(
            serve_args(
                &[argv(BASE), vec!["--backups".into(), String::new()]].concat(),
                None,
                None
            )
            .err(),
            Some(ServeFlagError::MissingValue("--backups"))
        );
    }

    #[test]
    fn a_repeated_or_unknown_flag_is_refused_by_name() {
        assert_eq!(
            parse(&format!("{BASE} --budgets /a --budgets /b")).err(),
            Some(ServeFlagError::Repeated("--budgets"))
        );
        assert_eq!(
            parse(&format!("{BASE} --backup /b")).err(),
            Some(ServeFlagError::Unknown("--backup".into()))
        );
        assert_eq!(
            parse("serve --ledger /l").err(),
            Some(ServeFlagError::Required("--work"))
        );
    }

    #[test]
    fn a_relative_backups_root_is_refused() {
        assert_eq!(
            parse(&format!("{BASE} --backups rel/path")).err(),
            Some(ServeFlagError::RelativeBackups("rel/path".into()))
        );
    }

    #[test]
    fn require_backups_is_one_or_zero_or_unset() {
        let with = |v: &str| serve_args(&argv(BASE), Some(OsString::from(v)), None);
        assert_eq!(with("1").map(|a| a.require_backups), Ok(true));
        assert_eq!(with("0").map(|a| a.require_backups), Ok(false));
        assert_eq!(parse(BASE).map(|a| a.require_backups), Ok(false));
        for bad in ["true", "yes", "", " 1"] {
            assert_eq!(
                with(bad).err(),
                Some(ServeFlagError::RequireBackups(bad.into())),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn budgets_flag_wins_over_the_env() {
        let env = Some(OsString::from("/env.json"));
        let parsed = serve_args(&argv(BASE), None, env.clone()).map(|a| a.budgets);
        assert_eq!(parsed, Ok(Some(PathBuf::from("/env.json"))));
        let flagged = serve_args(&argv(&format!("{BASE} --budgets /f.json")), None, env);
        assert_eq!(
            flagged.map(|a| a.budgets),
            Ok(Some(PathBuf::from("/f.json")))
        );
    }

    /// The unit's exact `ExecStart` argv (systemd specifiers expanded) parses to its paths,
    /// with `HEE4_REQUIRE_BACKUPS=1` as the unit sets it.
    #[test]
    fn the_unit_exec_start_parses() -> Result<(), String> {
        let unit = include_str!("../../../systemd/hee4.service");
        let exec = unit
            .lines()
            .find_map(|l| l.strip_prefix("ExecStart="))
            .ok_or("no ExecStart")?
            .replace("%h", "/home/u")
            .replace("%t", "/run/user/1");
        let words: Vec<String> = exec.split_whitespace().skip(1).map(str::to_owned).collect();
        let parsed = serve_args(&words, Some("1".into()), None).map_err(|e| e.to_string())?;
        assert_eq!(
            parsed.socket,
            PathBuf::from("/run/user/1/hee4/control.sock")
        );
        assert_eq!(
            parsed.ledger,
            PathBuf::from("/home/u/.local/share/hee4/ledger.sqlite3")
        );
        assert_eq!(parsed.work, PathBuf::from("/home/u/.local/share/hee4/work"));
        assert!(
            parsed
                .backups
                .as_deref()
                .is_some_and(std::path::Path::is_absolute)
        );
        assert!(parsed.require_backups);
        Ok(())
    }
}
