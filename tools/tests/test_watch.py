import json, os, re, shutil, tempfile, unittest
from common import TOOLS, run

WATCH = os.path.join(TOOLS, "watch")
ROSTER = os.path.join(os.path.dirname(TOOLS), ".claude", "agents", "ROSTER.md")
HEAD = "abcdef012345" + "0" * 28


def roster_watchers():
    """The `hee4-watch-<name>` rows of ROSTER.md's `## Watchers` table: the one denominator."""
    with open(ROSTER) as f:
        text = f.read()
    sec = text.split("\n## Watchers", 1)[1].split("\n## ", 1)[0]
    return [m.group(1) for m in re.finditer(r"^\| `hee4-watch-([a-z]+)`", sec, re.M)]


def write(path, body, mode=0o755):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(body)
    os.chmod(path, mode)


class World:
    """A fake repo of stub detectors, gate logs, drill root and PATH stubs for hee4 and fm-db,
    every one green until a test plants one fault."""

    def __init__(self):
        self.d = tempfile.mkdtemp(prefix="ww-")
        self.repo, self.bin = os.path.join(self.d, "repo"), os.path.join(self.d, "bin")
        self.gate, self.drill, self.ev = (os.path.join(self.d, x) for x in ("gate", "drill", "drive"))
        os.makedirs(self.bin); os.makedirs(self.ev)
        self.tool("layers", 'echo "apparatus_ratio=0.50 bound=1.5 largest_file=a lines=1 tree=x verdict=PASS"')
        self.tool("features-check", 'echo "features-check verdict=PASS files=1 drifted=0"')
        self.tool("push-scan", 'echo "push-scan range=github/main..HEAD commits=2 files=1 hits=0 verdict=PASS"')
        self.check("cite_pins.py", "pins keys_cited=2 fresh=2/2 stale=0 unpinned=0 snapshot_mismatch=0 unmapped_cites=0")
        self.check("module_funnel.py", "funnel verdict=PASS checks_failed=0 diagnostics=0")
        self.summary([{"name": "test", "rc": 0, "ok": True, "flags": []}])
        self.rehearsal("1/1")
        self.tasks(["accepted", "running"])
        self.fmdb([{"unit_id": "U-1", "planned_agents": 3, "spawned": 2, "open_andon": 0}])

    def tool(self, name, body, rc=0):
        write(os.path.join(self.repo, "tools", name), f"#!/usr/bin/env bash\n{body}\nexit {rc}\n")

    def check(self, name, line, rc=0):
        write(os.path.join(self.repo, "ops", "checks", name), f"import sys\nprint({line!r})\nsys.exit({rc})\n", 0o644)

    def summary(self, steps, subject=HEAD, tier="cut", stamp="20261005T000000Z", mtime=None):
        path = os.path.join(self.gate, f"{stamp}-{subject[:12]}", "summary.json")
        write(path, json.dumps({"subject": subject, "tier": tier, "steps": steps}), 0o644)
        if mtime is not None:
            os.utime(path, (mtime, mtime))

    def rehearsal(self, acked, submitted=None):
        if submitted is None:
            submitted = int(acked.split("/")[1])
        write(os.path.join(self.drill, HEAD[:12], "rehearsal.json"),
              json.dumps({"tree": HEAD[:12], "submitted": submitted, "acked_present": acked,
                          "steps": [{"name": "kill9", "status": "MEASURED"}]}), 0o644)

    def tasks(self, phases):
        body = json.dumps({"body": {"tasks": [{"phase": p, "task_id": f"t-{i}"} for i, p in enumerate(phases)]}})
        write(os.path.join(self.bin, "hee4"), f"#!/usr/bin/env bash\necho '{body}'\n")

    def fmdb(self, units):
        body = json.dumps({"exit": 0, "open_units": units, "verb": "status"})
        write(os.path.join(self.bin, "fm-db"), f"#!/usr/bin/env bash\necho '{body}'\n")

    def run(self):
        rc, out, err = run(WATCH, "--repo", self.repo, "--gate-logs", self.gate, "--drill-root", self.drill,
                           "--evidence-root", self.ev,
                           env={"HEE4_HEAD": HEAD, "PATH": self.bin + ":" + os.environ["PATH"]})
        lines = out.strip().splitlines()
        rows = {l.split()[0][len("watch-"):]: l for l in lines if l.startswith("watch-")}
        return rc, rows, lines[-1] if lines else "", out + err


def verdict(line):
    return line.split(" verdict=", 1)[1].split()[0]


