"""The service family: service.inspect, service.probe, service.action (gates/features/service.*.md).

Reads go through the probe of the `drive` service (hee4-drive.service); nothing here acts on
`self` (hee4.service) or `model` (ollama.service): send_action refuses them client-side. The
action path needs a transient unit (`systemd-run --user --unit hee4-drive.service --collect
/usr/bin/sleep 300`), which starts a process under the user manager, so it runs only when
HEE4_DRIVE_SERVICE_UNIT=1; otherwise its unit paths are UNMEASURED naming that variable.
"""
import hashlib
import os
import subprocess
import uuid

from drive_d import is_result, ledger_ro

UNIT = "hee4-drive.service"
SID = "drive"
NEVER_ACT = ("self", "model")
UNIT_FLAG = "HEE4_DRIVE_SERVICE_UNIT"
OWNER = hashlib.sha256(b"deploy").hexdigest()
NO_LEDGER = "--ledger not passed"


def sh(*cmd, timeout=30):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return r.returncode, r.stdout.strip(), r.stderr.strip()
    except (OSError, subprocess.TimeoutExpired) as e:
        return None, "", str(e)


def bus_absent():
    """The reason the user bus cannot be reached, or None."""
    xdg = os.environ.get("XDG_RUNTIME_DIR")
    if not xdg or not os.path.exists(os.path.join(xdg, "bus")):
        return "$XDG_RUNTIME_DIR/bus absent"
    return None


def probe_body(**over):
    b = {"service_id": SID, "probe_id": "active_state", "probe_version": 1, "max_cost_microunits": 0, "network_scope": "none"}
    b.update(over)
    return b


def inspect(F, sid=SID, operation=None):
    return F.req("service.inspect", {"service_id": sid, "operation": operation})


def send_action(F, sid, action, generation, owner=OWNER, key=None, precondition=True):
    """One service.action frame; None (nothing sent) for a service the drive must never act on."""
    if sid in NEVER_ACT:
        return None
    body = {"service_id": sid, "unit_id": UNIT, "action": action, "expected_owner_sha256": owner}
    pre = {"resource": "service", "id": sid, "generation": generation} if precondition else None
    return F.req("service.action", body, key=key or str(uuid.uuid4()), precondition=pre)


def unmeasured_all(F, names, reason):
    for n in names:
        F.check(n, False, reason, unmeasured=True)


def d_inspect(F, ctx):
    r = inspect(F)
    b = (r or {}).get("body", {})
    F.check("facts", is_result(r) and b.get("unit_id") == UNIT and b.get("owner_id") == "deploy"
            and b.get("owner_sha256") == OWNER and isinstance(b.get("generation"), int) and b["generation"] >= 1
            and (b.get("cached_health") is None or isinstance(b.get("cached_health"), dict)) and b.get("operation") is None, f"{r}")
    F.refuse("unknown_service", inspect(F, sid="nope"), "not_found", "/body/service_id")
    F.refuse("operation_required", F.req("service.inspect", {"service_id": SID}), "invalid_argument", "/body/operation")
    F.refuse("unknown_operation", inspect(F, operation={"source_action": "service.probe", "idempotency_key": str(uuid.uuid4())}),
             "not_found", "/body/operation")
    why = bus_absent()
    if why:
        return unmeasured_all(F, ["equals_probe", "by_key"], why)
    k = str(uuid.uuid4())
    p = F.req("service.probe", probe_body(), key=k)
    pb = (p or {}).get("body", {})
    after = (inspect(F) or {}).get("body", {})
    F.check("equals_probe", is_result(p) and after.get("cached_health") == pb.get("observation"), f"probe={p} inspect={after}")
    by = (inspect(F, operation={"source_action": "service.probe", "idempotency_key": k}) or {}).get("body", {})
    F.check("by_key", (by.get("operation") or {}).get("operation_id") == pb.get("operation_id") and pb.get("operation_id"), f"{by}")


