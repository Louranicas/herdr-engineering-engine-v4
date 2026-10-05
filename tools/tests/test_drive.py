import importlib.machinery, importlib.util, json, os, re, shutil, socket, sqlite3, subprocess, sys, tempfile, textwrap, threading, time, unittest
from common import TOOLS, run

DRIVE = os.path.join(TOOLS, "drive")
REPO = os.path.dirname(TOOLS)
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.expanduser("~/.cache/hee4-target-gate"))
BIN = os.path.join(TARGET, "debug", "hee4")
MARK = "(rev 2026-10-05 drive)"


def load_tool(name, path):
    """Import an extensionless tools/ script (or tools/drive.d/__init__.py as 'drive_d') as a module."""
    kw = {"submodule_search_locations": [os.path.dirname(path)]} if name == "drive_d" else {}
    spec = importlib.util.spec_from_file_location(name, path, loader=importlib.machinery.SourceFileLoader(name, path), **kw)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


def write(path, body):
    with open(path, "w") as f:
        f.write(body)


def plant_repo(plugins=None, features=None):
    """A temp repo: the real gates/features, FLOW.md and tools/drive.d, plus planted plugins and feature files."""
    r = tempfile.mkdtemp(prefix="drv-repo-")
    shutil.copytree(os.path.join(REPO, "gates/features"), os.path.join(r, "gates/features"))
    shutil.copytree(os.path.join(REPO, "tools/drive.d"), os.path.join(r, "tools/drive.d"))
    os.makedirs(os.path.join(r, "crates/hee4-app")); shutil.copy(os.path.join(REPO, "crates/hee4-app/FLOW.md"), os.path.join(r, "crates/hee4-app/"))
    for name, body in (plugins or {}).items():
        write(os.path.join(r, "tools/drive.d", name + ".py"), textwrap.dedent(body))
    for name, body in (features or {}).items():
        write(os.path.join(r, "gates/features", name + ".md"), body)
    return r


class Proxy(threading.Thread):
    """Forwards each frame to the real socket and renames one refusal in the reply: the plant."""
    def __init__(self, path, upstream, old, new):
        super().__init__(daemon=True)
        self.s = socket.socket(socket.AF_UNIX); self.s.bind(path); self.s.listen(8)
        self.up, self.old, self.new = upstream, old.encode(), new.encode()
    def run(self):
        while True:
            try:
                c, _ = self.s.accept()
            except OSError:
                return
            try:
                line = c.makefile("rb").readline()
                u = socket.socket(socket.AF_UNIX); u.connect(self.up); u.sendall(line)
                reply = u.makefile("rb").readline(); u.close()
                c.sendall(reply.replace(self.old, self.new))
            except OSError:
                pass
            c.close()


class DriveTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        r = subprocess.run(["cargo", "build", "-p", "hee4-app", "--offline"], cwd=REPO, capture_output=True, text=True,
                           env={**os.environ, "CARGO_TARGET_DIR": TARGET})
        assert r.returncode == 0, r.stderr[-800:]
        cls.d = tempfile.mkdtemp(prefix="drv-")
        os.mkdir(os.path.join(cls.d, "rt"), 0o700)
        cls.sock = os.path.join(cls.d, "rt", "control.sock")
        cls.ledger = os.path.join(cls.d, "l.sqlite3")
        cls.srv = subprocess.Popen([BIN, "serve", "--socket", cls.sock, "--ledger", cls.ledger,
                                    "--work", os.path.join(cls.d, "w")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                   env={k: v for k, v in os.environ.items() if k != "HEE4_LIVE_MODEL"})
        for _ in range(100):
            if os.path.exists(cls.sock):
                break
            time.sleep(0.1)
        cls.ev = os.path.join(cls.d, "ev")

    @classmethod
    def tearDownClass(cls):
        cls.srv.kill(); cls.srv.wait(); shutil.rmtree(cls.d, ignore_errors=True)

    def drive(self, *a, sock=None, repo=None):
        return run(DRIVE, "--socket", sock or self.sock, "--evidence-root", self.ev, *( ["--repo", repo] if repo else []), *a, timeout=100)

    def lines(self, out, feat):
        return [l for l in out.splitlines() if l.startswith(f"drive feature={feat} ")]

    def test_quiet_on_a_healthy_unit(self):
        rc, out, err = self.drive()
        for f in ("health", "task.submit", "task.get", "task.cancel"):
            self.assertRegex(self.lines(out, f)[0], r"verdict=(PASS|UNMEASURED) ", out)
        self.assertIn("verdict=UNMEASURED paths=2/3", self.lines(out, "task.list")[0])
        self.assertIn("--allow-restart not passed", out)
        self.assertNotIn("verdict=FAIL", out)
        m = re.fullmatch(r"drive verdict=UNMEASURED features=(\d+)/(\d+) unserved=\d+ head=\w+", out.splitlines()[-1])
        self.assertIsNotNone(m, out.splitlines()[-1])
        k, n = int(m.group(1)), int(m.group(2))
        self.assertTrue(1 <= n and k <= n, (k, n))
        self.assertEqual(n, len([l for l in out.splitlines() if l.startswith("drive feature=") and "scope=unserved" not in l]))
        self.assertEqual(rc, 3, err)

    def test_fires_on_a_planted_wrong_refusal_name(self):
        p = os.path.join(self.d, "rt", "proxy.sock")
        Proxy(p, self.sock, '"invalid_argument"', '"bad_argument"').start()
        rc, out, _ = self.drive(sock=p)
        self.assertIn("verdict=FAIL", self.lines(out, "health")[0], out)
        self.assertIn("code=bad_argument want invalid_argument", out)
        self.assertEqual(rc, 1)
        self.assertRegex(out.splitlines()[-1], r"^drive verdict=FAIL ")

    def test_evidence_holds_every_frame_sent_and_received(self):
        self.drive()
        head = os.listdir(self.ev)[0]
        with open(os.path.join(self.ev, head, "task.submit.jsonl")) as f:
            rows = [json.loads(l) for l in f]
        sent = [r for r in rows if r["dir"] == "send"]; got = [r for r in rows if r["dir"] == "recv"]
        self.assertEqual((len(sent), len(got)), (7, 7))
        self.assertTrue(all(json.loads(r["raw"])["kind"] in ("result", "error") for r in got))
        self.assertEqual(json.loads(sent[0]["raw"])["action"], "task.submit")
        self.assertEqual(list(json.loads(sent[0]["raw"])), ["request_id", "action", "action_version", "idempotency_key", "body"])

    def test_doctor_first_refuses_a_missing_unit_or_socket(self):
        rc, out, _ = self.drive("--doctor-first", "--unit", "hee4-nonexistent.service")
        self.assertEqual(rc, 3)
        self.assertIn("check=unit status=UNMEASURED", out)
        self.assertIn("(doctor refused)", out.splitlines()[-1])
        rc, out, _ = self.drive("--doctor-first", sock=os.path.join(self.d, "rt", "absent.sock"))
        self.assertEqual(rc, 3)
        self.assertTrue(any(l.startswith("check=socket status=UNMEASURED") for l in out.splitlines()) or "check=unit status=" in out, out)

    def test_unwritten_procedure_is_unmeasured_with_its_reason(self):
        r = plant_repo()
        p = os.path.join(r, "gates/features/task.get.md")
        with open(p) as f:
            text = f.read()
        write(p, text.replace(MARK, "(rev none)"))
        rc, out, _ = self.drive(repo=r)
        line = self.lines(out, "task.get")[0]
        self.assertIn("verdict=UNMEASURED paths=0/0", line)
        self.assertIn("procedure UNWRITTEN", line)
        shutil.rmtree(r)

    def test_plugin_naming_a_feature_without_a_file_is_a_fail_naming_the_module(self):
        r = plant_repo(plugins={"zz_bogus": 'FEATURES = [("no.such.feature", lambda F, c: None)]\n'})
        rc, out, _ = self.drive(repo=r)
        last = out.splitlines()[-1]
        self.assertEqual(rc, 1, out)
        self.assertRegex(last, r"^drive verdict=FAIL features=0/0 unserved=0 head=\w+ reason=plugin zz_bogus: ")
        self.assertIn("no.such.feature", last)
        shutil.rmtree(r)

    def test_only_runs_the_named_features_and_refuses_unknown(self):
        rc, out, _ = self.drive("--only", "health,task.submit")
        feats = [l for l in out.splitlines() if l.startswith("drive feature=")]
        served = [l for l in feats if "scope=unserved" not in l]
        self.assertEqual([l.split()[1] for l in served], ["feature=health", "feature=task.submit"], out)
        self.assertTrue(all("verdict=PASS" in l for l in served), out)
        self.assertTrue(all("reason=no procedure in tools/drive.d" in l for l in feats if "scope=unserved" in l), out)
        self.assertRegex(out.splitlines()[-1], r"^drive verdict=PASS features=2/2 unserved=[1-9]\d* head=\w+$")
        self.assertEqual(rc, 0)
        rc, out, err = self.drive("--only", "nosuch")
        self.assertEqual(rc, 2)
        self.assertIn("--only nosuch is not a plugin feature", err)
        self.assertIn("health", err)

    def test_refuse_because_and_extra_and_req_precondition(self):
        drive = load_tool("drive_tool", DRIVE)
        evp = os.path.join(self.d, "unit.jsonl")
        F = drive.Feature("x", evp, os.path.join(self.d, "no.sock"), {"invalid_argument": "never"})
        reply = {"kind": "error", "code": "invalid_argument", "retry": "never", "field": "/", "message": "m", "because": "scope v4.1"}
        F.refuse("x", reply, "invalid_argument", because="v4.1", extra={"current_generation": 3})
        F.refuse("y", reply, "invalid_argument", because="v4.2")
        F.refuse("z", dict(reply, current_generation=3), "invalid_argument", because="v4.1", extra={"current_generation": 3})
        F.refuse("w", reply, "invalid_argument")
        st = {p[0]: (p[1], p[2]) for p in F.paths}
        self.assertEqual(st["x"][0], "FAIL"); self.assertIn("current_generation missing", st["x"][1])
        self.assertEqual(st["y"][0], "FAIL"); self.assertIn("because lacks 'v4.2'", st["y"][1])
        self.assertEqual(st["z"][0], "PASS")
        self.assertEqual(st["w"][0], "PASS")
        F.req("task.cancel", {"task_id": "t"}, key="k")
        F.req("roster.update", {}, key="k", precondition={"resource": "roster", "id": "r", "generation": 1})
        F.f.close()
        with open(evp) as f:
            sent = [json.loads(json.loads(l)["raw"]) for l in f if json.loads(l)["dir"] == "send"]
        self.assertEqual(list(sent[0]), ["request_id", "action", "action_version", "idempotency_key", "body"])
        self.assertEqual(sent[1]["precondition"], {"resource": "roster", "id": "r", "generation": 1})
        self.assertEqual(list(sent[1]), ["request_id", "action", "action_version", "idempotency_key", "precondition", "body"])

    def test_ledger_is_read_only_and_unmeasured_without_the_flag(self):
        plugin = '''
            import sqlite3
            from drive_d import NO_LEDGER_REASON, ledger_ro

            def d_zz(F, ctx):
                c = ledger_ro(ctx)
                if c is None:
                    return F.check("ledger_side_effect", False, NO_LEDGER_REASON, unmeasured=True)
                n = c.execute("select count(*) from sqlite_master").fetchone()[0]
                try:
                    c.execute("create table zz_probe(x)")
                    F.check("ledger_read_only", False, "a write succeeded on the ledger")
                except sqlite3.OperationalError as e:
                    F.check("ledger_read_only", n >= 1, f"read {n} tables; write refused: {e}")

            FEATURES = [("zz.ledger", d_zz)]
        '''
        r = plant_repo(plugins={"zz_ledger": plugin}, features={"zz.ledger": f"## Driving it with hee4\n\nx {MARK}\n"})
        rc, out, _ = self.drive("--only", "zz.ledger", repo=r)
        self.assertEqual(rc, 3, out)
        self.assertIn("path=ledger_side_effect status=UNMEASURED detail=--ledger not passed", out)
        self.assertRegex(out.splitlines()[-1], r"^drive verdict=UNMEASURED features=0/1 ")
        rc, out, _ = self.drive("--only", "zz.ledger", "--ledger", self.ledger, repo=r)
        self.assertEqual(rc, 0, out)
        self.assertIn("drive feature=zz.ledger verdict=PASS paths=1/1", out)
        self.assertRegex(out.splitlines()[-1], r"^drive verdict=PASS features=1/1 ")
        with sqlite3.connect(f"file:{self.ledger}?mode=ro", uri=True) as c:
            self.assertIsNone(c.execute("select name from sqlite_master where name='zz_probe'").fetchone())
        shutil.rmtree(r)

    def test_load_plugins_refuses_a_duplicate_feature_name(self):
        drive_d = load_tool("drive_d", os.path.join(TOOLS, "drive.d", "__init__.py"))
        r = plant_repo(plugins={"zz_dup": 'FEATURES = [("health", lambda F, c: None), ("health", lambda F, c: None)]\n'})
        with self.assertRaises(drive_d.PluginFault) as cm:
            drive_d.load_plugins(os.path.join(r, "tools/drive.d"), os.path.join(r, "gates/features"))
        self.assertEqual((cm.exception.module, cm.exception.reason), ("zz_dup", "duplicate feature health"))
        os.remove(os.path.join(r, "tools/drive.d/zz_dup.py"))
        write(os.path.join(r, "tools/drive.d/zz_again.py"), 'FEATURES = [("health", lambda F, c: None)]\n')
        with self.assertRaises(drive_d.PluginFault) as cm:  # across modules: task.py already serves health
            drive_d.load_plugins(os.path.join(r, "tools/drive.d"), os.path.join(r, "gates/features"))
        self.assertEqual((cm.exception.module, cm.exception.reason), ("zz_again", "duplicate feature health"))
        os.remove(os.path.join(r, "tools/drive.d/zz_again.py"))
        write(os.path.join(r, "tools/drive.d/zz_shape.py"), 'FEATURES = {"health": None}\n')
        with self.assertRaises(drive_d.PluginFault) as cm:
            drive_d.load_plugins(os.path.join(r, "tools/drive.d"), os.path.join(r, "gates/features"))
        self.assertEqual(cm.exception.module, "zz_shape"); self.assertIn("FEATURES must be", cm.exception.reason)
        with self.assertRaises(drive_d.PluginFault) as cm:
            drive_d.load_plugins(os.path.join(r, "tools/absent.d"), os.path.join(r, "gates/features"))
        self.assertEqual(cm.exception.module, "drive.d")
        shutil.rmtree(r)


if __name__ == "__main__":
    unittest.main()
