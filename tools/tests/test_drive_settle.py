"""tools/drive settle(): the drive leaves no debris of its own and never touches another task.
The drive's restart path (persistence_restart) restarts the live unit while a task it submitted may
be verifying; recovery then quarantines it (R12) or marks it effect_unknown (R08). settle() abandons
exactly this run's such tasks through task.resolve and fails the drive if any stays open."""
import importlib.machinery, importlib.util, json, os, socket, tempfile, threading, unittest
from common import TOOLS


def load_drive():
    loader = importlib.machinery.SourceFileLoader("drive_mod", os.path.join(TOOLS, "drive"))
    spec = importlib.util.spec_from_loader("drive_mod", loader)
    mod = importlib.util.module_from_spec(spec); loader.exec_module(mod)
    return mod


class FakeUnit:
    """A unix socket answering task.get from `phases` and recording task.resolve calls."""
    def __init__(self, phases):
        self.d = tempfile.mkdtemp(prefix="settle-")
        self.sock = os.path.join(self.d, "control.sock")
        self.phases, self.resolved = dict(phases), []
        self.s = socket.socket(socket.AF_UNIX); self.s.bind(self.sock); self.s.listen(8)
        threading.Thread(target=self.serve, daemon=True).start()

    def serve(self):
        while True:
            try:
                c, _ = self.s.accept()
            except OSError:
                return
            req = json.loads(c.makefile("rb").readline() or b"{}")
            t = (req.get("body") or {}).get("task_id")
            if req.get("action") == "task.resolve":
                self.resolved.append(t); self.phases[t] = "abandoned"
            body = {"task_id": t, "phase": self.phases.get(t)}
            c.sendall((json.dumps({"kind": "result", "request_id": req.get("request_id"), "replayed": False, "body": body}) + "\n").encode())
            c.close()

    def close(self):
        self.s.close()


class SettleTests(unittest.TestCase):
    def setUp(self):
        self.drive = load_drive()
        self.ev = tempfile.mkdtemp(prefix="settle-ev-")

    def test_fire_abandons_only_this_runs_blocked_and_effect_unknown_tasks(self):
        own_blocked, own_unknown, own_done, foreign = "t-" + "a" * 24, "t-" + "b" * 24, "t-" + "c" * 24, "t-" + "f" * 24
        u = FakeUnit({own_blocked: "blocked", own_unknown: "effect_unknown", own_done: "accepted", foreign: "blocked"})
        self.addCleanup(u.close)
        self.drive.MINE[:] = [(u.sock, own_blocked), (u.sock, own_unknown), (u.sock, own_done), ("/elsewhere/sock", "t-" + "e" * 24)]
        self.assertEqual(self.drive.settle(u.sock, self.ev, {}, budget=5), "PASS")
        self.assertEqual(sorted(u.resolved), sorted([own_blocked, own_unknown]))
        self.assertNotIn(foreign, u.resolved)
        self.assertEqual(u.phases[foreign], "blocked")

    def test_fire_a_task_still_open_after_the_budget_is_unmeasured_by_name(self):
        running = "t-" + "d" * 24
        u = FakeUnit({running: "running"})
        self.addCleanup(u.close)
        self.drive.MINE[:] = [(u.sock, running)]
        self.assertEqual(self.drive.settle(u.sock, self.ev, {}, budget=1), "UNMEASURED")

    def test_quiet_nothing_submitted_is_a_pass_with_nothing_sent(self):
        self.drive.MINE[:] = [("/other/sock", "t-" + "9" * 24)]  # submitted elsewhere: not this socket's to settle
        self.assertEqual(self.drive.settle("/nonexistent/sock", self.ev, {}, budget=1), "PASS")


if __name__ == "__main__":
    unittest.main()