def d_probe(F, ctx):
    F.refuse("probe_version", F.req("service.probe", probe_body(probe_version=2), key=str(uuid.uuid4())),
             "unsupported_action_version", "/body/probe_version")
    F.refuse("network_scope", F.req("service.probe", probe_body(network_scope="loopback"), key=str(uuid.uuid4())),
             "forbidden", "/body/network_scope")
    F.refuse("probe_id", F.req("service.probe", probe_body(probe_id="cpu"), key=str(uuid.uuid4())),
             "invalid_argument", "/body/probe_id")
    F.refuse("unknown_service", F.req("service.probe", probe_body(service_id="nope"), key=str(uuid.uuid4())),
             "not_found", "/body/service_id")
    F.refuse("missing_key", F.req("service.probe", probe_body()), "invalid_argument", "/idempotency_key")
    why = bus_absent()
    if why:
        return unmeasured_all(F, ["success", "replay", "readback", "ledger"], why)
    k = str(uuid.uuid4())
    r = F.req("service.probe", probe_body(), key=k)
    b = (r or {}).get("body", {})
    obs = b.get("observation") or {}
    F.check("success", is_result(r) and r.get("replayed") is False and obs.get("outcome") == "pass"
            and (obs.get("evidence") or [{}])[0].get("label") == "active_state" and obs.get("source") == "service.probe"
            and b.get("external_effect") == "none" and b.get("cost_microunits") == 0, f"{r}")
    rr = F.req("service.probe", probe_body(), key=k)
    F.check("replay", is_result(rr) and rr.get("replayed") is True and rr.get("body") == b, f"{rr}")
    by = (inspect(F, operation={"source_action": "service.probe", "idempotency_key": k}) or {}).get("body", {})
    F.check("readback", by.get("cached_health") == obs and (by.get("operation") or {}).get("operation_id") == b.get("operation_id"), f"{by}")
    db = ledger_ro(ctx)
    if db is None:
        return F.check("ledger", False, NO_LEDGER, unmeasured=True)
    n = db.execute("SELECT count(*) FROM operations WHERE action='service.probe' AND idem_key=?", (k,)).fetchone()[0]
    F.check("ledger", n == 1, f"operations rows for the key={n}")


def d_action(F, ctx):
    for sid in NEVER_ACT:
        F.check(f"never_{sid}", send_action(F, sid, "stop", 1) is None, f"client-side refusal: no frame sent for {sid}")
    r = inspect(F)
    b = (r or {}).get("body", {})
    g, owner = b.get("generation"), b.get("owner_sha256")
    if not is_result(r) or not isinstance(g, int):
        return F.check("inspect_first", False, f"{r}")
    F.refuse("no_precondition", send_action(F, SID, "stop", g, precondition=False), "invalid_argument", "/precondition")
    F.refuse("wrong_digest", send_action(F, SID, "stop", g, owner="0" * 64), "conflict", "/body/expected_owner_sha256")
    F.refuse("generation_ahead", send_action(F, SID, "stop", g + 1), "stale_generation", "/precondition/generation",
             extra={"current_generation": g})
    paths = ["stop", "systemctl_agrees", "replay", "old_generation", "ledger"]
    why = bus_absent()
    if why:
        return unmeasured_all(F, paths, why)
    if os.environ.get(UNIT_FLAG) != "1":
        return unmeasured_all(F, paths, f"transient unit not started: {UNIT_FLAG}=1 not set")
    sh("systemctl", "--user", "stop", UNIT)
    rc, _, err = sh("systemd-run", "--user", "--unit", UNIT, "--collect", "/usr/bin/sleep", "300")
    if rc != 0:
        return unmeasured_all(F, paths, f"systemd-run failed rc={rc}: {err}")
    try:
        k = str(uuid.uuid4())
        s = send_action(F, SID, "stop", g, owner=owner, key=k)
        sb = (s or {}).get("body", {})
        F.check("stop", is_result(s) and sb.get("observed_state") == "inactive"
                and str(sb.get("owner_job_id", "")).startswith("/org/freedesktop/systemd1/job/")
                and (sb.get("useful_health") or {}).get("outcome") == "pass", f"{s}")
        _, out, _ = sh("systemctl", "--user", "show", "-p", "ActiveState", UNIT)
        F.check("systemctl_agrees", out == "ActiveState=inactive", out)
        rr = send_action(F, SID, "stop", g, owner=owner, key=k)
        F.check("replay", is_result(rr) and rr.get("replayed") is True and rr.get("body") == sb, f"{rr}")
        F.refuse("old_generation", send_action(F, SID, "stop", g, owner=owner), "stale_generation", "/precondition/generation",
                 extra={"current_generation": g + 1})
        db = ledger_ro(ctx)
        if db is None:
            return F.check("ledger", False, NO_LEDGER, unmeasured=True)
        n = db.execute("SELECT count(*) FROM operations WHERE action='service.action' AND idem_key=?", (k,)).fetchone()[0]
        gen = db.execute("SELECT generation FROM service_facts WHERE service_id=?", (SID,)).fetchone()[0]
        F.check("ledger", n == 1 and gen == g + 1, f"operations rows for the key={n} generation={gen} want {g + 1}")
    finally:
        sh("systemctl", "--user", "stop", UNIT)


FEATURES = [
    ("service.inspect", d_inspect),
    ("service.probe", d_probe),
    ("service.action", d_action),
]
