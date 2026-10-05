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
            let (Some(ledger), Some(work)) = (flag(&args, "--ledger"), flag(&args, "--work"))
            else {
                return fail("serve needs --socket, --ledger and --work");
            };
            let serve = ServeArgs {
                socket: sock,
                ledger: ledger.into(),
                work: work.into(),
                // The one place a budgets path is named: the flag, else the unit's env.
                budgets: flag(&args, "--budgets")
                    .map(PathBuf::from)
                    .or_else(|| std::env::var_os("HEE4_BUDGETS").map(PathBuf::from)),
                backups: flag(&args, "--backups").map(PathBuf::from),
                require_backups: std::env::var("HEE4_REQUIRE_BACKUPS").as_deref() == Ok("1"),
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
