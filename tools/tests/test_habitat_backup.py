import hashlib, importlib.machinery, importlib.util, json, os, re, shutil, subprocess, tempfile, unittest
from common import TOOLS, run

HB = os.path.join(TOOLS, "habitat-backup")
HOST = os.path.expanduser("~/.cache/hee4-host")


def load():
    loader = importlib.machinery.SourceFileLoader("habitat_backup", HB)
    spec = importlib.util.spec_from_loader("habitat_backup", loader)
    mod = importlib.util.module_from_spec(spec); loader.exec_module(mod)
    return mod


class HabitatBackupTests(unittest.TestCase):
    def setUp(self):
        os.makedirs(HOST, exist_ok=True)
        self.src = tempfile.mkdtemp(prefix="hb-test-src-", dir=HOST)  # the sources: the home disk
        self.other = tempfile.mkdtemp(prefix="hb-test-dest-")  # the temp dir: another device on this host
        self.addCleanup(shutil.rmtree, self.src, True); self.addCleanup(shutil.rmtree, self.other, True)
        self.assertNotEqual(os.stat(self.src).st_dev, os.stat(self.other).st_dev, "the fixture needs two devices")
        work, self.bare = os.path.join(self.src, "work"), os.path.join(self.src, "origin.git")
        g = lambda *x: subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *x], check=True, capture_output=True)
        g("init", "-q", work)
        with open(os.path.join(work, "a.txt"), "w") as f:
            f.write("a\n")
        g("-C", work, "add", "-A"); g("-C", work, "commit", "-qm", "one"); g("clone", "-q", "--bare", work, self.bare)
        for rel in ("evidence/reviews/r.md", "evidence/e.txt", "handoffs/h.md"):
            os.makedirs(os.path.dirname(os.path.join(self.src, rel)), exist_ok=True)
            with open(os.path.join(self.src, rel), "w") as f:
                f.write(rel + "\n")

    def hb(self, dest, *extra, evidence=None):
        return run(HB, "--dest", dest, "--codebase", self.bare, "--evidence", evidence or os.path.join(self.src, "evidence"),
                   "--handoffs", os.path.join(self.src, "handoffs"), *extra, timeout=300)

    def test_backup_to_other_device_pass_and_manifest_last(self):
        dest = os.path.join(self.other, "dest")
        rc, out, err = self.hb(dest)
        self.assertEqual(rc, 0, out + err)
        last = out.strip().splitlines()[-1]
        self.assertRegex(last, r"^habitat-backup verdict=PASS objects=4 bytes=[1-9]\d* backup=h-\d{8}T\d{6}-\d{6}Z ")
        bid = re.search(r"backup=(\S+)", last).group(1)
        self.assertEqual(sorted(os.listdir(dest)), [".lock", bid])
        root = os.path.join(dest, bid)
        man = json.load(open(os.path.join(root, "manifest.json")))
        self.assertEqual(man["objects"], 4)
        on_disk = sorted(os.path.relpath(os.path.join(d, n), root) for d, _, ns in os.walk(root) for n in ns)
        self.assertEqual(on_disk, sorted(["manifest.json", *man["files"]]))
        for rel, digest in man["files"].items():
            self.assertEqual(hashlib.sha256(open(os.path.join(root, rel), "rb").read()).hexdigest(), digest, rel)
        mt = os.stat(os.path.join(root, "manifest.json")).st_mtime_ns
        self.assertTrue(all(os.stat(os.path.join(root, rel)).st_mtime_ns <= mt for rel in man["files"]), "manifest not last")
        heads = subprocess.run(["git", "bundle", "list-heads", os.path.join(root, "codebase.bundle")], capture_output=True, text=True)
        self.assertEqual(heads.returncode, 0, heads.stderr); self.assertIn("refs/heads/", heads.stdout)

    def test_prune_keeps_newest_k_and_drops_partials_only(self):
        dest = os.path.join(self.other, "dest")
        os.makedirs(os.path.join(dest, "h-20000101T000000-000000Z.partial")); os.makedirs(os.path.join(dest, "keep-me"))
        ids = []
        for _ in range(3):
            rc, out, err = self.hb(dest, "--keep", "2")
            self.assertEqual(rc, 0, out + err); ids.append(re.search(r"backup=(\S+)", out).group(1))
        self.assertEqual(sorted(os.listdir(dest)), sorted([".lock", "keep-me", *ids[1:]]))

    def test_same_device_dest_refused_by_name_nothing_written(self):
        dest = os.path.join(self.src, "dest")
        rc, out, _ = self.hb(dest)
        self.assertEqual(rc, 20, out)
        self.assertIn(f"habitat-backup refused reason=same_device source=codebase dest={dest}", out)
        self.assertRegex(out.strip().splitlines()[-1], r"^habitat-backup verdict=FAIL objects=0 .* reason=same_device$")
        self.assertFalse(os.path.exists(dest))

    def test_missing_source_refused_by_name_nothing_written(self):
        dest, gone = os.path.join(self.other, "dest"), os.path.join(self.src, "no-evidence")
        rc, out, _ = self.hb(dest, evidence=gone)
        self.assertEqual(rc, 20, out)
        self.assertIn(f"habitat-backup refused reason=missing_source source=evidence path={gone}", out)
        self.assertTrue(out.strip().endswith("reason=missing_source"), out)
        self.assertFalse(os.path.exists(dest))

    def test_control_pass(self):
        rc, out, err = run(HB, "--control", timeout=300)
        self.assertEqual(rc, 0, out + err)
        self.assertEqual(out.strip().splitlines()[-1], "habitat-backup control cases=3/3 verdict=PASS")
        for case in ("same_device", "missing_source", "other_device_backup"):
            self.assertRegex(out, rf"control case={case} .*detected=yes")

    def test_exit_for_is_the_roster_contract_from_the_last_verdict_line(self):
        m = load()
        self.assertEqual(m.exit_for("habitat-backup verdict=PASS objects=1"), 0)
        self.assertEqual(m.exit_for("x verdict=PASS_WITH_GAPS"), 10)
        self.assertEqual(m.exit_for("a verdict=PASS\nb verdict=FAIL objects=0"), 20)
        self.assertEqual(m.exit_for("a verdict=FAIL\nb verdict=PASS\ntrailing"), 0)
        self.assertEqual(m.exit_for("Traceback (most recent call last):\nOSError"), 30)
        self.assertEqual(m.exit_for(""), 30)

    def test_service_exit_is_derived_from_the_verdict_line_and_logged(self):
        log = os.path.join(self.other, "state", "habitat-backup.log")
        rc, out, _ = self.hb(os.path.join(self.src, "dest"), "--service", "--log", log)  # same device: FAIL
        self.assertEqual(rc, 20, out); self.assertRegex(out, r"run ts=\S+ child_rc=20 exit=20")
        rc, out, _ = self.hb(os.path.join(self.other, "dest"), "--service", "--log", log)
        self.assertEqual(rc, 0, out)
        # a crash (the dest's parent is a file on the other device): no verdict line, so 30, whatever rc python had
        blocker = os.path.join(self.other, "a-file")
        open(blocker, "w").close()
        rc, out, _ = self.hb(os.path.join(blocker, "dest"), "--service", "--log", log)
        self.assertEqual(rc, 30, out); self.assertRegex(out, r"child_rc=1 exit=30")
        verdicts = [l.split()[1] for l in open(log).read().splitlines() if l.startswith("habitat-backup verdict=")]
        self.assertEqual(verdicts, ["verdict=FAIL", "verdict=PASS"])
        self.assertEqual(len(re.findall(r"^habitat-backup run ", open(log).read(), re.M)), 3)


if __name__ == "__main__":
    unittest.main()
