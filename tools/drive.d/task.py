"""The task family: health, task.submit, task.get, task.list, task.cancel (moved verbatim from
tools/drive), task.preview, task.resolve (E2E-09) and events.subscribe (the stream helper)."""
import json
import os
import re
import socket
import stat
import subprocess
import time
import uuid

from drive_d import BRIEF, NO_LEDGER_REASON, PHASES, TERMINAL, VACUOUS_VERIFY_BRIEF, is_result, ledger_ro
from drive_d.roster import items_of, list_body

# A brief whose VERIFY keeps the task non-terminal for at least five seconds once dispatched, so
# E2E-09's cancel and resolve land on a live task without a race against the verdict.
SLEEP_BRIEF = (BRIEF.replace("VERIFY: /usr/bin/test -w .", "VERIFY: /usr/bin/sleep 5")
               .replace("RESTATEMENT: check the work dir is writable", "RESTATEMENT: run sleep in the namespace"))
# The three vacuous VERIFY spellings the door refuses (hee4-contracts verify.rs no-op table).
VACUOUS = {"sh_true": BRIEF.replace("VERIFY: /usr/bin/test -w .", "VERIFY: sh: true"),
           "abs_true": VACUOUS_VERIFY_BRIEF,
           "nothing_runs": BRIEF.replace("VERIFY: /usr/bin/test -w .", "VERIFY: model: hi")}
NO_TASK = "t-" + "0" * 24
WAIT_S = 10  # the longest any socket read waits (brief: <= 10 s)



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




def body_of(r):
    return ((r or {}).get("body") or {}) if is_result(r) else {}


def task_count(F):
    r = F.req("task.list", {})
    return len(body_of(r).get("tasks", [])) if is_result(r) else None


def ops_count(ctx):
    c = ledger_ro(ctx)
    if c is None:
        return None
    try:
        return c.execute("select count(*) from operations").fetchone()[0]
    finally:
        c.close()


def disabled_models(F):
    """The model record ids this serve holds disabled (roster.disable retired them; re-enable is not an action)."""
    r = F.req("roster.list", list_body(kinds=["model"], include_disabled=True))
    return {i.get("id") for i in items_of(r) if i.get("disabled") is True}


def d_preview(F, ctx):
    ops0 = ops_count(ctx)
    n0 = task_count(F)
    r = F.req("task.preview", {"brief": BRIEF})
    b = body_of(r)
    retired = disabled_models(F) if is_result(r) and b.get("refusal") == "no_route" else set()
    if b.get("exclusions") and set(b["exclusions"]) <= retired:
        F.check("success", False, f"no eligible model: {sorted(retired)} disabled on this serve by an earlier roster.disable", unmeasured=True)
    else:
        F.check("success", is_result(r) and b.get("eligible") is True and isinstance(b.get("model"), str) and b["model"] != "", f"{r}")
    restatement = BRIEF[BRIEF.index("RESTATEMENT:"):]
    m = body_of(F.req("task.preview", {"brief": BRIEF.replace(restatement, "")}))
    F.check("missing_restatement", m.get("eligible") is False and m.get("refusal") == "invalid_argument"
            and "RESTATEMENT" in str(m.get("message")), f"{m}")
    for name, brief in VACUOUS.items():
        v = body_of(F.req("task.preview", {"brief": brief}))
        F.check(f"preview_vacuous_{name}", v.get("eligible") is False and v.get("refusal") == "invalid_argument"
                and "VERIFY" in str(v.get("message")), f"{v}")
    F.refuse("brief_not_string", F.req("task.preview", {"brief": 5}), "invalid_argument", "/body/brief")
    n1 = task_count(F)
    F.check("no_admission", n0 is not None and n0 == n1, f"task.list rows before={n0} after={n1}")
    ops1 = ops_count(ctx)
    if ops0 is None:
        F.check("no_operations_row", False, NO_LEDGER_REASON, unmeasured=True)
    else:
        F.check("no_operations_row", ops0 == ops1, f"operations before={ops0} after={ops1}")


def get_phase(F, t):
    return body_of(F.req("task.get", {"task_id": t})).get("phase")


def resolve(F, t, resolution, reason=None, key=True):
    body = {"task_id": t, "resolution": resolution}
    if reason is not None:
        body["reason"] = reason
    return F.req("task.resolve", body, key=str(uuid.uuid4()) if key else None)


def submit_sleep(F):
    return body_of(F.req("task.submit", {"brief": SLEEP_BRIEF}, key=str(uuid.uuid4()))).get("task_id")


RUNNING_WAIT_S = 10  # the bound on waiting for a SLEEP_BRIEF task to reach `running` before cancel


def stopped_first(t, ph):
    """Why the dispatcher, not the drive, ended `t` in phase `ph`: abandoned names no eligible model
    (e.g. roster.disable earlier in the run), cancelled names the cancel-before-dispatch race; None
    for any other phase."""
    if ph == "abandoned":
        return f"dispatcher abandoned {t} before the drive's frame (no eligible model on this serve)"
    if ph == "cancelled":
        return f"dispatcher cancelled {t} before the drive's frame (cancel-before-dispatch race: the cancel landed before running)"
    return None


