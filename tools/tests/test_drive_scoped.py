"""tools/drive.d/scoped.py through tools/drive: an out-of-release feature is driven by its refusal.

Out of release means tools.inspect says served=false (the owner is not composed in this binary),
never a version. A planted serve (a thread on an AF_UNIX socket) answers tools.inspect with a
planted `scope` and `served` and the action with a planted reply; the scope and its `because`
text come from tools.inspect and from catalogue.rs, so each test changes the plant and the
expectation follows it. RealServeTests drives a disposable `hee4 serve` directly, no proxy.
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
UNSERVED_V42 = ("thread.get", "thread.list", "analysis.get", "analysis.request")


class PlantedServe(threading.Thread):
    """tools.inspect -> {action, version, effect} plus inspect[action] (e.g. {scope, served});
    any other action -> replies[action](request)."""

    def __init__(self, path, inspect, replies):
        super().__init__(daemon=True)
        self.inspect, self.replies, self.seen = inspect, replies, []
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
                    body = {"action": name, "version": 1, "effect": "read", **self.inspect.get(name, {})}
                    reply = {"kind": "result", "request_id": rid, "replayed": False, "body": body}
                else:
                    reply = self.replies[act](req)
                    reply["request_id"] = rid
                c.sendall(json.dumps(reply).encode() + b"\n")
            except (OSError, ValueError, KeyError):
                pass
            c.close()


def unserved(scope):
    return {"scope": scope, "served": False}


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

    def drive(self, inspect, replies, features):
        srv = PlantedServe(self.sock, inspect, replies); srv.start()
        rc, out, err = run(DRIVE, "--socket", self.sock, "--repo", plant_repo(features),
                           "--evidence-root", os.path.join(self.d, "ev"), timeout=60)
        srv.s.close()
        return rc, out, err, srv

    def line(self, out, name):
        hits = [l for l in out.splitlines() if l.startswith(f"drive feature={name} ")]
        self.assertEqual(len(hits), 1, out)
        return hits[0]

    def test_out_of_release_features_pass_by_their_refusal(self):
        names = list(UNSERVED_V42)
        rc, out, err, srv = self.drive({n: unserved("v42") for n in names}, {n: refused("v42") for n in names},
                                       {n: real_feature(n) for n in names})
        for n in names:
            self.assertRegex(self.line(out, n),
                             r"verdict=PASS paths=2/2 .* scope=v4\.2 served=false \(refused by release scope, as catalogued\)$")
        self.assertRegex(out.splitlines()[-1], rf"^drive verdict=PASS features={len(names)}/{len(names)} unserved=0 ")
        self.assertEqual(rc, 0, out + err)
        # the action got the minimal well-formed body named by its feature file
        sent = {r["action"]: r["body"] for r in srv.seen if r["action"] != "tools.inspect"}
        self.assertEqual(sorted(sent["thread.get"]), ["expected_brief_revision", "thread_id"])

    def test_a_served_answer_fails_the_scoped_path_by_name(self):
        ok = lambda req: {"kind": "result", "replayed": False, "body": {"thread_id": "x", "state": "open"}}
        rc, out, _, _ = self.drive({"thread.get": unserved("v42")}, {"thread.get": ok},
                                   {"thread.get": real_feature("thread.get")})
        self.assertIn("verdict=FAIL paths=1/2", self.line(out, "thread.get"))
        self.assertRegex(out, r"path=refused_by_scope status=FAIL detail=expected error unavailable")
        self.assertRegex(out.splitlines()[-1], r"^drive verdict=FAIL ")
        self.assertEqual(rc, 1)

    def test_a_served_action_with_no_procedure_stays_unmeasured(self):
        # served=true decides, even with a scope other than v4.0: never driven by its refusal
        rc, out, _, srv = self.drive({"zz.planted": {"scope": "v42", "served": True}}, {},
                                     {"zz.planted": "# zz.planted\n\n## Driving it with hee4\n\nnone\n"})
        line = self.line(out, "zz.planted")
        self.assertIn("verdict=UNMEASURED paths=0/0", line)
        self.assertIn("scope=unserved reason=no procedure in tools/drive.d (tools.inspect zz.planted says served=true)", line)
        self.assertNotIn("zz.planted", [r["action"] for r in srv.seen])
        self.assertRegex(out.splitlines()[-1], r"unserved=1 ")

    def test_out_of_release_is_served_false_never_the_version(self):
        # a v4.0-scoped action this binary does not serve is still driven by its refusal
        rc, out, _, _ = self.drive({"thread.get": unserved("v40")}, {"thread.get": refused("v40")},
                                   {"thread.get": real_feature("thread.get")})
        self.assertRegex(self.line(out, "thread.get"), r"verdict=PASS paths=2/2 .* scope=v4\.0 served=false ")
        self.assertEqual(rc, 0, out)

    def test_no_served_member_stays_unmeasured_naming_it(self):
        rc, out, _, _ = self.drive({"thread.get": {"scope": "v42"}}, {}, {"thread.get": real_feature("thread.get")})
        self.assertIn("scope=unserved reason=no procedure in tools/drive.d (tools.inspect thread.get carries no served member)",
                      self.line(out, "thread.get"))

    def test_the_scope_comes_from_tools_inspect(self):
        others = sorted(s for s in BECAUSE if s != "v42")
        self.assertTrue(others, BECAUSE)
        planted = others[0]
        feats = {"thread.get": real_feature("thread.get")}
        rc, out, _, _ = self.drive({"thread.get": unserved(planted)}, {"thread.get": refused(planted)}, feats)
        self.assertIn("verdict=PASS paths=2/2", self.line(out, "thread.get"))
        self.assertIn(f"scope={scoped.display(planted)} served=false (refused", self.line(out, "thread.get"))
        os.unlink(self.sock)
        # the serve still refuses with v4.2's text while tools.inspect now says `planted`: the because no longer matches
        rc, out, _, _ = self.drive({"thread.get": unserved(planted)}, {"thread.get": refused("v42")}, feats)
        self.assertIn("verdict=FAIL", self.line(out, "thread.get"))
        self.assertRegex(out, r"path=refused_by_scope status=FAIL detail=because=")
        self.assertEqual(rc, 1)


class RealServeTests(unittest.TestCase):
    """A disposable `hee4 serve`, driven directly: no proxy, tools.inspect's own scope and served."""

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

    def inspect(self, name):
        c = socket.socket(socket.AF_UNIX); c.connect(self.sock)
        c.sendall(json.dumps({"request_id": "t", "action": "tools.inspect", "action_version": 1,
                              "idempotency_key": None, "body": {"action": name, "version": 1}}).encode() + b"\n")
        reply = json.loads(c.makefile("rb").readline()); c.close()
        return reply.get("body") or {}

    def test_every_unserved_feature_passes_by_its_refusal_on_a_disposable_serve(self):
        fdir = os.path.join(REPO, "gates", "features")
        plugins = sorted(drive_d.load_plugins(os.path.join(TOOLS, "drive.d"), fdir))
        no_procedure = sorted(n for n in (f[:-3] for f in os.listdir(fdir) if f.endswith(".md") and f != "README.md")
                              if n not in plugins)
        inspected = {n: self.inspect(n) for n in no_procedure}
        out_of_release = [n for n in no_procedure if inspected[n].get("served") is False]
        for n in UNSERVED_V42:
            self.assertIn(n, out_of_release, inspected)
        rc, out, err = run(DRIVE, "--socket", self.sock, "--only", ",".join(out_of_release), "--evidence-root",
                           os.path.join(self.d, "ev"), timeout=100)
        for n in out_of_release:
            want = scoped.display(inspected[n]["scope"])
            hit = [l for l in out.splitlines() if l.startswith(f"drive feature={n} ")]
            self.assertEqual(len(hit), 1, out)
            self.assertRegex(hit[0], rf"verdict=PASS paths=2/2 .* scope={re.escape(want)} served=false "
                                     r"\(refused by release scope, as catalogued\)$")
        self.assertRegex(out.splitlines()[-1], rf"^drive verdict=PASS features={len(out_of_release)}/{len(out_of_release)} ")
        self.assertEqual(rc, 0, out + err)

    def test_only_leaves_an_unnamed_unserved_feature_undriven(self):
        rc, out, err = run(DRIVE, "--socket", self.sock, "--only", "thread.get", "--evidence-root",
                           os.path.join(self.d, "ev2"), timeout=100)
        self.assertRegex(out, r"(?m)^drive feature=thread\.get verdict=PASS paths=2/2 .* served=false ")
        self.assertRegex(out, r"(?m)^drive feature=thread\.list verdict=UNMEASURED .*scope=unserved "
                              r"reason=no procedure in tools/drive\.d \(not selected by --only\)$")
        self.assertEqual(rc, 0, out + err)


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
