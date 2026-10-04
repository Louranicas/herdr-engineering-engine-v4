import json, os, shutil, socket, subprocess, tempfile, threading, time, unittest
from common import TOOLS, run

DRIVE = os.path.join(TOOLS, "drive")
REPO = os.path.dirname(TOOLS)
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.expanduser("~/.cache/hee4-target-gate"))
BIN = os.path.join(TARGET, "debug", "hee4")


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
        cls.srv = subprocess.Popen([BIN, "serve", "--socket", cls.sock, "--ledger", os.path.join(cls.d, "l.sqlite3"),
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
            self.assertIn("verdict=PASS", self.lines(out, f)[0], out)
        self.assertIn("verdict=UNMEASURED paths=2/3", self.lines(out, "task.list")[0])
        self.assertIn("--allow-restart not passed", out)
        self.assertNotIn("verdict=FAIL", out)
        self.assertRegex(out.splitlines()[-1], r"^drive verdict=UNMEASURED features=4/5 unserved=\d+ head=\w+$")
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
        rows = [json.loads(l) for l in open(os.path.join(self.ev, head, "task.submit.jsonl"))]
        sent = [r for r in rows if r["dir"] == "send"]; got = [r for r in rows if r["dir"] == "recv"]
        self.assertEqual((len(sent), len(got)), (7, 7))
        self.assertTrue(all(json.loads(r["raw"])["kind"] in ("result", "error") for r in got))
        self.assertEqual(json.loads(sent[0]["raw"])["action"], "task.submit")

    def test_doctor_first_refuses_a_missing_unit_or_socket(self):
        rc, out, _ = self.drive("--doctor-first", "--unit", "hee4-nonexistent.service")
        self.assertEqual(rc, 3)
        self.assertIn("check=unit status=UNMEASURED", out)
        self.assertIn("(doctor refused)", out.splitlines()[-1])
        rc, out, _ = self.drive("--doctor-first", sock=os.path.join(self.d, "rt", "absent.sock"))
        self.assertEqual(rc, 3)
        self.assertTrue(any(l.startswith("check=socket status=UNMEASURED") for l in out.splitlines()) or "check=unit status=" in out, out)

    def test_unwritten_procedure_is_unmeasured_with_its_reason(self):
        r = tempfile.mkdtemp(prefix="drv-repo-")
        shutil.copytree(os.path.join(REPO, "gates/features"), os.path.join(r, "gates/features"))
        os.makedirs(os.path.join(r, "crates/hee4-app")); shutil.copy(os.path.join(REPO, "crates/hee4-app/FLOW.md"), os.path.join(r, "crates/hee4-app/"))
        p = os.path.join(r, "gates/features/task.get.md")
        open(p, "w").write(open(p).read().replace("(rev 2026-10-05 drive)", "(rev none)"))
        rc, out, _ = self.drive(repo=r)
        line = self.lines(out, "task.get")[0]
        self.assertIn("verdict=UNMEASURED paths=0/0", line)
        self.assertIn("procedure UNWRITTEN", line)
        shutil.rmtree(r)


if __name__ == "__main__":
    unittest.main()
