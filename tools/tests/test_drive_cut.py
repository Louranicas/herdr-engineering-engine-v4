"""tools/drive-cut: the cut's drive, run live and on a disposable serve and merged path by path.

Merge rules (unit): FAIL in either run wins; PASS in either run (and FAIL in neither) is PASS;
UNMEASURED only when both runs left the path unmeasured, with both reasons kept. Integration: the
"live" side is itself a disposable serve here (never the deployed unit), and the disposable-only
paths (crash-restart, mutating roster paths) come back measured."""
import os, re, shutil, subprocess, tempfile, time, unittest
from common import TOOLS, run
from test_drive import BIN, REPO, TARGET, load_tool

CUT = os.path.join(TOOLS, "drive-cut")


def feat(name, verdict, k, n, paths=(), tail=""):
    lines = [f"drive feature={name} verdict={verdict} paths={k}/{n} evidence=x{tail}"]
    lines += [f"  path={p} status={s} detail={d}" for p, s, d in paths]
    return "\n".join(lines)


class MergeTests(unittest.TestCase):
    def setUp(self):
        self.m = load_tool("drive_cut", CUT)

    def merged(self, live, disp):
        a, _ = self.m.parse(live + "\ndrive verdict=X features=0/0 unserved=0 head=abc\n")
        b, _ = self.m.parse(disp + "\ndrive verdict=X features=0/0 unserved=0 head=abc\n")
        return self.m.merge(a, b)

    def test_pass_in_one_run_is_pass(self):
        out = self.merged(feat("roster.update", "UNMEASURED", 4, 5, [("mutate", "UNMEASURED", "disposable serve")]),
                          feat("roster.update", "PASS", 5, 5))
        self.assertEqual(out["roster.update"][:3], ("PASS", 5, 5))

    def test_fail_in_either_run_wins(self):
        out = self.merged(feat("events.subscribe", "PASS", 10, 10),
                          feat("events.subscribe", "FAIL", 9, 10, [("replay", "FAIL", "frames=172")]))
        v, k, n, paths, _ = out["events.subscribe"]
        self.assertEqual((v, k, n), ("FAIL", 9, 10))
        self.assertIn("frames=172", paths["replay"][1])

    def test_unmeasured_only_when_both_runs_left_it_unmeasured(self):
        out = self.merged(feat("service.action", "UNMEASURED", 5, 10, [("stop", "UNMEASURED", "flag unset")]),
                          feat("service.action", "UNMEASURED", 9, 10, [("stop", "UNMEASURED", "bus absent")]))
        v, k, n, paths, _ = out["service.action"]
        self.assertEqual((v, k, n), ("UNMEASURED", 9, 10))
        self.assertEqual(paths["stop"], ("UNMEASURED", "flag unset | bus absent"))

    def test_a_feature_in_one_run_only_keeps_that_line(self):
        out = self.merged(feat("thread.get", "PASS", 2, 2, tail=" scope=v4.2 served=false"), "")
        self.assertEqual(out["thread.get"][0], "PASS")
        self.assertIn("served=false", out["thread.get"][4])


class IntegrationTests(unittest.TestCase):
    """drive-cut end to end, with a disposable serve playing the live unit."""

    @classmethod
    def setUpClass(cls):
        r = subprocess.run(["cargo", "build", "-p", "hee4-app", "--offline"], cwd=REPO, capture_output=True, text=True,
                           env={**os.environ, "CARGO_TARGET_DIR": TARGET})
        assert r.returncode == 0, r.stderr[-800:]

    def test_disposable_only_paths_come_back_measured(self):
        d = tempfile.mkdtemp(prefix="drive-cut-test-")
        self.addCleanup(shutil.rmtree, d, True)
        rt = os.path.join(d, "live", "rt"); os.makedirs(rt, mode=0o700)
        sock, ledger = os.path.join(rt, "control.sock"), os.path.join(d, "live", "ledger.sqlite3")
        env = {k: v for k, v in os.environ.items() if k != "HEE4_LIVE_MODEL"}
        srv = subprocess.Popen([BIN, "serve", "--socket", sock, "--ledger", ledger, "--work", os.path.join(d, "live", "w")],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
        self.addCleanup(lambda: (srv.kill(), srv.wait()))
        t0 = time.monotonic()
        while not os.path.exists(sock) and time.monotonic() - t0 < 20:
            time.sleep(0.1)
        live_ev = os.path.join(d, "drive-ev")
        rc, out, err = run(CUT, "--binary", BIN, "--socket", sock, "--ledger", ledger, "--live-args", "",
                           "--evidence-root", os.path.join(d, "ev"), "--live-evidence-root", live_ev,
                           timeout=900, env=env)
        # The live run's frames land where check-deployed D9 looks for drive task ids, so a task
        # the live drive submitted is excluded from 'use' (a check-deployed reader finds them).
        cd = load_tool("check_deployed", os.path.join(TOOLS, "check-deployed"))
        ids, files = cd.drive_task_ids(live_ev)
        self.assertGreater(files, 0, "no live drive evidence where D9 reads it")
        self.assertGreater(len(ids), 0, "D9 would see no drive task ids from the live run")
        self.assertIn(rc, (0, 3), out + err)  # 3 only for paths unmeasured in BOTH runs, named below
        self.assertRegex(out, r"drive settle-disposable serve=stopped")
        self.assertRegex(out.strip().splitlines()[-1], r"^drive verdict=(PASS|UNMEASURED) features=\d+/\d+ unserved=0 head=\S+ \(merged live\+disposable\)$")
        for name in ("crash-restart", "roster.update", "roster.disable"):
            line = next(l for l in out.splitlines() if l.startswith(f"drive feature={name} "))
            self.assertIn("verdict=PASS", line, out)
        self.assertNotIn("disposable serve (the live unit", out, "a disposable-only reason survived the merge")


if __name__ == "__main__":
    unittest.main()
