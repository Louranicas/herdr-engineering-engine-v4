"""The full sweep against a disposable serve: under gate.toml's drive budget, no FAIL, journeys covered by name."""
import os, re, time, tomllib, unittest
from test_drive_task_extra import REPO, DisposableServe


class JourneysDriveTests(DisposableServe):
    def test_full_sweep_is_under_budget_and_journeys_are_named(self):
        t0 = time.monotonic()
        rc, out, _, _ = self.drive()
        elapsed = time.monotonic() - t0
        print(f"drive_elapsed_s={elapsed:.1f}")
        with open(os.path.join(REPO, "gate.toml"), "rb") as f:
            budget = tomllib.load(f)["step"]["drive"]["budget_s"]
        self.assertLess(elapsed, budget, out)
        self.assertNotIn("verdict=FAIL", out)
        self.assertIn(rc, (0, 3), out)
        self.assertIn("  journey=E2E-11 covered_by=judge.inspect.md", out.splitlines())
        self.assertIn("  journey=E2E-09 covered_by=task.resolve.md", out.splitlines())
        self.assertIsNone(re.search(r"path=\S*E2E-(0[1-9]|11)", out), out)
        self.assertRegex(out, r"(?m)^  journey=E2E-10 elapsed_s=\d+\.\d$")
        self.assertRegex(out, r"(?m)^  journey=E2E-12 elapsed_s=\d+\.\d$")
        line = self.line(out, "multi-surface-journeys")
        if "verdict=PASS " not in line:
            self.assertIn("verdict=UNMEASURED ", line)
            block = out.split(line, 1)[1].split("\ndrive feature=", 1)[0]
            bad = [l for l in block.splitlines() if l.startswith("  path=") and "user bus" not in l]
            self.assertEqual(bad, [], out)
        for f in ("task.preview", "task.resolve", "events.subscribe", "tools.list", "tools.inspect", "judge.inspect", "multi-surface-journeys"):
            l = self.line(out, f)
            self.assertNotIn("scope=unserved", l)
            self.assertNotIn("procedure UNWRITTEN", l)


if __name__ == "__main__":
    unittest.main()
