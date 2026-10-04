import os, unittest
from common import TOOLS, run, make_repo

GATE = os.path.join(TOOLS, "gate")

def toml(*steps, tiers=None):
    body = '[gate]\nbase = "HEAD~1"\n[tier.t]\nsteps = [%s]\n' % ",".join(f'"{s[0]}"' for s in steps)
    for name, cmd, budget, expect in steps:
        body += f'[step.{name}]\ncmd = {cmd!r}\nbudget_s = {budget}\nexpect = "{expect}"\n'
    return body

def gate(repo, tier="t", *extra):
    return run(GATE, tier, "--repo", repo, *extra, cwd=repo)

class GateTests(unittest.TestCase):
    def test_help(self):
        rc, out, _ = run(GATE, "--help")
        self.assertEqual(rc, 0); self.assertIn("verdict=PASS|FAIL", out)

    def test_quiet_all_green(self):
        d, sha = make_repo(toml(("a", "true", 5, "none"), ("b", "echo 'test result: ok. 3 passed'", 5, "tests_ran")))
        rc, out, _ = gate(d)
        self.assertEqual(rc, 0, out)
        last = out.strip().splitlines()[-1]
        self.assertEqual(last, f"gate tier=t verdict=PASS steps=2/2 subject={sha[:12]}")
        self.assertIn("step=a rc=0 elapsed=", out); self.assertIn("/5s margin=", out)

    def test_fire_planted_failing_step(self):
        d, sha = make_repo(toml(("good", "true", 5, "none"), ("planted", "exit 9", 5, "none")))
        rc, out, _ = gate(d)
        self.assertEqual(rc, 1)
        self.assertIn("step=planted rc=9", out)
        self.assertEqual(out.strip().splitlines()[-1], f"gate tier=t verdict=FAIL steps=1/2 subject={sha[:12]}")

    def test_looked_at_nothing_fails(self):
        d, _ = make_repo(toml(("test", "echo 'test result: ok. 0 passed'", 5, "tests_ran")))
        rc, out, _ = gate(d)
        self.assertEqual(rc, 1); self.assertIn("looked_at_nothing", out); self.assertIn("verdict=FAIL", out)

    def test_budget_overrun_is_killed_and_red(self):
        d, _ = make_repo(toml(("slow", "sleep 20", 1, "none")))
        rc, out, _ = gate(d)
        self.assertEqual(rc, 1); self.assertIn("rc=124", out); self.assertIn("budget_exceeded", out)

    def test_subject_is_an_export_not_the_worktree(self):
        d, _ = make_repo(toml(("untracked_absent", "test ! -e scratch.untracked", 5, "none")))
        open(os.path.join(d, "scratch.untracked"), "w").close()
        rc, out, _ = gate(d)
        self.assertEqual(rc, 0, out)

    def test_ddf_seals_input_sha256(self):
        d, _ = make_repo(toml(("ddf", "deep-diff-forge --stdin-patch --rank --json --require-files --require-hunks", 30, "ddf_sealed")))
        rc, out, _ = gate(d)
        self.assertEqual(rc, 0, out); self.assertRegex(out, r"input_sha256=[0-9a-f]{64}")

    def test_ddf_rc7_is_fail_with_reason(self):
        d, _ = make_repo(toml(("ddf", "deep-diff-forge --stdin-patch --rank --json --require-files --require-hunks", 30, "ddf_sealed")))
        rc, out, _ = gate(d, "t", "--base", "HEAD")
        self.assertEqual(rc, 1); self.assertIn("rc=7", out); self.assertIn("refused: 0 files", out)

    def test_step_without_budget_is_refused(self):
        d, _ = make_repo('[tier.t]\nsteps=["a"]\n[step.a]\ncmd="true"\n')
        rc, _, err = gate(d)
        self.assertEqual(rc, 2); self.assertIn("no budget_s", err)

    def test_repo_gate_toml_declares_budgets_and_no_count_literal(self):
        import tomllib
        with open(os.path.join(os.path.dirname(TOOLS), "gate.toml"), "rb") as f:
            cfg = tomllib.load(f)
        self.assertEqual(set(cfg["tier"]), {"commit", "stack", "cut"})
        for name, st in cfg["step"].items():
            self.assertIn("budget_s", st, name)
            self.assertNotRegex(st.get("expect", ""), r"\d")

if __name__ == "__main__":
    unittest.main()
