import datetime, json, os, shutil, subprocess, sys, tempfile, unittest
from common import TOOLS, run

ADV = os.path.join(TOOLS, "advisories")


def stub_bin(lines, rc):
    """A PATH dir holding a fake cargo-deny that prints <lines> (JSON) and exits <rc>, plus git, python3 and cat."""
    d = tempfile.mkdtemp(prefix="adv-bin-")
    body = "".join(json.dumps(l) + "\n" for l in lines)
    with open(os.path.join(d, "cargo-deny"), "w") as f:
        f.write(f"#!/bin/sh\ncat <<'EOF' >&2\n{body}EOF\nexit {rc}\n")
    os.chmod(os.path.join(d, "cargo-deny"), 0o755)
    for tool in ("git", "python3", "cat"):
        os.symlink(shutil.which(tool), os.path.join(d, tool))
    return d


def db_root(age_days):
    """An advisory-dbs root holding one git checkout whose last commit is <age_days> old."""
    root = tempfile.mkdtemp(prefix="adv-db-")
    d = os.path.join(root, "advisory-db-0000")
    os.makedirs(d)
    when = (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(days=age_days)).isoformat()
    env = dict(os.environ, GIT_COMMITTER_DATE=when, GIT_AUTHOR_DATE=when)
    g = lambda *a: subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", *a],
                                  check=True, capture_output=True, env=env)
    g("init", "-q"); g("commit", "-q", "--allow-empty", "-m", "db")
    return root


SUMMARY = lambda errors: {"type": "summary", "fields": {"advisories": {"errors": errors, "warnings": 0, "notes": 0, "helps": 0}}}
VULN = {"type": "diagnostic", "fields": {"severity": "error", "code": "vulnerability", "message": "x",
                                          "advisory": {"id": "RUSTSEC-2099-0001", "package": "zed"}}}


def adv(bin_dir, root, *extra):
    return run(sys.executable, ADV, "--db-root", root, *extra, env={"PATH": bin_dir})


class AdvisoriesTests(unittest.TestCase):
    def last(self, out):
        return out.strip().splitlines()[-1]

    def test_clean_graph_fresh_db_passes_and_names_the_db(self):
        rc, out, _ = adv(stub_bin([SUMMARY(0)], 0), db_root(1))
        self.assertEqual(rc, 0, out)
        self.assertRegex(self.last(out), r"^advisories verdict=PASS db=[0-9a-f]{12} db_date=\S+ db_age_days=1 errors=0$")

    def test_an_advisory_fails_with_its_id(self):
        rc, out, _ = adv(stub_bin([VULN, SUMMARY(1)], 1), db_root(0))
        self.assertEqual(rc, 1)
        self.assertIn("advisory id=RUSTSEC-2099-0001 crate=zed severity=error code=vulnerability", out)
        self.assertRegex(self.last(out), r"verdict=FAIL .* errors=1$")

    def test_no_summary_is_unmeasured_not_pass(self):
        rc, out, _ = adv(stub_bin([], 1), db_root(0))
        self.assertEqual(rc, 3)
        self.assertRegex(self.last(out), r"verdict=UNMEASURED .* reason=no_summary_rc=1$")

    def test_a_stale_database_checked_nothing(self):
        rc, out, _ = adv(stub_bin([SUMMARY(0)], 0), db_root(30))
        self.assertEqual(rc, 3)
        self.assertRegex(self.last(out), r"db_age_days=30 .*reason=advisory_db_older_than_7d$")
        rc, _, _ = adv(stub_bin([SUMMARY(0)], 0), db_root(30), "--max-age-days", "60")
        self.assertEqual(rc, 0)

    def test_absent_database_is_unmeasured(self):
        rc, out, _ = adv(stub_bin([SUMMARY(0)], 0), tempfile.mkdtemp(prefix="adv-empty-"))
        self.assertEqual(rc, 3)
        self.assertIn("reason=advisory_db_absent", out)

    def test_refused_without_cargo_deny(self):
        d = stub_bin([], 0)
        os.remove(os.path.join(d, "cargo-deny"))
        rc, _, err = adv(d, db_root(0))
        self.assertEqual(rc, 2)
        self.assertIn("cargo-deny not on PATH", err)


if __name__ == "__main__":
    unittest.main()
