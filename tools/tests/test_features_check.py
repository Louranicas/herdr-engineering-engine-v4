import os, re, shutil, tempfile, textwrap, unittest
from common import TOOLS, run

CHECK = os.path.join(TOOLS, "features-check")
REPO = os.path.dirname(TOOLS)
MARK = "(rev 2026-10-05 drive)"
H2S = ["## Sub-features", "## How to get to it (user POV)", "## Driving it with hee4", "## Gotchas"]


def feature_file(h2s=H2S, driving_tail=""):
    """A feature file with the given H2s; `driving_tail` is appended under the Driving H2."""
    out = []
    for h in h2s:
        out.append(h + "\n\nbody\n\n" + (driving_tail + "\n" if h == "## Driving it with hee4" else ""))
    return "# f\n\n" + "".join(out)


def write(path, body):
    with open(path, "w") as f:
        f.write(body)


def plant(features, plugin=None):
    """A temp repo with gates/features/{README.md, <name>.md...} and, when given, tools/drive.d/zz.py."""
    r = tempfile.mkdtemp(prefix="fc-")
    fdir = os.path.join(r, "gates/features"); os.makedirs(fdir)
    write(os.path.join(fdir, "README.md"), "# readme\n\n## Purpose\n")
    for name, body in features.items():
        write(os.path.join(fdir, name + ".md"), body)
    if plugin is not None:
        os.makedirs(os.path.join(r, "tools/drive.d"))
        write(os.path.join(r, "tools/drive.d/zz.py"), textwrap.dedent(plugin))
    return r


def check(repo, *a):
    return run(CHECK, "--repo", repo, *a)


