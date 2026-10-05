import json, os, re, shutil, subprocess, tempfile, unittest
from common import TOOLS, run

WATCH = os.path.join(TOOLS, "watch")
JUSTFILE = os.path.join(os.path.dirname(TOOLS), "justfile")
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


# ---- `just cut-check` and `just tag` (the justfile's cut recipes; tools/watch is one of their doors) ----

STUBS = {
    "gate": 'echo "gate tier=$1 verdict=PASS steps=1/1 subject=$(git rev-parse --short=12 HEAD)"; [ -z "${CW_KILL:-}" ] || kill -9 $PPID',
    "check-deployed": """if [ "${1:-}" = --control ]; then echo 'check-deployed control cases=9/9 verdict=PASS'; exit 0; fi
echo "${CW_D8:-D8 flows=12 drive_rc=0 l2=5 reasoned=19 unexplained=0 PASS}"
dep=${CW_DEPLOYED:-9/9}; v=PASS; [ "${dep%/*}" = "${dep#*/}" ] || v=FAIL
echo "deployed=$dep tree=$(git rev-parse --short=12 HEAD) dirty=$(git status --porcelain | wc -l) verdict=$v"; [ $v = PASS ]""",
    "cold-clone": 'echo "cold-clone sha=${2:0:12} origin=$PWD/origin.git steps=1 matched=1 verdict=PASS"',
    "push-scan": 'echo "push-scan range=github/main..HEAD commits=1 files=1 hits=${CW_HITS:-0} verdict=PASS"',
    "layers": 'echo "apparatus_ratio=0.5 bound=1.5 largest_file=a lines=1 tree=$(git rev-parse --short=12 HEAD) verdict=PASS"',
    "watch": 'echo "watch verdict=PASS watchers=6/6 unmeasured=0 head=$(git rev-parse --short=12 HEAD)"',
}
SIX = ["scoreboard flows=12 l2=5 reasoned=19 unexplained=0", "deployed=9/9", "cold-clone steps=1 matched=1",
       "push-scan hits=0", "apparatus_ratio=0.5"]


class CutWorld:
    """A git repo with a bare `origin`, stub doors printing canned last lines, and a PATH `hee4` whose
    --version head is the repo's HEAD (the deployed state cut-check admits)."""

    def __init__(self):
        self.d = tempfile.mkdtemp(prefix="cw-")
        self.cut, self.bin = os.path.join(self.d, "cut"), os.path.join(self.d, "bin")
        for name, body in STUBS.items():
            write(os.path.join(self.d, "tools", name), f"#!/usr/bin/env bash\n{body}\n")
        write(os.path.join(self.d, ".gitignore"), "cut/\nbin/\norigin.git/\n", 0o644)
        self.git("init", "-q", "-b", "main"); self.git("add", "-A"); self.git("commit", "-qm", "world")
        subprocess.run(["git", "init", "-q", "--bare", os.path.join(self.d, "origin.git")], check=True)
        self.git("remote", "add", "origin", os.path.join(self.d, "origin.git"))
        self.deploy()

    def git(self, *a):
        return subprocess.run(["git", "-C", self.d, "-c", "user.name=t", "-c", "user.email=t@t", *a],
                              check=True, capture_output=True, text=True).stdout.strip()

    def head12(self):
        return self.git("rev-parse", "--short=12", "HEAD")

    def deploy(self, head12=None):
        write(os.path.join(self.bin, "hee4"), f'#!/usr/bin/env bash\necho "hee4 x {head12 or self.head12()}"\n')

    def just(self, *args, **env):
        e = {"HEE4_CUT_ROOT": self.cut, "PATH": self.bin + ":" + os.environ["PATH"], **env}
        rc, out, err = run("just", "--justfile", JUSTFILE, "--working-directory", self.d, *args, env=e, timeout=120)
        lines = out.strip().splitlines()
        return rc, lines[-1] if lines else "", out + err

    def record(self):
        with open(os.path.join(self.cut, self.head12(), "cut-check.json")) as f:
            return json.load(f)


