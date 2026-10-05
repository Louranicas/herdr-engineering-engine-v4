"""tools.list and tools.inspect through tools/drive against a disposable serve."""
import json, os, re, subprocess, unittest
from test_drive import Proxy
from test_drive_task_extra import BIN, DisposableServe


class ToolsDriveTests(DisposableServe):
    def test_both_pass_and_the_revision_matches_the_binary(self):
        rc, out, _, ev = self.drive("--only", "tools.list,tools.inspect")
        self.assertIn("verdict=PASS ", self.line(out, "tools.list"), out)
        self.assertIn("verdict=PASS ", self.line(out, "tools.inspect"), out)
        self.assertEqual(rc, 0, out)
        revs = [json.loads(g["raw"])["body"]["catalogue_revision"] for s, g in self.rows(ev, "tools.list")
                if json.loads(g["raw"] or "{}").get("kind") == "result"]
        version = subprocess.run([BIN, "--version"], capture_output=True, text=True).stdout
        token = re.search(r"catalogue=([0-9a-f]+)", version).group(1)
        self.assertTrue(revs and revs[0][:12] == token, (revs[:1], version))

    def test_fires_when_unknown_action_is_planted_away(self):
        p = os.path.join(self.d, "rt", "px.sock")
        Proxy(p, self.sock, '"unknown_action"', '"bad_action"').start()
        rc, out, _, _ = self.drive("--only", "tools.inspect", sock=p)
        self.assertIn("verdict=FAIL", self.line(out, "tools.inspect"), out)
        self.assertIn("code=bad_action want unknown_action", out)
        self.assertEqual(rc, 1)


if __name__ == "__main__":
    unittest.main()
