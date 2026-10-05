"""tools/drive.d/scoped.py through tools/drive: an out-of-release feature is driven by its refusal.

A planted serve (a thread on an AF_UNIX socket) answers tools.inspect with a planted scope and the
action with a planted reply. The scope and its `because` text come from tools.inspect and from
catalogue.rs, so each test changes the plant and the expectation follows it.
"""
import json, os, re, shutil, socket, subprocess, sys, tempfile, threading, time, unittest
from common import TOOLS, run

sys.path.insert(0, TOOLS)
from test_drive import load_tool  # noqa: E402

DRIVE = os.path.join(TOOLS, "drive")
REPO = os.path.dirname(TOOLS)
drive_d = load_tool("drive_d", os.path.join(TOOLS, "drive.d", "__init__.py"))
scoped = load_tool("drive_d.scoped", os.path.join(TOOLS, "drive.d", "scoped.py"))
BECAUSE = scoped.scope_because(REPO)


class PlantedServe(threading.Thread):
    """tools.inspect -> {action, scope: scopes[action]} (no scope member when absent);
    any other action -> replies[action](request)."""

    def __init__(self, path, scopes, replies):
        super().__init__(daemon=True)
        self.scopes, self.replies, self.seen = scopes, replies, []
        self.s = socket.socket(socket.AF_UNIX); self.s.bind(path); self.s.listen(16)

    def run(self):
        while True:
            try:
                c, _ = self.s.accept()
            except OSError:
                return
            try:
                req = json.loads(c.makefile("rb").readline())
                self.seen.append(req)
                act, rid = req.get("action"), req.get("request_id")
                if act == "tools.inspect":
                    name = req["body"]["action"]
                    body = {"action": name, "version": 1, "effect": "read"}
                    if name in self.scopes:
                        body["scope"] = self.scopes[name]
                    reply = {"kind": "result", "request_id": rid, "replayed": False, "body": body}
                else:
                    reply = self.replies[act](req)
                    reply["request_id"] = rid
                c.sendall(json.dumps(reply).encode() + b"\n")
            except (OSError, ValueError, KeyError):
                pass
            c.close()


def refused(scope):
    return lambda req: {"kind": "error", "code": "unavailable", "field": "/action", "retry": "after_condition",
                        "message": "owner not registered in this release", "because": BECAUSE[scope]}


def plant_repo(features):
    """Cargo.toml, catalogue.rs, FLOW.md, drive.d with only __init__ and scoped, and the named feature files."""
    r = tempfile.mkdtemp(prefix="drv-scoped-")
    os.makedirs(os.path.join(r, "tools", "drive.d")); os.makedirs(os.path.join(r, "gates", "features"))
    for f in ("__init__.py", "scoped.py"):
        shutil.copy(os.path.join(TOOLS, "drive.d", f), os.path.join(r, "tools", "drive.d", f))
    for rel in ("Cargo.toml", scoped.CATALOGUE, "crates/hee4-app/FLOW.md"):
        os.makedirs(os.path.dirname(os.path.join(r, rel)) or r, exist_ok=True)
        shutil.copy(os.path.join(REPO, rel), os.path.join(r, rel))
    for name, text in features.items():
        with open(os.path.join(r, "gates", "features", name + ".md"), "w") as f:
            f.write(text)
    return r


def real_feature(name):
    with open(os.path.join(REPO, "gates", "features", name + ".md")) as f:
        return f.read()


