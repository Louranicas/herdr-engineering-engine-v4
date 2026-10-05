"""The task family: health, task.submit, task.get, task.list, task.cancel (moved verbatim from tools/drive)."""
import os
import re
import stat
import subprocess
import time
import uuid

from drive_d import BRIEF, PHASES, TERMINAL, VACUOUS_VERIFY_BRIEF, is_result



def sh(*cmd, timeout=30):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return r.returncode, r.stdout.strip(), r.stderr.strip()
    except (OSError, subprocess.TimeoutExpired) as e:
        return None, "", str(e)


def d_health(F, ctx):
    r = F.req("health", {})
    b = (r or {}).get("body", {})
    F.check("success", is_result(r) and b.get("ok") is True and b.get("recovery_complete") is True
           and re.fullmatch(r"[0-9a-f]{40}|unknown", str(b.get("head_sha"))) is not None and r.get("replayed") is False, f"{r}")
    r2 = F.req("health", {})
    F.check("repeat_not_replayed", is_result(r2) and r2.get("replayed") is False and r2["request_id"] != (r or {}).get("request_id"), "second health replayed=false")
    F.refuse("malformed_frame", F.frame("{not json"), "invalid_argument", "/")
    F.refuse("unknown_action", F.req("health.explode", {}), "unknown_action", "/action")
    F.refuse("bad_version", F.req("health", {}, version=2), "unsupported_action_version", "/action_version")
    d = os.path.dirname(F.sock)
    try:
        md, ms = stat.S_IMODE(os.stat(d).st_mode), stat.S_IMODE(os.stat(F.sock).st_mode)
        F.check("perms", md == 0o700 and ms == 0o600, f"dir={md:o} sock={ms:o}")
    except OSError as e:
        F.check("perms", False, str(e))


def d_submit(F, ctx):
    k = str(uuid.uuid4())
    r = F.req("task.submit", {"brief": BRIEF}, key=k)
    b = (r or {}).get("body", {})
    ok = is_result(r) and r.get("replayed") is False and b.get("phase") == "admitted" and re.fullmatch(r"t-[0-9a-f]{24}", str(b.get("task_id"))) is not None
    F.check("success", ok, f"{r}")
    if ok:
        ctx["task"] = b["task_id"]
    rr = F.req("task.submit", {"brief": BRIEF}, key=k)
    F.check("replay", is_result(rr) and rr.get("replayed") is True and (rr.get("body") or {}).get("task_id") == b.get("task_id"), f"{rr}")
    F.refuse("conflict", F.req("task.submit", {"brief": BRIEF.replace("GOAL: drive", "GOAL: other")}, key=k), "conflict", "/idempotency_key")
    F.refuse("empty_brief", F.req("task.submit", {"brief": ""}, key=str(uuid.uuid4())), "invalid_argument", "/body/brief", "GOAL")
    restatement = BRIEF[BRIEF.index("RESTATEMENT:"):]
    F.refuse("missing_restatement", F.req("task.submit", {"brief": BRIEF.replace(restatement, "")}, key=str(uuid.uuid4())),
             "invalid_argument", "/body/brief", "RESTATEMENT")
    F.refuse("empty_restatement", F.req("task.submit", {"brief": BRIEF.replace(restatement, "RESTATEMENT:\n")}, key=str(uuid.uuid4())),
             "invalid_argument", "/body/brief")
    # The V4-94 door, driven: a no-op VERIFY is refused at admission (Refusal::VacuousVerify), no row.
    before = F.req("task.list", {})
    F.refuse("vacuous_verify", F.req("task.submit", {"brief": VACUOUS_VERIFY_BRIEF}, key=str(uuid.uuid4())), "invalid_argument",
             "/body/brief", "VERIFY is vacuous")
    after = F.req("task.list", {})
    n0, n1 = (len(((x or {}).get("body") or {}).get("tasks", [])) if is_result(x) else None for x in (before, after))
    F.check("vacuous_verify_admits_nothing", n0 is not None and n0 == n1, f"task.list rows before={n0} after={n1}")
    F.refuse("missing_key", F.req("task.submit", {"brief": BRIEF}), "invalid_argument", "/idempotency_key")


