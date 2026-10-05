import importlib.machinery, importlib.util, json, os, shutil, signal, subprocess, sys, tempfile, time, types, unittest, unittest.mock
from common import TOOLS, run

CD = os.path.join(TOOLS, "check-deployed")
ROWS = [f"D{n}" for n in range(1, 10)]

# A `systemctl --user show -p ExecStart hee4.service` line saved from the live host (2026-10-05), the owner's home rewritten to /home/op.
LIVE_EXECSTART = ("{ path=/home/op/.local/bin/hee4 ; argv[]=/home/op/.local/bin/hee4 serve "
                  "--socket /run/user/1000/hee4/control.sock --ledger /home/op/.local/share/hee4/ledger.sqlite3 "
                  "--work /home/op/.local/share/hee4/work ; ignore_errors=no ; start_time=[Mon 2026-10-05 10:18:21 AEDT] ; "
                  "stop_time=[n/a] ; pid=2531320 ; code=(null) ; status=0/0 }")


def load():
    loader = importlib.machinery.SourceFileLoader("check_deployed", CD)
    spec = importlib.util.spec_from_loader("check_deployed", loader)
    mod = importlib.util.module_from_spec(spec); loader.exec_module(mod)
    return mod


def row(out, name):
    return [l for l in out.splitlines() if l.startswith(name + " ")]


