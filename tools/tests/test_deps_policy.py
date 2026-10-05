import os, re, shutil, subprocess, tempfile, unittest
from common import TOOLS

REPO = os.path.dirname(TOOLS)
CMD = ["cargo-deny", "--offline", "check", "bans", "licenses", "sources"]


def workspace(mutate):
    """A copy of the live workspace (manifests, lock, sources, deny.toml) with one edit applied."""
    d = tempfile.mkdtemp(prefix="deps-")
    for name in ("Cargo.toml", "Cargo.lock", "deny.toml"):
        shutil.copy(os.path.join(REPO, name), d)
    shutil.copytree(os.path.join(REPO, "crates"), os.path.join(d, "crates"),
                    ignore=shutil.ignore_patterns("target", "tests", "examples", "benches"))
    mutate(d)
    return d


def edit(d, rel, old, new):
    p = os.path.join(d, rel)
    with open(p) as f:
        s = f.read()
    assert old in s, (rel, old)
    with open(p, "w") as f:
        f.write(s.replace(old, new, 1))


def deny(d):
    r = subprocess.run(CMD, cwd=d, capture_output=True, text=True, timeout=120)
    return r.returncode, r.stdout + r.stderr


class DepsPolicyTests(unittest.TestCase):
    """The `deps` step's policy (deny.toml) on the live graph, and one planted violation per rule."""

    @classmethod
    def setUpClass(cls):
        if shutil.which("cargo-deny") is None:
            raise AssertionError("cargo-deny not on PATH: the commit tier's deps step needs it (mise: cargo-deny 0.20.2)")

    def test_live_graph_passes(self):
        rc, out = deny(workspace(lambda d: None))
        self.assertEqual(rc, 0, out[-800:])
        self.assertIn("bans ok, licenses ok, sources ok", out)

    def test_a_license_off_the_allow_list_fails(self):
        rc, out = deny(workspace(lambda d: edit(d, "deny.toml", '"MIT", ', "")))
        self.assertNotEqual(rc, 0)
        self.assertIn("licenses FAILED", out)

    def test_a_publishable_crate_fails_licenses_and_wildcards(self):
        rc, out = deny(workspace(lambda d: edit(d, "crates/hee4-host/Cargo.toml", "publish.workspace = true\n", "")))
        self.assertNotEqual(rc, 0)
        self.assertIn("licenses FAILED", out)
        self.assertRegex(out, r"bans FAILED")

    def test_an_unknown_registry_is_refused(self):
        rc, out = deny(workspace(lambda d: edit(d, "deny.toml", 'allow-registry = ["https://github.com/rust-lang/crates.io-index"]',
                                                 "allow-registry = []")))
        self.assertNotEqual(rc, 0)
        self.assertIn("sources FAILED", out)


if __name__ == "__main__":
    unittest.main()
