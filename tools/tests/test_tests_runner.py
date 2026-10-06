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


def runner(tests_dir, home, *args, env=None):
    return run(RUNNER, *([tests_dir] if tests_dir else []), *args, env={"HOME": home, **(env or {})}, timeout=120)


# a failure whose message holds a rule, a `Ran N tests` summary and an OK line of its own (as nested
# runner output does): neither the block nor the ran= count may come from it
NESTED = textwrap.dedent("""\
    import unittest
    NESTED = "\\n".join(["before", "-" * 70, "Ran 7 tests in 0.001s", "", "OK", "middle-marker", "after"])
    class Planted(unittest.TestCase):
        def test_planted_failure(self):
            self.fail(NESTED)
""")

# passes only when the runner kept the caller's HEE4_HEAD and HEE4_GATE_* away from the suite
SCRUBBED = textwrap.dedent("""\
    import os, unittest
    class Planted(unittest.TestCase):
        def test_planted_env(self):
            leaked = sorted(k for k in os.environ if k == "HEE4_HEAD" or k.startswith("HEE4_GATE_"))
            self.assertEqual(leaked, [])
""")

# passes only inside a git checkout whose tools/tests holds it (the gate's export has no .git)
IN_CHECKOUT = textwrap.dedent("""\
    import os, subprocess, unittest
    ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    class Planted(unittest.TestCase):
        def test_planted_checkout(self):
            r = subprocess.run(["git", "-C", ROOT, "rev-parse", "--show-toplevel"], capture_output=True, text=True)
            self.assertEqual((r.returncode, r.stdout.strip()), (0, ROOT), r.stderr)
""")


def subject_repo():
    """A git repo whose first commit's tools/tests passes in a checkout and whose second fails."""
    d = tempfile.mkdtemp(prefix="ttr-repo-")
    os.makedirs(os.path.join(d, "tools", "tests"))
    g = lambda *a: run("git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", *a)[1].strip()
    g("init", "-q")
    shas = []
    for body in (IN_CHECKOUT, FAILING):
        with open(os.path.join(d, "tools", "tests", "test_planted.py"), "w") as f:
            f.write(body)
        g("add", "-A"); g("commit", "-qm", "planted"); shas.append(g("rev-parse", "HEAD"))
    return d, shas


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

    def test_a_ran_line_inside_a_message_is_not_the_summary(self):
        rc, out, _ = runner(plant(NESTED), self.home)
        self.assertEqual(rc, 1, out)
        self.assertEqual(out.strip().splitlines()[-1], "tools-tests verdict=FAIL rc=1 ran=1")
        self.assertIn("FAIL: test_planted_failure", out)
        self.assertIn("middle-marker", out, "the block runs past the rule and Ran line in its message")
        self.assertIn("after", out)

    def test_the_directory_is_an_argument_and_the_old_variable_is_ignored(self):
        rc, out, _ = runner(plant(PASSING), self.home, env={"HEE4_TOOLS_TESTS_DIR": plant(FAILING)})
        self.assertEqual((rc, out.strip().splitlines()[-1]), (0, "tools-tests verdict=PASS ran=1"), out)

    def test_the_suite_never_sees_the_gate_head_or_gate_variables(self):
        leak = {"HEE4_HEAD": "0" * 40, "HEE4_GATE_SUBJECT": "0" * 40, "HEE4_GATE_REPO": "/nonexistent"}
        rc, out, _ = runner(plant(SCRUBBED), self.home, env=leak)
        self.assertEqual((rc, out.strip().splitlines()[-1]), (0, "tools-tests verdict=PASS ran=1"), out)

    def test_checkout_runs_the_named_revision_in_a_git_clone_and_removes_it(self):
        repo, (passing, failing) = subject_repo()
        tmp = tempfile.mkdtemp(prefix="ttr-tmp-")
        rc, out, _ = runner(None, self.home, "--checkout", repo, passing, env={"TMPDIR": tmp})
        self.assertEqual((rc, out.strip().splitlines()[-1]), (0, "tools-tests verdict=PASS ran=1"), out)
        self.assertIn(f"tools-tests checkout={passing} repo={repo}", out)
        self.assertEqual(os.listdir(tmp), [], "the clone is removed on exit")
        rc, out, _ = runner(None, self.home, "--checkout", repo, failing, env={"TMPDIR": tmp})
        self.assertEqual((rc, out.strip().splitlines()[-1]), (1, "tools-tests verdict=FAIL rc=1 ran=1"), out)
        self.assertIn("FAIL: test_planted_failure", out)

    def test_checkout_of_an_unknown_revision_is_a_setup_failure(self):
        repo, _ = subject_repo()
        rc, out, err = runner(None, self.home, "--checkout", repo, "f" * 40)
        self.assertEqual((rc, out.strip().splitlines()[-1]), (2, "tools-tests verdict=FAIL rc=2 ran=none"), out + err)
        self.assertIn("cannot check out", err)


if __name__ == "__main__":
    unittest.main()
