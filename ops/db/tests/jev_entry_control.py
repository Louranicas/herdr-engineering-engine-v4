#!/usr/bin/env python3
"""Negative control for `hee4db jev-entry` (the ATLAS P0 Jev entry read-back; V4-74). Synthetic world only.

Every case builds its own world in a temp dir: three v4 homes with files, a MODULES.toml, a handoffs dir, the four
sender logs, a FAKE door (`jev-boundary` stand-in: rc 3 for a v4/v3 engine name or path, rc 0 otherwise, with
fault switches), and door/logger files whose mtimes set the default window. hee4db reads all of it through its own
HEE4DB_* seams; the real door, the real Jev logs and the real DB are never touched.

Each fault case requires its exit code AND a diagnostic only that rule prints; quiet cases require PASS. Then every
verdict rule is neutered in a copy of hee4db and its named case must turn red (`neuters killed=N/N`).
Prints one line per case, then `cases=F/F quiet=Q/Q neuters killed=K/K verdict=PASS|FAIL`. Exit 0 / 20 / 3."""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
HEE4DB = Path(os.environ.get("HEE4DB_BIN", HERE.parent / "hee4db"))
T0 = 1_790_000_000            # the door's mtime (2026-09-21): the default window opens here
AFTER, BEFORE = "2026-09-25T00:00:00+00:00", "2026-09-10T00:00:00+00:00"

FAKE_DOOR = r'''#!/usr/bin/env python3
import os, re, sys
ENG = re.compile(r"(?i)(hee[- ]?v?4|herdr.engineering.engine|hee3-evidence)")
mode = sys.argv[1]
x = sys.stdin.read() if mode == "text" else " ".join(sys.argv[2:])
if os.environ.get("FAKE_DOOR_RC_ALL"): sys.exit(int(os.environ["FAKE_DOOR_RC_ALL"]))
odd = os.environ.get("FAKE_DOOR_ODD_SUBSTR")
if odd and odd in x: sys.exit(1)
ps = os.environ.get("FAKE_DOOR_PASS_SUBSTR")
if ps and ps in x: sys.exit(0)
sys.exit(3 if ENG.search(x) else 0)
'''


def world(base: Path, *, recall: str | None = "clean", crates=("hee4-contracts", "hee4-core")) -> tuple[dict, Path]:
    repo, ev, vault, ho, st = (base / n for n in ("herdr-engineering-engine-v4", "hee4-evidence",
                                                  "herdr-engineering-engine-v4.vault", "handoffs", "state"))
    for d in (repo / "modules", ev, vault, ho, st / "jev-verifier", st / "jev-router", st / "jev-read-gate", st / "jev-recall"):
        d.mkdir(parents=True, exist_ok=True)
    (repo / "modules" / "MODULES.toml").write_text("".join(f'[[module]]\nname = "m{i}"\ncrate = "{c}"\n' for i, c in enumerate(crates)))
    (repo / "CHARTER.md").write_text("charter\n"); (ev / "x.md").write_text("e\n"); (vault / "n.md").write_text("v\n")
    (ho / "HEE4_RESTART.md").write_text("r\n")
    door = base / "bin" / "jev-boundary"; door.parent.mkdir(); door.write_text(FAKE_DOOR); door.chmod(0o755)
    lib = base / "boundary.py"; lib.write_text("#\n"); os.utime(lib, (T0, T0))
    code = base / "jev_recall.py"; code.write_text("#\n"); os.utime(code, (T0, T0))
    w = lambda p, rows: p.write_text("".join(json.dumps(r) + "\n" for r in rows))
    w(st / "jev-verifier/verdicts.jsonl", [{"ts": BEFORE, "cwd": "/home/u", "claim_head": "old", "usage": "x"},
                                           {"ts": AFTER, "cwd": "/home/u", "claim_head": "tests pass", "usage": "$0.0001"}])
    w(st / "jev-router/decisions.jsonl", [{"ts": BEFORE, "description": "old"}, {"ts": AFTER, "description": "fix typo", "cwd": "/home/u", "votes": {}}])
    w(st / "jev-read-gate/decisions.jsonl", [{"ts": BEFORE, "action": "skipped_small", "sent_chars": 0, "path_sha256_12": "x"},
                                             {"ts": AFTER, "action": "sent", "sent_chars": 10, "path_sha256_12": "000000000000"}])
    if recall == "clean":
        w(st / "jev-recall/sends.jsonl", [{"ts": AFTER, "cwd": "/home/u", "kind": "no_match", "sent": True, "prompt_head": "what time is it"}])
    env = dict(os.environ, HEE4DB_REPO=str(repo), HEE4DB_EVIDENCE=str(ev), HEE4DB_VAULT=str(vault), HEE4DB_HANDOFFS=str(ho),
               HEE4DB_JEV_STATE=str(st), HEE4DB_JEV_DOOR=str(door), HEE4DB_JEV_DOOR_LIB=str(lib), HEE4DB_JEV_RECALL_CODE=str(code),
               HEE4DB_PATH=str(base / "none.db"))
    for k in ("FAKE_DOOR_RC_ALL", "FAKE_DOOR_ODD_SUBSTR", "FAKE_DOOR_PASS_SUBSTR", "HEE4DB_JEV_ENTRY_ROW_CAP"):
        env.pop(k, None)
    return env, base


