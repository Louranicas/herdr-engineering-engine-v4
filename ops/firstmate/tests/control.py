#!/usr/bin/env python3
"""Negative control for fm-db. Works in temp Firstmate homes only; ~/firstmate/data/firstmate.db is never opened.

Each case plants one fault and requires (a) the expected exit code and (b) the diagnostic token that only that
refusal site prints. Quiet cases run the same verbs on a clean world and require no refusal; the quiet brief case
recomputes brief_sha and standing_sha with hashlib and compares them with what fm-db recorded (no sha is ever
accepted from the caller). fm-db runs as a subprocess with FM_HOME=<tempdir> (FM_DB unset) and HEE4_ROOT=<temp tree
holding agents/standing-orders.md copied from the repo>; --head-sha is always passed except in the case that proves
its absence is refused. Read-back goes through `fm-db status` and `fm-db q` only (never sqlite3 against any DB).

Prints one line per case, then `fm-db-control cases=k/n quiet=q/q real_db_unchanged=yes verdict=PASS|FAIL`.
Exit 0 on PASS, 20 on FAIL, 3 on setup failure.
"""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

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


def sha(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest() if p.exists() else "absent"


def standing_body() -> list[str]:
    lines = STANDING_SRC.read_text().splitlines()
    return [ln for ln in lines[1:] if ln.strip()]


def brief_text(standing: list[str] | None = None, drop: str | None = None, indent: str | None = None) -> str:
    out = []
    for ln in DRIVE_LINES:
        label = ln.split(":")[0]
        if label == drop:
            continue
        out.append(("  " + ln) if label == indent else ln)
        if label == "STANDING":
            out.extend(standing if standing is not None else standing_body())
    return "\n".join(out) + "\n"


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

    def fm(self, *args: str) -> tuple[int, dict]:
        p = subprocess.run([sys.executable, str(FM_DB_BIN), *args], env=self.env, capture_output=True, text=True, stdin=subprocess.DEVNULL)
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

            p = w.brief_file(brief_text())
            rc, j = w.fm("record", "brief", "--unit", "U1", "--path", str(p))     # no git under the temp HEE4_ROOT
            case("brief-no-head-sha", "fault", rc, j, 20, "head_sha_unmeasured")

            # --- spawn door -------------------------------------------------------------------------------------
            w = World(base, "spawn"); w.ok("init"); w.unit("U1"); w.unit("U2")
            rc, j = w.fm("record", "spawn", "--task-id", "t1", "--unit-id", "U1", "--agent", "hee4-gate", "--harness", "agent")
            case("spawn-before-brief", "fault", rc, j, 20, "no brief recorded")
            b1 = w.brief("U1")
            rc, j = w.fm("record", "spawn", "--task-id", "t1", "--unit-id", "U1", "--agent", "hee4-gate", "--harness", "agent", "--brief-sha", b1)
            case("spawn-after-brief", "quiet", rc, j, 0, None)
            w.brief("U2", brief_text().replace("GOAL: drive", "GOAL: other"))
            rc, j = w.fm("record", "spawn", "--task-id", "t2", "--unit-id", "U2", "--agent", "hee4-gate", "--harness", "agent", "--brief-sha", b1)
            case("spawn-brief-sha-other-unit", "fault", rc, j, 20, "brief_sha_unit_mismatch")

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