def wait_running(F, t, bound=RUNNING_WAIT_S):
    """Poll task.get until `t` is `running`: (True, None), or (False, reason). A terminal phase seen
    first gives its cause (stopped_first) at once; only an exhausted bound names the bound and the
    last phase seen. A cancel on a running task with an open attempt stays cancellation_requested;
    one that lands before dispatch is Stopped to `cancelled` by the dispatcher (wave 5)."""
    ph, t0 = None, time.monotonic()
    while time.monotonic() - t0 < bound:
        ph = get_phase(F, t)
        if ph == "running":
            return True, None
        if ph in TERMINAL:
            return False, stopped_first(t, ph) or f"task={t} reached terminal phase={ph} before running: no task to cancel"
        time.sleep(0.05)
    return False, f"task={t} never reached running within {bound}s (last phase={ph}): cancel would race the dispatch"


def raced(F, t, reply):
    """The reason a lifecycle path cannot be measured: a conflict on a task the dispatcher already
    ended (stopped_first); None when the reply is no conflict or the task is not so ended."""
    if (reply or {}).get("code") != "conflict":
        return None
    return stopped_first(t, get_phase(F, t))


def d_resolve(F, ctx):
    """E2E-09: cancel then abandon -> cancelled; quarantine -> blocked; abandon a blocked task -> abandoned."""
    # task.resolve's catalogue entry is PreconditionRule::None (crates/hee4-contracts/src/catalogue.rs:351).
    print("  note=stale_generation unreachable reason=task.resolve PreconditionRule::None (catalogue.rs:351)")
    t = submit_sleep(F)
    up, why = wait_running(F, t) if t else (False, None)
    c = F.req("task.cancel", {"task_id": t}, key=str(uuid.uuid4())) if up else None
    why = why or (raced(F, t, c) if up else None)
    if why:
        F.check("cancel_then_abandon", False, why, unmeasured=True)
    else:
        a = resolve(F, t, "abandon", "attempt_failed")
        got = get_phase(F, t)
        ok = body_of(c).get("phase") == "cancellation_requested" and body_of(a).get("phase") == "cancelled" and got == "cancelled"
        F.check("cancel_then_abandon", ok, f"task={t} cancel={c} resolve={a} get={got}")
        if ok:
            ctx["resolved_task"] = t
    q = submit_sleep(F)
    qr = resolve(F, q, "quarantine") if q else None
    why = raced(F, q, qr) if q else None
    if why:
        F.check("quarantine", False, why, unmeasured=True)
        F.check("abandon_blocked", False, why, unmeasured=True)
    else:
        got = get_phase(F, q)
        F.check("quarantine", body_of(qr).get("phase") == "blocked" and got == "blocked", f"task={q} resolve={qr} get={got}")
        ab = resolve(F, q, "abandon")
        F.check("abandon_blocked", body_of(ab).get("phase") == "abandoned", f"{ab}")
    if q:
        F.refuse("resolve_terminal", resolve(F, q, "abandon"), "conflict", "/body/task_id")
    else:
        F.check("resolve_terminal", False, "no task admitted for the terminal path")
    F.refuse("not_found", resolve(F, NO_TASK, "abandon"), "not_found", "/body/task_id")
    F.refuse("bad_resolution", resolve(F, q or NO_TASK, "retry"), "invalid_argument", "/body/resolution")
    F.refuse("bad_reason", resolve(F, q or NO_TASK, "quarantine", "attempt_failed"), "invalid_argument", "/body/reason")
    F.refuse("missing_key", resolve(F, q or NO_TASK, "abandon", key=False), "invalid_argument", "/idempotency_key")


def stream(F, body, timeout, until=lambda fr: False, after_ack=None):
    """events.subscribe over one socket kept open: [ack, frame, ...] until `until(frame)`, EOF or
    `timeout` seconds. Every line sent and received is logged to F.f as {"t","dir","raw"}; no read
    waits longer than WAIT_S. `after_ack()` runs once the ack is in (the live-follow producer)."""
    obj = {"request_id": "drive-" + uuid.uuid4().hex[:12], "action": "events.subscribe", "action_version": 1,
           "idempotency_key": None, "body": body}
    raw = json.dumps(obj, separators=(",", ":"))
    F.f.write(json.dumps({"t": time.time(), "dir": "send", "raw": raw}) + "\n")
    out, deadline = [], time.monotonic() + timeout
    s = socket.socket(socket.AF_UNIX)
    try:
        s.settimeout(min(WAIT_S, timeout))
        s.connect(F.sock)
        s.sendall(raw.encode() + b"\n")
        buf = b""
        while time.monotonic() < deadline:
            s.settimeout(max(0.05, min(WAIT_S, deadline - time.monotonic())))
            try:
                c = s.recv(65536)
            except socket.timeout:
                break
            if not c:
                break
            buf += c
            while b"\n" in buf:
                line, buf = buf.split(b"\n", 1)
                text = line.decode(errors="replace")
                F.f.write(json.dumps({"t": time.time(), "dir": "recv", "raw": text}) + "\n")
                try:
                    fr = json.loads(text)
                except ValueError:
                    fr = None
                out.append(fr)
                if len(out) == 1 and after_ack:
                    after_ack()
                elif len(out) > 1 and fr and until(fr):
                    return out
    except OSError as e:
        F.f.write(json.dumps({"t": time.time(), "dir": "io_error", "raw": str(e)}) + "\n")
    finally:
        s.close()
        F.f.flush()
    return out


