import http.server, json, os, socket, stat, tempfile, threading, unittest
from common import TOOLS, run

DOCTOR, DRILL = os.path.join(TOOLS, "doctor"), os.path.join(TOOLS, "drill")

def stub(dirpath, name, body):
    p = os.path.join(dirpath, name)
    with open(p, "w") as f:
        f.write("#!/usr/bin/env bash\n" + body)
    os.chmod(p, 0o755)

class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        b = json.dumps({"models": [{"name": "m", "size_vram": 1}]}).encode()
        self.send_response(200); self.end_headers(); self.wfile.write(b)
    def log_message(self, *a): pass

class World:
    """Fake unit, socket, hee4 and model, all under a temp dir."""
    def __init__(self, drill=False):
        self.d = tempfile.mkdtemp(prefix="dw-")
        self.bin = os.path.join(self.d, "bin"); os.mkdir(self.bin)
        rt = os.path.join(self.d, "rt"); os.mkdir(rt, 0o700)
        self.sockpath = os.path.join(rt, "control.sock")
        self.s = socket.socket(socket.AF_UNIX); self.s.bind(self.sockpath); os.chmod(self.sockpath, 0o600)
        self.srv = http.server.HTTPServer(("127.0.0.1", 0), H)
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.srv.server_port}"
        self.pidfile = os.path.join(self.d, "pid")
        self.proc = None
        self.start_main()
        stub(self.bin, "systemctl", f'echo LoadState=loaded; echo ActiveState=active; echo MainPID=$(cat {self.pidfile})\n')
        stub(self.bin, "hee4", 'case "$1" in --version) echo "hee4 HEADSHA";; *) echo "ready=true recovery=complete database=ready socket=owned";; esac\n')
        self.env = {"PATH": self.bin + ":" + os.environ["PATH"]}
    def start_main(self):
        import subprocess
        self.proc = subprocess.Popen(["sleep", "300"])
        open(self.pidfile, "w").write(str(self.proc.pid))
        threading.Thread(target=self.proc.wait, daemon=True).start()  # reap, so a killed main is gone, not a zombie
    def close(self):
        self.srv.shutdown(); self.s.close()
        if self.proc: self.proc.kill()

def head_of(repo):
    return run("git", "-C", repo, "rev-parse", "HEAD")[1].strip()[:12]

class DoctorTests(unittest.TestCase):
    def test_help(self):
        for t in (DOCTOR, DRILL):
            rc, out, _ = run(t, "--help"); self.assertEqual(rc, 0); self.assertIn("verdict=", out)

    def test_fire_absent_unit_is_unmeasured_never_pass(self):
        w = World(); self.addCleanup(w.close)
        rc, out, _ = run(DOCTOR, "--unit", "nope-xyz.service", "--socket", "/nonexistent/s", "--model-url", w.url, "--repo", TOOLS)
        self.assertEqual(rc, 3, out)
        self.assertIn("check=unit status=UNMEASURED", out); self.assertIn("check=socket status=UNMEASURED", out)
        self.assertIn("check=tool_cargo status=MEASURED", out)
        self.assertRegex(out.strip().splitlines()[-1], r"^doctor verdict=UNMEASURED ")

    def test_fire_model_down_is_fail(self):
        rc, out, _ = run(DOCTOR, "--unit", "nope.service", "--model-url", "http://127.0.0.1:9", "--repo", TOOLS)
        self.assertEqual(rc, 1); self.assertIn("check=model status=FAIL", out)

    def test_quiet_healthy_world_passes(self):
        w = World(); self.addCleanup(w.close)
        head = head_of(TOOLS)
        stub(w.bin, "hee4", f'case "$1" in --version) echo "hee4 {head}";; *) echo "ready=true recovery=complete database=ready socket=owned";; esac\n')
        rc, out, _ = run(DOCTOR, "--socket", w.sockpath, "--model-url", w.url, "--repo", TOOLS, env=w.env)
        self.assertEqual(rc, 0, out)
        self.assertRegex(out.strip().splitlines()[-1], r"^doctor verdict=PASS measured=(\d+)/\1 unmeasured=0 fail=0 ")

    def test_fire_bad_socket_perms(self):
        w = World(); self.addCleanup(w.close)
        os.chmod(w.sockpath, 0o666)
        rc, out, _ = run(DOCTOR, "--socket", w.sockpath, "--model-url", w.url, "--repo", TOOLS, env=w.env)
        self.assertEqual(rc, 1); self.assertIn("check=socket status=FAIL", out)

class DrillTests(unittest.TestCase):
    def test_fire_absent_unit_every_step_unmeasured(self):
        rc, out, _ = run(DRILL, "--unit", "nope-xyz.service", "--repo", TOOLS)
        self.assertEqual(rc, 3)
        self.assertEqual(out.count("status=UNMEASURED"), out.count("drill_step="))
        self.assertNotIn("status=MEASURED", out)
        self.assertRegex(out.strip().splitlines()[-1], r"^drill verdict=UNMEASURED steps=0/")

    def test_quiet_kill9_and_restart(self):
        w = World(); self.addCleanup(w.close)
        old = int(open(w.pidfile).read())
        # a restarter: when the old main dies, a new one appears
        stub(w.bin, "restarter", f'while kill -0 {old} 2>/dev/null; do sleep 0.1; done; sleep 0.5; sleep 300 & echo $! > {w.pidfile}; wait\n')
        import subprocess
        r = subprocess.Popen([os.path.join(w.bin, "restarter")], env={**os.environ, **w.env}, start_new_session=True)
        self.addCleanup(lambda: (os.killpg(r.pid, 9), None))
        rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "10", "--repo", TOOLS, env=w.env)
        self.assertEqual(rc, 0, out)
        self.assertRegex(out.strip().splitlines()[-1], r"^drill verdict=PASS steps=(\d+)/\1 ")
        self.assertIn(f"old={old}", out)

    def test_fire_unit_never_restarts(self):
        w = World(); self.addCleanup(w.close)
        rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "1", "--repo", TOOLS, env=w.env)
        self.assertEqual(rc, 1); self.assertIn("drill_step=unit_restarted status=FAIL", out)

if __name__ == "__main__":
    unittest.main()
