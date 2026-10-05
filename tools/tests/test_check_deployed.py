import importlib.machinery, importlib.util, os, unittest
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
