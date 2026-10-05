"""tools/drive.d/roster.py against a disposable serve: the four roster features drive with no FAIL.

Each test class starts its own serve (the disable path retires the serve's only model record, so
a serve is never shared with another family's tests). With --ledger every ledger-side path is
measured; without it each one is UNMEASURED naming --ledger. The only other non-PASS path allowed
is the operator-capability `forbidden` path, UNMEASURED by name (no grant exists; grants slice).
"""
import json, os, re, shutil, sqlite3, subprocess, sys, tempfile, time, unittest
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # `python3 -m unittest tools/tests/test_drive_roster.py` from the root
from common import TOOLS, run

DRIVE = os.path.join(TOOLS, "drive")
REPO = os.path.dirname(TOOLS)
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.expanduser("~/.cache/hee4-target-gate"))
BIN = os.path.join(TARGET, "debug", "hee4")
ROSTER = ["roster.list", "roster.inspect", "roster.update", "roster.disable"]
ALLOWED_UNMEASURED = {"forbidden_operator_capability"}


class Serve:
    def __init__(self):
        self.d = tempfile.mkdtemp(prefix="drv-roster-")
        os.mkdir(os.path.join(self.d, "rt"), 0o700)
        self.sock = os.path.join(self.d, "rt", "control.sock")
        self.ledger = os.path.join(self.d, "l.sqlite3")
        self.log = open(os.path.join(self.d, "serve.log"), "w")
        self.srv = subprocess.Popen([BIN, "serve", "--socket", self.sock, "--ledger", self.ledger,
                                     "--work", os.path.join(self.d, "w")], stdout=self.log, stderr=self.log,
                                    env={k: v for k, v in os.environ.items() if k != "HEE4_LIVE_MODEL"})
        for _ in range(100):
            if os.path.exists(self.sock):
                break
            time.sleep(0.1)

    def drive(self, ledger, ev="ev"):
        args = [DRIVE, "--socket", self.sock, "--evidence-root", os.path.join(self.d, ev), "--only", ",".join(ROSTER)]
        return run(*(args + (["--ledger", self.ledger] if ledger else [])), timeout=100)

    def stop(self):
        self.srv.kill(); self.srv.wait(); self.log.close()

    def serve_log(self):
        return open(os.path.join(self.d, "serve.log")).read()


def build():
    r = subprocess.run(["cargo", "build", "-p", "hee4-app", "--offline"], cwd=REPO, capture_output=True, text=True,
                       env={**os.environ, "CARGO_TARGET_DIR": TARGET})
    assert r.returncode == 0, r.stderr[-800:]


def feature_lines(out):
    """feature -> verdict for the served (non scope=unserved) feature lines."""
    return {m.group(1): m.group(2) for m in re.finditer(r"^drive feature=(\S+) verdict=(\S+) (?!.*scope=unserved).*$", out, re.M)}


def fresh_writes(ev_root):
    """action -> the mutating frames the drive sent that the serve answered with a non-replayed result."""
    counts = {}
    for dirpath, _, files in os.walk(ev_root):
        for fn in files:
            sent = None
            for line in open(os.path.join(dirpath, fn)):
                rec = json.loads(line)
                if rec.get("dir") == "send":
                    sent = json.loads(rec["raw"])
                elif rec.get("dir") == "recv" and sent and rec.get("raw"):
                    reply = json.loads(rec["raw"])
                    if sent["action"] in ("roster.update", "roster.disable") and reply.get("kind") == "result" \
                            and reply.get("replayed") is False:
                        counts[sent["action"]] = counts.get(sent["action"], 0) + 1
                    sent = None
    return counts


def odd_paths(out):
    """(path, status, detail) for every non-PASS path the drive printed."""
    return re.findall(r"^  path=(\S+) status=(\S+) detail=(.*)$", out, re.M)


