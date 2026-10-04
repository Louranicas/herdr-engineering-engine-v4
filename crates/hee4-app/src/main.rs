//! `hee4`: `serve`, `doctor`, `--version`, and the one-frame client verbs
//! (`health`, `task.submit`, `task.get`, `task.list`, `task.cancel`).

use std::path::PathBuf;
use std::process::ExitCode;

use hee4_app::{ServeArgs, dispatcher, doctor, head12, socket, wire};
use serde_json::{Value, json};

const USAGE: &str = "usage: hee4 --version | serve --socket P --ledger P --work D | doctor [--unit U] [--socket P] [--repo D]
       | health | task.list | task.get ID | task.cancel ID --key K | task.submit --brief-file F --key K   [--socket P]";

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
            println!("hee4 {} {}", hee4_app::VERSION, head12());
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

fn client(verb: &str, args: &[String], sock: &std::path::Path) -> ExitCode {
    let key = flag(args, "--key");
    let body = match verb {
        "health" | "task.list" => json!({}),
        "task.get" | "task.cancel" => match positional(args) {
            Some(id) => json!({ "task_id": id }),
            None => return fail("task id required"),
        },
        "task.submit" => match flag(args, "--brief-file").map(std::fs::read_to_string) {
            Some(Ok(brief)) => json!({ "brief": brief }),
            Some(Err(e)) => return fail(&format!("brief file: {e}")),
            None => return fail("--brief-file required"),
        },
        other => return fail(&format!("unknown verb {other}")),
    };
    let id = format!("cli-{}", std::process::id());
    let reply = match socket::request(sock, &wire::request(&id, verb, key.as_deref(), body)) {
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
