import contextlib, hashlib, importlib.machinery, importlib.util, io, json, os, re, shutil, subprocess, tempfile, types, unittest
from common import TOOLS, run

HB = os.path.join(TOOLS, "habitat-backup")
HOST = os.path.expanduser("~/.cache/hee4-host")


def slurp(path):
    with open(path) as f:
        return f.read()


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

    def hb(self, dest, *extra, evidence=None, env=None):
        return run(HB, "--dest", dest, "--codebase", self.bare, "--evidence", evidence or os.path.join(self.src, "evidence"),
                   "--handoffs", os.path.join(self.src, "handoffs"), *extra, timeout=300, env=env)

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
        self.assertEqual(man["objects"], 4); self.assertEqual(man["dest_dev"], os.stat(self.other).st_dev)
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

    def test_prune_never_drops_this_runs_backup_under_future_dated_names(self):
        # clock skew left two complete backups whose names sort after any backup taken today
        dest = os.path.join(self.other, "dest")
        for name, ts in (("h-20991231T000000-000000Z", 4102358400), ("h-20991231T000001-000000Z", 4102358401)):
            os.makedirs(os.path.join(dest, name))
            with open(os.path.join(dest, name, "manifest.json"), "w") as f:
                json.dump({"id": name, "ts": ts, "objects": 0, "files": {}}, f)
        rc, out, err = self.hb(dest, "--keep", "1")
        self.assertEqual(rc, 0, out + err)
        bid = re.search(r"backup=(\S+)", out).group(1)
        self.assertTrue(os.path.isfile(os.path.join(dest, bid, "manifest.json")), out)
        self.assertEqual(sorted(os.listdir(dest)), [".lock", bid])
        self.assertIn(" pruned=2", out.strip().splitlines()[-1])

    def test_prune_orders_by_manifest_ts_not_name(self):
        dest = os.path.join(self.other, "dest")
        for name, ts in (("h-20991231T000000-000000Z", 1), ("h-20000101T000000-000000Z", 4102358400)):
            os.makedirs(os.path.join(dest, name))
            with open(os.path.join(dest, name, "manifest.json"), "w") as f:
                json.dump({"id": name, "ts": ts, "objects": 0, "files": {}}, f)
        rc, out, err = self.hb(dest, "--keep", "2")
        self.assertEqual(rc, 0, out + err)
        bid = re.search(r"backup=(\S+)", out).group(1)
        self.assertEqual(sorted(os.listdir(dest)), sorted([".lock", "h-20000101T000000-000000Z", bid]))

    def test_symlink_dotdot_dest_onto_the_sources_device_refused(self):
        # text: <other>/lnk/../escape is on the other device; kernel: lnk -> <src>/back/sub, so .. is <src>/back
        back = os.path.join(self.src, "back")
        os.makedirs(os.path.join(back, "sub"))
        os.symlink(os.path.join(back, "sub"), os.path.join(self.other, "lnk"))
        dest = os.path.join(self.other, "lnk", "..", "escape")
        rc, out, _ = self.hb(dest)
        self.assertEqual(rc, 20, out)
        self.assertIn(f"habitat-backup refused reason=same_device source=codebase dest={dest}", out)
        self.assertFalse(os.path.exists(os.path.join(back, "escape")))
        self.assertFalse(os.path.exists(os.path.join(self.other, "escape")))

    def test_written_dir_device_is_checked_even_when_admission_is_fooled(self):
        m = load()
        dest = os.path.join(self.src, "dest")  # the sources' device
        a = types.SimpleNamespace(dest=dest, codebase=self.bare, evidence=os.path.join(self.src, "evidence"),
                                  handoffs=os.path.join(self.src, "handoffs"), keep=14)
        m.admit = lambda sources, d: []  # admission says yes; the stat of the dir actually created must still say no
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            rc = m.backup(a)
        out = buf.getvalue()
        self.assertEqual(rc, 20, out)
        self.assertIn(f"habitat-backup refused reason=same_device source=codebase dest={dest}", out)
        self.assertTrue(out.strip().endswith("reason=same_device"), out)
        self.assertEqual(sorted(os.listdir(dest)), [".lock"])  # no backup, no partial

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
        self.assertEqual(out.strip().splitlines()[-1], "habitat-backup control cases=4/4 verdict=PASS")
        for case in ("same_device", "missing_source", "symlink_dotdot", "other_device_backup"):
            self.assertRegex(out, rf"control case={case} .*detected=yes")

    def test_exit_for_is_the_roster_contract_from_the_last_verdict_line(self):
        m = load()
        self.assertEqual(m.exit_for("habitat-backup verdict=PASS objects=1"), 0)
        self.assertEqual(m.exit_for("habitat-backup verdict=PASS_WITH_GAPS"), 10)
        self.assertEqual(m.exit_for("habitat-backup verdict=PASS\nhabitat-backup verdict=FAIL objects=0"), 20)
        self.assertEqual(m.exit_for("habitat-backup verdict=FAIL\nhabitat-backup verdict=PASS\ntrailing"), 0)
        self.assertEqual(m.exit_for("Traceback (most recent call last):\nOSError"), 30)
        self.assertEqual(m.exit_for(""), 30)
        # a verdict= token that is not at the start of a verdict line is not a verdict
        self.assertEqual(m.exit_for("habitat-backup child_stderr='NotADirectoryError: /x/verdict=PASS'"), 30)
        self.assertEqual(m.exit_for("habitat-backup verdict=FAIL\nOSError: verdict=PASS"), 20)
        self.assertEqual(m.exit_for("habitat-backup verdict=PASSED"), 30)

    def test_last_run_is_the_last_block_only(self):
        m = load()
        self.assertIsNone(m.last_run(""))
        self.assertIsNone(m.last_run("habitat-backup verdict=PASS objects=1\n"))  # a verdict outside any run block
        log = ("habitat-backup run ts=t1 invocation=aaa child_rc=0 exit=0\nhabitat-backup verdict=PASS objects=1\n"
               "habitat-backup run ts=t2 invocation=bbb child_rc=1 exit=30\n"
               "habitat-backup child_stderr='OSError: verdict=PASS'\n")
        self.assertEqual(m.last_run(log), ({"ts": "t2", "invocation": "bbb", "child_rc": "1", "exit": "30"}, None))
        self.assertEqual(m.last_run(log + "habitat-backup run ts=t3 invocation=ccc child_rc=0 exit=0\n"
                                    "habitat-backup verdict=PASS objects=4\n")[1], "PASS")

    def test_service_exit_is_derived_from_the_verdict_line_and_logged(self):
        log = os.path.join(self.other, "state", "habitat-backup.log")
        rc, out, _ = self.hb(os.path.join(self.src, "dest"), "--service", "--log", log)  # same device: FAIL
        self.assertEqual(rc, 20, out); self.assertRegex(out, r"run ts=\S+ invocation=\S+ child_rc=20 exit=20")
        rc, out, _ = self.hb(os.path.join(self.other, "dest"), "--service", "--log", log)
        self.assertEqual(rc, 0, out)
        # a crash (the dest's parent is a file on the other device): no verdict line, so 30, whatever rc python had
        blocker = os.path.join(self.other, "a-file")
        open(blocker, "w").close()
        rc, out, _ = self.hb(os.path.join(blocker, "dest"), "--service", "--log", log)
        self.assertEqual(rc, 30, out); self.assertRegex(out, r"child_rc=1 exit=30")
        # the same crash, its message quoting verdict=PASS (the dest's name): still no verdict line, still 30
        rc, out, _ = self.hb(os.path.join(blocker, "verdict=PASS"), "--service", "--log", log)
        self.assertEqual(rc, 30, out); self.assertRegex(out, r"child_rc=1 exit=30")
        self.assertIn("verdict=PASS", [l for l in out.splitlines() if "child_stderr=" in l][0])
        verdicts = [l.split()[1] for l in open(log).read().splitlines() if l.startswith("habitat-backup verdict=")]
        self.assertEqual(verdicts, ["verdict=FAIL", "verdict=PASS"])
        self.assertEqual(len(re.findall(r"^habitat-backup run ", open(log).read(), re.M)), 4)
        self.assertEqual(load().last_run(slurp(log))[1], None)  # the last block crashed: no verdict

    def test_service_records_the_systemd_invocation_id(self):
        log = os.path.join(self.other, "state", "habitat-backup.log")
        rc, out, _ = self.hb(os.path.join(self.other, "dest"), "--service", "--log", log,
                             env={"INVOCATION_ID": "0123456789abcdef0123456789abcdef"})
        self.assertEqual(rc, 0, out)
        fields, verdict = load().last_run(slurp(log))
        self.assertEqual((fields["invocation"], fields["exit"], verdict), ("0123456789abcdef0123456789abcdef", "0", "PASS"))
        env = dict(os.environ); env.pop("INVOCATION_ID", None)
        r = subprocess.run([HB, "--dest", os.path.join(self.other, "dest"), "--codebase", self.bare, "--evidence",
                            os.path.join(self.src, "evidence"), "--handoffs", os.path.join(self.src, "handoffs"),
                            "--service", "--log", log], capture_output=True, text=True, env=env, timeout=300)
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertEqual(load().last_run(slurp(log))[0]["invocation"], "none")


if __name__ == "__main__":
    unittest.main()
