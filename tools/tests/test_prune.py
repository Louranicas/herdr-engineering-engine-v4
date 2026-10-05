import fcntl, os, re, subprocess, tempfile, time, unittest
from common import TOOLS, run

PRUNE = os.path.join(TOOLS, "prune")


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


if __name__ == "__main__":
    unittest.main()
