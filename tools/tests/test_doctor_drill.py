import http.server, json, os, shutil, socket, stat, tempfile, threading, unittest, uuid
from common import TOOLS, run

DOCTOR, DRILL = os.path.join(TOOLS, "doctor"), os.path.join(TOOLS, "drill")

def stub(dirpath, name, body):
    p = os.path.join(dirpath, name)
    with open(p, "w") as f:
        f.write("#!/usr/bin/env bash\n" + body)
    os.chmod(p, 0o755)

def hee4_stub(dirpath, head):
    """A fake `hee4`: `--version` names `head`; `health` ends with the JSON reply the doctor reads
    (body.budgets, served since U-stack-04 wave 3); `doctor` prints the binary's own budgets row
    with the same key=value set."""
    health = json.dumps({"kind": "result", "request_id": "cli-1", "replayed": False,
                         "body": {"ok": True, "recovery_complete": True, "uptime_s": 1,
                                  "budgets": {"socket": {"max_connections": 256}, "door": {"pool": 8}}}})
    stub(dirpath, "hee4", f'case "$1" in --version) echo "hee4 {head}";; '
         'doctor) echo "doctor row=budgets present door.pool=8 socket.max_connections=256";; '
         "*) echo 'ready=true recovery=complete database=ready socket=owned'; echo '" + health + "';; esac\n")

def df_stub(dirpath, space, inodes, rc=0):
    """A fake `df --output=pcent,ipcent`: the planted reading."""
    stub(dirpath, "df", f'echo "Use% IUse%"; echo " {space}%  {inodes}%"; exit {rc}\n')

def tmp_row(out):
    return [l for l in out.splitlines() if l.startswith("check=tmp_usage ")][0]

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
        self.phases, self.resolved = {}, []  # task.get phases a test plants; task.resolve calls received
        threading.Thread(target=self.serve, daemon=True).start()
        self.srv = http.server.HTTPServer(("127.0.0.1", 0), ps_handler([{"name": "m", "size_vram": 1}] if models is None else models))
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.srv.server_port}"
        self.pidfile = os.path.join(self.d, "pid")
        self.proc = None
        self.start_main()
        stub(self.bin, "systemctl", f'echo LoadState=loaded; echo ActiveState=active; echo MainPID=$(cat {self.pidfile})\n')
        hee4_stub(self.bin, "HEADSHA")
        df_stub(self.bin, 10, 10)  # the host's own /tmp never decides a test
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
                elif act == "task.get":
                    t = req.get("body", {}).get("task_id")
                    body = {"task_id": t, "phase": self.phases.get(t, "accepted")}
                elif act == "task.resolve":
                    t = req.get("body", {}).get("task_id"); self.resolved.append(t); self.phases[t] = "abandoned"
                    body = {"task_id": t, "phase": "abandoned"}
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
    hee4_stub(w.bin, head)
    return run(DOCTOR, "--socket", w.sockpath, "--model-url", w.url, "--repo", TOOLS, *extra, env=w.env)

