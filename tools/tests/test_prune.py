import fcntl, os, re, subprocess, sys, tempfile, time, unittest
from common import TOOLS, run

PRUNE = os.path.join(TOOLS, "prune")

# Runs tools/prune's main() with plan() wrapped: after the plan has judged every dir (unlocked, old),
# a "build" starts on argv[1] -- it takes the cargo lock (lock) or touches debug/ (touch) -- before
# any rmtree. The plan's own in_use/young tests can no longer see it; only the apply-time re-check can.
AFTER_PLAN = """
import fcntl, os, sys
from importlib.machinery import SourceFileLoader
prune = SourceFileLoader("prune", sys.argv[1]).load_module()
target, how = sys.argv[2], sys.argv[3]
orig, held = prune.plan, []
def plan(a, root):
    out = orig(a, root)
    if how == "lock":
        fd = os.open(os.path.join(target, "debug", ".cargo-lock"), os.O_RDONLY)
        fcntl.flock(fd, fcntl.LOCK_EX); held.append(fd)
    else:
        os.utime(os.path.join(target, "debug"))
    return out
prune.plan = plan
sys.argv = ["prune"] + sys.argv[4:]
prune.main()
"""


def git_repo():
    d = tempfile.mkdtemp(prefix="pr-repo-")
    g = lambda *a: subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", *a],
                                  check=True, capture_output=True, text=True)
    g("init", "-q"); g("commit", "-q", "--allow-empty", "-m", "one")
    return d, g("rev-parse", "HEAD").stdout.strip()[:12]


def mkdir(path, age_h=0.0, payload=b"x" * 10):
    os.makedirs(os.path.join(path, "debug"), exist_ok=True)
    with open(os.path.join(path, "debug", "blob"), "wb") as f:
        f.write(payload)
    t = time.time() - age_h * 3600
    for p in (os.path.join(path, "debug"), path):
        os.utime(p, (t, t))