def d_get(F, ctx):
    t = ctx.get("task")
    if not t:
        F.check("found", False, "no task from task.submit", unmeasured=True)
    else:
        r = F.req("task.get", {"task_id": t}); b = (r or {}).get("body", {})
        F.check("found", is_result(r) and b.get("task_id") == t and b.get("phase") in PHASES and isinstance(b.get("events"), int), f"{r}")
    F.refuse("not_found", F.req("task.get", {"task_id": "t-" + "0" * 24}), "not_found", "/body/task_id")
    F.refuse("bad_id", F.req("task.get", {"task_id": "bad id/.."}), "invalid_argument", "/body/task_id")


def d_list(F, ctx):
    t = ctx.get("task")
    r = F.req("task.list", {})
    ids = [x.get("task_id") for x in (r or {}).get("body", {}).get("tasks", [])]
    if not t:
        F.check("lists_submitted", False, "no task from task.submit", unmeasured=True)
    else:
        F.check("lists_submitted", is_result(r) and t in ids, f"{len(ids)} tasks listed")
    F.check("rows_typed", is_result(r) and all(x.get("phase") in PHASES for x in r["body"]["tasks"]), "every row has a known phase")
    if not ctx["restart_ok"]:
        return F.check("persistence_restart", False, ctx["restart_why"], unmeasured=True)
    rc, _, err = sh("systemctl", "--user", "restart", ctx["unit"], timeout=60)
    if rc != 0:
        return F.check("persistence_restart", False, f"restart rc={rc} {err}")
    up, t0 = None, time.monotonic()
    while time.monotonic() - t0 < 30 and not is_result(up):
        time.sleep(0.5); up = F.req("health", {})
    r2 = F.req("task.list", {})
    ids2 = [x.get("task_id") for x in (r2 or {}).get("body", {}).get("tasks", [])]
    F.check("persistence_restart", is_result(r2) and t in ids2 and len(ids2) >= len(ids), f"after restart {len(ids2)} tasks, submitted present={t in ids2}")


def d_cancel(F, ctx):
    r = F.req("task.submit", {"brief": BRIEF}, key=str(uuid.uuid4()))
    t = (r or {}).get("body", {}).get("task_id")
    if not t:
        F.check("cancel_admitted", False, f"submit failed: {r}")
    else:
        c = F.req("task.cancel", {"task_id": t}, key=str(uuid.uuid4()))
        ph = (c or {}).get("body", {}).get("phase")
        if is_result(c):
            F.check("cancel_admitted", ph in {"cancellation_requested", "cancelled"}, f"phase={ph}")
        else:  # the dispatcher won the race to a terminal phase: the State map says Illegal -> conflict
            F.refuse("cancel_admitted", c, "conflict", "/body/task_id")
    term = ctx.get("task")
    ph, t0 = None, time.monotonic()
    while term and time.monotonic() - t0 < 25 and ph not in TERMINAL:
        g = F.req("task.get", {"task_id": term}); ph = (g or {}).get("body", {}).get("phase")
        if ph not in TERMINAL: time.sleep(0.5)
    if ph in TERMINAL:
        F.refuse("cancel_terminal", F.req("task.cancel", {"task_id": term}, key=str(uuid.uuid4())), "conflict", "/body/task_id")
    else:
        F.check("cancel_terminal", False, f"submitted task not terminal in 25s (phase={ph})", unmeasured=True)
    F.refuse("cancel_not_found", F.req("task.cancel", {"task_id": "t-" + "0" * 24}, key=str(uuid.uuid4())), "not_found", "/body/task_id")
    F.refuse("cancel_missing_key", F.req("task.cancel", {"task_id": term or "t-" + "0" * 24}), "invalid_argument", "/idempotency_key")


FEATURES = [("health", d_health), ("task.submit", d_submit), ("task.get", d_get), ("task.list", d_list), ("task.cancel", d_cancel)]
