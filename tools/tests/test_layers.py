import os, subprocess, tempfile, unittest
from common import TOOLS, run

LAYERS = os.path.join(TOOLS, "layers")
REPO = os.path.dirname(TOOLS)
NAMES = ("product", "tests", "apparatus", "governance", "planning")

def config(bound=2.0, trigger=1500):
    body = ('[layer.product]\npaths = ["src/**"]\n[layer.tests]\npaths = ["tests/**"]\n'
            '[layer.apparatus]\npaths = ["tools/**", "layers.toml"]\n[layer.governance]\npaths = ["*.md"]\n'
            '[layer.planning]\npaths = ["plan/**"]\n'
            '[[exclude]]\npath = "migrated/"\nreason = "staged verbatim, not authored here"\n'
            f'[bounds]\napparatus_ratio_max = {bound}\nlargest_file_trigger = {trigger}\n')
    return body

def repo(files):
    """One commit holding <files>; returns the dir."""
    d = tempfile.mkdtemp(prefix="ly-")
    for name, body in files.items():
        os.makedirs(os.path.dirname(os.path.join(d, name)), exist_ok=True)
        with open(os.path.join(d, name), "w") as f:
            f.write(body)
    g = lambda *a: subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", *a], check=True, capture_output=True)
    g("init", "-q"); g("add", "-A"); g("commit", "-qm", "one")
    return d

def lines(n):
    return "".join(f"l{i}\n" for i in range(n))

class LayersTests(unittest.TestCase):
    def world(self, bound=2.0):
        files = {"layers.toml": config(bound), "migrated/old.rs": lines(900),
                 "src/a.rs": lines(40), "src/b.rs": lines(60),
                 "tests/a.rs": lines(7), "tests/b.rs": lines(13),
                 "tools/a": lines(30), "tools/b": lines(90),
                 "A.md": lines(3), "B.md": lines(4),
                 "plan/a.md": lines(11), "plan/b.md": lines(19)}
        d = repo(files)
        counted = {p: b.count("\n") for p, b in files.items() if not p.startswith("migrated/")}
        by = {"product": ["src/a.rs", "src/b.rs"], "tests": ["tests/a.rs", "tests/b.rs"],
              "apparatus": ["tools/a", "tools/b", "layers.toml"], "governance": ["A.md", "B.md"],
              "planning": ["plan/a.md", "plan/b.md"]}
        totals = {k: sum(counted[p] for p in v) for k, v in by.items()}
        return d, totals, counted

    def test_quiet_exact_denominators(self):
        d, totals, counted = self.world()
        rc, out, err = run(LAYERS, "--repo", d)
        self.assertEqual(rc, 0, out + err)
        got = [l for l in out.splitlines() if l.startswith("layer=")]
        want = [f"layer={n} lines={totals[n]} ratio={totals[n] / totals['product']:.2f}" for n in NAMES]
        self.assertEqual(got, want)
        ratio = totals["apparatus"] / totals["product"]
        largest = max(counted, key=lambda p: (counted[p], p))
        last = out.strip().splitlines()[-1]
        self.assertTrue(last.startswith(f"apparatus_ratio={ratio:.2f} bound=2.0 largest_file={largest} lines={counted[largest]} trigger=1500 over=no tree="), last)
        self.assertTrue(last.endswith(" verdict=PASS"), last)
        self.assertNotIn("migrated", out)

    def test_fire_over_bound_needs_reason(self):
        d, totals, _ = self.world()
        ratio = totals["apparatus"] / totals["product"]
        with open(os.path.join(d, "layers.toml"), "w") as f:
            f.write(config(bound=round(ratio - 0.01, 2)))
        rc, out, _ = run(LAYERS, "--repo", d)
        self.assertEqual(rc, 1, out); self.assertTrue(out.strip().endswith("verdict=FAIL"))
        rc, out, _ = run(LAYERS, "--repo", d, "--reason", "why")
        self.assertEqual(rc, 0, out); self.assertIn("reason=why\n", out); self.assertTrue(out.strip().endswith("verdict=PASS"))
        rc, _, err = run(LAYERS, "--repo", d, "--reason", "")
        self.assertEqual(rc, 2); self.assertIn("empty", err)

    def test_fire_unclassified_path_refused(self):
        d, _, _ = self.world()
        with open(os.path.join(d, "stray.cfg"), "w") as f:
            f.write("x\n")
        subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", "add", "-A"], check=True)
        subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "stray"], check=True)
        rc, out, err = run(LAYERS, "--repo", d)
        self.assertEqual(rc, 2, out + err)
        self.assertIn("unclassified=1\nunclassified path=stray.cfg\n", out)
        self.assertNotIn("ratio=", out); self.assertNotIn("layer=", out)

    def test_fire_exclude_without_reason_refused(self):
        d = repo({"layers.toml": config().replace('reason = "staged verbatim, not authored here"\n', ""), "src/a.rs": lines(2)})
        rc, _, err = run(LAYERS, "--repo", d)
        self.assertEqual(rc, 2); self.assertIn("no reason", err)

    def test_product_without_lines_is_unmeasured(self):
        d = repo({"layers.toml": config(), "A.md": lines(2)})
        rc, out, _ = run(LAYERS, "--repo", d)
        self.assertEqual(rc, 3, out); self.assertIn("apparatus_ratio=UNMEASURED(", out)

    def test_counts_the_tree_not_the_worktree(self):
        d, totals, _ = self.world()
        with open(os.path.join(d, "src/a.rs"), "a") as f:
            f.write(lines(500))
        rc, out, _ = run(LAYERS, "--repo", d)
        self.assertEqual(rc, 0, out); self.assertIn(f"layer=product lines={totals['product']} ", out)

    def test_repo_layers_toml_classifies_every_tracked_path(self):
        rc, out, err = run(LAYERS, "--repo", REPO, timeout=120)
        self.assertIn(rc, (0, 1), out + err)
        self.assertNotIn("unclassified=", out)
        self.assertEqual(len([l for l in out.splitlines() if l.startswith("layer=")]), len(NAMES))
        self.assertRegex(out.strip().splitlines()[-1], r"^apparatus_ratio=\d+\.\d\d bound=\S+ largest_file=\S+ lines=\d+ trigger=\d+ over=(yes|no) tree=[0-9a-f]{12} verdict=(PASS|FAIL)$")

if __name__ == "__main__":
    unittest.main()
