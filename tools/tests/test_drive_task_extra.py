"""task.preview, task.resolve (E2E-09) and events.subscribe through tools/drive against a disposable serve.

Also the serve fixture the other drive-completion tests reuse (DisposableServe): a build under
CARGO_TARGET_DIR, `hee4 serve` in a mkdtemp with a 0700 rt dir, HEE4_LIVE_MODEL dropped.
"""
import json, os, re, shutil, subprocess, tempfile, time, unittest
from common import TOOLS, run
from test_drive import Proxy

DRIVE = os.path.join(TOOLS, "drive")
REPO = os.path.dirname(TOOLS)
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.expanduser("~/.cache/hee4-target-gate"))
BIN = os.path.join(TARGET, "debug", "hee4")


class DisposableServe(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        r = subprocess.run(["cargo", "build", "-p", "hee4-app", "--offline"], cwd=REPO, capture_output=True, text=True,
                           env={**os.environ, "CARGO_TARGET_DIR": TARGET})
        assert r.returncode == 0, r.stderr[-800:]
        cls.d = tempfile.mkdtemp(prefix="drvx-")
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

    @classmethod
    def tearDownClass(cls):
        cls.srv.kill(); cls.srv.wait(); shutil.rmtree(cls.d, ignore_errors=True)

    def drive(self, *a, sock=None):
        ev = tempfile.mkdtemp(prefix="ev-", dir=self.d)
        rc, out, err = run(DRIVE, "--socket", sock or self.sock, "--ledger", self.ledger, "--evidence-root", ev, *a, timeout=110)
        return rc, out, err, ev

    def line(self, out, feat):
        got = [l for l in out.splitlines() if l.startswith(f"drive feature={feat} ")]
        self.assertEqual(len(got), 1, out)
        return got[0]

    def rows(self, ev, feat):
        head = os.listdir(ev)[0]
        with open(os.path.join(ev, head, f"{feat}.jsonl")) as f:
            rows = [json.loads(l) for l in f]
        return list(zip([r for r in rows if r["dir"] == "send"], [r for r in rows if r["dir"] == "recv"]))


class TaskExtraDriveTests(DisposableServe):
    FEATS = ("task.submit", "task.preview", "task.resolve", "events.subscribe")

    def test_four_task_features_pass_and_the_vacuous_verify_frames_are_refused(self):
        rc, out, _, ev = self.drive("--only", ",".join(self.FEATS))
        for f in self.FEATS:
            self.assertIn("verdict=PASS ", self.line(out, f), out)
        self.assertEqual(rc, 0, out)
        self.assertIn("note=stale_generation unreachable reason=task.resolve PreconditionRule::None", out)
        self.assertRegex(out, r"(?m)^  elapsed_s=\d+\.\d$")
        sub = [json.loads(g["raw"]) for s, g in self.rows(ev, "task.submit") if "VERIFY: /usr/bin/true" in s["raw"]]
        self.assertTrue(sub and all(r["code"] == "invalid_argument" and "VERIFY" in r["message"] for r in sub), sub)
        pv = self.rows(ev, "task.preview")
        for verify in ("VERIFY: sh: true", "VERIFY: /usr/bin/true", "VERIFY: model: hi"):
            got = [json.loads(g["raw"])["body"] for s, g in pv if verify in str(json.loads(s["raw"])["body"].get("brief"))]
            self.assertTrue(got and all(b["eligible"] is False and b["refusal"] == "invalid_argument" and "VERIFY" in b["message"]
                                        for b in got), (verify, got))

    def test_fires_when_resync_required_is_planted_away(self):
        p = os.path.join(self.d, "rt", "px.sock")
        Proxy(p, self.sock, '"resync_required"', '"bad_resync"').start()
        rc, out, _, _ = self.drive("--only", "events.subscribe", sock=p)
        self.assertIn("verdict=FAIL", self.line(out, "events.subscribe"), out)
        self.assertIn("code=bad_resync want resync_required", out)
        self.assertEqual(rc, 1)


if __name__ == "__main__":
    unittest.main()