FRAME_KEYS = {"kind", "seq", "task_id", "event", "phase_after", "ts"}


def d_subscribe(F, ctx):
    t0 = time.monotonic()
    ack = F.req("events.subscribe", {"since_seq": 0, "epoch": None})
    a = body_of(ack)
    epoch, hw = a.get("epoch"), a.get("high_water")
    ok = (is_result(ack) and a.get("stream") == "events" and isinstance(epoch, str) and epoch != ""
          and isinstance(hw, int) and hw >= 0 and a.get("since_seq") == 0)
    F.check("ack", ok, f"{ack}")
    if not ok:
        return F.check("replay", False, "no ack to resume from")
    frames = stream(F, {"since_seq": 0, "epoch": epoch}, WAIT_S, until=lambda fr: fr.get("seq", -1) >= hw)[1:] if hw else []
    seqs = [fr.get("seq") for fr in frames if fr]
    shaped = all(fr and set(fr) >= FRAME_KEYS and fr.get("kind") == "event" for fr in frames)
    ascending = all(isinstance(x, int) for x in seqs) and all(x < y for x, y in zip(seqs, seqs[1:]))
    F.check("replay", bool(frames) and shaped and ascending and seqs[-1] >= hw, f"frames={len(frames)} high_water={hw} shaped={shaped} ascending={ascending}")
    mine = [fr for fr in frames if fr and fr.get("task_id") == ctx.get("task")]
    if not ctx.get("task"):
        F.check("replay_submit_admitted", False, "no task from task.submit", unmeasured=True)
    else:
        F.check("replay_submit_admitted", bool(mine) and mine[0].get("phase_after") == "admitted", f"first frame for {ctx['task']}: {mine[:1]}")
    rt = ctx.get("resolved_task")
    if not rt:
        F.check("replay_resolve_producer", False, "no resolved task from task.resolve (cancel_then_abandon not measured)", unmeasured=True)
    else:
        F.check("replay_resolve_producer", any(fr.get("task_id") == rt and fr.get("phase_after") == "cancelled" for fr in frames if fr),
                f"a cancelled frame for {rt}")
    live = {}

    def produce():
        live["task"] = body_of(F.req("task.submit", {"brief": BRIEF}, key=str(uuid.uuid4()))).get("task_id")

    got = stream(F, {"since_seq": hw, "epoch": epoch}, WAIT_S, after_ack=produce,
                 until=lambda fr: fr.get("task_id") == live.get("task") and fr.get("phase_after") == "admitted")
    hit = [fr for fr in got[1:] if fr and fr.get("task_id") == live.get("task") and fr.get("phase_after") == "admitted"]
    F.check("live_follow", bool(live.get("task")) and bool(hit), f"task={live.get('task')} frames={len(got) - 1 if got else 0}")
    last = max([x for x in seqs if isinstance(x, int)] + [fr.get("seq") for fr in got[1:] if fr and isinstance(fr.get("seq"), int)] or [hw])
    again = stream(F, {"since_seq": last, "epoch": epoch}, 1)
    dup = [fr.get("seq") for fr in again[1:] if fr and isinstance(fr.get("seq"), int) and fr["seq"] <= last]
    F.check("resume_exactly_once", is_result(again[0] if again else None) and not dup, f"since_seq={last} frames at or below it={dup}")
    F.refuse("bad_since_seq", F.req("events.subscribe", {"since_seq": -1, "epoch": None}), "invalid_argument", "/body/since_seq")
    F.refuse("resync_wrong_epoch", F.req("events.subscribe", {"since_seq": 0, "epoch": "not-the-epoch"}), "resync_required", "/body/epoch")
    F.refuse("resync_future_seq", F.req("events.subscribe", {"since_seq": hw + 1000, "epoch": epoch}), "resync_required", "/body/since_seq")
    F.check("slot_freed", is_result(F.req("health", {})), "health answers after every stream closed")
    print(f"  elapsed_s={time.monotonic() - t0:.1f}")


FEATURES = [("health", d_health), ("task.submit", d_submit), ("task.get", d_get), ("task.list", d_list), ("task.cancel", d_cancel),
            ("task.preview", d_preview), ("task.resolve", d_resolve), ("events.subscribe", d_subscribe)]
