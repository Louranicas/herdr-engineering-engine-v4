//! `hee4`: `serve`, `doctor`, `--version`, and the one-frame client. The five positional verbs
//! (`health`, `task.list`, `task.get ID`, `task.cancel ID --key K`, `task.submit --brief-file F
//! --key K`) keep their forms; every catalogued action is reachable as `hee4 <action> [--key K]
//! [--body JSON | --body-file F] [--precondition JSON]`. An action the catalogue does not carry
//! is exit 2 with the catalogue's ids.

use std::path::PathBuf;
use std::process::ExitCode;

use hee4_app::{ServeArgs, catalogue12, dispatcher, doctor, head12, socket, wire};
use hee4_contracts::catalogue::{self, CATALOGUE};
use serde_json::{Value, json};

const USAGE: &str = "usage: hee4 --version | serve --socket P --ledger P --work D | doctor [--unit U] [--socket P] [--repo D]
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
        _ => client(verb, &args, &sock),
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