class ScopedDriveTests(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp(prefix="drv-sc-")
        self.sock = os.path.join(self.d, "s.sock")

    def drive(self, scopes, replies, features):
        srv = PlantedServe(self.sock, scopes, replies); srv.start()
        rc, out, err = run(DRIVE, "--socket", self.sock, "--repo", plant_repo(features),
                           "--evidence-root", os.path.join(self.d, "ev"), timeout=60)
        srv.s.close()
        return rc, out, err, srv

    def line(self, out, name):
        hits = [l for l in out.splitlines() if l.startswith(f"drive feature={name} ")]
        self.assertEqual(len(hits), 1, out)
        return hits[0]

    def test_out_of_release_features_pass_by_their_refusal(self):
        names = ["thread.get", "thread.list", "analysis.get", "analysis.request"]
        rc, out, err, srv = self.drive({n: "v42" for n in names}, {n: refused("v42") for n in names},
                                       {n: real_feature(n) for n in names})
        for n in names:
            self.assertRegex(self.line(out, n), r"verdict=PASS paths=2/2 .* scope=v4\.2 \(refused by release scope, as catalogued\)$")
        self.assertRegex(out.splitlines()[-1], rf"^drive verdict=PASS features={len(names)}/{len(names)} unserved=0 ")
        self.assertEqual(rc, 0, out + err)
        # the action got the minimal well-formed body named by its feature file
        sent = {r["action"]: r["body"] for r in srv.seen if r["action"] != "tools.inspect"}
        self.assertEqual(sorted(sent["thread.get"]), ["expected_brief_revision", "thread_id"])

    def test_a_served_answer_fails_the_scoped_path_by_name(self):
        ok = lambda req: {"kind": "result", "replayed": False, "body": {"thread_id": "x", "state": "open"}}
        rc, out, _, _ = self.drive({"thread.get": "v42"}, {"thread.get": ok}, {"thread.get": real_feature("thread.get")})
        self.assertIn("verdict=FAIL paths=1/2", self.line(out, "thread.get"))
        self.assertRegex(out, r"path=refused_by_scope status=FAIL detail=expected error unavailable")
        self.assertRegex(out.splitlines()[-1], r"^drive verdict=FAIL ")
        self.assertEqual(rc, 1)

    def test_a_v40_feature_with_no_procedure_stays_unmeasured(self):
        release = scoped.release_scope(REPO)
        rc, out, _, srv = self.drive({"zz.planted": release}, {}, {"zz.planted": "# zz.planted\n\n## Driving it with hee4\n\nnone\n"})
        line = self.line(out, "zz.planted")
        self.assertIn("verdict=UNMEASURED paths=0/0", line)
        self.assertIn("scope=unserved reason=no procedure in tools/drive.d", line)
        self.assertIn(f"scope {scoped.display(release)} is this release's", line)
        self.assertNotIn("zz.planted", [r["action"] for r in srv.seen])  # in-release: never driven by refusal
        self.assertRegex(out.splitlines()[-1], r"unserved=1 ")

    def test_no_scope_member_stays_unmeasured_naming_it(self):
        rc, out, _, _ = self.drive({}, {}, {"thread.get": real_feature("thread.get")})
        self.assertIn("scope=unserved reason=no procedure in tools/drive.d (tools.inspect thread.get carries no scope member)",
                      self.line(out, "thread.get"))

    def test_the_scope_comes_from_tools_inspect(self):
        others = sorted(s for s in BECAUSE if s not in ("v42", scoped.release_scope(REPO)))
        self.assertTrue(others, BECAUSE)
        planted = others[0]
        feats = {"thread.get": real_feature("thread.get")}
        rc, out, _, _ = self.drive({"thread.get": planted}, {"thread.get": refused(planted)}, feats)
        self.assertIn(f"verdict=PASS paths=2/2", self.line(out, "thread.get"))
        self.assertIn(f"scope={scoped.display(planted)} (refused", self.line(out, "thread.get"))
        os.unlink(self.sock)
        # the serve still refuses with v4.2's text while tools.inspect now says `planted`: the because no longer matches
        rc, out, _, _ = self.drive({"thread.get": planted}, {"thread.get": refused("v42")}, feats)
        self.assertIn("verdict=FAIL", self.line(out, "thread.get"))
        self.assertRegex(out, r"path=refused_by_scope status=FAIL detail=because=")
        self.assertEqual(rc, 1)


def catalogue_scopes():
    """id -> wire scope, from the catalogue entries in catalogue.rs (the server's own table)."""
    with open(os.path.join(REPO, scoped.CATALOGUE)) as f:
        src = f.read()
    return {i: v.lower() for i, v in re.findall(r'id: "([\w.]+)",.*?scope: Scope::(\w+),', src, re.S)}


class InjectScope(threading.Thread):
    """Forwards each frame to a real serve; adds `scope` (from catalogue.rs) to tools.inspect results.

    The real engine's tools.inspect carries no scope member today (tools.rs inspect); this proxy
    is the plant that shows the real refusals agree with the catalogue once it does."""

    def __init__(self, path, upstream):
        super().__init__(daemon=True)
        self.up, self.scopes = upstream, catalogue_scopes()
        self.s = socket.socket(socket.AF_UNIX); self.s.bind(path); self.s.listen(16)

    def run(self):
        while True:
            try:
                c, _ = self.s.accept()
            except OSError:
                return
            try:
                line = c.makefile("rb").readline()
                u = socket.socket(socket.AF_UNIX); u.connect(self.up); u.sendall(line)
                reply = json.loads(u.makefile("rb").readline()); u.close()
                body = reply.get("body") if reply.get("kind") == "result" else None
                if json.loads(line).get("action") == "tools.inspect" and isinstance(body, dict):
                    body["scope"] = self.scopes.get(body.get("action"))
                c.sendall(json.dumps(reply).encode() + b"\n")
            except (OSError, ValueError):
                pass
            c.close()


class RealServeTests(unittest.TestCase):
    """A disposable `hee4 serve`; tools.inspect's scope injected from catalogue.rs by InjectScope."""

    @classmethod
    def setUpClass(cls):
        target = os.environ.get("CARGO_TARGET_DIR", os.path.expanduser("~/.cache/hee4-target-gate"))
        r = subprocess.run(["cargo", "build", "-p", "hee4-app", "--offline"], cwd=REPO, capture_output=True, text=True,
                           env={**os.environ, "CARGO_TARGET_DIR": target})
        assert r.returncode == 0, r.stderr[-800:]
        cls.d = tempfile.mkdtemp(prefix="drv-scr-")
        os.mkdir(os.path.join(cls.d, "rt"), 0o700)
        cls.sock = os.path.join(cls.d, "rt", "control.sock")
        cls.srv = subprocess.Popen([os.path.join(target, "debug", "hee4"), "serve", "--socket", cls.sock, "--ledger",
                                    os.path.join(cls.d, "l.sqlite3"), "--work", os.path.join(cls.d, "w")],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                   env={k: v for k, v in os.environ.items() if k != "HEE4_LIVE_MODEL"})
        for _ in range(100):
            if os.path.exists(cls.sock):
                break
            time.sleep(0.1)

    @classmethod
    def tearDownClass(cls):
        cls.srv.kill(); cls.srv.wait(); shutil.rmtree(cls.d, ignore_errors=True)

    def test_every_out_of_release_feature_passes_on_a_disposable_serve(self):
        release = scoped.release_scope(REPO)
        fdir = os.path.join(REPO, "gates", "features")
        unserved = sorted(n for n in (f[:-3] for f in os.listdir(fdir) if f.endswith(".md") and f != "README.md")
                          if n not in drive_d.load_plugins(os.path.join(TOOLS, "drive.d"), fdir))
        out_of_release = [n for n in unserved if catalogue_scopes().get(n, release) != release]
        self.assertTrue(out_of_release, unserved)
        p = os.path.join(self.d, "rt", "inject.sock")
        InjectScope(p, self.sock).start()
        plugins = sorted(drive_d.load_plugins(os.path.join(TOOLS, "drive.d"), fdir))
        rc, out, err = run(DRIVE, "--socket", p, "--only", plugins[0], "--evidence-root", os.path.join(self.d, "ev"), timeout=100)
        for n in out_of_release:
            want = scoped.display(catalogue_scopes()[n])
            hit = [l for l in out.splitlines() if l.startswith(f"drive feature={n} ")]
            self.assertEqual(len(hit), 1, out)
            self.assertRegex(hit[0], rf"verdict=PASS paths=2/2 .* scope={re.escape(want)} \(refused by release scope, as catalogued\)$")
        self.assertNotIn("scope=unserved", "\n".join(l for l in out.splitlines() if any(f"feature={n} " in l for n in out_of_release)))


class ScopeTableTests(unittest.TestCase):
    def test_because_texts_are_read_from_the_catalogue(self):
        with open(os.path.join(REPO, scoped.CATALOGUE)) as f:
            src = f.read()
        variants = re.findall(r"^    (V\d+|Held),$", src.split("pub enum Scope {", 1)[1].split("}", 1)[0], re.M)
        self.assertEqual(len(BECAUSE), len(variants), BECAUSE)
        self.assertTrue(all(t and t in src for t in BECAUSE.values()), BECAUSE)

    def test_minimal_body_names_each_socket_member(self):
        self.assertEqual(scoped.minimal_body("Socket: request `body` `{task_id, states[], page}` (FACT)"),
                         {"task_id": "drive-scope", "states": "drive-scope", "page": "drive-scope"})
        self.assertEqual(scoped.minimal_body("no socket line"), {})


if __name__ == "__main__":
    unittest.main()
