"""crash-restart (gates/features/crash-restart.md, "Driving it with hee4", the drive leg).

Starts its OWN disposable `hee4 serve` (socket, ledger and work dir under
~/.cache/hee4-crash/<run>/, never /tmp, never the live unit), using the binary of the serve the
drive is driving (SO_PEERCRED on --socket -> /proc/<pid>/exe). It submits one task whose VERIFY
sleeps, waits for `running`, SIGKILLs that serve mid-attempt, restarts it on the same ledger, and
requires RL-7 to name the task's rule (R08 WorkerAbsent or R12 VerificationBoundary) with an
after-state that is terminal, `blocked` or `effect_unknown` (the R08 row of the reconcile table),
then a SIGTERM stop and a third start whose recovery moves that task nowhere (the policy is pure:
a second pass may name it again, e.g. R10 effect_unknown -> effect_unknown, but never changes it).
On the live unit's socket every path is UNMEASURED reason=disposable serve: the live kill is
tools/drill, run by the captain. Nothing here signals any process but the serve it started, or a
serve a dead drive left behind (reap_stale).

The serve cannot outlive the drive: it starts with PR_SET_PDEATHSIG=SIGKILL, so the kernel kills it
when the drive dies however it dies (SIGKILL, SIGTERM, a caller's timeout), and each run dir holds
an `owner` file (drive pid and start time) so a later leg reaps a run whose owner is gone.
"""
import ctypes
import json
import os
import re
import shutil
import signal
import socket
import struct
import subprocess
import time
import uuid

from drive_d import BRIEF, TERMINAL, is_result
from drive_d.roster import is_live

PATHS = ["kill9_mid_attempt", "acked_present", "recovery_complete", "named_rule", "second_pass_pure"]
LIVE_REASON = "disposable serve (the live unit's kill is tools/drill, run by the captain)"
RULES = {"R08WorkerAbsent", "R12VerificationBoundary"}
AFTER_OK = TERMINAL | {"blocked", "effect_unknown"}
# Long enough that the attempt is still running when it is killed; bwrap --die-with-parent ends it.
CRASH_BRIEF = (BRIEF.replace("VERIFY: /usr/bin/test -w .", "VERIFY: /usr/bin/sleep 30")
               .replace("TIMEBOX: 10s", "TIMEBOX: 60s")
               .replace("RESTATEMENT: check the work dir is writable", "RESTATEMENT: sleep until killed"))
ACKED = 2  # quick tasks acked behind the running one: admitted at the kill, present after it
WAIT_S = 10
ROOT = os.path.expanduser("~/.cache/hee4-crash")
RUN_RE = re.compile(r"^[0-9a-f]{12}$")
PR_SET_PDEATHSIG = 1


def proc_start(pid):
    """The start time (clock ticks since boot, /proc/<pid>/stat field 22) of `pid`, or None if it is gone."""
    try:
        with open(f"/proc/{pid}/stat") as fh:
            return fh.read().rsplit(")", 1)[1].split()[19]
    except (OSError, IndexError):
        return None


def die_with(parent):
    """preexec_fn: the kernel SIGKILLs this child when `parent` dies; if it already died, exit now."""
    def arm():
        if ctypes.CDLL(None, use_errno=True).prctl(PR_SET_PDEATHSIG, signal.SIGKILL, 0, 0, 0) != 0:
            os._exit(127)
        if os.getppid() != parent:  # the drive died between fork and prctl
            os._exit(127)
    return arm


def serves_of(run):
    """Pids of hee4 serves whose --socket is under `run`."""
    sock, pids = os.path.join(run, "rt", "control.sock"), []
    for d in os.listdir("/proc"):
        if not d.isdigit():
            continue
        try:
            with open(f"/proc/{d}/cmdline", "rb") as fh:
                argv = fh.read().split(b"\0")
            exe = os.readlink(f"/proc/{d}/exe")
        except OSError:
            continue
        if os.path.basename(exe) == "hee4" and b"serve" in argv and sock.encode() in argv:
            pids.append(int(d))
    return pids