def add(env: dict, rel: str, row: dict) -> None:
    with open(Path(env["HEE4DB_JEV_STATE"]) / rel, "a") as fh:
        fh.write(json.dumps(row) + "\n")


def h12(p: str) -> str:
    return hashlib.sha256(p.encode()).hexdigest()[:12]


def cases() -> list[tuple]:
    """(name, kind, setup(env, base) -> extra_args, want_rc, needles)"""
    def none(env, base): return []
    def pass_substr(env, base): env["FAKE_DOOR_PASS_SUBSTR"] = "hee4db"; return []
    def ver_eng(env, base): add(env, "jev-verifier/verdicts.jsonl", {"ts": AFTER, "cwd": "/home/u", "claim_head": "HEE v4 main pushed", "usage": "$0"}); return []
    def ver_eng_before(env, base): add(env, "jev-verifier/verdicts.jsonl", {"ts": BEFORE, "cwd": "/home/u", "claim_head": "HEE v4 main pushed", "usage": "$0"}); return []
    def ver_eng_not_sent(env, base): add(env, "jev-verifier/verdicts.jsonl", {"ts": AFTER, "cwd": "/home/u", "claim_head": "HEE v4 main pushed", "usage": None}); return []
    def rg_hash(env, base): add(env, "jev-read-gate/decisions.jsonl", {"ts": AFTER, "action": "sent", "sent_chars": 5, "path_sha256_12": h12(env["HEE4DB_REPO"] + "/CHARTER.md")}); return []
    def rg_runhost(env, base): add(env, "jev-read-gate/decisions.jsonl", {"ts": AFTER, "action": "sent", "sent_chars": 5, "path_sha256_12": h12("/run/host" + env["HEE4DB_EVIDENCE"] + "/x.md")}); return []
    def rg_refused(env, base): add(env, "jev-read-gate/decisions.jsonl", {"ts": AFTER, "action": "refused", "sent_chars": 5, "path_sha256_12": h12(env["HEE4DB_REPO"] + "/CHARTER.md")}); return []
    def rec_eng(env, base): add(env, "jev-recall/sends.jsonl", {"ts": AFTER, "cwd": "/home/u", "kind": "matched", "sent": True, "prompt_head": "resume hee4 now"}); return []
    def rou_eng(env, base): add(env, "jev-router/decisions.jsonl", {"ts": AFTER, "description": "hee4 slice", "cwd": "/home/u", "votes": {}}); return []
    def rec_absent(env, base): (Path(env["HEE4DB_JEV_STATE"]) / "jev-recall/sends.jsonl").unlink(); return []
    def rec_logger_late(env, base): return ["--since", "2026-09-15T00:00:00+00:00"]   # after the other logs begin (09-10), before the logger (T0, 09-21)
    def rec_rows_later_than_since(env, base):   # the logger was ready at T0 and has rows only later: complete, measured
        p = Path(env["HEE4DB_JEV_STATE"]) / "jev-recall/sends.jsonl"; p.write_text(json.dumps({"ts": "2026-09-30T00:00:00+00:00", "cwd": "/h", "kind": "no_match", "sent": True, "prompt_head": "x"}) + "\n"); return []
    def ctl_wrong(env, base): env["FAKE_DOOR_RC_ALL"] = "3"; return []
    def odd_rc(env, base): env["FAKE_DOOR_ODD_SUBSTR"] = "hee4-core"; return []
    def row_cap(env, base): env["HEE4DB_JEV_ENTRY_ROW_CAP"] = "0"; return []
    def no_world(env, base): (Path(env["HEE4DB_REPO"]) / "modules/MODULES.toml").write_text('[[module]]\nname = "x"\ncrate = "tooling"\n'); return []
    def bad_since(env, base): return ["--since", "2026-10-02T00:00:00"]
    def not_iso(env, base): return ["--since", "yesterday"]
    def modules_absent(env, base): (Path(env["HEE4DB_REPO"]) / "modules/MODULES.toml").unlink(); return []
    def door_lib_absent(env, base): env["HEE4DB_JEV_DOOR_LIB"] = str(base / "no-such-boundary.py"); return []
    def paths_cap(env, base): env["HEE4DB_JEV_ENTRY_PATH_CAP"] = "1"; return []
    def door_absent(env, base): env["HEE4DB_JEV_DOOR"] = str(base / "no-such-door"); return []
    return [
        ("quiet_clean_world", "quiet", none, 0, ["verdict=PASS", "sent_engine_rows=0/4", "senders_measured=4/4"]),
        ("quiet_row_before_window", "quiet", ver_eng_before, 0, ["verdict=PASS", "sent_engine_rows=0/4"]),
        ("quiet_not_sent_row", "quiet", ver_eng_not_sent, 0, ["verdict=PASS", "sent_engine_rows=0/4"]),
        ("quiet_refused_readgate_row", "quiet", rg_refused, 0, ["verdict=PASS"]),
        ("quiet_recall_logger_ready", "quiet", rec_rows_later_than_since, 0, ["verdict=PASS", "senders_measured=4/4"]),
        ("jp0_v4_line_passes", "fault", pass_substr, 20, ["jp0_v4_line_passes=1", "jp0_line_not_refused rc=0"]),
        ("verifier_engine_row", "fault", ver_eng, 20, ["sent_engine_rows_from=verifier", "sent_engine_rows=1/5"]),
        ("router_engine_row", "fault", rou_eng, 20, ["sent_engine_rows_from=router"]),
        ("recall_engine_row", "fault", rec_eng, 20, ["sent_engine_rows_from=recall"]),
        ("readgate_v4_path_hash", "fault", rg_hash, 20, ["sent_engine_rows_from=read-gate"]),
        ("readgate_runhost_spelling", "fault", rg_runhost, 20, ["sent_engine_rows_from=read-gate"]),
        ("recall_log_absent", "fault", rec_absent, 30, ["sender_unmeasured=recall", "UNMEASURED (absent:"]),
        ("recall_logger_after_since", "fault", rec_logger_late, 30, ["reasons=sender_unmeasured=recall", "UNMEASURED (log ready", "senders_measured=3/4"]),
        ("clean_control_wrong", "fault", ctl_wrong, 30, ["reasons=control_wrong=clean_control", "control name=clean_control rc=3 want=0 WRONG"]),
        ("door_rc_unexpected", "fault", odd_rc, 30, ["door_rc_unexpected=1"]),
        ("rows_over_cap", "fault", row_cap, 20, ["jev_entry_rows_over_cap"]),
        ("no_hee4_crate", "fault", no_world, 20, ["jev_entry_no_world", "names no hee4 crate"]),
        ("modules_unreadable", "fault", modules_absent, 20, ["jev_entry_no_world", "MODULES.toml unreadable"]),
        ("door_lib_absent", "fault", door_lib_absent, 20, ["jev_entry_no_door", "pass --since"]),
        ("paths_over_cap", "fault", paths_cap, 20, ["jev_entry_paths_over_cap", "cap=1"]),
        ("door_absent", "fault", door_absent, 30, ["reasons=control_wrong=v3_control,clean_control", "rc=-1"]),
        ("since_without_offset", "fault", bad_since, 2, ["has no UTC offset"]),
        ("since_not_iso", "fault", not_iso, 2, ["is not ISO-8601"]),
    ]


