"""multi-surface-journeys: drives E2E-10 (service observation) and E2E-12 (roster install).

The other journeys are covered by the feature file that drives them and are printed as
`  journey=E2E-nn covered_by=<file>` lines, never re-driven as paths: E2E-09 by task.resolve.md,
E2E-11 (held, H-8) by judge.inspect.md (`  journey=E2E-11 covered_by=judge.inspect.md`), the rest by the first feature file their `Files:` line
names in gates/features/multi-surface-journeys.md. Bodies come from roster.py and service.py; no
action is sent for the service ids `self` or `model`.
"""
import os
import re
import time
import uuid

from drive_d import NO_LEDGER_REASON, is_result, ledger_ro
from drive_d.roster import DEPLOY_ACTION, UNIDENTIFIED, deploy_id, items_of, list_body
from drive_d.service import NEVER_ACT, SID, bus_absent, inspect, probe_body

JOURNEYS_MD = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
                           "gates", "features", "multi-surface-journeys.md")
DRIVEN = {"E2E-10", "E2E-12"}
COVERED_BY = {"E2E-09": "task.resolve.md", "E2E-11": "judge.inspect.md"}


def covered():
    """E2E-nn -> the file that covers it, for every journey this module does not drive."""
    out, cur = {}, None
    for line in open(JOURNEYS_MD):
        m = re.match(r"## (E2E-\d\d) ", line)
        if m:
            cur = m.group(1)
        elif cur and cur not in DRIVEN and cur not in out and line.startswith("Files:"):
            f = re.search(r"`([\w.-]+\.md)`", line)
            out[cur] = COVERED_BY.get(cur) or (f.group(1) if f else "none")
    return out


def body_of(r):
    return ((r or {}).get("body") or {}) if is_result(r) else {}


def e2e10(F, ctx):
    names = ["e2e10_probe", "e2e10_inspect_equal", "e2e10_inspect_by_key"]
    why = bus_absent()
    if why:
        for n in names:
            F.check(n, False, f"user bus: {why}", unmeasured=True)
        return
    assert SID not in NEVER_ACT
    k = str(uuid.uuid4())
    p = F.req("service.probe", probe_body(), key=k)
    pb = body_of(p)
    F.check("e2e10_probe", is_result(p) and isinstance(pb.get("observation"), dict) and isinstance(pb.get("operation_id"), str), f"{p}")
    i = body_of(inspect(F))
    F.check("e2e10_inspect_equal", bool(pb) and i.get("cached_health") == pb.get("observation"), f"inspect={i}")
    by = body_of(inspect(F, operation={"source_action": "service.probe", "idempotency_key": k}))
    F.check("e2e10_inspect_by_key", bool(pb) and (by.get("operation") or {}).get("operation_id") == pb.get("operation_id"), f"{by}")


def e2e12(F, ctx):
    names = ("e2e12_list", "e2e12_inspect", "e2e12_update_space_empty")
    rid = deploy_id(F, dict(ctx))  # a copy: ctx["deploy_id"] is roster.py's has-run signal
    if rid is None:
        for n in names:
            F.check(n, False, UNIDENTIFIED, unmeasured=True)
        return
    body = list_body(kinds=["model"])
    r = F.req("roster.list", body)
    items = items_of(r)
    everything = items_of(F.req("roster.list", list_body(kinds=["model"], include_disabled=True)))
    if any(i.get("id") == rid and i.get("disabled") is True for i in everything):
        for n in names:
            F.check(n, False, f"{rid} was retired by an earlier roster.disable on this serve (re-enable is not an action)", unmeasured=True)
        return
    F.check("e2e12_list", is_result(r) and 1 <= len(items) <= body["page"]["limit"] and rid in {i.get("id") for i in items},
            f"record={rid} items={len(items)} limit={body['page']['limit']}")
    ins = F.req("roster.inspect", {"selector": {"record_id": rid}})
    last = body_of(ins).get("last_operation") or {}
    F.check("e2e12_inspect", last.get("action") == DEPLOY_ACTION and last.get("action") != "roster.update", f"record={rid} last_operation={last}")
    if "deploy_id" in ctx:  # roster.py caches the deploy id once it has run in this process
        return F.check("e2e12_update_space_empty", False, "roster.py ran earlier in this process; its roster.update rows are its own",
                       unmeasured=True)
    c = ledger_ro(ctx)
    if c is None:
        return F.check("e2e12_update_space_empty", False, NO_LEDGER_REASON, unmeasured=True)
    try:
        n = c.execute("select count(*) from operations where action = 'roster.update' and subject = ?", (rid,)).fetchone()[0]
    finally:
        c.close()
    F.check("e2e12_update_space_empty", n == 0, f"operations under roster.update about {rid}: {n}")


def d_journeys(F, ctx):
    for j, f in sorted(covered().items()):
        print(f"  journey={j} covered_by={f}")
    for j, fn in (("E2E-10", e2e10), ("E2E-12", e2e12)):
        t0 = time.monotonic()
        fn(F, ctx)
        print(f"  journey={j} elapsed_s={time.monotonic() - t0:.1f}")


FEATURES = [("multi-surface-journeys", d_journeys)]
