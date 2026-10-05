"""tools/tests/run against planted test directories: it names the failing test, keeps the full log
under $HOME/.cache/hee4-tools-tests (newest 20), and keeps its last-line contract."""
import os, tempfile, textwrap, unittest
from common import TOOLS, run

RUNNER = os.path.join(TOOLS, "tests", "run")

# a traceback longer than 4 lines: the old `tail -n 4` runner dropped the FAIL header
FAILING = textwrap.dedent("""\
    import unittest
    def deeper(): assert 1 == 2, "planted"
    def deep(): deeper()
    class Planted(unittest.TestCase):
        def test_planted_failure(self):
            deep()
""")
PASSING = "import unittest\nclass Planted(unittest.TestCase):\n    def test_planted_pass(self):\n        pass\n"


def plant(body=None):
    d = tempfile.mkdtemp(prefix="ttr-")
    if body is not None:
        with open(os.path.join(d, "test_planted.py"), "w") as f:
            f.write(body)
    return d


def runner(tests_dir, home):
    return run(RUNNER, env={"HEE4_TOOLS_TESTS_DIR": tests_dir, "HOME": home}, timeout=120)


class TestsRunner(unittest.TestCase):
    def setUp(self):
        self.home = tempfile.mkdtemp(prefix="ttr-home-")
        self.logdir = os.path.join(self.home, ".cache", "hee4-tools-tests")

    def test_failure_is_named_with_its_traceback_and_log(self):
        rc, out, _ = runner(plant(FAILING), self.home)
        lines = out.strip().splitlines()
        self.assertEqual(rc, 1, out)
        self.assertEqual(lines[-1], "tools-tests verdict=FAIL rc=1 ran=1")
        self.assertIn("FAIL: test_planted_failure", out)
        self.assertIn("AssertionError: planted", out)
        logs = [l[len("tools-tests log="):] for l in lines if l.startswith("tools-tests log=")]
        self.assertEqual(len(logs), 1, out)
        self.assertEqual(os.path.dirname(logs[0]), self.logdir)
        with open(logs[0]) as f:
            self.assertIn("FAIL: test_planted_failure", f.read())

    def test_pass_keeps_its_last_line_and_writes_a_log(self):
        rc, out, _ = runner(plant(PASSING), self.home)
        self.assertEqual(rc, 0, out)
        self.assertEqual(out.strip().splitlines()[-1], "tools-tests verdict=PASS ran=1")
        self.assertEqual(len(os.listdir(self.logdir)), 1)

    def test_zero_tests_are_refused(self):
        rc, out, _ = runner(plant(), self.home)
        self.assertEqual(rc, 1, out)
        self.assertRegex(out.strip().splitlines()[-1], r"^tools-tests verdict=FAIL rc=\d+ ran=(0|none)$")

    def test_newest_twenty_logs_are_kept(self):
        os.makedirs(self.logdir)
        planted = [f"20000101T0000{i:02d}.000000000Z-1.log" for i in range(22)]
        for name in planted:
            open(os.path.join(self.logdir, name), "w").close()
        rc, out, _ = runner(plant(PASSING), self.home)
        self.assertEqual(rc, 0, out)
        kept = sorted(os.listdir(self.logdir))
        self.assertEqual(len(kept), 20)
        self.assertEqual(kept[:-1], planted[-19:], "the oldest planted logs go first")
        self.assertNotIn(kept[-1], planted, "this run's own log is kept")


if __name__ == "__main__":
    unittest.main()
