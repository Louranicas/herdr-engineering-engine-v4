import os, tempfile, unittest
from common import TOOLS, run

RATCHET = os.path.join(TOOLS, "lint-ratchet")

WS = ('[workspace]\nmembers = ["crates/a"]\n'
      '[workspace.lints.rust]\nunsafe_code = "forbid"\n'
      '[workspace.lints.clippy]\npedantic = { level = "warn", priority = -1 }\nunwrap_used = "deny"\n')
FLOOR = ('[workspace.rust]\nunsafe_code = "forbid"\n'
         '[workspace.clippy]\npedantic = "warn"\nunwrap_used = "deny"\n'
         '[allows]\nsrc_unreasoned_max = 1\n')
CRATE = '[package]\nname = "a"\n[lints]\nworkspace = true\n'


def world(files):
    d = tempfile.mkdtemp(prefix="lr-")
    base = {"Cargo.toml": WS, "tools/lint-floor.toml": FLOOR, "crates/a/Cargo.toml": CRATE,
            "crates/a/src/lib.rs": "//! a\n", "crates/a/tests/t.rs": "#![allow(clippy::unwrap_used)]\n"}
    base.update(files)
    for name, body in base.items():
        if body is None:
            continue
        p = os.path.join(d, name)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w") as f:
            f.write(body)
    return d


def ratchet(d, *args):
    return run(RATCHET, "--repo", d, *args)


class LintRatchetTests(unittest.TestCase):
    def test_clean_tree_passes_and_test_files_may_unwrap(self):
        rc, out, _ = ratchet(world({}))
        self.assertEqual(rc, 0)
        self.assertEqual(out.strip().splitlines()[-1],
                         "lint-ratchet verdict=PASS lints=3/3 crates_inherit=1/1 deny_off=0 src_unreasoned=0/1 findings=0")

    def test_stricter_than_floor_passes(self):
        rc, out, _ = ratchet(world({"Cargo.toml": WS.replace('unwrap_used = "deny"', 'unwrap_used = "forbid"')}))
        self.assertEqual(rc, 0, out)

    def test_downgrade_fails_with_the_lint_named(self):
        rc, out, _ = ratchet(world({"Cargo.toml": WS.replace('unwrap_used = "deny"', 'unwrap_used = "warn"')}))
        self.assertEqual(rc, 1)
        self.assertIn("finding rule=R1 path=Cargo.toml detail=clippy::unwrap_used=warn floor=deny", out)

    def test_table_form_level_is_read(self):
        ws = WS.replace('pedantic = { level = "warn", priority = -1 }', 'pedantic = { level = "allow", priority = -1 }')
        rc, out, _ = ratchet(world({"Cargo.toml": ws}))
        self.assertEqual(rc, 1)
        self.assertIn("detail=clippy::pedantic=allow floor=warn", out)

    def test_crate_without_workspace_lints_fails(self):
        rc, out, _ = ratchet(world({"crates/a/Cargo.toml": '[package]\nname = "a"\n'}))
        self.assertEqual(rc, 1)
        self.assertIn("finding rule=R2 path=crates/a/Cargo.toml", out)

    def test_deny_lint_allowed_in_src_fails_bare_cfg_attr_and_multiline(self):
        for attr in ("#![allow(clippy::unwrap_used)]\n",
                     "#[cfg_attr(test, allow(clippy::unwrap_used))]\nfn f() {}\n",
                     '#[expect(\n    clippy::unwrap_used,\n    reason = "x"\n)]\nfn f() {}\n',
                     "#![allow(warnings)]\n"):
            rc, out, _ = ratchet(world({"crates/a/src/lib.rs": attr}))
            self.assertEqual(rc, 1, attr)
            self.assertIn("finding rule=R3 path=crates/a/src/lib.rs:", out)

    def test_unreasoned_allow_cap_and_reasoned_expect_is_free(self):
        one = "#[allow(dead_code)]\nfn f() {}\n"
        rc, _, _ = ratchet(world({"crates/a/src/lib.rs": one}))
        self.assertEqual(rc, 0)
        rc, out, _ = ratchet(world({"crates/a/src/lib.rs": one * 2}))
        self.assertEqual(rc, 1)
        self.assertIn("finding rule=R4 path=crates detail=src_unreasoned=2 max=1", out)
        reasoned = '#[expect(dead_code, reason = "kept for the R-table")]\nfn f() {}\n'
        rc, _, _ = ratchet(world({"crates/a/src/lib.rs": one + reasoned * 3}))
        self.assertEqual(rc, 0)

    def test_cap_lints_rustflags_fail(self):
        rc, out, _ = ratchet(world({".cargo/config.toml": '[build]\nrustflags = ["--cap-lints", "warn"]\n'}))
        self.assertEqual(rc, 1)
        self.assertIn("finding rule=R5", out)

    def test_refuses_without_floor_or_crates(self):
        rc, _, err = ratchet(world({"tools/lint-floor.toml": None}))
        self.assertEqual(rc, 2)
        self.assertIn("floor unreadable", err)
        rc, _, err = ratchet(world({"crates/a/Cargo.toml": None}))
        self.assertEqual(rc, 2)

    def test_control_catches_every_plant_on_the_live_repo(self):
        rc, out, _ = run(RATCHET, "--control")
        self.assertEqual(rc, 0, out)
        self.assertNotIn("caught=no", out)
        self.assertRegex(out.strip().splitlines()[-1], r"^lint-ratchet control cases=(\d+)/\1 verdict=PASS$")


if __name__ == "__main__":
    unittest.main()
