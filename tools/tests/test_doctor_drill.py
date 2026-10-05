import http.server, json, os, socket, stat, tempfile, threading, unittest, uuid
from common import TOOLS, run

DOCTOR, DRILL = os.path.join(TOOLS, "doctor"), os.path.join(TOOLS, "drill")

def stub(dirpath, name, body):
    p = os.path.join(dirpath, name)
    with open(p, "w") as f:
        f.write("#!/usr/bin/env bash\n" + body)
    os.chmod(p, 0o755)

def ps_handler(models):
    class H(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            b = json.dumps({"models": models}).encode()
            self.send_response(200); self.end_headers(); self.wfile.write(b)
        def log_message(self, *a): pass
    return H

class World:
    """Fake unit, socket (answering health/task.submit/task.list), hee4 and model, all under a temp dir.
    `missing` drops that many of the submitted ids from task.list (the plant)."""
    def __init__(self, models=None, missing=0):
        self.d = tempfile.mkdtemp(prefix="dw-")
        self.bin = os.path.join(self.d, "bin"); os.mkdir(self.bin)
        rt = os.path.join(self.d, "rt"); os.mkdir(rt, 0o700)
        self.sockpath = os.path.join(rt, "control.sock")
        self.s = socket.socket(socket.AF_UNIX); self.s.bind(self.sockpath); os.chmod(self.sockpath, 0o600); self.s.listen(8)
        self.missing, self.submitted = missing, []
        threading.Thread(target=self.serve, daemon=True).start()
        self.srv = http.server.HTTPServer(("127.0.0.1", 0), ps_handler([{"name": "m", "size_vram": 1}] if models is None else models))
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.srv.server_port}"
        self.pidfile = os.path.join(self.d, "pid")
        self.proc = None
        self.start_main()
        stub(self.bin, "systemctl", f'echo LoadState=loaded; echo ActiveState=active; echo MainPID=$(cat {self.pidfile})\n')
        stub(self.bin, "hee4", 'case "$1" in --version) echo "hee4 HEADSHA";; *) echo "ready=true recovery=complete database=ready socket=owned";; esac\n')
        self.env = {"PATH": self.bin + ":" + os.environ["PATH"]}
    def serve(self):
        while True:
            try:
                c, _ = self.s.accept()
            except OSError:
                return
            try:
                req = json.loads(c.makefile("rb").readline() or b"{}")
                act = req.get("action")
                if act == "task.submit":
                    t = "t-" + uuid.uuid4().hex[:24]; self.submitted.append(t)
                    body = {"task_id": t, "phase": "admitted"}
                elif act == "task.list":
                    shown = self.submitted[:len(self.submitted) - self.missing] if self.missing else self.submitted
                    body = {"tasks": [{"task_id": t, "phase": "admitted"} for t in shown]}
                else:
                    body = {"ok": True, "head_sha": "0" * 40, "recovery_complete": True, "uptime_s": 1}
                c.sendall((json.dumps({"kind": "result", "request_id": req.get("request_id"), "replayed": False, "body": body}) + "\n").encode())
            except (OSError, ValueError):
                pass
            c.close()
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