class TestPrune(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp(prefix="pr-cache-")
        self.gt = os.path.join(self.root, "hee4-gate-target")
        os.makedirs(self.gt)
        self.repo, self.head = git_repo()
        for name in ("CACHEDIR.TAG", ".rustc_info.json"):
            with open(os.path.join(self.gt, name), "w") as f:
                f.write("{}")
        # five gate-target dirs, distinct mtimes; HEAD's is the newest, so --keep 2 keeps it as `head`
        self.shas = [self.head] + [f"{i:012x}" for i in range(1, 5)]
        for i, sha in enumerate(self.shas):
            mkdir(os.path.join(self.gt, sha), age_h=i)

    def prune(self, *args):
        env = {"HEE4_CACHE_ROOT": self.root, "CARGO_TARGET_DIR": os.path.join(self.root, "hee4-target-mine")}
        rc, out, err = run(PRUNE, "--repo", self.repo, *args, env=env)
        cands = re.findall(r"^prune_candidate path=(\S+) ", out, re.M)
        return rc, out + err, cands

    def test_beyond_keep_are_candidates_and_head_is_kept(self):
        rc, out, cands = self.prune("--keep", "2")
        self.assertEqual(rc, 0, out)
        self.assertEqual(sorted(cands), sorted(os.path.join(self.gt, s) for s in self.shas[2:]), out)
        self.assertIn(f"prune_keep path={os.path.join(self.gt, self.head)} reason=head", out)
        self.assertIn("origin_main=UNMEASURED", out)
        self.assertNotIn("CACHEDIR.TAG", out); self.assertNotIn(".rustc_info.json", out)
        self.assertIn(f"mode=dry candidates={len(self.shas) - 2} ", out)
        self.assertIn("removed=0", out)
        for s in self.shas:
            self.assertTrue(os.path.isdir(os.path.join(self.gt, s)))

    def test_unexpected_dir_name_refuses_and_removes_nothing(self):
        os.makedirs(os.path.join(self.gt, "not-a-sha"))
        rc, out, _ = self.prune("--keep", "2", "--apply")
        self.assertEqual(rc, 1, out)
        self.assertIn(f"refuse_prune unexpected_name={os.path.join(self.gt, 'not-a-sha')}", out)
        self.assertRegex(out, r"prune verdict=FAIL mode=apply .* removed=0 ")
        for s in self.shas + ["not-a-sha"]:
            self.assertTrue(os.path.isdir(os.path.join(self.gt, s)), s)

    def test_flocked_sibling_is_in_use_and_protected_never_candidates(self):
        busy = os.path.join(self.root, "hee4-target-x")
        mkdir(busy, age_h=48)
        for name in ("hee4-target", "hee4-target-gate", "hee4-target-mine"):
            mkdir(os.path.join(self.root, name), age_h=48)
        stale = os.path.join(self.root, "hee4-target-old")
        mkdir(stale, age_h=48)
        young = os.path.join(self.root, "hee4-target-young")
        mkdir(young, age_h=0)
        lock = os.path.join(busy, "debug", ".cargo-lock")
        open(lock, "w").close()
        t = time.time() - 48 * 3600
        os.utime(os.path.join(busy, "debug"), (t, t))
        with open(lock) as f:
            fcntl.flock(f, fcntl.LOCK_EX)
            rc, out, cands = self.prune("--keep", "2")
        self.assertEqual(rc, 0, out)
        self.assertIn(f"prune_keep path={busy} reason=in_use", out)
        self.assertIn(f"prune_keep path={young} reason=young", out)
        self.assertIn(stale, cands)
        for name in ("hee4-target", "hee4-target-gate", "hee4-target-mine"):
            self.assertIn(f"prune_keep path={os.path.join(self.root, name)} reason=protected", out)
            self.assertNotIn(os.path.join(self.root, name), cands)

    def test_apply_removes_exactly_the_listed_candidates_and_is_idempotent(self):
        stale = os.path.join(self.root, "hee4-target-old")
        mkdir(stale, age_h=48)
        _, dry, listed = self.prune("--keep", "2")
        before = set(os.listdir(self.gt)) | set(os.listdir(self.root))
        rc, out, cands = self.prune("--keep", "2", "--apply")
        self.assertEqual(rc, 0, out)
        self.assertEqual(sorted(cands), sorted(listed))
        self.assertIn(f"removed={len(listed)} ", out)
        for p in listed:
            self.assertFalse(os.path.exists(p), p)
        gone = before - set(os.listdir(self.gt)) - set(os.listdir(self.root))
        self.assertEqual(gone, {os.path.basename(p) for p in listed})
        rc, out, _ = self.prune("--keep", "2", "--apply")
        self.assertEqual(rc, 0, out)
        self.assertIn("candidates=0 ", out); self.assertIn("removed=0 ", out)

    def apply_with_build_after_plan(self, how):
        busy, stale = os.path.join(self.root, "hee4-target-busy"), os.path.join(self.root, "hee4-target-old")
        mkdir(busy, age_h=48); mkdir(stale, age_h=48)
        open(os.path.join(busy, "debug", ".cargo-lock"), "w").close()
        t = time.time() - 48 * 3600
        os.utime(os.path.join(busy, "debug"), (t, t))
        env = {"HEE4_CACHE_ROOT": self.root, "CARGO_TARGET_DIR": os.path.join(self.root, "hee4-target-mine")}
        rc, out, err = run(sys.executable, "-c", AFTER_PLAN, PRUNE, busy, how, "--repo", self.repo, "--keep", "2", "--apply", env=env)
        cands = re.findall(r"^prune_candidate path=(\S+) ", out, re.M)
        self.assertIn(busy, cands, out + err)  # the plan judged it stale and unlocked
        self.assertTrue(os.path.isdir(busy), out + err)
        self.assertFalse(os.path.exists(stale), out + err)
        self.assertEqual(rc, 0, out + err)
        return out

    def test_lock_taken_after_the_plan_keeps_the_dir_in_use(self):
        out = self.apply_with_build_after_plan("lock")
        self.assertIn(f"prune_keep path={os.path.join(self.root, 'hee4-target-busy')} reason=in_use", out)

    def test_build_touching_debug_after_the_plan_keeps_the_dir_young(self):
        out = self.apply_with_build_after_plan("touch")
        self.assertIn(f"prune_keep path={os.path.join(self.root, 'hee4-target-busy')} reason=young", out)


if __name__ == "__main__":
    unittest.main()