class FeaturesCheckTests(unittest.TestCase):
    def line(self, out, name):
        return [l for l in out.splitlines() if l.startswith(f"feature={name} ")][0]

    def test_repo_feature_files_pass_with_and_without_served(self):
        n = len([f for f in os.listdir(os.path.join(REPO, "gates/features")) if f.endswith(".md") and f != "README.md"])
        for flags in ((), ("--served",)):
            rc, out, err = run(CHECK, *flags)
            self.assertEqual(rc, 0, out + err)
            self.assertEqual(len([l for l in out.splitlines() if l.startswith("feature=")]), n, out)
            self.assertRegex(out.splitlines()[-1], rf"^features-check verdict=PASS files={n} drifted=0 head=\w+$")
        self.assertIn("feature=health h2=4/4 marker=present ", out)
        self.assertTrue(all(re.search(r" dead=0 status=PASS$", l) for l in out.splitlines() if l.startswith("feature=")), out)

    def test_three_h2s_is_a_fail_naming_the_file(self):
        r = plant({"a.one": feature_file(H2S[:3]), "ok": feature_file()})
        rc, out, _ = check(r)
        self.assertEqual(rc, 1, out)
        self.assertRegex(self.line(out, "a.one"), r"^feature=a\.one h2=3/4 marker=absent hee4_sh=0 dead=0 status=FAIL reason=missing H2 ## Gotchas$")
        self.assertRegex(self.line(out, "ok"), r" h2=4/4 .* status=PASS$")
        self.assertRegex(out.splitlines()[-1], r"^features-check verdict=FAIL files=2 drifted=1 head=\w+$")
        shutil.rmtree(r)

    def test_out_of_order_and_fifth_h2_are_fails(self):
        r = plant({"b.order": feature_file([H2S[0], H2S[2], H2S[1], H2S[3]]), "b.fifth": feature_file(H2S + ["## Notes"])})
        rc, out, _ = check(r)
        self.assertEqual(rc, 1, out)
        self.assertRegex(self.line(out, "b.order"), r" h2=2/4 .* status=FAIL reason=H2 order ")  # first and last sit in place
        self.assertRegex(self.line(out, "b.fifth"), r" h2=4/4 .* status=FAIL reason=extra H2 ## Notes$")
        self.assertRegex(out.splitlines()[-1], r"^features-check verdict=FAIL files=2 drifted=2 ")
        shutil.rmtree(r)

    def test_served_file_without_marker_fails_only_under_served(self):
        r = plant({"srv": feature_file()}, plugin='FEATURES = [("srv", lambda F, c: None)]\n')
        rc, out, _ = check(r)
        self.assertEqual(rc, 0, out)
        self.assertRegex(self.line(out, "srv"), r" marker=absent .* status=PASS$")
        rc, out, _ = check(r, "--served")
        self.assertEqual(rc, 1, out)
        self.assertRegex(self.line(out, "srv"), r"^feature=srv h2=4/4 marker=absent hee4_sh=0 dead=0 status=FAIL reason=served file lacks the marker ")
        self.assertRegex(out.splitlines()[-1], r"^features-check verdict=FAIL files=1 drifted=1 ")
        shutil.rmtree(r)

    def test_dead_hee4_sh_line_in_a_served_marker_block(self):
        tail = f"Concrete {MARK}:\n\n```bash\nhee4-sh health\n```\n"
        body = feature_file(driving_tail=tail)
        dead_at = body.splitlines().index("hee4-sh health") + 1
        r = plant({"srv": body}, plugin='FEATURES = [("srv", lambda F, c: None)]\n')
        rc, out, _ = check(r, "--served")
        self.assertEqual(rc, 1, out)
        self.assertRegex(self.line(out, "srv"), rf"^feature=srv h2=4/4 marker=present hee4_sh=1 dead=1 status=FAIL reason=dead hee4-sh line\(s\) {dead_at}$")
        rc, out, _ = check(r)  # without --served the dead line is reported, not judged
        self.assertEqual(rc, 0, out)
        self.assertRegex(self.line(out, "srv"), r" hee4_sh=1 dead=1 status=PASS$")
        write(os.path.join(r, "gates/features/srv.md"), body.replace("hee4-sh health", "hee4-sh health UNMEASURED"))
        rc, out, _ = check(r, "--served")
        self.assertEqual(rc, 0, out)
        self.assertRegex(self.line(out, "srv"), r" marker=present hee4_sh=1 dead=0 status=PASS$")
        shutil.rmtree(r)

    def test_hee4_sh_before_the_marker_is_not_dead(self):
        tail = f"```bash\nhee4-sh health\n```\n\nConcrete {MARK}:\n\nhee4 health\n"
        r = plant({"srv": feature_file(driving_tail=tail)}, plugin='FEATURES = [("srv", lambda F, c: None)]\n')
        rc, out, _ = check(r, "--served")
        self.assertEqual(rc, 0, out)
        self.assertRegex(self.line(out, "srv"), r" marker=present hee4_sh=1 dead=0 status=PASS$")
        shutil.rmtree(r)

    def test_plugin_naming_a_missing_file_is_a_fail_naming_the_module(self):
        r = plant({"ok": feature_file()}, plugin='FEATURES = [("ghost", lambda F, c: None)]\n')
        rc, out, _ = check(r, "--served")
        self.assertEqual(rc, 1, out)
        self.assertRegex(out.splitlines()[-1], r"^features-check verdict=FAIL files=0 drifted=0 head=\w+ reason=plugin zz: feature ghost has no file ghost\.md")
        rc, out, _ = check(r)  # the plugins are not consulted without --served
        self.assertEqual(rc, 0, out)
        shutil.rmtree(r)

    def test_journeys_file_is_exempt_from_the_h2_rule(self):
        r = plant({"multi-surface-journeys": "# j\n\n## E2E-01 · one\n\nx\n\n## E2E-02 · two\n\ny\n", "ok": feature_file()})
        rc, out, _ = check(r)
        self.assertEqual(rc, 0, out)
        self.assertRegex(self.line(out, "multi-surface-journeys"), r"^feature=multi-surface-journeys h2=exempt marker=absent hee4_sh=0 dead=0 status=PASS$")
        shutil.rmtree(r)

    def test_missing_directory_is_unmeasured(self):
        r = tempfile.mkdtemp(prefix="fc-")
        rc, out, _ = check(r)
        self.assertEqual(rc, 3, out)
        self.assertRegex(out.splitlines()[-1], r"^features-check verdict=UNMEASURED files=0 drifted=0 head=\w+ reason=no directory ")
        shutil.rmtree(r)


if __name__ == "__main__":
    unittest.main()