def healthy_doctor(w, *extra):
    head = head_of(TOOLS)
    stub(w.bin, "hee4", f'case "$1" in --version) echo "hee4 {head}";; *) echo "ready=true recovery=complete database=ready socket=owned";; esac\n')
    return run(DOCTOR, "--socket", w.sockpath, "--model-url", w.url, "--repo", TOOLS, *extra, env=w.env)

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
        self.assertEqual(rc, 1); self.assertIn("check=model_endpoint status=FAIL", out)

    def test_quiet_healthy_world_passes(self):
        w = World(); self.addCleanup(w.close)
        rc, out, _ = healthy_doctor(w, "--model", "m")
        self.assertEqual(rc, 0, out)
        self.assertIn("check=model_endpoint status=MEASURED", out); self.assertIn("check=model_gpu status=MEASURED", out)
        self.assertRegex(out.strip().splitlines()[-1], r"^doctor verdict=PASS measured=(\d+)/\1 unmeasured=0 fail=0 advisory=0 ")

    def test_quiet_idle_endpoint_gpu_is_advisory(self):
        w = World(models=[]); self.addCleanup(w.close)
        rc, out, _ = healthy_doctor(w, "--model", "m")
        self.assertEqual(rc, 0, out)
        self.assertIn("check=model_endpoint status=MEASURED", out)
        gpu = [l for l in out.splitlines() if l.startswith("check=model_gpu ")][0]
        self.assertIn("status=UNMEASURED", gpu); self.assertIn("not resident", gpu); self.assertIn("scope=advisory", gpu)
        self.assertRegex(out.strip().splitlines()[-1], r"^doctor verdict=PASS .* advisory=1 ")
        self.assertEqual([l for l in out.splitlines() if "status=MEASURED" in l and "UNMEASURED" in l], [])

    def test_fire_idle_endpoint_with_require_gpu_is_unmeasured(self):
        w = World(models=[]); self.addCleanup(w.close)
        rc, out, _ = healthy_doctor(w, "--model", "m", "--require-gpu")
        self.assertEqual(rc, 3, out)
        self.assertIn("check=model_gpu status=UNMEASURED", out); self.assertIn("scope=required", out)
        self.assertRegex(out.strip().splitlines()[-1], r"^doctor verdict=UNMEASURED .* advisory=0 ")

    def test_fire_cpu_copy_is_fail(self):
        w = World(models=[{"name": "m", "size_vram": 0}]); self.addCleanup(w.close)
        rc, out, _ = healthy_doctor(w, "--model", "m")
        self.assertEqual(rc, 1, out); self.assertIn("check=model_gpu status=FAIL", out)

    def test_fire_bad_socket_perms(self):
        w = World(); self.addCleanup(w.close)
        os.chmod(w.sockpath, 0o666)
        rc, out, _ = run(DOCTOR, "--socket", w.sockpath, "--model-url", w.url, "--repo", TOOLS, env=w.env)
        self.assertEqual(rc, 1); self.assertIn("check=socket status=FAIL", out)

def restarter(tc, w):
    """When the old main dies, a new one appears (the unit's Restart=)."""
    old = int(open(w.pidfile).read())
    stub(w.bin, "restarter", f'while kill -0 {old} 2>/dev/null; do sleep 0.1; done; sleep 0.5; sleep 300 & echo $! > {w.pidfile}; wait\n')
    import subprocess
    r = subprocess.Popen([os.path.join(w.bin, "restarter")], env={**os.environ, **w.env}, start_new_session=True)
    tc.addCleanup(lambda: (os.killpg(r.pid, 9), None))
    return old

