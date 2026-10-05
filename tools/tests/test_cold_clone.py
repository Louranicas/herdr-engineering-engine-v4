import calendar, glob, os, signal, subprocess, sys, tempfile, time, unittest
from importlib.machinery import SourceFileLoader
from common import TOOLS, run, make_repo

GATE, COLD = os.path.join(TOOLS, "gate"), os.path.join(TOOLS, "cold-clone")

def toml(cmd):
    return f'[gate]\nbase = "HEAD~1"\n[tier.t]\nsteps = ["s"]\n[step.s]\ncmd = {cmd!r}\nbudget_s = 5\nexpect = "none"\n'

# tier `big` includes `t` (as cut includes stack includes commit); tier `other` holds the same step but not `t`
NESTED = toml("true") + '[tier.big]\nincludes = "t"\nsteps = ["u"]\n[tier.other]\nsteps = ["s"]\n' \
    + '[step.u]\ncmd = "true"\nbudget_s = 5\nexpect = "none"\n'

class ColdWorld:
    """A throwaway repo, its bare mirror, and a private HOME so every cache lands under the test."""
    def __init__(self, cmd="true", gate_toml=None):
        self.repo, self.sha = make_repo(gate_toml or toml(cmd))
        self.bare = tempfile.mkdtemp(prefix="gt-bare-") + "/origin.git"
        subprocess.run(["git", "clone", "-q", "--bare", self.repo, self.bare], check=True, capture_output=True)
        self.home = tempfile.mkdtemp(prefix="gt-home-")
        self.env = {"HOME": self.home}
    def warm(self, tier="t"):
        return run(GATE, tier, "--repo", self.repo, env=self.env, cwd=self.repo)
    def cold(self, *extra):
        return run(COLD, "--sha", self.sha, "--origin", self.bare, "--tier", "t", "--repo", self.repo, *extra, env=self.env, cwd=self.repo)
    def cache_entries(self):
        return sorted(os.listdir(os.path.join(self.home, ".cache")))