def run_case(bin_: Path, case) -> tuple[bool, str]:
    name, kind, setup, want, needles = case
    base = Path(tempfile.mkdtemp(prefix=f"jevent-{name}-"))
    try:
        env, base = world(base)
        extra = setup(env, base)
        r = subprocess.run([sys.executable, str(bin_), "jev-entry", *extra], capture_output=True, text=True, env=env,
                           stdin=subprocess.DEVNULL, timeout=300)
        out = r.stdout + r.stderr
        miss = [n for n in needles if n not in out]
        ok = r.returncode == want and not miss
        return ok, f"rc={r.returncode} want={want} missing={miss}" + ("" if ok else f" tail={out.strip().splitlines()[-1][:160] if out.strip() else ''!r}")
    finally:
        shutil.rmtree(base, ignore_errors=True)


NEUTERS = {   # rule site -> (old, new, case that must turn red)
    "passed_line_ignored": ("    if passed:\n        reasons.append(f\"jp0_v4_line_passes", "    if False:\n        reasons.append(f\"jp0_v4_line_passes", "jp0_v4_line_passes"),
    "engine_rows_ignored": ("    if engine:\n        reasons.append(\"sent_engine_rows_from=", "    if False:\n        reasons.append(\"sent_engine_rows_from=", "verifier_engine_row"),
    "odd_rc_ignored": ("    if odd:\n        reasons.append(f\"door_rc_unexpected", "    if False:\n        reasons.append(f\"door_rc_unexpected", "door_rc_unexpected"),
    "controls_ignored": ("    if bad_ctl:\n        return EXIT_UNMEASURED", "    if False:\n        return EXIT_UNMEASURED", "clean_control_wrong"),
    "unmeasured_ignored": ("    if unm:\n", "    if False:\n", "recall_log_absent"),
    "window_ignored": ("        if t >= since:\n            rows.append(rec)", "        if True:\n            rows.append(rec)", "quiet_row_before_window"),
    "sent_predicate_ignored": ("    sent = [r for r in rows if sent_pred(r)]", "    sent = list(rows)", "quiet_not_sent_row"),
    "runhost_spelling_dropped": ("        for s in (p, \"/run/host\" + p):", "        for s in (p,):", "readgate_runhost_spelling"),
    "logger_ready_ignored": ("    ready = logger_ready(sender) or first", "    ready = first", "quiet_recall_logger_ready"),
    "replay_skipped": ("        eng = sum(rc == 3 for rc in rcs)", "        eng = 0", "recall_engine_row"),
    "row_cap_dropped": ("    if len(sent) > JEV_ENTRY_ROW_CAP:", "    if False:", "rows_over_cap"),
    # one neuter per raise site (the census counts SITES, not reason names: CLAUDE.md §4, F140)
    "no_crate_raise_dropped": ("    if not crates:\n        raise Refusal(\"jev_entry_no_world\"", "    if False:\n        raise Refusal(\"jev_entry_no_world\"", "no_hee4_crate"),
    "modules_unreadable_swallowed": ("raise Refusal(\"jev_entry_no_world\", f\"modules/MODULES.toml unreadable ({e.__class__.__name__})\") from e",
                                     "mods = []", "modules_unreadable"),
    "path_cap_dropped": ("            if len(paths) > JEV_ENTRY_PATH_CAP:", "            if False:", "paths_over_cap"),
    "door_lib_raise_swallowed": ("raise Refusal(\"jev_entry_no_door\", f\"{JEV_DOOR_LIB} unreadable ({e.__class__.__name__}); pass --since\") from e",
                                 "since = dt.datetime.now(dt.timezone.utc)", "door_lib_absent"),
    "not_iso_swallowed": ("raise Refusal(\"usage\", f\"--since {a.since!r} is not ISO-8601\", EXIT_USAGE) from e",
                          "since = dt.datetime.now(dt.timezone.utc)", "since_not_iso"),
    "no_offset_dropped": ("        if since.tzinfo is None:\n            raise Refusal(\"usage\"", "        if False:\n            raise Refusal(\"usage\"", "since_without_offset"),
}