def owner_alive(run):
    try:
        with open(os.path.join(run, "owner")) as fh:
            pid, start = fh.read().split()
    except (OSError, ValueError):
        return False
    return proc_start(int(pid)) == start


def reap_stale(root=ROOT):
    """SIGKILL the serves of, and remove, every <root>/<12hex> run whose owning drive is gone; returns the reaped runs."""
    reaped = []
    try:
        names = os.listdir(root)
    except OSError:
        return reaped
    for name in names:
        run = os.path.join(root, name)
        if not RUN_RE.match(name) or owner_alive(run):
            continue
        for pid in serves_of(run):
            try:
                os.kill(pid, signal.SIGKILL)
            except OSError:
                pass
        shutil.rmtree(run, ignore_errors=True)
        reaped.append(name)
    return reaped


def unmeasured_all(F, reason):
    for p in PATHS:
        F.check(p, False, reason, unmeasured=True)


def peer_binary(sock):
    """(path, None) of the hee4 binary serving `sock`, or (None, why) when it cannot be named."""
    s = socket.socket(socket.AF_UNIX)
    s.settimeout(2)
    try:
        s.connect(sock)
        pid, _, _ = struct.unpack("3i", s.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i")))
    except OSError as e:
        return None, f"no peer on {sock}: {e}"
    finally:
        s.close()
    try:
        exe = os.readlink(f"/proc/{pid}/exe")
    except OSError as e:
        return None, f"peer pid {pid} exe unreadable: {e}"
    if os.path.basename(exe) != "hee4" or not os.path.isfile(exe):
        return None, f"peer pid {pid} is not a hee4 serve binary on disk: {exe}"
    return exe, None


class Serve:
    """One disposable serve on <run>/rt/control.sock and <run>/ledger.sqlite3; stderr to <run>/<tag>.err."""

    def __init__(self, exe, run):
        self.exe, self.run = exe, run
        self.sock = os.path.join(run, "rt", "control.sock")
        self.proc, self.err = None, None

    def start(self, tag):
        self.err = os.path.join(self.run, f"{tag}.err")
        env = {k: v for k, v in os.environ.items() if k != "HEE4_LIVE_MODEL"}  # no model call from a drill serve
        with open(self.err, "w") as fh:
            self.proc = subprocess.Popen([self.exe, "serve", "--socket", self.sock, "--ledger", os.path.join(self.run, "ledger.sqlite3"),
                                          "--work", os.path.join(self.run, "work")], stdout=fh, stderr=fh, env=env,
                                         preexec_fn=die_with(os.getpid()))

    def stderr(self):
        try:
            with open(self.err) as fh:
                return fh.read()
        except OSError:
            return ""

    def stop(self, sig):
        """Send `sig` to this serve only and reap it; returns its exit code (None if it outlived the wait)."""
        if self.proc is None or self.proc.poll() is not None:
            return None if self.proc is None else self.proc.returncode
        self.proc.send_signal(sig)
        try:
            return self.proc.wait(timeout=WAIT_S * 2)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait()
            return None


def until(fn, budget=WAIT_S, step=0.2):
    end = time.monotonic() + budget
    while True:
        v = fn()
        if v or time.monotonic() >= end:
            return v
        time.sleep(step)


def phase(F, t):
    r = F.req("task.get", {"task_id": t})
    return ((r or {}).get("body") or {}).get("phase") if is_result(r) else None


def health(F):
    r = F.req("health", {})
    b = ((r or {}).get("body") or {}) if is_result(r) else {}
    return b if b.get("ok") is True and b.get("recovery_complete") is True else None


def recovery_rows(text, t):
    """The `recovery task=<t> rule=.. reason=.. workspace=.. <before> -> <after>` lines of serve's stderr for `t`."""
    rows = []
    for line in text.splitlines():
        if not line.startswith(f"recovery task={t} "):
            continue
        kv = dict(w.split("=", 1) for w in line.split() if "=" in w)
        arrow = line.rsplit(" -> ", 1)
        rows.append({"rule": kv.get("rule"), "reason": kv.get("reason"), "before": arrow[0].split()[-1] if len(arrow) == 2 else None,
                     "after": arrow[1].strip() if len(arrow) == 2 else None, "line": line})
    return rows


def record(F, tag, text):
    F.f.write(json.dumps({"t": time.time(), "dir": f"serve_stderr:{tag}", "raw": text}) + "\n")
    F.f.flush()


def submit(F, brief):
    r = F.req("task.submit", {"brief": brief}, key=str(uuid.uuid4()))
    return ((r or {}).get("body") or {}).get("task_id") if is_result(r) else None


def drill(F, srv):
    srv.start("first")
    if not until(lambda: health(F)):
        return F.check("kill9_mid_attempt", False, f"first serve never answered health: {srv.stderr()[-300:]}")
    t = submit(F, CRASH_BRIEF)
    ids = [t] + [submit(F, BRIEF) for _ in range(ACKED)]
    running = bool(t) and until(lambda: phase(F, t) == "running")
    rc = srv.stop(signal.SIGKILL)
    record(F, "first", srv.stderr())
    F.check("kill9_mid_attempt", running and rc == -signal.SIGKILL, f"task={t} running_at_kill={running} serve_rc={rc}")
    if not running:
        return
    srv.start("second")
    h = until(lambda: health(F))
    F.check("recovery_complete", bool(h), f"health after restart on the same ledger: {h}")
    present = [i for i in ids if i and phase(F, i)]
    F.check("acked_present", len(present) == len(ids), f"acked_present={len(present)}/{len(ids)} ids={ids}")
    rows = recovery_rows(srv.stderr(), t)
    got = phase(F, t)
    ok = (len(rows) == 1 and rows[0]["rule"] in RULES and rows[0]["before"] == "running"
          and rows[0]["after"] in AFTER_OK and got == rows[0]["after"])
    F.check("named_rule", ok, f"task={t} rows={[r['line'] for r in rows]} task.get={got}")
    until(lambda: all(phase(F, i) in TERMINAL for i in ids[1:]), budget=WAIT_S * 2)  # let the quick tasks settle before the drain
    rc2 = srv.stop(signal.SIGTERM)
    record(F, "second", srv.stderr())
    srv.start("third")
    h2 = until(lambda: health(F))
    again = recovery_rows(srv.stderr(), t)
    got2 = phase(F, t)
    moved = [r["line"] for r in again if not r["before"] == r["after"] == got]  # R10 may re-read it; nothing may move it
    F.check("second_pass_pure", bool(h2) and not moved and got2 == got,
            f"sigterm_rc={rc2} recovery_complete={bool(h2)} rows_for_task={[r['line'] for r in again]} phase {got} -> {got2}")
    srv.stop(signal.SIGTERM)
    record(F, "third", srv.stderr())


def d_crash(F, ctx):
    if is_live(F):
        return unmeasured_all(F, LIVE_REASON)
    exe, why = peer_binary(F.sock)
    if exe is None:
        return unmeasured_all(F, f"disposable serve not startable: {why}")
    reaped = reap_stale()
    if reaped:
        record(F, "reap_stale", f"reaped runs whose drive is gone: {reaped}")
    run = os.path.join(ROOT, uuid.uuid4().hex[:12])
    staging = run + ".new"  # not a 12hex name: a concurrent reap_stale never sees a run without its owner
    os.makedirs(os.path.join(staging, "rt"), mode=0o700)
    with open(os.path.join(staging, "owner"), "w") as fh:
        fh.write(f"{os.getpid()} {proc_start(os.getpid())}\n")
    os.rename(staging, run)
    srv, driven = Serve(exe, run), F.sock
    F.sock = srv.sock  # every frame of the leg goes to the disposable serve and into this feature's evidence
    try:
        drill(F, srv)
    finally:
        F.sock = driven
        srv.stop(signal.SIGKILL)
        shutil.rmtree(run, ignore_errors=True)
    done = {p[0] for p in F.paths}
    for p in PATHS:
        if p not in done:
            F.check(p, False, "not reached: kill9_mid_attempt did not hold", unmeasured=True)


FEATURES = [("crash-restart", d_crash)]
