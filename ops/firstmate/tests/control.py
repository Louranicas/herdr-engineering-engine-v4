#!/usr/bin/env python3
"""Negative control for fm-db. Works in temp Firstmate homes only; ~/firstmate/data/firstmate.db is never opened.

Each case plants one fault and requires (a) the expected exit code and (b) the diagnostic token that only that
refusal site prints. Quiet cases run the same verbs on a clean world and require no refusal; the quiet brief case
recomputes brief_sha and standing_sha with hashlib and compares them with what fm-db recorded (no sha is ever
accepted from the caller). fm-db runs as a subprocess with FM_HOME=<tempdir> (FM_DB unset) and HEE4_ROOT=<temp tree
holding agents/standing-orders.md copied from the repo>; --head-sha is always passed except in the case that proves
its absence is refused. Read-back goes through `fm-db status` and `fm-db q`; sqlite3 opens only a temp home's DB, to
build a 001-only database (the schema-behind case) and to try raw SQL around the spawn door (schema/003's triggers).
The init-atomicity case SIGKILLs fm-db between a migration's DDL and its schema_migrations row (fm-db's
FM_DB_CONTROL_KILL_BEFORE_ROW seam) and requires the rerun to apply that migration whole. The section-parity case reads
the U-stack-04 roster briefs ($FM_ROSTER) and the workflow curator's section reader ($WFC_BIN) read-only, importing both
scripts' functions. The temp homes live under $TMPDIR (tempfile's rule).

Prints one line per case, then `fm-db-control cases=k/n quiet=q/q real_db_unchanged=yes verdict=PASS|FAIL`.
Exit 0 on PASS, 20 on FAIL, 3 on setup failure.
"""
from __future__ import annotations

import hashlib
import importlib.machinery
import importlib.util
import json
import os
import shutil
import signal
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent
FM_DB_BIN = HERE.parent / "fm-db"
REPO = HERE.parent.parent.parent
STANDING_SRC = REPO / "agents" / "standing-orders.md"
REAL_DB = Path.home() / "firstmate" / "data" / "firstmate.db"
TURSODB = Path(os.environ.get("HEE4DB_TURSODB", str(Path.home() / ".local/bin/tursodb")))
# The brief fixture is tools/drive's eleven `LABEL:` lines; STANDING carries the standing-orders body verbatim.
DRIVE_LINES = ["GOAL: drive", "SCOPE: s", "CONTEXT: c", "ACCEPTANCE: a", "VERIFY: /usr/bin/true", "TIMEBOX: 10s",
               "FORBIDDEN: f", "REPORT: r", "STANDING:", "RECON: r", "RESTATEMENT: run true"]
HEAD = "0123456789abcdef0123456789abcdef01234567"
# fm-db's control seam for the init-atomicity case: SIGKILL after the named migration's DDL, before its row.
KILL_SEAM = "FM_DB_CONTROL_KILL_BEFORE_ROW"
# The parity case's two worlds, outside this repo: the curator's section reader and the roster briefs it was measured on.
CURATOR_BIN = Path(os.environ.get("WFC_BIN") or "/mnt/storage-10tb/workflow-curator/bin/workflow-curator")
ROSTER = Path(os.environ.get("FM_ROSTER") or "/mnt/storage-10tb/hee4-evidence/roster")
MIGRATIONS = sorted(p.name for p in (HERE.parent / "schema").glob("*.sql"))
FIRST_MIGRATION, LATER_MIGRATION = MIGRATIONS[0], MIGRATIONS[-1]


