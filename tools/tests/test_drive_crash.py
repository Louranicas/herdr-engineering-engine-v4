"""crash-restart's drive leg (tools/drive.d/crash.py) and the no-silent-UNMEASURED rule (drive_d.reasoned,
drive_d.feature_reason, drive_d.unexplained): every feature line that cannot run says why."""
import os, re, shutil, subprocess, tempfile, textwrap, time, unittest
from common import TOOLS, run
from test_drive import BIN, MARK, REPO, TARGET, load_tool, plant_repo

DRIVE = os.path.join(TOOLS, "drive")


class CrashDriveTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        r = subprocess.run(["cargo", "build", "-p", "hee4-app", "--offline"], cwd=REPO, capture_output=True, text=True,
                           env={**os.environ, "CARGO_TARGET_DIR": TARGET})
        assert r.returncode == 0, r.stderr[-800:]
        cls.d = tempfile.mkdtemp(prefix="drvc-", dir=os.path.expanduser("~/.cache"))
        os.mkdir(os.path.join(cls.d, "rt"), 0o700)
        cls.sock = os.path.join(cls.d, "rt", "control.sock")
        cls.ledger = os.path.join(cls.d, "l.sqlite3")
        cls.srv = subprocess.Popen([BIN, "serve", "--socket", cls.sock, "--ledger", cls.ledger, "--work", os.path.join(cls.d, "w")],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                   env={k: v for k, v in os.environ.items() if k != "HEE4_LIVE_MODEL"})
        for _ in range(100):
            if os.path.exists(cls.sock):
                break
            time.sleep(0.1)
        cls.ev = os.path.join(cls.d, "ev")
        cls.drive_d = load_tool("drive_d", os.path.join(TOOLS, "drive.d", "__init__.py"))

    @classmethod
    def tearDownClass(cls):
        cls.srv.kill(); cls.srv.wait(); shutil.rmtree(cls.d, ignore_errors=True)

    def drive(self, *a, repo=None):
        return run(DRIVE, "--socket", self.sock, "--evidence-root", self.ev, *(["--repo", repo] if repo else []), *a, timeout=300)

    def test_crash_restart_passes_on_a_disposable_serve(self):
        rc, out, _ = self.drive("--only", "crash-restart")
        line = [l for l in out.splitlines() if l.startswith("drive feature=crash-restart ")][0]
        m = re.search(r" verdict=(\w+) paths=(\d+)/(\d+) ", line)
        self.assertEqual(m.group(1), "PASS", out)
        self.assertGreaterEqual(int(m.group(3)), 3, line)
        self.assertEqual(m.group(2), m.group(3), line)
        self.assertEqual(rc, 0, out)
        self.assertTrue(self.srv.poll() is None, "the drive's own serve was touched")

    def test_no_unmeasured_feature_line_without_a_reason_on_a_disposable_serve(self):
        rc, out, _ = self.drive("--ledger", self.ledger)
        lines = [l for l in out.splitlines() if l.startswith("drive feature=")]
        self.assertGreater(len(lines), 10, out)
        silent = [l for l in self.drive_d.unexplained(lines) if " verdict=UNMEASURED" in l]
        self.assertEqual(silent, [], out)

    def test_unexplained_fires_on_a_planted_unmeasured_line_with_no_reason(self):
        planted = "drive feature=zz verdict=UNMEASURED paths=0/1 evidence=/e"
        reasoned = planted + " reason=--ledger not passed"
        self.assertEqual(self.drive_d.unexplained([planted, reasoned, "drive feature=u verdict=UNMEASURED paths=0/0 evidence=/e scope=unserved"]),
                         [planted])
        self.assertEqual(self.drive_d.feature_reason([("a", "UNMEASURED", "x  y"), ("b", "UNMEASURED", "x y"), ("c", "PASS", "")]), " reason=x y")
        self.assertEqual(self.drive_d.feature_reason([("c", "PASS", "")]), "")

    def test_a_plugin_path_unmeasured_with_no_reason_is_a_fail_not_a_silent_line(self):
        plugin = '''
            def d_zz(F, ctx):
                F.check("silent", False, "  ", unmeasured=True)

            def d_none(F, ctx):
                pass

            FEATURES = [("zz.silent", d_zz), ("zz.none", d_none)]
        '''
        body = f"## Driving it with hee4\n\nx {MARK}\n"
        r = plant_repo(plugins={"zz_silent": textwrap.dedent(plugin)}, features={"zz.silent": body, "zz.none": body})
        rc, out, _ = self.drive("--only", "zz.silent,zz.none", repo=r)
        shutil.rmtree(r)
        lines = [l for l in out.splitlines() if l.startswith("drive feature=zz.")]
        self.assertEqual(len(lines), 2, out)
        self.assertTrue(all(" verdict=FAIL " in l and "reason=" not in l for l in lines), out)
        self.assertIn("path=silent status=FAIL detail=UNMEASURED path gave no reason", out)
        self.assertIn("path=driver status=FAIL detail=procedure ran no path", out)
        self.assertEqual(rc, 1)

    def wait_for(self, fn, budget=60):
        end = time.monotonic() + budget
        while time.monotonic() < end:
            v = fn()
            if v:
                return v
            time.sleep(0.2)
        return fn()

    def test_a_sigkilled_drive_leaves_no_disposable_serve_behind(self):
        crash = load_tool("drive_crash", os.path.join(TOOLS, "drive.d", "crash.py"))
        drv = subprocess.Popen([DRIVE, "--socket", self.sock, "--evidence-root", self.ev, "--only", "crash-restart"],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

        def mine():
            for n in (os.listdir(crash.ROOT) if os.path.isdir(crash.ROOT) else []):
                run_dir = os.path.join(crash.ROOT, n)
                try:
                    with open(os.path.join(run_dir, "owner")) as fh:
                        if int(fh.read().split()[0]) == drv.pid and crash.serves_of(run_dir):
                            return run_dir
                except (OSError, ValueError, IndexError):
                    pass
        run_dir = self.wait_for(mine)
        try:
            self.assertTrue(run_dir, "the drive never started its disposable serve")
            pids = crash.serves_of(run_dir)
            drv.kill(); drv.wait()
            gone = self.wait_for(lambda: not crash.serves_of(run_dir), budget=10)
            self.assertTrue(gone, f"serve {pids} of run {os.path.basename(run_dir)} outlived its SIGKILLed drive")
        finally:
            if drv.poll() is None:
                drv.kill(); drv.wait()
            if run_dir:
                for pid in crash.serves_of(run_dir):  # a failed run must not leave its orphan behind
                    os.kill(pid, 9)
                shutil.rmtree(run_dir, ignore_errors=True)
        self.assertTrue(self.srv.poll() is None, "the drive's own serve was touched")

    def test_reap_stale_kills_an_orphan_serve_whose_drive_is_gone_and_spares_a_live_one(self):
        crash = load_tool("drive_crash", os.path.join(TOOLS, "drive.d", "crash.py"))
        root = tempfile.mkdtemp(prefix="reap-", dir=os.path.expanduser("~/.cache"))
        dead = subprocess.Popen(["/usr/bin/true"]); dead.wait()
        runs, srvs = {}, []
        for name, owner in (("aaaaaaaaaaaa", f"{dead.pid} 0"), ("bbbbbbbbbbbb", f"{os.getpid()} {crash.proc_start(os.getpid())}")):
            r = runs[name] = os.path.join(root, name)
            os.makedirs(os.path.join(r, "rt"), 0o700)
            with open(os.path.join(r, "owner"), "w") as fh:
                fh.write(owner + "\n")
            srvs.append(subprocess.Popen([BIN, "serve", "--socket", os.path.join(r, "rt", "control.sock"), "--ledger",
                                          os.path.join(r, "ledger.sqlite3"), "--work", os.path.join(r, "work")],
                                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                         env={k: v for k, v in os.environ.items() if k != "HEE4_LIVE_MODEL"}))
        try:
            self.assertTrue(self.wait_for(lambda: all(crash.serves_of(r) for r in runs.values()), budget=10))
            self.assertEqual(crash.reap_stale(root), ["aaaaaaaaaaaa"])
            self.assertEqual(srvs[0].wait(timeout=10), -9)
            self.assertFalse(os.path.exists(runs["aaaaaaaaaaaa"]))
            self.assertIsNone(srvs[1].poll(), "a run whose drive is alive was reaped")
            self.assertTrue(os.path.isdir(runs["bbbbbbbbbbbb"]))
        finally:
            for p in srvs:
                if p.poll() is None:
                    p.kill(); p.wait()
            shutil.rmtree(root, ignore_errors=True)

    def test_on_the_live_socket_every_crash_path_is_unmeasured_disposable_serve(self):
        drive = load_tool("drive_tool", DRIVE)
        served = self.drive_d.load_plugins(os.path.join(TOOLS, "drive.d"), os.path.join(REPO, "gates/features"))
        xdg = os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}")
        F = drive.Feature("crash-restart", os.path.join(self.d, "live.jsonl"), f"{xdg}/hee4/control.sock", {})
        served["crash-restart"](F, {})
        F.f.close()
        self.assertGreaterEqual(len(F.paths), 3)
        self.assertTrue(all(st == "UNMEASURED" and d.startswith("disposable serve") for _, st, d in F.paths), F.paths)
        self.assertIn(" reason=disposable serve", self.drive_d.feature_reason(F.paths))
        with open(os.path.join(self.d, "live.jsonl")) as f:
            self.assertEqual(f.read(), "", "a frame went to the live socket")


if __name__ == "__main__":
    unittest.main()
