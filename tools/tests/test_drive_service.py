"""The service drive (tools/drive.d/service.py) against disposable `hee4 serve` processes.

One serve is honest; one is started with HEE4_BUSCTL_SHA256 planted to 64 zeros, so its probe
path must be `unavailable` because "busctl digest". Without a user bus every test prints
UNMEASURED and passes. The action's transient-unit path runs only with HEE4_DRIVE_SERVICE_UNIT=1.
"""
import json, os, re, shutil, socket, subprocess, tempfile, time, unittest
from common import TOOLS, run

DRIVE = os.path.join(TOOLS, "drive")
REPO = os.path.dirname(TOOLS)
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.expanduser("~/.cache/hee4-target-gate"))
BIN = os.path.join(TARGET, "debug", "hee4")
FEATURES = "service.inspect,service.probe,service.action"
XDG = os.environ.get("XDG_RUNTIME_DIR", "")
BUS = os.path.join(XDG, "bus") if XDG else ""


def serve(d, env_extra=None):
    os.mkdir(os.path.join(d, "rt"), 0o700)
    sock, ledger, log = os.path.join(d, "rt", "control.sock"), os.path.join(d, "l.sqlite3"), os.path.join(d, "serve.log")
    env = {k: v for k, v in os.environ.items() if k not in ("HEE4_LIVE_MODEL", "HEE4_BUSCTL_SHA256")}
    env.update(env_extra or {})
    with open(log, "w") as f:
        srv = subprocess.Popen([BIN, "serve", "--socket", sock, "--ledger", ledger, "--work", os.path.join(d, "w")],
                               stdout=f, stderr=subprocess.STDOUT, env=env)
    for _ in range(150):
        if os.path.exists(sock):
            break
        time.sleep(0.1)
    return srv, sock, ledger, log


def frame(sock, obj):
    s = socket.socket(socket.AF_UNIX); s.settimeout(20)
    try:
        s.connect(sock); s.sendall(json.dumps(obj).encode() + b"\n")
        return json.loads(s.makefile("rb").readline())
    finally:
        s.close()


class ServiceDriveTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        r = subprocess.run(["cargo", "build", "-p", "hee4-app", "--offline"], cwd=REPO, capture_output=True, text=True,
                           env={**os.environ, "CARGO_TARGET_DIR": TARGET})
        assert r.returncode == 0, r.stderr[-800:]
        cls.d = tempfile.mkdtemp(prefix="drvs-")
        cls.srv, cls.sock, cls.ledger, cls.log = serve(cls.d)

    @classmethod
    def tearDownClass(cls):
        cls.srv.kill(); cls.srv.wait(); shutil.rmtree(cls.d, ignore_errors=True)

    def no_bus(self):
        if not (BUS and os.path.exists(BUS)):
            print("UNMEASURED: $XDG_RUNTIME_DIR/bus absent; service drive not run")
            return True
        return False

    def line(self, out, feat):
        got = [l for l in out.splitlines() if l.startswith(f"drive feature={feat} ")]
        self.assertEqual(len(got), 1, out)
        return got[0]

    def test_three_service_features_drive_and_serve_logs_the_digest(self):
        if self.no_bus():
            return
        rc, out, _ = run(DRIVE, "--socket", self.sock, "--ledger", self.ledger, "--evidence-root", os.path.join(self.d, "ev"),
                         "--only", FEATURES, timeout=110)
        self.assertNotIn("verdict=FAIL", out)
        self.assertIn("verdict=PASS ", self.line(out, "service.inspect"))
        self.assertIn("verdict=PASS ", self.line(out, "service.probe"))
        action = self.line(out, "service.action")
        if os.environ.get("HEE4_DRIVE_SERVICE_UNIT") == "1":
            self.assertIn("verdict=PASS ", action)
            self.assertEqual(rc, 0, out)
        else:
            print("UNMEASURED: HEE4_DRIVE_SERVICE_UNIT=1 not set; service.action unit paths not driven")
            self.assertRegex(action, r"verdict=UNMEASURED paths=\d+/\d+ ")
        with open(self.log) as f:
            self.assertRegex(f.read(), r"busctl_sha256=[0-9a-f]{64} source=measured")

    def test_planted_digest_makes_the_probe_unavailable_by_name(self):
        if self.no_bus():
            return
        d = tempfile.mkdtemp(prefix="drvs-pin-")
        srv, sock, _, log = serve(d, {"HEE4_BUSCTL_SHA256": "0" * 64})
        try:
            rc, out, _ = run(DRIVE, "--socket", sock, "--evidence-root", os.path.join(d, "ev"), "--only", "service.probe", timeout=60)
            self.assertIn("verdict=FAIL", self.line(out, "service.probe"))
            r = frame(sock, {"request_id": "pin", "action": "service.probe", "action_version": 1, "idempotency_key": "pin-1",
                             "body": {"service_id": "drive", "probe_id": "active_state", "probe_version": 1,
                                      "max_cost_microunits": 0, "network_scope": "none"}})
            self.assertEqual((r.get("code"), r.get("retry")), ("unavailable", "after_condition"), r)
            self.assertIn("busctl digest", r.get("because", ""), r)
            with open(log) as f:
                self.assertRegex(f.read(), r"busctl_sha256=0{64} source=env")
        finally:
            srv.kill(); srv.wait(); shutil.rmtree(d, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