@unittest.skipUnless(shutil.which("just"), "just absent")
class TestCutRecipes(unittest.TestCase):
    def setUp(self):
        self.w = CutWorld()

    def green(self):
        rc, last, out = self.w.just("cut-check")
        self.assertEqual(rc, 0, out)
        self.assertTrue(last.startswith("cut-check verdict=PASS sha="), last)
        return last

    def refused(self, rc_want, reason, *args, **env):
        rc, last, out = self.w.just(*args, **env)
        self.assertEqual(rc, rc_want, out)
        self.assertIn(f"verdict=REFUSED reason={reason} ", last + " ", out)
        self.assertEqual(self.w.git("tag", "-l"), "", out)
        return last

    def test_green_cut_then_tag_lays_the_six_lines_locally_only(self):
        last = self.green()
        self.assertIn("deployed=9/9 cold=steps=1 matched=1 push=hits=0 apparatus_ratio=0.5", last)
        self.refused(2, "no_confirm", "tag", "v4.0.0")
        rc, last, out = self.w.just("tag", "v4.0.0", "confirm")
        self.assertEqual(rc, 0, out)
        self.assertIn("tag verdict=PASS name=v4.0.0", last)
        body = self.w.git("cat-file", "-p", "v4.0.0").split("\n\n", 1)[1].splitlines()
        sha = self.w.git("rev-parse", "HEAD")
        self.assertEqual(body, SIX + [f"tree={sha} dirty=0"])
        self.assertEqual(self.w.git("ls-remote", "--tags", "origin"), "")

    def test_cut_check_refuses_an_undeployed_head_before_any_door(self):
        self.w.deploy("0123456789ab")
        last = self.refused(2, "binary_head_mismatch", "cut-check")
        self.assertIn(f"binary=0123456789ab head={self.w.head12()}", last)
        self.assertFalse(os.path.exists(self.w.cut))

    def test_deployed_8_of_9_is_bad_field(self):
        rc, last, out = self.w.just("cut-check", CW_DEPLOYED="8/9")
        self.assertEqual(rc, 1, out)
        self.assertRegex(last, r"^cut-check verdict=FAIL .*bad_field=deployed")
        self.refused(3, "cut_check_failed", "tag", "v4.0.0", "confirm")

    def test_scoreboard_without_unexplained_is_missing_field(self):
        rc, last, out = self.w.just("cut-check", CW_D8="D8 flows=12 drive_rc=0 l2=5 reasoned=19 PASS")
        self.assertEqual(rc, 1, out)
        self.assertTrue(last.startswith("cut-check verdict=FAIL missing_field=scoreboard "), last)

    def test_push_scan_hit_is_bad_field(self):
        rc, last, out = self.w.just("cut-check", CW_HITS="1")
        self.assertEqual(rc, 1, out)
        self.assertRegex(last, r"^cut-check verdict=FAIL .*bad_field=push_scan")

    def test_tag_without_a_record_at_head(self):
        self.refused(3, "no_cut_check_at_sha", "tag", "v4.0.0", "confirm")

    def test_tag_after_a_new_commit_has_no_record_at_head(self):
        self.green()
        self.w.git("commit", "-q", "--allow-empty", "-m", "two"); self.w.deploy()
        self.refused(3, "no_cut_check_at_sha", "tag", "v4.0.0", "confirm")

    def test_tag_refuses_an_untracked_file(self):
        self.green()
        write(os.path.join(self.w.d, "stray.txt"), "x", 0o644)
        self.refused(3, "dirty", "tag", "v4.0.0", "confirm")

    def test_tag_refuses_an_existing_tag(self):
        self.green()
        self.w.git("tag", "v4.0.0")
        rc, last, out = self.w.just("tag", "v4.0.0", "confirm")
        self.assertEqual(rc, 3, out)
        self.assertIn("verdict=REFUSED reason=tag_exists ", last)

    def test_tag_refuses_when_the_installed_binary_moved_off_head(self):
        self.green()
        self.w.deploy("0123456789ab")
        self.refused(3, "binary_head_mismatch", "tag", "v4.0.0", "confirm")

    def test_a_killed_rerun_leaves_no_pass_record_to_tag_from(self):
        self.green()
        self.w.just("cut-check", CW_KILL="1")
        self.assertEqual(self.w.record()["verdict"], "RUNNING")
        self.refused(3, "cut_check_incomplete", "tag", "v4.0.0", "confirm")

    def test_a_failed_rerun_replaces_the_earlier_pass(self):
        self.green()
        self.w.just("cut-check", CW_DEPLOYED="8/9")
        self.assertEqual(self.w.record()["verdict"], "FAIL")
        self.refused(3, "cut_check_failed", "tag", "v4.0.0", "confirm")

    def test_a_record_older_than_the_newest_run_is_stale(self):
        self.green()
        os.makedirs(os.path.join(self.w.cut, f"99991231T235959Z-{self.w.head12()}"))
        self.refused(3, "stale_cut_check", "tag", "v4.0.0", "confirm")


if __name__ == "__main__":
    unittest.main()