class TestWatch(unittest.TestCase):
    def setUp(self):
        self.names = roster_watchers()

    def assert_only(self, w, failing, want="FAIL", rc_want=1):
        rc, rows, last, out = w.run()
        self.assertEqual(sorted(rows), sorted(self.names), out)
        for name, line in rows.items():
            self.assertIn("agent=UNMEASURED(agent)", line)
            self.assertEqual(verdict(line), want if name == failing else "PASS", line)
        self.assertTrue(last.startswith(f"watch verdict={want} "), last)
        self.assertEqual(rc, rc_want, out)
        return rows

    def test_all_green_is_pass_over_the_roster_watchers(self):
        rc, rows, last, out = World().run()
        n = len(self.names)
        self.assertEqual(sorted(rows), sorted(self.names), out)
        self.assertIn(f"watch verdict=PASS watchers={n}/{n} unmeasured=0 head={HEAD[:12]}", last)
        self.assertEqual(rc, 0, out)

    def test_layers_exit_1_fails_drift(self):
        w = World(); w.tool("layers", 'echo "apparatus_ratio=2.00 bound=1.5 largest_file=a lines=1 tree=x verdict=FAIL"', rc=1)
        self.assertIn("apparatus_ratio=2.00", self.assert_only(w, "drift")["drift"])

    def test_cite_pins_fail_fails_contradiction_by_name(self):
        w = World(); w.check("cite_pins.py", "cite_pins verdict=FAIL")
        self.assertIn("cite_pins=FAIL", self.assert_only(w, "contradiction")["contradiction"])

    def test_cite_pins_stale_pin_line_fails_contradiction(self):
        w = World(); w.check("cite_pins.py", "pins keys_cited=2 fresh=1/2 stale=1 unpinned=0 snapshot_mismatch=0 unmapped_cites=0")
        self.assert_only(w, "contradiction")

    def test_gate_step_that_looked_at_nothing_fails_evidence(self):
        w = World(); w.summary([{"name": "test", "rc": 0, "ok": True, "flags": ["looked_at_nothing"]}])
        self.assertIn("not_ok=test", self.assert_only(w, "evidence")["evidence"])

    def test_newer_foreign_subject_pass_never_hides_head_fail(self):
        w = World(); w.summary([{"name": "test", "rc": 1, "ok": False, "flags": []}], mtime=1_000_000)
        w.summary([{"name": "test", "rc": 0, "ok": True, "flags": []}], subject="a2036d948d97" + "0" * 28,
                  tier="t", stamp="20261006T000000Z", mtime=2_000_000)
        row = self.assert_only(w, "evidence")["evidence"]
        self.assertIn(f"subject={HEAD[:12]} not_ok=test", row)

    def test_only_a_foreign_subject_summary_is_unmeasured(self):
        w = World(); shutil.rmtree(w.gate)
        w.summary([{"name": "test", "rc": 0, "ok": True, "flags": []}], subject="a2036d948d97" + "0" * 28, tier="cut")
        row = self.assert_only(w, "evidence", "UNMEASURED", 3)["evidence"]
        self.assertIn(f"UNMEASURED(no gate summary at {HEAD[:12]})", row)

    def test_rehearsal_that_submitted_nothing_is_unmeasured_never_pass(self):
        w = World(); w.rehearsal("0/0", submitted=0)
        row = self.assert_only(w, "recovery", "UNMEASURED", 3)["recovery"]
        self.assertIn("UNMEASURED(rehearsal submitted=0; run tools/drill --submit N)", row)

    def test_missing_acked_task_fails_recovery(self):
        w = World(); w.rehearsal("2/3")
        self.assertIn("acked_present=2/3", self.assert_only(w, "recovery")["recovery"])

    def test_effect_unknown_task_fails_recovery(self):
        w = World(); w.tasks(["effect_unknown", "accepted"])
        self.assertIn("effect_unknown=1", self.assert_only(w, "recovery")["recovery"])

    def test_push_scan_hit_fails_fence(self):
        w = World(); w.tool("push-scan", 'echo "push-scan range=a..b commits=2 files=1 hits=1 verdict=FAIL"', rc=1)
        self.assertIn("hits=1", self.assert_only(w, "fence")["fence"])

    def test_empty_range_is_unmeasured_never_pass(self):
        w = World(); w.tool("push-scan", 'echo "push-scan range=a..b commits=0 files=0 hits=UNMEASURED(empty range) verdict=PASS"', rc=3)
        self.assert_only(w, "fence", want="UNMEASURED", rc_want=3)

    def test_spawn_over_plan_fails_budget(self):
        w = World(); w.fmdb([{"unit_id": "U-1", "planned_agents": 2, "spawned": 3, "open_andon": 0}])
        self.assertIn("over=U-1", self.assert_only(w, "budget")["budget"])

    def test_absent_detector_is_unmeasured_naming_it(self):
        w = World(); os.remove(os.path.join(w.repo, "tools", "layers"))
        rows = self.assert_only(w, "drift", want="UNMEASURED", rc_want=3)
        self.assertIn("watch-drift verdict=UNMEASURED detail=tools/layers absent", rows["drift"])

    def test_no_rehearsal_and_no_gate_summary_are_unmeasured(self):
        w = World(); os.remove(os.path.join(w.drill, HEAD[:12], "rehearsal.json"))
        self.assertIn(f"no rehearsal at {HEAD[:12]}", self.assert_only(w, "recovery", "UNMEASURED", 3)["recovery"])

    def test_watch_writes_nothing(self):
        w = World()
        before = sorted(os.path.join(d, f) for d, _, fs in os.walk(w.d) for f in fs)
        w.run()
        self.assertEqual(before, sorted(os.path.join(d, f) for d, _, fs in os.walk(w.d) for f in fs))


if __name__ == "__main__":
    unittest.main()
