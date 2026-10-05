"""task.preview, task.resolve (E2E-09) and events.subscribe through tools/drive against a disposable serve.

Also the serve fixture the other drive-completion tests reuse (DisposableServe): a build under
CARGO_TARGET_DIR, `hee4 serve` in a mkdtemp with a 0700 rt dir, HEE4_LIVE_MODEL dropped.
"""
import json, os, re, shutil, subprocess, tempfile, time, unittest
from common import TOOLS, run
from test_drive import Proxy, load_tool

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

    def frames(self, ev, feat):
        """Every send/recv frame of one feature's evidence, in order, parsed, with its direction as `_dir`."""
        head = os.listdir(ev)[0]
        with open(os.path.join(ev, head, f"{feat}.jsonl")) as f:
            rows = [json.loads(l) for l in f]
        return [json.loads(r["raw"]) | {"_dir": r["dir"]} for r in rows if r["dir"] in ("send", "recv")]


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

    def test_cancel_then_abandon_cancels_only_once_the_task_is_running(self):
        rc, out, _, ev = self.drive("--only", "task.submit,task.resolve,events.subscribe")
        self.assertIn("verdict=PASS paths=8/8 ", self.line(out, "task.resolve"), out)
        self.assertIn("verdict=PASS ", self.line(out, "events.subscribe"), out)
        self.assertNotIn("path=replay_resolve_producer", out)
        self.assertEqual(rc, 0, out)
        log = self.frames(ev, "task.resolve")
        cancel = next(i for i, r in enumerate(log) if r["_dir"] == "send" and r.get("action") == "task.cancel")
        t = log[cancel]["body"]["task_id"]
        seen = [r.get("body", {}).get("phase") for r in log[:cancel]
                if r["_dir"] == "recv" and r.get("kind") == "result" and r.get("body", {}).get("task_id") == t]
        self.assertEqual(seen[-1], "running", seen)
        self.assertEqual(log[cancel + 1].get("body", {}).get("phase"), "cancellation_requested", log[cancel + 1])

    def test_a_serve_that_never_dispatches_leaves_cancel_then_abandon_unmeasured_naming_the_bound(self):
        # Two plants in a chain: the task is never seen running, nor finishing its 5 s VERIFY.
        q, p = (os.path.join(self.d, "rt", n) for n in ("never-done.sock", "never-running.sock"))
        Proxy(q, self.sock, '"accepted"', '"admitted"').start()
        Proxy(p, q, '"running"', '"admitted"').start()
        rc, out, _, ev = self.drive("--only", "task.resolve", sock=p)
        self.assertIn("verdict=UNMEASURED ", self.line(out, "task.resolve"), out)
        self.assertRegex(out, r"(?m)^  path=cancel_then_abandon status=UNMEASURED detail=task=t-[0-9a-f]{24} "
                              r"never reached running within 10s \(last phase=admitted\)")
        self.assertEqual(rc, 3, out)
        sent = [r.get("action") for r in self.frames(ev, "task.resolve") if r["_dir"] == "send"]
        self.assertIn("task.get", sent)
        self.assertNotIn("task.cancel", sent, "a cancel was sent to a task never seen running")

    def test_fires_when_resync_required_is_planted_away(self):
        p = os.path.join(self.d, "rt", "px.sock")
        Proxy(p, self.sock, '"resync_required"', '"bad_resync"').start()
        rc, out, _, _ = self.drive("--only", "events.subscribe", sock=p)
        self.assertIn("verdict=FAIL", self.line(out, "events.subscribe"), out)
        self.assertIn("code=bad_resync want resync_required", out)
        self.assertEqual(rc, 1)


class RacedTests(unittest.TestCase):
    """raced() names why a lifecycle path could not run, from the task's phase after a conflict."""

    def raced(self, phase, code="conflict"):
        load_tool("drive_d", os.path.join(TOOLS, "drive.d", "__init__.py"))
        task = load_tool("drive_d.task", os.path.join(TOOLS, "drive.d", "task.py"))
        F = type("F", (), {"req": lambda self, a, b, key=None: {"kind": "result", "body": {"phase": phase}}})()
        return task.raced(F, "t-" + "1" * 24, {"kind": "error", "code": code})

    def test_a_conflict_on_an_already_cancelled_task_names_the_cancel_before_dispatch_race(self):
        self.assertIn("cancel-before-dispatch race", self.raced("cancelled"))

    def test_a_conflict_on_an_abandoned_task_names_no_eligible_model(self):
        self.assertIn("no eligible model", self.raced("abandoned"))

    def test_no_reason_for_a_live_task_or_a_non_conflict(self):
        self.assertIsNone(self.raced("running"))
        self.assertIsNone(self.raced("cancelled", code="not_found"))


class WaitRunningTests(unittest.TestCase):
    """wait_running() names the cause of a terminal phase at once, and the bound only when it ran out."""

    def wait(self, phase, bound=10):
        load_tool("drive_d", os.path.join(TOOLS, "drive.d", "__init__.py"))
        task = load_tool("drive_d.task", os.path.join(TOOLS, "drive.d", "task.py"))
        F = type("F", (), {"req": lambda self, a, b, key=None: {"kind": "result", "body": {"phase": phase}}})()
        t0 = time.monotonic()
        up, why = task.wait_running(F, "t-" + "1" * 24, bound=bound)
        return up, why, time.monotonic() - t0

    def test_an_abandoned_task_names_no_eligible_model_not_the_bound(self):
        up, why, dt = self.wait("abandoned")
        self.assertFalse(up)
        self.assertIn("no eligible model", why)
        self.assertNotIn("within 10s", why)
        self.assertLess(dt, 1.0, "a terminal phase must end the wait at once, not run out the bound")

    def test_a_cancelled_task_names_the_cancel_before_dispatch_race(self):
        up, why, _ = self.wait("cancelled")
        self.assertIn("cancel-before-dispatch race", why)
        self.assertNotIn("within", why)

    def test_another_terminal_phase_names_the_phase_not_the_bound(self):
        up, why, _ = self.wait("failed")
        self.assertIn("terminal phase=failed", why)
        self.assertNotIn("within", why)

    def test_only_an_exhausted_bound_names_the_bound(self):
        up, why, dt = self.wait("admitted", bound=0.2)
        self.assertFalse(up)
        self.assertIn("never reached running within 0.2s (last phase=admitted)", why)
        self.assertGreaterEqual(dt, 0.2)

    def test_running_is_up(self):
        self.assertEqual(self.wait("running")[:2], (True, None))


if __name__ == "__main__":
    unittest.main()