def main() -> int:
    if not HEE4DB.exists():
        print(f"setup hee4db not found at {HEE4DB}"); return 3
    cs = cases(); fk = qk = 0; nf = sum(c[1] == "fault" for c in cs); nq = len(cs) - nf; base_ok = {}
    for c in cs:
        ok, det = run_case(HEE4DB, c)
        print(f"{'ok  ' if ok else 'FAIL'} {c[1]} {c[0]} {det}")
        fk += ok and c[1] == "fault"; qk += ok and c[1] == "quiet"; base_ok[c[0]] = ok
    src = HEE4DB.read_text(); kn = 0; by = {c[0]: c for c in cs}
    for n, (a, b, case) in NEUTERS.items():
        if src.count(a) != 1:
            print(f"neuter INVALID {n}: anchor count {src.count(a)}"); continue
        if not base_ok[case]:   # a case already red on the real code cannot show the neuter did anything
            print(f"neuter INVALID {n}: its case {case} is red without the neuter"); continue
        d = Path(tempfile.mkdtemp(prefix="jevent-neuter-")); m = d / "hee4db"; m.write_text(src.replace(a, b))
        ok, _ = run_case(m, by[case]); shutil.rmtree(d, ignore_errors=True)
        kn += not ok; print(f"neuter {'killed  ' if not ok else 'SURVIVED'} {n} (by {case})")
    good = fk == nf and qk == nq and kn == len(NEUTERS)
    print(f"cases={fk}/{nf} quiet={qk}/{nq} neuters killed={kn}/{len(NEUTERS)} verdict={'PASS' if good else 'FAIL'}")
    return 0 if good else 20


if __name__ == "__main__":
    t = time.time(); rc = main(); print(f"elapsed_s={time.time() - t:.1f}"); sys.exit(rc)