class DrillTests(unittest.TestCase):
    def test_fire_absent_unit_every_step_unmeasured(self):
        rc, out, _ = run(DRILL, "--unit", "nope-xyz.service", "--repo", TOOLS, "--drill-root", tempfile.mkdtemp(prefix="dr-"))
        self.assertEqual(rc, 3)
        self.assertEqual(out.count("status=UNMEASURED"), out.count("drill_step="))
        self.assertNotIn("status=MEASURED", out)
        self.assertRegex(out.strip().splitlines()[-1], r"^drill verdict=UNMEASURED steps=0/")

    def test_quiet_kill9_and_restart(self):
        w = World(); self.addCleanup(w.close)
        old = restarter(self, w)
        rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "10", "--repo", TOOLS, "--drill-root", tempfile.mkdtemp(prefix="dr-"), env=w.env)
        self.assertEqual(rc, 0, out)
        self.assertRegex(out.strip().splitlines()[-1], r"^drill verdict=PASS steps=(\d+)/\1 ")
        self.assertIn(f"old={old}", out)

    def test_fire_unit_never_restarts(self):
        w = World(); self.addCleanup(w.close)
        rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "1", "--repo", TOOLS, "--drill-root", tempfile.mkdtemp(prefix="dr-"), env=w.env)
        self.assertEqual(rc, 1); self.assertIn("drill_step=unit_restarted status=FAIL", out)

    def test_quiet_submit3_acked_present_and_rehearsal(self):
        w = World(); self.addCleanup(w.close)
        restarter(self, w)
        root = tempfile.mkdtemp(prefix="dr-")
        rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "10", "--repo", TOOLS, "--submit", "3", "--drill-root", root, env=w.env)
        self.assertEqual(rc, 0, out)
        steps = [l.split()[0] for l in out.splitlines() if l.startswith("drill_step=")]
        self.assertEqual(steps, ["drill_step=unit_active", "drill_step=submit", "drill_step=kill9", "drill_step=unit_restarted",
                                 "drill_step=socket_perms", "drill_step=health_ready", "drill_step=acked_present"])
        self.assertRegex(out, r"drill_step=submit status=MEASURED .* detail=acked=3/3 ids=t-[0-9a-f]{24},t-")
        self.assertRegex(out, r"drill_step=acked_present status=MEASURED .* detail=3/3 missing=none")
        self.assertRegex(out.strip().splitlines()[-1], r"^drill verdict=PASS steps=(\d+)/\1 unit=hee4.service head=\S+ submitted=3 acked_present=3/3$")
        rec = json.load(open(os.path.join(root, head_of(TOOLS), "rehearsal.json")))
        self.assertEqual(sorted(rec["task_ids"]), sorted(w.submitted)); self.assertEqual(rec["acked_present"], "3/3")
        self.assertEqual([s["status"] for s in rec["steps"]], ["MEASURED"] * len(steps))

    def test_fire_missing_ack_is_fail_and_rehearsal_still_written(self):
        w = World(missing=1); self.addCleanup(w.close)
        restarter(self, w)
        root = tempfile.mkdtemp(prefix="dr-")
        rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "10", "--repo", TOOLS, "--submit", "3", "--drill-root", root, env=w.env)
        self.assertEqual(rc, 1, out)
        self.assertRegex(out, r"drill_step=acked_present status=FAIL .* detail=2/3 missing=t-[0-9a-f]{24}")
        self.assertRegex(out.strip().splitlines()[-1], r"^drill verdict=FAIL .* submitted=3 acked_present=2/3$")
        rec = json.load(open(os.path.join(root, head_of(TOOLS), "rehearsal.json")))
        self.assertEqual(rec["acked_present"], "2/3"); self.assertEqual(len(rec["task_ids"]), 3)

    def test_quiet_second_submit_run_keeps_every_id(self):
        w = World(); self.addCleanup(w.close)
        root = tempfile.mkdtemp(prefix="dr-")
        for _ in range(2):  # the record is append-only across runs at one tree: no drill id ever counts as use
            restarter(self, w)
            rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "10", "--repo", TOOLS, "--submit", "3", "--drill-root", root, env=w.env)
            self.assertEqual(rc, 0, out)
        rec = json.load(open(os.path.join(root, head_of(TOOLS), "rehearsal.json")))
        self.assertEqual(sorted(rec["task_ids"]), sorted(w.submitted)); self.assertEqual(rec["acked_present"], "3/3")

    def test_fire_mainpid_zero_is_refused_and_nothing_is_killed(self):
        import subprocess
        w = World(); self.addCleanup(w.close)
        open(w.pidfile, "w").write("0")
        r = subprocess.run([DRILL, "--socket", w.sockpath, "--repo", TOOLS, "--drill-root", tempfile.mkdtemp(prefix="dr-")],
                           env={**os.environ, **w.env}, capture_output=True, text=True, start_new_session=True, timeout=60)
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("drill_step=unit_active status=FAIL", r.stdout); self.assertIn("MainPID=0: active unit without a main process", r.stdout)
        self.assertEqual(r.stdout.count("drill_step="), 5); self.assertNotIn("drill_step=kill9 status=MEASURED", r.stdout)
        self.assertIsNone(w.proc.poll(), "the fake main was killed")

if __name__ == "__main__":
    unittest.main()
