import os, subprocess, unittest
from common import TOOLS, run, make_repo

SCAN = os.path.join(TOOLS, "push-scan")

# Every planted string is assembled here at runtime so no tracked file carries a literal hit.
AKIA = "AKIA" + "Z" * 16
V3 = "~/" + "hee3-" + "evidence" + "/x"
OUTWARD = "/home/" + "lour" + "anicas" + "/y"

def commit_files(repo, files):
    """A further commit on <repo> holding <files>; returns nothing (the range is HEAD~1..HEAD)."""
    for name, body in files.items():
        os.makedirs(os.path.dirname(os.path.join(repo, name)) or repo, exist_ok=True)
        with open(os.path.join(repo, name), "w") as f:
            f.write(body)
    g = lambda *a: subprocess.run(["git", "-C", repo, "-c", "user.name=t", "-c", "user.email=t@t", *a], check=True, capture_output=True)
    g("add", "-A"); g("commit", "-qm", "planted")

def planted(files):
    d, _ = make_repo("# no gate here\n")
    commit_files(d, files)
    return d

def scan(repo, *extra):
    return run(SCAN, "HEAD~1..HEAD", "--repo", repo, *extra, cwd=repo)

def hits(out):
    return [l for l in out.splitlines() if l.startswith("hit ")]

class PushScanTests(unittest.TestCase):
    def test_control_plants_three_classes(self):
        rc, out, _ = run(SCAN, "--control")
        self.assertEqual(rc, 0, out)
        self.assertEqual(out.strip().splitlines()[-1], "push-scan control cases=3/3 verdict=PASS")
        self.assertEqual(sorted(l.split()[1] for l in hits(out)), ["class=outward_name", "class=secret", "class=v3_path"])

    def test_fire_two_planted_classes_give_two_hits(self):
        d = planted({"p.rs": f"let k = \"{AKIA}\";\n// {V3}\n"})
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1)
        self.assertEqual(sorted(l.split()[1] for l in hits(out)), ["class=secret", "class=v3_path"])
        self.assertRegex(out.strip().splitlines()[-1], r"^push-scan range=HEAD~1\.\.HEAD commits=1 files=1 hits=2 verdict=FAIL$")

    def test_quiet_clean_range(self):
        d = planted({"p.rs": "fn main() {}\n"})
        rc, out, _ = scan(d)
        self.assertEqual(rc, 0, out)
        self.assertEqual(hits(out), [])
        self.assertRegex(out.strip().splitlines()[-1], r"^push-scan range=HEAD~1\.\.HEAD commits=1 files=1 hits=0 verdict=PASS$")

    def test_fire_planted_secret_named_by_rule_and_line(self):
        d = planted({"src/k.rs": f"// a\n// b\nlet k = \"{AKIA}\";\n"})
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1)
        self.assertIn("hit class=secret file=src/k.rs line=3 rule=aws_akia\n", out)
        self.assertIn("verdict=FAIL", out.strip().splitlines()[-1])

    def test_md_skips_v3_path_but_not_outward_name(self):
        d = planted({"note.md": f"forbid {V3}\n"})
        rc, out, _ = scan(d)
        self.assertEqual(rc, 0, out); self.assertEqual(hits(out), [])
        d = planted({"note.md": f"owner {OUTWARD}\n"})
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1); self.assertIn("hit class=outward_name file=note.md line=1 rule=home_path\n", out)

    def test_exclusion_row_needs_a_reason(self):
        d = planted({"ops/x.rs": f"// {V3}\n"})
        exc = os.path.join(d, "exc.txt")
        with open(exc, "w") as f:
            f.write("repo:ops/ |\n")
        rc, out, _ = scan(d, "--exclusions", exc)
        self.assertEqual(rc, 1); self.assertEqual(len(hits(out)), 1)
        with open(exc, "w") as f:
            f.write("repo:ops/ | holds the planted line on purpose\n")
        rc, out, _ = scan(d, "--exclusions", exc)
        self.assertEqual(rc, 0, out); self.assertEqual(hits(out), [])

    def test_commit_message_is_scanned(self):
        d = planted({"p.rs": "fn main() {}\n"})
        with open(os.path.join(d, "q.rs"), "w") as f:
            f.write("// q\n")
        subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", "add", "-A"], check=True)
        subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", f"touch\n\nsee {OUTWARD}"], check=True)
        head = run("git", "-C", d, "rev-parse", "HEAD")[1].strip()
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1); self.assertIn(f"hit class=outward_name file=commit:{head[:12]} line=3 rule=home_path\n", out)

    def test_fire_unknown_range_refused(self):
        d = planted({"p.rs": "fn main() {}\n"})
        rc, out, err = run(SCAN, "nosuch..HEAD", "--repo", d)
        self.assertEqual(rc, 2); self.assertIn("does not resolve", err); self.assertNotIn("verdict=", out)
        rc, _, err = run(SCAN, "--repo", d)
        self.assertEqual(rc, 2); self.assertIn("no github remote", err)

if __name__ == "__main__":
    unittest.main()