class ColdCloneTests(unittest.TestCase):
    def test_help(self):
        rc, out, _ = run(COLD, "--help")
        self.assertEqual(rc, 0); self.assertIn("matched=N verdict=PASS|FAIL|UNMEASURED", out)

    def test_quiet_matched_against_warm_run(self):
        w = ColdWorld()
        rc, out, _ = w.warm()
        self.assertEqual(rc, 0, out)
        rc, out, err = w.cold()
        self.assertEqual(rc, 0, out + err)
        self.assertEqual(out.strip().splitlines()[-1], f"cold-clone sha={w.sha[:12]} origin={w.bare} tier=t steps=1 matched=1 verdict=PASS")
        self.assertIn(f"gate={GATE}\n", out); self.assertIn("registry=host_cache\n", out)
        self.assertIn("step=s rc=0", out)  # the cold gate's lines are relayed
        leftovers = [e for e in w.cache_entries() if e.startswith("hee4-cold-") and e != "hee4-cold-target"]
        self.assertEqual(leftovers, [], "clone temp dir not removed")
        self.assertTrue(os.path.isdir(os.path.join(w.home, ".cache", "hee4-cold-target", w.sha[:12])), "target dir kept")

    def test_target_dir_flag_is_kept(self):
        w = ColdWorld()
        w.warm()
        target = os.path.join(w.home, "kept-target")
        rc, out, _ = w.cold("--target-dir", target)
        self.assertEqual(rc, 0, out); self.assertIn(f"target_dir={target}\n", out); self.assertTrue(os.path.isdir(target))

    def test_fire_planted_failing_step_is_fail(self):
        w = ColdWorld("exit 9")
        rc, _, _ = w.warm()
        self.assertEqual(rc, 1)
        rc, out, _ = w.cold()
        self.assertEqual(rc, 1, out)
        self.assertRegex(out.strip().splitlines()[-1], r"^cold-clone .* steps=1 matched=1 verdict=FAIL$")

    def test_fire_no_warm_run_is_unmeasured(self):
        w = ColdWorld()
        rc, out, _ = w.cold()
        self.assertEqual(rc, 3, out)
        last = out.strip().splitlines()[-1]
        self.assertIn(f"matched=UNMEASURED(no warm run at {w.sha[:12]})", last); self.assertIn("verdict=UNMEASURED", last)
        self.assertNotIn("matched=0", out)
        leftovers = [e for e in w.cache_entries() if e.startswith("hee4-cold-") and e != "hee4-cold-target"]
        self.assertEqual(leftovers, [])

    def test_fire_second_cold_run_is_still_unmeasured(self):
        # a cold run's summary lands under the same root the warm reference is read from; it must never count as warm
        w = ColdWorld()
        for _ in range(2):
            rc, out, _ = w.cold()
            self.assertEqual(rc, 3, out)
            self.assertIn("matched=UNMEASURED(", out.strip().splitlines()[-1]); self.assertNotIn("matched=0", out)
        rc, out, _ = w.warm()
        self.assertEqual(rc, 0, out)
        warm_summary = [l for l in out.splitlines() if l.startswith("summary=")][0][len("summary="):]
        for _ in range(2):  # with a warm run present, every cold run matches against it, not the previous cold run
            rc, out, _ = w.cold()
            self.assertEqual(rc, 0, out)
            self.assertIn(f"warm={warm_summary}\n", out)
            self.assertNotIn(f"cold={warm_summary}\n", out)

    def test_warm_run_of_an_including_tier_is_the_reference(self):
        # cut-check's only warm run at the sha is `gate cut`; its commit-tier steps are the reference
        w = ColdWorld(gate_toml=NESTED)
        rc, out, _ = w.warm("big")
        self.assertEqual(rc, 0, out)
        warm_summary = [l for l in out.splitlines() if l.startswith("summary=")][0][len("summary="):]
        rc, out, err = w.cold()
        self.assertEqual(rc, 0, out + err)
        self.assertIn(f"warm={warm_summary}\n", out)
        self.assertEqual(out.strip().splitlines()[-1], f"cold-clone sha={w.sha[:12]} origin={w.bare} tier=t steps=1 matched=1 verdict=PASS")

    def test_fire_warm_run_of_a_tier_not_including_it_is_unmeasured(self):
        w = ColdWorld(gate_toml=NESTED)
        rc, out, _ = w.warm("other")
        self.assertEqual(rc, 0, out)
        rc, out, _ = w.cold()
        self.assertEqual(rc, 3, out)
        self.assertIn(f"matched=UNMEASURED(no warm run at {w.sha[:12]})", out.strip().splitlines()[-1])

    def test_fire_cold_clone_killed_after_the_cold_summary_leaves_it_untrusted(self):
        # the cold gate has written its summary.json, but cold-clone is killed (kill -9) before it can
        # mark it cold: a later cold run at the sha must not read that summary back as the warm reference
        w = ColdWorld()
        done = os.path.join(w.home, "gate-done")
        stall = os.path.join(w.home, "stall-gate")
        with open(stall, "w") as f:  # runs the real gate, hides its lines, then stalls so the kill lands now
            f.write(f'#!/bin/sh\n{GATE} "$@" > {w.home}/gate.out 2>&1\ntouch {done}\nexec sleep 60\n')
        os.chmod(stall, 0o755)
        p = subprocess.Popen([COLD, "--sha", w.sha, "--origin", w.bare, "--tier", "t", "--repo", w.repo, "--gate", stall],
                             env=dict(os.environ, **w.env), cwd=w.repo, stdout=subprocess.DEVNULL,
                             stderr=subprocess.DEVNULL, start_new_session=True)
        deadline = time.time() + 30
        while not os.path.exists(done) and time.time() < deadline:
            time.sleep(0.05)
        self.assertTrue(os.path.exists(done), "the stalled gate never finished")
        os.killpg(p.pid, signal.SIGKILL)
        p.wait()
        orphan = glob.glob(os.path.join(w.home, ".cache", "hee4-gate", f"*-{w.sha[:12]}", "summary.json"))
        self.assertEqual(len(orphan), 1)
        self.assertFalse(os.path.exists(os.path.join(os.path.dirname(orphan[0]), "cold-clone")), "the crash left no marker")
        rc, out, _ = w.cold()
        self.assertEqual(rc, 3, out)
        self.assertIn(f"matched=UNMEASURED(no warm run at {w.sha[:12]})", out.strip().splitlines()[-1])
        self.assertNotIn(f"warm={orphan[0]}", out)

    def test_killed_run_intent_window_spares_a_later_warm_run(self):
        cc = SourceFileLoader("cold_clone", COLD).load_module()
        root, sha = tempfile.mkdtemp(prefix="gt-warm-"), "ab" * 20
        t0 = calendar.timegm(time.strptime("20261005T120000Z", "%Y%m%dT%H%M%SZ"))
        os.makedirs(os.path.join(root, cc.INTENTS))
        with open(os.path.join(root, cc.INTENTS, f"{sha[:12]}.1.1"), "w") as f:
            f.write(f"{sha} {t0}\n")
        for stamp in ("20261005T120002Z", "20261005T121000Z"):  # the killed run's own, and a warm run 10 min on
            os.makedirs(os.path.join(root, f"{stamp}-{sha[:12]}"))
            with open(os.path.join(root, f"{stamp}-{sha[:12]}", "summary.json"), "w") as f:
                f.write(f'{{"subject": "{sha}", "tier": "t", "steps": [], "verdict": "PASS"}}')
        path, _ = cc.warm_summary(root, sha, {"t"})
        self.assertEqual(path, os.path.join(root, f"20261005T121000Z-{sha[:12]}", "summary.json"))
        os.unlink(path)
        self.assertEqual(cc.warm_summary(root, sha, {"t"}), (None, None))

    def test_fire_unknown_sha_refused(self):
        w = ColdWorld()
        unknown = "0123456789abcdef0123456789abcdef01234567"
        rc, out, err = run(COLD, "--sha", unknown, "--origin", w.bare, "--tier", "t", "--repo", w.repo, env=w.env)
        self.assertEqual(rc, 2); self.assertIn(f"not in {w.bare}", err); self.assertNotIn("verdict=", out)

    def test_fire_cut_tier_refused(self):
        w = ColdWorld()
        rc, _, err = run(COLD, "--sha", w.sha, "--origin", w.bare, "--tier", "cut", "--repo", w.repo, env=w.env)
        self.assertEqual(rc, 2); self.assertIn("cut tier", err)

if __name__ == "__main__":
    unittest.main()
