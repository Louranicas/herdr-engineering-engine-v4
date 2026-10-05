"""judge.inspect (held, H-8) through tools/drive and the generic client against a disposable serve."""
import os, subprocess, unittest
from test_drive import Proxy
from test_drive_task_extra import BIN, DisposableServe


class JudgeDriveTests(DisposableServe):
    def test_held_refusal_drives_and_the_client_exits_one(self):
        rc, out, _, _ = self.drive("--only", "judge.inspect")
        self.assertIn("verdict=PASS ", self.line(out, "judge.inspect"), out)
        self.assertEqual(rc, 0, out)
        r = subprocess.run([BIN, "judge.inspect", "--socket", self.sock, "--body", "{}"], capture_output=True, text=True, timeout=30)
        both = r.stdout + r.stderr
        self.assertEqual(r.returncode, 1, both)
        self.assertIn("unavailable", both)
        self.assertIn("H-8", both)

    def test_fires_when_unavailable_is_planted_away(self):
        p = os.path.join(self.d, "rt", "px.sock")
        Proxy(p, self.sock, '"unavailable"', '"bad_unavailable"').start()
        rc, out, _, _ = self.drive("--only", "judge.inspect", sock=p)
        self.assertIn("verdict=FAIL", self.line(out, "judge.inspect"), out)
        self.assertIn("code=bad_unavailable want unavailable", out)
        self.assertEqual(rc, 1)


if __name__ == "__main__":
    unittest.main()