def sha(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest() if p.exists() else "absent"


def standing_body() -> list[str]:
    lines = STANDING_SRC.read_text().splitlines()
    return [ln for ln in lines[1:] if ln.strip()]


def brief_text(standing: list[str] | None = None, drop: str | None = None, indent: str | None = None,
               verify: list[str] | None = None) -> str:
    """The drive brief; `verify` replaces the VERIFY field with `VERIFY:` followed by these lines, one per line."""
    out = []
    for ln in DRIVE_LINES:
        label = ln.split(":")[0]
        if label == drop:
            continue
        if label == "VERIFY" and verify is not None:
            out.append("VERIFY:")
            out.extend(verify)
            continue
        out.append(("  " + ln) if label == indent else ln)
        if label == "STANDING":
            out.extend(standing if standing is not None else standing_body())
    return "\n".join(out) + "\n"


def load_script(name: str, path: Path) -> ModuleType:
    """A stdlib-only script without a .py suffix, imported for its functions; its `__main__` block does not run."""
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    mod = importlib.util.module_from_spec(importlib.util.spec_from_loader(name, loader))
    loader.exec_module(mod)
    return mod


def section_parity(fm_bin: Path = FM_DB_BIN) -> tuple[bool, str]:
    """The curator loads fm-db's detector, so a detector parity compares fm-db with itself. What can still differ is which
    lines each one reads as VERIFY. For every U-stack-04 roster brief, each line of the curator's VERIFY sections
    (`brief_sections`, which opens a section on `VERIFY (...):` too) that fm-db's detector flags must be among the
    door's own `verify_findings` for that brief. Absent curator or roster, or zero flagged lines, fails the case: a
    parity that read nothing is not one."""
    briefs = sorted((ROSTER / "U-stack-04").glob("*.md"))
    if not CURATOR_BIN.is_file() or not briefs:
        return False, f"unmeasured: curator={CURATOR_BIN.is_file()} briefs={len(briefs)}"
    fm, wfc = load_script("fm_db_parity", fm_bin), load_script("workflow_curator", CURATOR_BIN)
    reader = door = 0
    only: list[str] = []
    for p in briefs:
        text = p.read_text(errors="replace")
        found = {ln for _, _, ln in fm.verify_findings(text)}
        door += len(found)
        for ln in wfc.brief_sections(text).get("VERIFY", []):
            if fm.verify_cannot_fail(ln):
                reader += 1
                if ln.strip() not in found:
                    only.append(f"{p.name}: {ln.strip()[:80]}")
    ok = reader > 0 and not only
    return ok, f"briefs={len(briefs)} reader={reader} door={door} reader_only={len(only)} {'; '.join(only[:3])}".strip()


def migration_001_only(db: Path) -> None:
    """A populated pre-002 database: 001_init.sql and its schema_migrations row, one open unit, one spawn (no brief_sha)."""
    first = HERE.parent / "schema" / FIRST_MIGRATION
    db.parent.mkdir(parents=True, exist_ok=True)
    con = sqlite3.connect(db)
    try:
        con.executescript(first.read_text())
        con.execute("INSERT INTO schema_migrations VALUES (?,?,?)", (FIRST_MIGRATION, sha(first), "2026-10-04T13:21:02Z"))
        con.execute("INSERT INTO units VALUES ('U0','hee4','ship','test',0,'x',4,'2026-10-04T13:21:02Z',NULL)")
        con.execute("INSERT INTO spawns (task_id, unit_id, agent, harness, ts) VALUES ('s0','U0','hee4-gate','agent','2026-10-04T13:21:03Z')")
        con.commit()
    finally:
        con.close()


def raw_refusal(db: Path, sql: str, args: tuple = ()) -> str:
    """Run one raw write against a temp DB (foreign_keys off, sqlite3's default); the SQLite error text, or 'admitted'."""
    con = sqlite3.connect(db)
    try:
        con.execute(sql, args)
        con.commit()
        return "admitted"
    except sqlite3.DatabaseError as e:
        return str(e)
    finally:
        con.close()


class World:
    """One temp Firstmate home plus a temp HEE4_ROOT holding agents/standing-orders.md; git never finds a repo above it."""

    def __init__(self, base: Path, name: str) -> None:
        self.root = base / name
        self.fm_home = self.root / "fm"
        self.hee4_root = self.root / "hee4"
        (self.hee4_root / "agents").mkdir(parents=True)
        shutil.copyfile(STANDING_SRC, self.standing)
        self.fm_home.mkdir()
        self.env = {k: v for k, v in os.environ.items() if k not in ("FM_DB", "GIT_DIR", "GIT_WORK_TREE")}
        self.env.update({"FM_HOME": str(self.fm_home), "HEE4_ROOT": str(self.hee4_root), "GIT_CEILING_DIRECTORIES": str(base)})
        self.n = 0

    @property
    def standing(self) -> Path:
        return self.hee4_root / "agents" / "standing-orders.md"

    @property
    def db(self) -> Path:
        return self.fm_home / "data" / "firstmate.db"

    def fm(self, *args: str, env: dict[str, str] | None = None) -> tuple[int, dict]:
        p = subprocess.run([sys.executable, str(FM_DB_BIN), *args], env={**self.env, **(env or {})}, capture_output=True, text=True,
                           stdin=subprocess.DEVNULL)
        last = [ln for ln in p.stdout.splitlines() if ln.startswith("{")]
        try:
            j = json.loads(last[-1]) if last else {}
        except json.JSONDecodeError:
            j = {}
        j.setdefault("_stderr", p.stderr[-300:])
        return p.returncode, j

    def ok(self, *args: str) -> dict:
        rc, j = self.fm(*args)
        if rc != 0:
            raise RuntimeError(f"setup step refused: {' '.join(args)} -> rc={rc} {j}")
        return j

    def brief_file(self, text: str) -> Path:
        self.n += 1
        p = self.root / f"brief-{self.n}.md"
        p.write_text(text)
        return p

    def unit(self, uid: str, planned: int = 4) -> None:
        self.ok("record", "unit", "--unit-id", uid, "--project", "hee4", "--kind", "ship", "--mode", "test",
                "--brief-path", "x", "--planned-agents", str(planned))

    def brief(self, uid: str, text: str | None = None) -> str:
        p = self.brief_file(text if text is not None else brief_text())
        self.ok("record", "brief", "--unit", uid, "--path", str(p), "--head-sha", HEAD)
        return hashlib.sha256(p.read_bytes()).hexdigest()

    def spawn(self, uid: str, task: str, **kw: str) -> None:
        extra = [x for k, v in kw.items() for x in (f"--{k.replace('_', '-')}", v)]
        self.ok("record", "spawn", "--task-id", task, "--unit-id", uid, "--agent", "hee4-gate", "--harness", "agent", *extra)

    def exit(self, task: str) -> None:
        self.ok("record", "exit", "--task-id", task, "--verdict", "PASS", "--head-sha", HEAD)

    def open_units(self) -> list[str]:
        rc, j = self.fm("status")
        if rc != 0:
            raise RuntimeError(f"status rc={rc} {j}")
        return [u["unit_id"] for u in j["open_units"]]


def detail_of(j: dict) -> str:
    return json.dumps(j, sort_keys=True)


def main() -> int:
    if not FM_DB_BIN.exists() or not STANDING_SRC.exists():
        print(f"setup: missing {FM_DB_BIN} or {STANDING_SRC}")
        return 3
    real_before = sha(REAL_DB)
    results: list[tuple[str, str, bool, str]] = []   # (name, kind, passed, detail)

    def case(name: str, kind: str, rc: int, j: dict, want_rc: int, token: str | None, extra_ok: bool = True, extra: str = "") -> None:
        d = j.get("detail", "") or ""
        hit = token is None or token in d
        passed = rc == want_rc and hit and extra_ok
        results.append((name, kind, passed, f"rc={rc} want={want_rc} token={token!r} hit={hit} {extra}".strip()))
        print(f"case={name} kind={kind} verdict={'PASS' if passed else 'FAIL'} rc={rc} detail={detail_of(j)[:220]}{(' ' + extra) if extra else ''}")

    with tempfile.TemporaryDirectory(prefix="fm-db-control-") as tmp:
        base = Path(tmp)
        try:
            # --- brief door -------------------------------------------------------------------------------------
            w = World(base, "brief-ok"); w.ok("init"); w.unit("U1")
            fx = w.brief_file(brief_text())
            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(fx), "--head-sha", HEAD)
            want_b, want_s = hashlib.sha256(fx.read_bytes()).hexdigest(), hashlib.sha256(w.standing.read_bytes()).hexdigest()
            extra_ok, extra = True, ""
            if TURSODB.exists():
                qrc, qj = w.fm("q", "SELECT brief_sha, standing_sha, head_sha FROM briefs WHERE unit_id='U1'")
                rows = qj.get("rows") or [[]]
                got_b, got_s, got_h = (rows[0] + ["", "", ""])[:3]
                extra_ok = qrc == 0 and got_b == want_b and got_s == want_s and got_h == HEAD
                extra = f"brief_sha_match={got_b == want_b} standing_sha_match={got_s == want_s}"
            else:
                extra = "tursodb_absent: sha read-back skipped"
            case("brief-ok", "quiet", rc, j, 0, None, extra_ok, extra)

            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(fx), "--head-sha", HEAD, "--brief-sha", "deadbeef")
            case("brief-caller-sha-refused", "fault", rc, j, 2, "unknown keys")

            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(base / "nope.md"), "--head-sha", HEAD)
            case("brief-path-unreadable", "fault", rc, j, 20, "brief_path_unreadable")

            p = w.brief_file(brief_text(drop="RESTATEMENT"))
            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(p), "--head-sha", HEAD)
            case("brief-missing-RESTATEMENT", "fault", rc, j, 20, "brief_field_missing=RESTATEMENT")

            p = w.brief_file(brief_text(indent="GOAL"))
            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(p), "--head-sha", HEAD)
            case("brief-field-indented", "fault", rc, j, 20, "brief_field_missing=GOAL")

            body = standing_body()
            body[3] = "4. Every claim is labelled and has a witness."           # one standing line reworded
            p = w.brief_file(brief_text(standing=body))
            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(p), "--head-sha", HEAD)
            case("brief-standing-summarised", "fault", rc, j, 20, "standing_not_verbatim")

            p = w.brief_file(brief_text(standing=["Standing orders apply as in agents/standing-orders.md"]))
            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(p), "--head-sha", HEAD)
            case("brief-standing-sentence", "fault", rc, j, 20, "standing_not_verbatim")

            # A VERIFY line that cannot fail is refused naming its file line and shape; the line sits after a can-fail
            # line so the number is the offender's own. The fixture's line is found, never pinned.
            # A trailing `#` comment or `  (..)` annotation never hides the line before it (the first comment case is
            # roster/U-stack-04/app-runtime-budgets-attempts.md:67, verbatim); pipefail turned off, a PIPESTATUS mention,
            # `|&`, `; exit 0`, `|| echo` and a real subshell are each still refused.
            for name, bad, shape in (("tail", "cargo test --offline | tail -1", "shape=pipe_into_tail_head"),
                                     ("echo-rc", "cargo test --offline; echo rc=$?", "shape=echo_rc"),
                                     ("or-true", "cargo test --offline || true", "shape=or_true"),
                                     ("echo-rc-comment", "tools/doctor; echo rc=$?   # against the installed unit; the budgets row "
                                      "is MEASURED only if the deployed binary carries this slice, else UNMEASURED by name (do not "
                                      "deploy)", "shape=echo_rc"),
                                     ("or-true-comment", "cargo test --offline || true  # x", "shape=or_true"),
                                     ("tail-comment", "cargo test --offline | tail -1  # pipefail", "shape=pipe_into_tail_head"),
                                     ("tail-annotation", "cargo test --offline | tail -1  (rc=0; ...)", "shape=pipe_into_tail_head"),
                                     ("pipefail-off", "set +o pipefail; cargo test --offline | tail -1", "shape=pipe_into_tail_head"),
                                     ("pipestatus-mention", "echo PIPESTATUS; cargo test --offline | tail -1",
                                      "shape=pipe_into_tail_head"),
                                     ("pipe-amp", "cargo test --offline |& tail -1", "shape=pipe_into_tail_head"),
                                     ("exit-0", "cargo test --offline; exit 0", "shape=trailing_true"),
                                     ("or-echo", "cargo test --offline || echo FAILED", "shape=or_true"),
                                     ("subshell", "(cargo test --offline | tail -1)", "shape=pipe_into_tail_head"),
                                     ("described-pipeline", "(from the worktree root; cargo test --offline | tail -1)",
                                      "shape=pipe_into_tail_head"),
                                     ("bash-c", "bash -c 'cargo test --offline | tail -1'", "shape=pipe_into_tail_head"),
                                     ("env-bash-c", "env RUST_LOG=off bash -c 'cargo test --offline || true'", "shape=or_true")):
                text = brief_text(verify=["python3 ops/firstmate/tests/control.py", bad])
                n = text.splitlines().index(bad) + 1
                p = w.brief_file(text)
                rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(p), "--head-sha", HEAD)
                case(f"brief-verify-{name}", "fault", rc, j, 20, f"verify_line_cannot_fail line={n} {shape}")
            # Lines that can fail pass: a pipe into `grep -q`, a stated pipefail, a PIPESTATUS read, a plain command, a
            # subshell whose status is its check's, and a parenthesised description (fm-db verify_cannot_fail has the rule).
            # Then every line the workflow curator flags across the roster briefs must be refused by the door.
            for name, good in (("grep-q", "cargo test --offline | grep -q PASS"),
                               ("pipefail", "set -o pipefail; cargo test --offline | tail -1"),
                               ("pipefail-flags", "set -euo pipefail; cargo test --offline | tail -1"),
                               ("pipestatus-read", "cargo test --offline | tail -1 && exit ${PIPESTATUS[0]}"),
                               ("plain", "cargo fmt --all --check"),
                               ("subshell-can-fail", "(cd tools/tests && python3 -m unittest discover -s . -p 'test_*.py')"),
                               ("described", "(from the worktree root, each line judged by its own exit code)"),
                               ("bash-c-can-fail", "bash -c 'cargo test --offline | grep -q \"test result: ok\"'")):
                p = w.brief_file(brief_text(verify=[good]))
                rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(p), "--head-sha", HEAD)
                case(f"brief-verify-{name}", "quiet", rc, j, 0, None)
            # A second VERIFY section under a qualified label is judged like the `VERIFY:` field (the probe is audit 3's).
            text = brief_text() + "VERIFY (from the worktree root):\ncargo test --workspace 2>&1 | tail -1\n"
            n = text.splitlines().index("cargo test --workspace 2>&1 | tail -1") + 1
            p = w.brief_file(text)
            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(p), "--head-sha", HEAD)
            case("verify-paren-section", "fault", rc, j, 20, f"verify_line_cannot_fail line={n} shape=pipe_into_tail_head")
            # The detector itself, called as the curator calls it: a quoted `bash -c` pipeline and a prose-opener
            # parenthesis holding a pipeline are flagged; a pure description, a real subshell and a plain command are not.
            fm = load_script("fm_db", FM_DB_BIN)
            got = (fm.verify_cannot_fail("bash -c 'cargo test --workspace | tail -1'"),
                   fm.verify_cannot_fail("(from the worktree root; cargo test --workspace | tail -1)"))
            case("detector-quoted-and-prose", "fault", 0, {}, 0, None, got[0] == ["pipe_into_tail_head"] and bool(got[1]), f"got={got}")
            got = tuple(fm.verify_cannot_fail(x) for x in ("(from the worktree root)", "(cd crates && cargo test --workspace)",
                                                           "cargo test --workspace"))
            case("detector-negatives", "quiet", 0, {}, 0, None, got == ([], [], []), f"got={got}")
            parity_ok, parity = section_parity()
            case("brief-verify-section-parity", "fault", 0, {}, 0, None, parity_ok, parity)

            p = w.brief_file(brief_text())
            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(p))     # no git under the temp HEE4_ROOT
            case("brief-no-head-sha", "fault", rc, j, 20, "head_sha_unmeasured")

            # --- spawn door -------------------------------------------------------------------------------------
            w = World(base, "spawn"); w.ok("init"); w.unit("U1"); w.unit("U2")
            rc, j = w.fm("record", "spawn", "--task-id", "t1", "--unit-id", "U1", "--agent", "hee4-gate", "--harness", "agent")
            case("spawn-before-brief", "fault", rc, j, 20, "no brief recorded")
            b1 = w.brief("U1")
            rc, j = w.fm("record", "spawn", "--task-id", "t0", "--unit-id", "U1", "--agent", "hee4-gate", "--harness", "agent")
            spawned = next((u["spawned"] for u in w.fm("status")[1].get("open_units", []) if u["unit_id"] == "U1"), None)
            case("spawn-without-brief-sha", "fault", rc, j, 20, "brief_sha_missing", spawned == 0, f"spawned={spawned}")
            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(w.root / f"brief-{w.n}.md"), "--head-sha", HEAD)
            d = j.get("detail", "") or ""
            case("duplicate-brief-typed", "fault", rc, j, 20, f"brief_already_recorded sha={b1[:12]} unit=U1",
                 "UNIQUE constraint" not in d, f"raw_sqlite_text={'UNIQUE constraint' in d}")
            rc, j = w.fm("record", "spawn", "--task-id", "t1", "--unit-id", "U1", "--agent", "hee4-gate", "--harness", "agent", "--brief-sha", b1)
            case("spawn-after-brief", "quiet", rc, j, 0, None)
            w.brief("U2", brief_text().replace("GOAL: drive", "GOAL: other"))
            rc, j = w.fm("record", "spawn", "--task-id", "t2", "--unit-id", "U2", "--agent", "hee4-gate", "--harness", "agent", "--brief-sha", b1)
            case("spawn-brief-sha-other-unit", "fault", rc, j, 20, "brief_sha_unit_mismatch")
            rc, j = w.fm("record", "spawn", "--task-id", "t3", "--unit-id", "U1", "--agent", "hee4-gate", "--harness", "agent",
                         "--brief-sha", "0" * 64)
            case("spawn-brief-sha-unknown", "fault", rc, j, 20, "brief_sha_unknown")

            # --- schema level: a DB behind schema/*.sql is refused by every verb but init, which then levels it -------
            ws = World(base, "schema-behind")
            migration_001_only(ws.db)
            behind = f"schema_behind={MIGRATIONS[1]}"
            for verb in (("status",), ("record", "spawn", "--task-id", "t1", "--unit-id", "U0", "--agent", "a", "--harness", "agent",
                                       "--brief-sha", "0" * 64),
                         ("record", "brief", "--unit", "U0", "--path", str(ws.brief_file(brief_text())), "--head-sha", HEAD)):
                rc, j = ws.fm(*verb)
                case(f"schema-behind-refuses-{'-'.join(verb[:2])}", "fault", rc, j, 3, behind)
            rc, j = ws.fm("init")
            case("schema-behind-init-levels", "quiet", rc, j, 0, None, j.get("applied") == MIGRATIONS[1:], f"applied={j.get('applied')}")
            rc, j = ws.fm("status")
            units = j.get("open_units") or [{}]
            case("schema-behind-status-briefs", "quiet", rc, j, 0, None, "briefs" in units[0] and units[0].get("spawned") == 1,
                 f"open_units={units}")

            # --- schema/003 in SQLite itself: raw SQL cannot record a spawn without its unit's brief ----------------
            con = sqlite3.connect(f"file:{ws.db}?mode=ro", uri=True)
            kept = con.execute("SELECT task_id, brief_sha FROM spawns").fetchall()
            con.close()
            bu = ws.brief("U0")
            ws.spawn("U0", "t1", brief_sha=bu)
            ins = "INSERT INTO spawns (task_id, unit_id, agent, harness, ts, brief_sha) VALUES (?, 'U0', 'a', 'agent', 'x', ?)"
            null = raw_refusal(ws.db, ins, ("raw1", None))
            other = raw_refusal(ws.db, ins, ("raw2", "0" * 64))
            blank = raw_refusal(ws.db, "UPDATE spawns SET brief_sha = NULL WHERE task_id = 't1'")
            ok3 = kept == [("s0", None)] and "brief_sha_missing" in null and "brief_sha_not_of_unit" in other and "brief_sha_kept" in blank
            case("spawn-null-brief-sha-trigger", "fault", 0, {}, 0, None, ok3,
                 f"pre002_rows={kept} null={null!r} other={other!r} blank={blank!r}")

            # --- self-verification (001_init.sql trigger) ---------------------------------------------------------
            w.ok("record", "claim", "--task-id", "t1", "--text", "x", "--label", "MEASURED", "--witness-cmd", "true", "--head-sha", HEAD)
            rc, j = w.fm("record", "verify", "--claim-id", "1", "--verifier-task", "t1", "--verdict", "verified")
            case("self-verification", "fault", rc, j, 20, "claimant cannot verify its own claim")

            # --- close-unit -------------------------------------------------------------------------------------
            rc, j = w.fm("close-unit", "--id", "U1", "--by", "captain")
            still_open = "U1" in w.open_units()
            case("close-with-spawn-without-exit", "fault", rc, j, 20, "spawn_without_exit task_id=t1", still_open, f"still_open={still_open}")

            w.exit("t1")
            w.ok("record", "andon", "--unit-id", "U1", "--raised-by", "hee4-watch-fence", "--reason", "r", "--measured", "1")
            rc, j = w.fm("close-unit", "--id", "U1", "--by", "captain")
            case("close-with-open-andon", "fault", rc, j, 20, "open andon")
            w.ok("clear-andon", "--id", "1", "--by", "captain")

            rc, j = w.fm("close-unit", "--id", "U1", "--by", "captain")
            closed_gone = "U1" not in w.open_units()
            case("close-ok", "quiet", rc, j, 0, None, j.get("unverified") == 1 and j.get("spawns") == 1 and closed_gone,
                 f"unverified={j.get('unverified')} absent_from_status={closed_gone}")

            rc, j = w.fm("close-unit", "--id", "U1", "--by", "captain")
            case("close-twice", "fault", rc, j, 20, "already_closed")

            rc, j = w.fm("record", "spawn", "--task-id", "t9", "--unit-id", "U1", "--agent", "hee4-gate", "--harness", "agent", "--brief-sha", b1)
            case("spawn-into-closed-unit", "fault", rc, j, 20, "unit_closed")

            rc, j = w.fm("close-unit", "--id", "U-none", "--by", "captain")
            case("close-no-unit", "fault", rc, j, 20, "no unit")

            # --- init atomicity: a migration after 001 commits its DDL and its row together or not at all ----------
            w = World(base, "init-kill")
            rc, j = w.fm("init", env={KILL_SEAM: LATER_MIGRATION})           # SIGKILL after 002's DDL, before its row
            killed = rc == -signal.SIGKILL and "verb" not in j
            rc, j = w.fm("init")                                              # the rerun must apply 002 whole
            case("init-killed-before-row-rerun-applies", "fault", rc, j, 0, None,
                 killed and j.get("applied") == [LATER_MIGRATION] and j.get("already") == MIGRATIONS[:-1],
                 f"killed={killed} applied={j.get('applied')} already={j.get('already')}")
            rc, j = w.fm("init")
            case("init-twice", "quiet", rc, j, 0, None, j.get("applied") == [] and j.get("already") == MIGRATIONS,
                 f"applied={j.get('applied')} already={j.get('already')}")
            rc, j = w.fm("status")
            case("init-rerun-status", "quiet", rc, j, 0, None, j.get("open_units") == [], f"open_units={j.get('open_units')}")
            w = World(base, "init-kill-first")                                # 001 runs outside a transaction: IF NOT EXISTS carries it
            rc, j = w.fm("init", env={KILL_SEAM: FIRST_MIGRATION})
            killed = rc == -signal.SIGKILL and "verb" not in j
            rc, j = w.fm("init")
            case("init-killed-before-first-row-rerun-applies", "fault", rc, j, 0, None, killed and j.get("applied") == MIGRATIONS,
                 f"killed={killed} applied={j.get('applied')}")
        except RuntimeError as e:
            print(f"setup: {e}")
            return 3

    real_after = sha(REAL_DB)
    unchanged = real_before == real_after
    faults = [r for r in results if r[1] == "fault"]
    quiet = [r for r in results if r[1] == "quiet"]
    k, q = sum(r[2] for r in faults), sum(r[2] for r in quiet)
    passed = k == len(faults) and q == len(quiet) and unchanged and len(faults) > 0
    real = "yes" if unchanged else "NO"
    if real_before == "absent":
        real = "absent"
    print(f"fm-db-control cases={k}/{len(faults)} quiet={q}/{len(quiet)} real_db_unchanged={real} verdict={'PASS' if passed else 'FAIL'}")
    return 0 if passed else 20


if __name__ == "__main__":
    sys.exit(main())