class CheckDeployedTests(unittest.TestCase):
    def test_help(self):
        rc, out, _ = run(CD, "--help"); self.assertEqual(rc, 0); self.assertIn("deployed=", out)

    def test_parse_execstart_live_shape(self):
        m = load()
        ex = m.parse_execstart(LIVE_EXECSTART)
        self.assertEqual(ex, {"bin": "/home/op/.local/bin/hee4", "socket": "/run/user/1000/hee4/control.sock",
                              "ledger": "/home/op/.local/share/hee4/ledger.sqlite3", "work": "/home/op/.local/share/hee4/work"})
        self.assertEqual(m.parse_environment("HEE4_LIVE_MODEL=1 HEE4_MODEL=qwen2.5:0.5b"), {"HEE4_LIVE_MODEL": "1", "HEE4_MODEL": "qwen2.5:0.5b"})

    def test_quiet_control_every_plant_detected(self):
        rc, out, err = run(CD, "--control", timeout=180)
        self.assertEqual(rc, 0, out + err)
        self.assertEqual(out.strip().splitlines()[-1], "check-deployed control cases=9/9 verdict=PASS")
        for n in ROWS:
            self.assertEqual(len(row(out, "control " + n)), 1, n)
            self.assertIn("detected=yes", row(out, "control " + n)[0])
        self.assertIn("control_ledger=synthetic", out)
        self.assertIn("in_mainpid_fds=no", row(out, "control D3")[0]); self.assertIn("held_by=", row(out, "control D3")[0])
        d1 = dict(t.split("=", 1) for t in row(out, "control D1")[0].split() if "=" in t)  # the PATH stub says the tree; D1 asked the unit's binary
        self.assertNotEqual(d1["binary"], d1["head"]); self.assertIn("exe_head", d1); self.assertNotEqual(d1["exe_head"], d1["binary"])

    def test_fire_killed_control_leaves_no_listener_and_is_swept(self):
        # A private root under ~/.cache (never /tmp): no other control on the host can sweep it.
        root = tempfile.mkdtemp(prefix="cd-test-root-", dir=os.path.expanduser("~/.cache"))
        self.addCleanup(shutil.rmtree, root, True)
        env = {**os.environ, "HEE4_CONTROL_ROOT": root}
        before = set(os.listdir(root))
        ctl = subprocess.Popen([CD, "--control"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
        server, t0 = None, time.monotonic()
        while server is None and time.monotonic() - t0 < 20:  # wait until the control's listener child exists
            for d in os.listdir("/proc"):
                if d.isdigit():
                    try:
                        argv = open(f"/proc/{d}/cmdline", "rb").read().split(b"\0")
                    except OSError:
                        continue
                    if len(argv) > 2 and b"cd-control-" in argv[1] and argv[1].endswith(b"server.py"):
                        server = (int(d), argv[1].decode()); break
            time.sleep(0.02)
        self.assertIsNotNone(server, "the control never started its server child")
        os.kill(ctl.pid, signal.SIGKILL); ctl.wait()
        home = server[1].split("/lib/")[0]
        t0 = time.monotonic()
        while time.monotonic() - t0 < 5 and os.path.exists(f"/proc/{server[0]}"):
            time.sleep(0.05)
        self.assertFalse(os.path.exists(f"/proc/{server[0]}"), "the listener outlived its killed control")
        holders = [d for d in os.listdir("/proc") if d.isdigit() and home.encode() in (open(f"/proc/{d}/cmdline", "rb").read() if os.path.exists(f"/proc/{d}/cmdline") else b"")]
        self.assertEqual(holders, [], f"processes still under {home}")
        self.assertTrue(os.path.isdir(home))  # the dir stays until the next control sweeps it
        rc, out, err = run(CD, "--control", timeout=180, env={"HEE4_CONTROL_ROOT": root})
        self.assertEqual(rc, 0, out + err)
        self.assertFalse(os.path.isdir(home), "the dead control's dir was not swept")
        self.assertRegex(out, r"swept_dead_controls=[1-9]")
        self.assertEqual(set(os.listdir(root)) - before, set())

    def test_fire_control_skip_removes_exactly_one_case(self):
        for n in ROWS:
            rc, out, err = run(CD, "--control", "--control-skip", n, timeout=180)
            self.assertEqual(rc, 1, n + out + err)
            self.assertEqual(out.strip().splitlines()[-1], "check-deployed control cases=8/9 verdict=FAIL", n)
            line = row(out, "control " + n)[0]
            self.assertIn("plant=skipped", line)
            if n == "D5":
                self.assertIn("UNMEASURED(no backup dir at", line); self.assertTrue(line.endswith(" UNMEASURED"), line)
            else:
                self.assertTrue(line.endswith(" PASS"), line)

    def test_fire_control_skip_unknown_row_refused(self):
        rc, out, _ = run(CD, "--control", "--control-skip", "D10")
        self.assertEqual(rc, 2); self.assertIn("refused", out)

    def test_parse_version_new_and_old_lines(self):
        m = load()  # `hee4 --version` since the catalogue landed (live 2026-10-05), and the line before it
        new = m.parse_version("hee4 4.0.0-skeleton d0bebb274a11 catalogue=74c45f9bd2d2")
        old = m.parse_version("hee4 4.0.0-skeleton d0bebb274a11")
        self.assertEqual(new, {"version": "4.0.0-skeleton", "head": "d0bebb274a11", "catalogue": "74c45f9bd2d2"})
        self.assertEqual(old, {"version": "4.0.0-skeleton", "head": "d0bebb274a11", "catalogue": None})
        self.assertEqual(m.parse_version("hee4 4.0.0 d0bebb274a117257abeb544ddca1003aab1016bc")["head"], "d0bebb274a11")
        for bad in ("", "Python 3.13.1", "hee4 4.0.0-skeleton", "hee4 4.0.0 catalogue=74c45f9bd2d2", "hee4 4.0.0 xyz"):
            self.assertIsNone(m.parse_version(bad), bad)

    def _d1(self, bin_line, exe_line, tree):
        """d1() against a fake unit: argv[0] and /proc/<pid>/exe are two scripts printing the given lines."""
        m, d = load(), tempfile.mkdtemp(prefix="cd-d1-")
        self.addCleanup(shutil.rmtree, d, True)
        def script(name, line):
            p = os.path.join(d, name)
            with open(p, "w") as f:
                f.write(f"#!/bin/sh\necho '{line}'\n")
            os.chmod(p, 0o755); return p
        binp, exe = script("hee4", bin_line), script("exe-hee4", exe_line)
        os.makedirs(os.path.join(d, "proc", "42")); os.symlink(exe, os.path.join(d, "proc", "42", "exe"))
        r = m.d1(types.SimpleNamespace(proc=os.path.join(d, "proc")), {"pid": 42, "bin": binp, "tree": tree})
        return r.line(), dict(t.split("=", 1) for t in r.line().split() if "=" in t)

    def test_d1_new_line_heads_agree_pass(self):
        line, kv = self._d1("hee4 4.0.0-skeleton d0bebb274a11 catalogue=74c45f9bd2d2",
                            "hee4 4.0.0-skeleton d0bebb274a11 catalogue=74c45f9bd2d2", "d0bebb274a11")
        self.assertEqual((kv["binary"], kv["exe_head"], kv["catalogue"]), ("d0bebb274a11", "d0bebb274a11", "74c45f9bd2d2"))
        self.assertEqual(kv["identity"], "match")  # the two scripts print the same line, so their bytes match
        self.assertNotIn("version_head", kv); self.assertTrue(line.endswith(" PASS"), line)

    def test_d1_planted_exe_version_head_differs_fails_naming_it(self):
        line, kv = self._d1("hee4 4.0.0-skeleton d0bebb274a11 catalogue=74c45f9bd2d2",
                            "hee4 4.0.0-skeleton 54fd592aaaaa catalogue=74c45f9bd2d2", "d0bebb274a11")
        self.assertEqual(kv["binary"], "d0bebb274a11"); self.assertEqual(kv["exe_head"], "54fd592aaaaa")
        self.assertEqual(kv["version_head"], "mismatch(binary=d0bebb274a11,exe=54fd592aaaaa)")
        self.assertTrue(line.endswith(" FAIL"), line)

    def test_feature_counts_every_line_lands_once(self):
        m = load()
        feats = ["drive feature=health verdict=PASS paths=6/6 evidence=e",
                 "drive feature=a verdict=UNMEASURED paths=0/0 evidence=e scope=unserved reason=no procedure in tools/drive.d",
                 "drive feature=b verdict=UNMEASURED paths=0/0 evidence=e reason=procedure UNWRITTEN",
                 "drive feature=c verdict=UNMEASURED paths=1/3 evidence=e",
                 "drive feature=d verdict=FAIL paths=2/3 evidence=e",
                 "drive feature=e verdict=FAIL paths=2/3 evidence=e reason=a FAIL is never excused",
                 "drive feature=f verdict=PASS_WITH_GAPS paths=2/3 evidence=e"]
        self.assertEqual(m.feature_counts(feats), (1, 2, 4))

    # A ledger backup manifest's top-level keys as hee4-core backup.rs writes them (live b-01a10b4a227a-0000001c, 2026-10-05).
    LEDGER_MANIFEST = {"boot": 28, "epoch": "1a107794ef1", "id": "b-01a10b4a227a-0000001c", "objects_bound": 1024,
                       "objects_n": 120, "task_count": 120, "ts_ms": 1791190770298, "files": {"ledger.sqlite3": "e9" * 32}}

    def test_d5_manifest_objects_reads_objects_n_with_its_bound(self):
        m = load()
        self.assertEqual(m.manifest_objects(self.LEDGER_MANIFEST), (120, 1024))
        self.assertEqual(m.manifest_objects({"objects": [1, 2]}), (None, None))  # the key D5 used to read: never None silently
        self.assertEqual(m.manifest_objects({"objects_n": True, "objects_bound": -1}), (None, None))

    INV = "0123456789abcdef0123456789abcdef"

    def _d5(self, enabled, verdict_line, status="0", result="success", exit_ts="Mon 2026-10-05 03:15:09 AEDT",
            unit_inv=INV, log_text=None):
        """d5() against a fixture world: a ledger backup on another device than HOME, a stubbed systemctl
        (is-enabled answers `enabled`; show answers Result, ExecMainStatus, ExecMainExitTimestamp, InvocationID),
        a stub restore, and the habitat tool's log (one run block of the unit's invocation, unless log_text)."""
        m = load()
        os.makedirs(os.path.expanduser("~/.cache/hee4-host"), exist_ok=True)
        home = tempfile.mkdtemp(prefix="cd-d5-home-", dir=os.path.expanduser("~/.cache/hee4-host"))
        other = tempfile.mkdtemp(prefix="cd-d5-backups-")
        self.addCleanup(shutil.rmtree, home, True); self.addCleanup(shutil.rmtree, other, True)
        self.assertNotEqual(os.stat(home).st_dev, os.stat(other).st_dev, "the fixture needs two devices")
        bid = self.LEDGER_MANIFEST["id"]
        os.makedirs(os.path.join(other, bid))
        with open(os.path.join(other, bid, "manifest.json"), "w") as f:
            json.dump(self.LEDGER_MANIFEST, f)
        with open(os.path.join(other, "restore.log"), "w") as f:
            f.write(f"restore backup={bid} ledger=e9580e780f40 objects=120/120 rto_s=0.075 verdict=PASS\n")
        bindir = os.path.join(home, "bin"); os.makedirs(bindir)
        def script(name, body):
            with open(os.path.join(bindir, name), "w") as f:
                f.write("#!/usr/bin/env bash\n" + body)
            os.chmod(os.path.join(bindir, name), 0o755)
        script("systemctl", f"""case " $* " in
  *" is-enabled hee4-backup.timer "*) {'echo enabled' if enabled else 'echo "Failed to get unit file state for hee4-backup.timer: No such file or directory" >&2; exit 1'} ;;
  *" show "*) echo Result={result}; echo ExecMainStatus={status}; echo "ExecMainExitTimestamp={exit_ts}"; echo InvocationID={unit_inv} ;;
  *) exit 1 ;;
esac
""")
        script("hee4", f'[ "$1" = restore ] && echo "restore backup={bid} ledger=e9580e780f40 objects=120/120 rto_s=0.01 verdict=PASS"\n')
        log = os.path.join(home, "habitat-backup.log")
        if log_text is None and verdict_line is not None:
            log_text = f"habitat-backup run ts=2026-10-05T03:15:00Z invocation={self.INV} child_rc=0 exit=0\n{verdict_line}\n"
        if log_text is not None:
            with open(log, "w") as f:
                f.write(log_text)
        env = {"HOME": home, "PATH": bindir + ":" + os.environ["PATH"]}
        with unittest.mock.patch.dict(os.environ, env):
            r = m.d5(types.SimpleNamespace(backups=other, habitat_log=log), {"bin": os.path.join(bindir, "hee4")})
        kv = {}
        for t in r.line().split():  # the first key wins: restore_log= quotes a line with its own objects=
            if "=" in t:
                kv.setdefault(*t.split("=", 1))
        return r.line(), kv

    PASS_LINE = "habitat-backup verdict=PASS objects=208 bytes=1 backup=h-20261005T031500-000000Z dest=/d keep=14 pruned=0"

    def test_d5_timer_enabled_and_last_verdict_pass_passes(self):
        line, kv = self._d5(True, self.PASS_LINE)
        self.assertEqual((kv["objects"], kv["objects_bound"], kv["timer"]), ("120", "1024", "enabled"))
        self.assertEqual((kv["habitat_verdict"], kv["habitat_agree"], kv["last_run"]), ("PASS", "yes", "success/0"))
        self.assertEqual((kv["unit_ran"], kv["habitat_run"]), ("yes", "unit"))
        self.assertTrue(line.endswith(" PASS"), line)

    def test_d5_never_run_unit_fails_even_with_a_manual_pass_in_the_log(self):
        # the live never-run shape (measured 2026-10-05): Result=success ExecMainStatus=0, empty timestamp and InvocationID;
        # the log holds a manual `--service` PASS from a shell whose INVOCATION_ID is some other unit's
        manual = ("habitat-backup run ts=2026-10-05T09:00:00Z invocation=1552d8453bb040148a87f6cfd8af62f0 child_rc=0 exit=0\n"
                  + self.PASS_LINE + "\n")
        line, kv = self._d5(True, None, exit_ts="", unit_inv="", log_text=manual)
        self.assertEqual((kv["unit_ran"], kv["habitat_verdict"]), ("never", "PASS"))
        self.assertTrue(kv["habitat_run"].startswith("other("), kv["habitat_run"])
        self.assertTrue(line.endswith(" FAIL"), line)

    def test_d5_crash_block_after_a_pass_fails(self):
        crash = (f"habitat-backup run ts=2026-10-04T03:15:00Z invocation=aaaa child_rc=0 exit=0\n{self.PASS_LINE}\n"
                 f"habitat-backup run ts=2026-10-05T03:15:00Z invocation={self.INV} child_rc=1 exit=30\n"
                 "habitat-backup child_stderr='NotADirectoryError: /x/verdict=PASS'\n")
        line, kv = self._d5(True, None, status="30", result="exit-code", log_text=crash)
        self.assertEqual((kv["habitat_verdict"], kv["habitat_run"]), ("absent", "unit"))
        self.assertTrue(line.endswith(" FAIL"), line)

    def test_d5_log_block_from_another_invocation_fails(self):
        line, kv = self._d5(True, self.PASS_LINE, unit_inv="ffffffffffffffffffffffffffffffff")
        self.assertEqual(kv["habitat_verdict"], "PASS"); self.assertEqual(kv["habitat_run"], "other(log=0123456789ab,unit=ffffffffffff)")
        self.assertTrue(line.endswith(" FAIL"), line)

    def test_d5_timer_absent_fails(self):
        line, kv = self._d5(False, None, status="", result="", exit_ts="", unit_inv="")
        self.assertEqual(kv["timer"], "rc=1"); self.assertEqual(kv["habitat_verdict"], "absent")
        self.assertTrue(line.endswith(" FAIL"), line)

    def test_d5_verdict_and_exit_status_disagree_fails(self):
        line, kv = self._d5(True, self.PASS_LINE, status="30", result="exit-code")  # a crash after an old PASS line
        self.assertEqual(kv["habitat_agree"], "no(verdict=PASS,status=30)"); self.assertTrue(line.endswith(" FAIL"), line)
        line, kv = self._d5(True, self.PASS_LINE.replace("PASS", "FAIL"), status="0")  # an rc that reads FAIL as success
        self.assertEqual(kv["habitat_agree"], "no(verdict=FAIL,status=0)"); self.assertTrue(line.endswith(" FAIL"), line)

    def test_no_declaration_is_parsed(self):
        src = open(CD).read()
        self.assertNotIn("store.rs", src); self.assertNotIn("SCHEMA_VERSION", src)


if __name__ == "__main__":
    unittest.main()