class DoctorTests(unittest.TestCase):
    def test_help(self):
        for t in (DOCTOR, DRILL):
            rc, out, _ = run(t, "--help"); self.assertEqual(rc, 0); self.assertIn("verdict=", out)

    def test_fire_missing_cargo_deny_is_fail_by_name(self):
        # A PATH with python3 but no cargo-deny: the commit tier's `deps` step could not run, so the
        # doctor says so by name before any gate does.
        w = World(); self.addCleanup(w.close)
        # Only the world's stubs and a python3 link: no host directory that might hold cargo-deny.
        os.symlink(shutil.which("python3"), os.path.join(w.bin, "python3"))
        rc, out, _ = run(DOCTOR, "--socket", w.sockpath, "--model-url", w.url, "--repo", TOOLS,
                         env={"PATH": w.bin})
        self.assertEqual(rc, 1, out)
        self.assertIn("check=tool_cargo-deny status=FAIL detail=cannot run cargo-deny", out)

    def test_quiet_cargo_deny_present_is_measured(self):
        w = World(); self.addCleanup(w.close)
        rc, out, _ = healthy_doctor(w, "--model", "m")
        self.assertIn("check=tool_cargo-deny status=MEASURED detail=cargo-deny ", out)

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

    def test_quiet_tmp_below_threshold_is_not_advisory(self):
        w = World(); self.addCleanup(w.close)
        rc, out, _ = healthy_doctor(w, "--model", "m")
        self.assertEqual(rc, 0, out)
        self.assertIn("status=MEASURED detail=path=", tmp_row(out))
        self.assertIn(" space=10% inodes=10% threshold=70% over=none ", tmp_row(out))
        self.assertRegex(out.strip().splitlines()[-1], r"^doctor verdict=PASS .* advisory=0 ")

    def test_fire_tmp_planted_full_is_advisory_never_fail(self):
        # 2026-10-05: /tmp (a RAM tmpfs) filled twice, by space then by inodes. The doctor says so,
        # but a full /tmp is the operator's to clear, never a reason to fail readiness.
        for space, inodes, over in ((90, 12, "space"), (12, 90, "inodes"), (90, 90, "space,inodes")):
            with self.subTest(space=space, inodes=inodes):
                w = World(); self.addCleanup(w.close)
                df_stub(w.bin, space, inodes)
                rc, out, _ = healthy_doctor(w, "--model", "m")
                self.assertEqual(rc, 0, out)
                row = tmp_row(out)
                self.assertIn(f"check=tmp_usage status=MEASURED detail=path=", row)
                self.assertIn(f" space={space}% inodes={inodes}% threshold=70% over={over} ", row)
                self.assertTrue(row.endswith(" scope=advisory"), row)
                self.assertRegex(out.strip().splitlines()[-1], r"^doctor verdict=PASS .* fail=0 advisory=1 ")

    def test_fire_tmp_df_unreadable_is_advisory_unmeasured(self):
        w = World(); self.addCleanup(w.close)
        df_stub(w.bin, 0, 0, rc=1)
        rc, out, _ = healthy_doctor(w, "--model", "m")
        self.assertEqual(rc, 0, out)
        self.assertIn("check=tmp_usage status=UNMEASURED detail=path=", tmp_row(out))
        self.assertIn("df unreadable: rc=1", tmp_row(out))
        self.assertRegex(out.strip().splitlines()[-1], r"^doctor verdict=PASS .* advisory=1 ")

    def test_tmp_top_names_the_three_largest_own_entries_largest_first(self):
        w = World(); self.addCleanup(w.close)
        tmp = tempfile.mkdtemp(prefix="tt-")
        for name, kib in (("big", 400), ("mid", 200), ("small", 100), ("tiny", 8)):
            os.mkdir(os.path.join(tmp, name))
            with open(os.path.join(tmp, name, "f"), "wb") as f:
                f.write(os.urandom(kib * 1024))
        rc, out, _ = healthy_doctor(w, "--model", "m", "--tmp-dir", tmp)
        self.assertEqual(rc, 0, out)
        row = tmp_row(out)
        self.assertIn(f"path={tmp} ", row)
        self.assertRegex(row, r" top=big:\d+k,mid:\d+k,small:\d+k scope=advisory$")

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

    def test_fire_rehearsal_settled_resolves_only_its_own_effect_unknown_tasks(self):
        # The kill leaves some of the drill's own tasks effect_unknown (R08): the drill abandons
        # those through task.resolve, waits until all its tasks are terminal, and never touches
        # a task it did not submit (here a planted foreign effect_unknown task).
        w = World(); self.addCleanup(w.close)
        restarter(self, w)
        w.phases["t-" + "f" * 24] = "effect_unknown"
        root = tempfile.mkdtemp(prefix="dr-")
        # every task this run submits reads effect_unknown until resolved
        class Unknown(dict):
            def get(self, k, d=None):
                return dict.get(self, k, "effect_unknown" if k in w.submitted else d)
        w.phases = Unknown(w.phases)
        rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "10", "--repo", TOOLS, "--submit", "3", "--drill-root", root, env=w.env)
        self.assertEqual(rc, 0, out)
        self.assertRegex(out, r"drill_step=rehearsal_settled status=MEASURED .* detail=terminal=3/3 resolved_effect_unknown=3 refused=none open=none")
        self.assertEqual(sorted(w.resolved), sorted(w.submitted))
        self.assertNotIn("t-" + "f" * 24, w.resolved)

    def test_quiet_submit3_acked_present_and_rehearsal(self):
        w = World(); self.addCleanup(w.close)
        restarter(self, w)
        root = tempfile.mkdtemp(prefix="dr-")
        rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "10", "--repo", TOOLS, "--submit", "3", "--drill-root", root, env=w.env)
        self.assertEqual(rc, 0, out)
        steps = [l.split()[0] for l in out.splitlines() if l.startswith("drill_step=")]
        self.assertEqual(steps, ["drill_step=unit_active", "drill_step=submit", "drill_step=kill9", "drill_step=unit_restarted",
                                 "drill_step=socket_perms", "drill_step=health_ready", "drill_step=acked_present", "drill_step=rehearsal_settled"])
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

    def test_fire_plain_run_keeps_the_submitting_record(self):
        w = World(); self.addCleanup(w.close)
        root = tempfile.mkdtemp(prefix="dr-")
        for extra in (["--submit", "3"], []):  # the cut tier's plain drill after the captain's --submit 3, same tree
            restarter(self, w)
            rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "10", "--repo", TOOLS, *extra, "--drill-root", root, env=w.env)
            self.assertEqual(rc, 0, out)
        self.assertIn(f"drill rehearsal=KEPT(prior submitted=3 at tree={head_of(TOOLS)}; this run acked=0 requested=0 does not replace it)", out)
        rec = json.load(open(os.path.join(root, head_of(TOOLS), "rehearsal.json")))
        self.assertEqual(rec["submitted"], 3); self.assertEqual(rec["acked_present"], "3/3")
        self.assertEqual(sorted(rec["task_ids"]), sorted(w.submitted))

    def test_fire_failed_write_leaves_the_old_record_whole(self):
        import argparse, importlib.machinery, importlib.util, io, contextlib
        from unittest import mock
        loader = importlib.machinery.SourceFileLoader("drill_under_test", DRILL)
        mod = importlib.util.module_from_spec(importlib.util.spec_from_loader(loader.name, loader)); loader.exec_module(mod)
        root = tempfile.mkdtemp(prefix="dr-")
        tree = os.path.join(root, "abcdef012345"); os.makedirs(tree)
        p = os.path.join(tree, "rehearsal.json")
        old = json.dumps({"tree": "abcdef012345", "submitted": 3, "acked_present": "3/3", "task_ids": ["t-old"], "steps": []})
        open(p, "w").write(old)
        a = argparse.Namespace(drill_root=root, unit="hee4.service", submit=2, restart_budget=1)
        r = mod.Run(a, "abcdef012345"); r.ids = ["t-new1", "t-new2"]
        buf = io.StringIO()
        with mock.patch("os.fsync", side_effect=OSError(28, "No space left on device")), contextlib.redirect_stdout(buf):
            r.save("0/2")  # the write dies after the bytes went out: a reader must still see the old record, whole
        self.assertIn("drill rehearsal=UNWRITTEN(", buf.getvalue())
        self.assertEqual(open(p).read(), old)
        self.assertEqual(sorted(os.listdir(tree)), [".lock", "rehearsal.json"], "a temp file was left behind")
        with contextlib.redirect_stdout(io.StringIO()):
            r.save("2/2")
        rec = json.load(open(p))
        self.assertEqual((rec["submitted"], rec["acked_present"], rec["task_ids"]), (2, "2/2", ["t-old", "t-new1", "t-new2"]))
        self.assertEqual(sorted(os.listdir(tree)), [".lock", "rehearsal.json"])

    def test_fire_submit_run_that_acked_nothing_keeps_the_submitting_record(self):
        # the cut tier always passes --submit 3: with the unit down (UNMEASURED, rc=3) or the socket gone
        # (submit FAIL, rc=1) it acks no task, and the door is keyed on what was acked, not on --submit
        for case in ("unit_inactive", "socket_gone"):
            with self.subTest(case=case):
                w = World(); self.addCleanup(w.close)
                root = tempfile.mkdtemp(prefix="dr-")
                restarter(self, w)
                rc, out, _ = run(DRILL, "--socket", w.sockpath, "--restart-budget", "10", "--repo", TOOLS, "--submit", "3", "--drill-root", root, env=w.env)
                self.assertEqual(rc, 0, out)
                sock = w.sockpath
                if case == "unit_inactive":
                    stub(w.bin, "systemctl", "echo LoadState=loaded; echo ActiveState=inactive; echo MainPID=0\n")
                else:
                    sock = os.path.join(w.d, "rt", "gone.sock")
                rc, out, _ = run(DRILL, "--socket", sock, "--restart-budget", "1", "--repo", TOOLS, "--submit", "3", "--drill-root", root, env=w.env)
                self.assertEqual(rc, 3 if case == "unit_inactive" else 1, out)
                self.assertIn(f"drill rehearsal=KEPT(prior submitted=3 at tree={head_of(TOOLS)}; this run acked=0 requested=3 does not replace it)", out)
                self.assertRegex(out.strip().splitlines()[-1], r" submitted=0 acked_present=0/0 requested=3$")
                rec = json.load(open(os.path.join(root, head_of(TOOLS), "rehearsal.json")))
                self.assertEqual((rec["submitted"], rec["acked_present"]), (3, "3/3"))
                # every step the submitting run recorded was MEASURED, the settle step among them (no pinned count)
                self.assertTrue(rec["steps"] and all(s["status"] == "MEASURED" for s in rec["steps"]), rec["steps"])
                self.assertIn("rehearsal_settled", [s.get("name") or s.get("step") for s in rec["steps"]])
                self.assertEqual(sorted(rec["task_ids"]), sorted(w.submitted))

    def test_fire_interleaved_saves_lose_no_record_and_no_id(self):
        # a second drill's save runs inside the first one's write (the read-merge-write window): with the
        # per-tree flock the inner save waits, then merges onto the outer record; without it one is erased
        import argparse, importlib.machinery, importlib.util, io, contextlib
        from unittest import mock
        loader = importlib.machinery.SourceFileLoader("drill_lock_under_test", DRILL)
        mod = importlib.util.module_from_spec(importlib.util.spec_from_loader(loader.name, loader)); loader.exec_module(mod)
        def ns(n):
            return argparse.Namespace(drill_root=root, unit="hee4.service", submit=n, restart_budget=1)
        for outer_ids, inner_ids in (([], ["t-sub1", "t-sub2", "t-sub3"]), (["t-a1", "t-a2"], ["t-b1", "t-b2", "t-b3"])):
            with self.subTest(outer=outer_ids, inner=inner_ids):
                root = tempfile.mkdtemp(prefix="dr-")
                outer, inner = mod.Run(ns(len(outer_ids)), "abcdef012345"), mod.Run(ns(len(inner_ids)), "abcdef012345")
                outer.ids, inner.ids = list(outer_ids), list(inner_ids)
                real, fired, t = mod.write_atomic, [], []
                def hook(path, text):
                    if not fired:
                        fired.append(1)
                        th = threading.Thread(target=inner.save, args=(f"{len(inner_ids)}/{len(inner_ids)}",)); th.start(); t.append(th)
                        th.join(1.0)  # with the lock the inner save is still blocked here; without it, it has written
                    real(path, text)
                with mock.patch.object(mod, "write_atomic", hook), contextlib.redirect_stdout(io.StringIO()):
                    outer.save(f"{len(outer_ids)}/{len(outer_ids)}")
                    t[0].join(10); self.assertFalse(t[0].is_alive())
                rec = json.load(open(os.path.join(root, "abcdef012345", "rehearsal.json")))
                self.assertEqual(rec["submitted"], len(inner_ids))
                self.assertEqual(sorted(rec["task_ids"]), sorted(outer_ids + inner_ids))

    def test_cut_tier_drill_step_submits(self):
        import re, tomllib
        with open(os.path.join(os.path.dirname(TOOLS), "gate.toml"), "rb") as f:
            cmd = tomllib.load(f)["step"]["drill"]["cmd"]
        m = re.search(r"(?:^|\s)--submit[ =](\d+)(?:\s|$)", cmd)
        self.assertIsNotNone(m, f"[step.drill] cmd does not submit: {cmd!r}")
        self.assertGreater(int(m.group(1)), 0, cmd)

if __name__ == "__main__":
    unittest.main()
