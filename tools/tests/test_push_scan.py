import hashlib, os, subprocess, unittest
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

def dg(line):
    """The digest push-scan pins a hit to: sha256/12 of the stripped line."""
    return hashlib.sha256(line.strip().encode()).hexdigest()[:12]

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
        self.assertIn(f"hit class=secret file=src/k.rs line=3 rule=aws_akia digest={dg(f'let k = \"{AKIA}\";')}\n", out)
        self.assertIn("verdict=FAIL", out.strip().splitlines()[-1])

    def test_md_skips_v3_path_but_not_outward_name(self):
        d = planted({"note.md": f"forbid {V3}\n"})
        rc, out, _ = scan(d)
        self.assertEqual(rc, 0, out); self.assertEqual(hits(out), [])
        d = planted({"note.md": f"owner {OUTWARD}\n"})
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1); self.assertIn(f"hit class=outward_name file=note.md line=1 rule=home_path digest={dg(f'owner {OUTWARD}')}\n", out)

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

    def write_exceptions(self, d, body):
        path = os.path.join(d, "exceptions.txt")
        with open(path, "w") as f:
            f.write(body)
        return path

    def test_exception_row_covers_exactly_one_hit(self):
        d = planted({"ops/x.rs": f"// {OUTWARD}\n// clean\n"})
        exc = self.write_exceptions(d, f"ops/x.rs | 1 | home_path | {dg(f'// {OUTWARD}')} | the planted line is reviewed\n")
        rc, out, _ = scan(d, "--exceptions", exc)
        self.assertEqual(rc, 0, out); self.assertEqual(hits(out), [])
        self.assertIn("exceptions=1/1 stale=0", out)
        # a new hit in the same file, same rule, another line is still a hit
        commit_files(d, {"ops/x.rs": f"// {OUTWARD}\n// clean\n// {OUTWARD}\n"})
        rc, out, _ = run(SCAN, "HEAD~2..HEAD", "--repo", d, "--exceptions", exc, cwd=d)
        self.assertEqual(rc, 1, out)
        self.assertEqual(hits(out), [f"hit class=outward_name file=ops/x.rs line=3 rule=home_path digest={dg(f'// {OUTWARD}')}"])
        self.assertIn("exceptions=1/1 stale=0", out)
        self.assertRegex(out.strip().splitlines()[-1], r" hits=1 verdict=FAIL$")

    def test_exception_row_does_not_cover_changed_content(self):
        # same file, same line, same rule, new content: the reviewed row no longer covers it
        d = planted({"ops/x.rs": f"// {OUTWARD}\n"})
        exc = self.write_exceptions(d, f"ops/x.rs | 1 | home_path | {dg(f'// {OUTWARD}')} | the planted line is reviewed\n")
        rc, out, _ = scan(d, "--exceptions", exc)
        self.assertEqual(rc, 0, out); self.assertEqual(hits(out), [])
        changed = f"// {OUTWARD} and /home/" + "lour" + "anicas/.ssh/id_ed25519"
        commit_files(d, {"ops/x.rs": changed + "\n"})
        rc, out, _ = run(SCAN, "HEAD~2..HEAD", "--repo", d, "--exceptions", exc, cwd=d)
        self.assertEqual(rc, 1, out)
        self.assertEqual(hits(out), [f"hit class=outward_name file=ops/x.rs line=1 rule=home_path digest={dg(changed)}"])
        self.assertIn("exceptions=0/1 stale=1", out)

    def test_secret_class_row_is_refused(self):
        d = planted({"ops/x.rs": f"let k = \"{AKIA}\";\n"})
        for rule in ("aws_akia", "private_key", "github_token", "generic_assignment"):
            body = f"ops/x.rs | 1 | {rule} | {dg(f'let k = \"{AKIA}\";')} | a long and careful reason\n"
            rc, out, err = scan(d, "--exceptions", self.write_exceptions(d, body))
            self.assertEqual(rc, 2, rule); self.assertIn("secret-class rule cannot be excepted", err)
            self.assertNotIn("verdict=", out)

    def commit_rows(self, d, body, commit=True):
        """Writes tools/push-scan-exceptions.txt in <d>; commits it when <commit>."""
        os.makedirs(os.path.join(d, "tools"), exist_ok=True)
        with open(os.path.join(d, "tools", "push-scan-exceptions.txt"), "w") as f:
            f.write(body)
        if commit:
            g = lambda *a: subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", *a], check=True, capture_output=True)
            g("add", "-A"); g("commit", "-qm", "rows")

    def test_default_rows_are_read_from_the_scanned_head(self):
        d = planted({"ops/x.rs": f"// {OUTWARD}\n"})
        row = f"ops/x.rs | 1 | home_path | {dg(f'// {OUTWARD}')} | the planted line is reviewed\n"
        self.commit_rows(d, row, commit=False)  # in the working tree only: covers nothing
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1, out); self.assertIn("exception_rows=absent(", out)
        self.assertRegex(out.strip().splitlines()[-1], r" hits=1 verdict=FAIL$")
        with open(os.path.join(d, ".git", "info", "exclude"), "a") as f:
            f.write("tools/\n")  # an ignored file is not committed either
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1, out)
        with open(os.path.join(d, ".git", "info", "exclude"), "w") as f:
            f.write("")
        self.commit_rows(d, row)  # committed in range: the row is the reviewed one
        rc, out, _ = run(SCAN, "HEAD~2..HEAD", "--repo", d, cwd=d)
        self.assertEqual(rc, 0, out); self.assertIn("exceptions=1/1 stale=0 exception_rows=1", out)
        # an edit after the commit does not change what the scan reads
        self.commit_rows(d, "ops/x.rs | 1 | home_path | 000000000000 | edited\n", commit=False)
        rc, out, _ = run(SCAN, "HEAD~2..HEAD", "--repo", d, cwd=d)
        self.assertEqual(rc, 0, out); self.assertIn("exceptions=1/1", out)

    def test_stale_row_fails_by_name_over_the_default_range(self):
        d = planted({"ops/x.rs": "// clean\n"})
        self.commit_rows(d, "ops/x.rs | 1 | home_path | 0123456789ab | reviewed once, now gone\n")
        subprocess.run(["git", "-C", d, "update-ref", "refs/remotes/github/main", "HEAD~2"], check=True)
        rc, out, _ = run(SCAN, "github/main..HEAD", "--repo", d, cwd=d)
        self.assertEqual(rc, 1, out)
        self.assertIn("stale file=ops/x.rs line=1 rule=home_path digest=0123456789ab\n", out)
        self.assertRegex(out.strip().splitlines()[-1], r" hits=0 stale=1 verdict=FAIL$")
        rc, out, _ = run(SCAN, "HEAD~1..HEAD", "--repo", d, cwd=d)  # a narrower range only counts it
        self.assertEqual(rc, 0, out); self.assertIn("stale=1", out)

    def test_exception_row_names_its_rule(self):
        d = planted({"ops/x.rs": f"let k = \"{AKIA}\"; // {OUTWARD}\n"})
        line = f"let k = \"{AKIA}\"; // {OUTWARD}"
        exc = self.write_exceptions(d, f"ops/x.rs | 1 | home_path | {dg(line)} | the home path is reviewed, the key is not\n")
        rc, out, _ = scan(d, "--exceptions", exc)
        self.assertEqual(rc, 1, out)
        self.assertEqual(hits(out), [f"hit class=secret file=ops/x.rs line=1 rule=aws_akia digest={dg(line)}"])

    def test_exception_row_with_empty_reason_is_refused(self):
        d = planted({"ops/x.rs": f"// {OUTWARD}\n"})
        h = dg(f"// {OUTWARD}")
        for body, why in ((f"ops/x.rs | 1 | home_path | {h} |\n", "empty reason"),
                          (f"ops/x.rs | 1 | home_path | {h} |   \n", "empty reason"),
                          ("ops/x.rs | 1 | home_path | r\n", "is not <file>"),
                          (f"ops/x.rs | one | home_path | {h} | r\n", "positive line number"),
                          (f"ops/x.rs | 1 | nosuch | {h} | r\n", "unknown rule"),
                          ("ops/x.rs | 1 | home_path | XYZ | r\n", "is not 12 lowercase hex"),
                          (f"ops/x.rs | 1 | home_path | {h} | r\nops/x.rs | 1 | home_path | {h} | r\n", "duplicate")):
            rc, out, err = scan(d, "--exceptions", self.write_exceptions(d, body))
            self.assertEqual(rc, 2, body); self.assertIn(why, err); self.assertNotIn("verdict=", out)
        rc, _, err = scan(d, "--exceptions", os.path.join(d, "absent.txt"))
        self.assertEqual(rc, 2); self.assertIn("not readable", err)

    def test_stale_row_is_counted_not_applied(self):
        d = planted({"ops/x.rs": f"// clean\n// {OUTWARD}\n"})
        exc = self.write_exceptions(d, f"ops/x.rs | 1 | home_path | {dg(f'// {OUTWARD}')} | reviewed when the hit was on line 1\n")
        rc, out, _ = scan(d, "--exceptions", exc)
        self.assertEqual(rc, 1, out); self.assertIn("exceptions=0/1 stale=1", out)
        self.assertEqual(hits(out), [f"hit class=outward_name file=ops/x.rs line=2 rule=home_path digest={dg(f'// {OUTWARD}')}"])

    def test_repo_exceptions_file_is_well_formed(self):
        rc, out, err = run(SCAN, "HEAD..HEAD", "--repo", os.path.dirname(TOOLS),
                           "--exceptions", os.path.join(TOOLS, "push-scan-exceptions.txt"))
        self.assertEqual(rc, 3, out + err); self.assertRegex(out, r"exception_rows=[1-9]\d*")

    def test_commit_message_is_scanned(self):
        d = planted({"p.rs": "fn main() {}\n"})
        with open(os.path.join(d, "q.rs"), "w") as f:
            f.write("// q\n")
        subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", "add", "-A"], check=True)
        subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", f"touch\n\nsee {OUTWARD}"], check=True)
        head = run("git", "-C", d, "rev-parse", "HEAD")[1].strip()
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1); self.assertIn(f"hit class=outward_name file=commit:{head[:12]} line=3 rule=home_path digest={dg(f'see {OUTWARD}')}\n", out)

    def test_fire_host_diff_config_cannot_blank_the_patch(self):
        # diff.noprefix, color.ui=always, diff.external and GIT_DIFF_OPTS each reshape git's default patch text
        d = planted({"p.rs": f"// a\nlet k = \"{AKIA}\";\n"})
        env = {"GIT_CONFIG_COUNT": "3", "GIT_CONFIG_KEY_0": "diff.noprefix", "GIT_CONFIG_VALUE_0": "true",
               "GIT_CONFIG_KEY_1": "color.ui", "GIT_CONFIG_VALUE_1": "always",
               "GIT_CONFIG_KEY_2": "diff.external", "GIT_CONFIG_VALUE_2": "false", "GIT_DIFF_OPTS": "--unified=3"}
        rc, out, err = run(SCAN, "HEAD~1..HEAD", "--repo", d, env=env, cwd=d)
        self.assertEqual(rc, 1, out + err); self.assertIn(f"hit class=secret file=p.rs line=2 rule=aws_akia digest={dg(f'let k = \"{AKIA}\";')}\n", out)
        self.assertRegex(out.strip().splitlines()[-1], r" files=1 hits=1 verdict=FAIL$")

    def test_fire_non_ascii_filename_is_named_and_scanned(self):
        d = planted({"\u00fc.rs": f"let k = \"{AKIA}\";\n"})
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1, out); self.assertIn(f"hit class=secret file=\u00fc.rs line=1 rule=aws_akia digest={dg(f'let k = \"{AKIA}\";')}\n", out)

    def test_fire_content_line_starting_plus_plus_is_content(self):
        # `++ x` arrives as `+++ x` in the patch: a header only between hunks, never inside one
        d = planted({"p.rs": f"++ x\nlet k = \"{AKIA}\";\n"})
        rc, out, _ = scan(d)
        self.assertEqual(rc, 1, out); self.assertIn(f"hit class=secret file=p.rs line=2 rule=aws_akia digest={dg(f'let k = \"{AKIA}\";')}\n", out)
        self.assertRegex(out.strip().splitlines()[-1], r" files=1 hits=1 verdict=FAIL$")

    def test_fire_empty_range_is_unmeasured(self):
        d = planted({"p.rs": "fn main() {}\n"})
        rc, out, _ = run(SCAN, "HEAD..HEAD", "--repo", d, cwd=d)
        self.assertEqual(rc, 3, out); self.assertNotIn("hits=0", out)
        self.assertEqual(out.strip().splitlines()[-1], "push-scan range=HEAD..HEAD commits=0 files=0 hits=UNMEASURED(empty range) verdict=UNMEASURED")
        rc, out, _ = run(SCAN, "HEAD..HEAD~1", "--repo", d, cwd=d)  # reversed: the removals would read as additions
        self.assertEqual(rc, 3, out); self.assertNotIn("hits=0", out)
        self.assertRegex(out.strip().splitlines()[-1], r"^push-scan range=HEAD\.\.HEAD~1 commits=0 files=1 hits=UNMEASURED\(reversed range.*\) verdict=UNMEASURED$")

    def test_fire_unknown_range_refused(self):
        d = planted({"p.rs": "fn main() {}\n"})
        rc, out, err = run(SCAN, "nosuch..HEAD", "--repo", d)
        self.assertEqual(rc, 2); self.assertIn("does not resolve", err); self.assertNotIn("verdict=", out)
        rc, _, err = run(SCAN, "--repo", d)
        self.assertEqual(rc, 2); self.assertIn("no github remote", err)

if __name__ == "__main__":
    unittest.main()
