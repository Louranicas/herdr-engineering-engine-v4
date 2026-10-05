import importlib.machinery, importlib.util, os, signal, subprocess, sys, time, unittest
from common import TOOLS, run

CD = os.path.join(TOOLS, "check-deployed")
ROWS = [f"D{n}" for n in range(1, 10)]

# A `systemctl --user show -p ExecStart hee4.service` line saved from the live host (2026-10-05).
LIVE_EXECSTART = ("{ path=/home/louranicas/.local/bin/hee4 ; argv[]=/home/louranicas/.local/bin/hee4 serve "
                  "--socket /run/user/1000/hee4/control.sock --ledger /home/louranicas/.local/share/hee4/ledger.sqlite3 "
                  "--work /home/louranicas/.local/share/hee4/work ; ignore_errors=no ; start_time=[Mon 2026-10-05 10:18:21 AEDT] ; "
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
        self.assertEqual(ex, {"bin": "/home/louranicas/.local/bin/hee4", "socket": "/run/user/1000/hee4/control.sock",
                              "ledger": "/home/louranicas/.local/share/hee4/ledger.sqlite3", "work": "/home/louranicas/.local/share/hee4/work"})
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
        root = os.path.expanduser("~/.cache/hee4-host")
        before = set(os.listdir(root)) if os.path.isdir(root) else set()
        ctl = subprocess.Popen([CD, "--control"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
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
        rc, out, err = run(CD, "--control", timeout=180)
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

    def test_no_declaration_is_parsed(self):
        src = open(CD).read()
        self.assertNotIn("store.rs", src); self.assertNotIn("SCHEMA_VERSION", src)


if __name__ == "__main__":
    unittest.main()