class RosterWithLedger(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()
        cls.s = Serve()
        cls.rc, cls.out, cls.err = cls.s.drive(ledger=True)
        cls.s.stop()

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.s.d, ignore_errors=True)

    def test_four_features_ran_and_none_failed(self):
        self.assertEqual(sorted(feature_lines(self.out)), sorted(ROSTER), self.out)
        self.assertNotIn("FAIL", feature_lines(self.out).values(), self.out)
        self.assertEqual(feature_lines(self.out)["roster.list"], "PASS", self.out)
        self.assertEqual(feature_lines(self.out)["roster.inspect"], "PASS", self.out)

    def test_only_the_named_grants_path_is_unmeasured(self):
        odd = odd_paths(self.out)
        self.assertTrue(odd, "the forbidden path must be printed UNMEASURED, never dropped: " + self.out)
        for path, status, detail in odd:
            self.assertEqual(status, "UNMEASURED", self.out)
            self.assertIn(path, ALLOWED_UNMEASURED, self.out)
            self.assertIn("grants slice", detail)

    def test_ledger_counts_name_one_deploy_install_and_only_driven_roster_rows(self):
        c = sqlite3.connect("file:" + self.s.ledger + "?mode=ro", uri=True)
        try:
            by_action = dict(c.execute("select action, count(*) from operations group by action").fetchall())
            deploy = c.execute("select principal from operations where action = 'deploy.install'").fetchall()
            revisions = c.execute("select count(*) from roster_revisions").fetchone()[0]
        finally:
            c.close()
        self.assertEqual(deploy, [("deploy",)], "one deploy.install row, under principal deploy")
        fresh = fresh_writes(os.path.join(self.s.d, "ev"))
        for action in ("roster.update", "roster.disable"):
            self.assertEqual(by_action.get(action, 0), fresh.get(action, 0), (action, by_action, fresh))
        self.assertGreater(fresh.get("roster.update", 0), 0, fresh)
        self.assertEqual(revisions, len(deploy) + sum(fresh.values()), (revisions, fresh))

    def test_serve_log_names_the_deploy_record_and_the_refused_route(self):
        log = self.s.serve_log()
        self.assertRegex(log, r"roster deploy record=model:\S+ replayed=false operation=op-[0-9a-f]{24}")
        self.assertRegex(log, r"dispatch task=\S+.*abandon reason=RouteRefused")


class RosterWithoutLedger(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()
        cls.s = Serve()
        cls.rc, cls.out, cls.err = cls.s.drive(ledger=False)
        cls.s.stop()

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.s.d, ignore_errors=True)

    def test_ledger_paths_are_unmeasured_naming_the_flag_and_nothing_fails(self):
        self.assertNotIn("FAIL", feature_lines(self.out).values(), self.out)
        odd = odd_paths(self.out)
        ledger_side = [p for p, st, d in odd if st == "UNMEASURED" and "--ledger" in d]
        self.assertGreaterEqual(len(ledger_side), 1, self.out)
        for path, status, detail in odd:
            self.assertEqual(status, "UNMEASURED", self.out)
            self.assertTrue("--ledger" in detail or path in ALLOWED_UNMEASURED, self.out)


class RosterDrivenTwice(unittest.TestCase):
    """A second drive on the same serve (tools/tests/test_drive.py shares one serve across runs)."""

    @classmethod
    def setUpClass(cls):
        build()
        cls.s = Serve()
        cls.first = cls.s.drive(ledger=True, ev="ev1")[1]
        cls.second = cls.s.drive(ledger=True, ev="ev2")[1]
        cls.s.stop()

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.s.d, ignore_errors=True)

    def test_second_drive_has_no_fail_and_still_refuses_the_retired_model(self):
        for out in (self.first, self.second):
            self.assertEqual(sorted(feature_lines(out)), sorted(ROSTER), out)
            self.assertNotIn("FAIL", feature_lines(out).values(), out)
        self.assertEqual(feature_lines(self.second)["roster.inspect"], "PASS", self.second)


if __name__ == "__main__":
    unittest.main()
