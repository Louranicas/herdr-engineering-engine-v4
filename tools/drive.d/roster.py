"""The roster family (v4.1): roster.list, roster.inspect, roster.update, roster.disable.

Order matters: the four procedures share ctx["roster"] (the record the update path creates) and
the disable path runs last because it disables the engine's only model record (the serve is
disposable; re-enable is not an action). Every refusal is checked against the FLOW.md table.
With --ledger the ledger-side checks read `operations` and `roster_revisions` read-only; without
it they print UNMEASURED naming --ledger.
"""
import hashlib
import json
import os
import time
import uuid

from drive_d import NO_LEDGER_REASON, TERMINAL, is_result, ledger_ro

MODEL = os.environ.get("HEE4_MODEL", "qwen2.5-coder:7b")
DEPLOY_ID = "model:" + MODEL
DEPLOY_ACTION = "deploy.install"
RECORD_ID = "model:drive-" + uuid.uuid4().hex[:8]
DEFINITION = {"kind": "model", "caps": {"ctx_tokens": 4096, "json_mode": True, "tool_use": False, "local": True},
              "cost_milli": 0, "latency_ms": 0, "quality": 1, "capability": None, "locality": "local"}
BRIEF = ("GOAL: drive\nSCOPE: s\nCONTEXT: c\nACCEPTANCE: a\nVERIFY: /usr/bin/true\nTIMEBOX: 10s\n"
         "FORBIDDEN: f\nREPORT: r\nSTANDING: s\nRECON: r\nRESTATEMENT: run true\n")


def list_body(include_disabled=False, kinds=(), capability=None, locality=None, limit=100, cursor=None):
    return {"kinds": list(kinds), "capability": capability, "locality": locality,
            "include_disabled": include_disabled, "page": {"limit": limit, "cursor": cursor}}


def canonical(v):
    return json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def filter_digest(body):
    f = {"kinds": sorted(set(body["kinds"])), "capability": body["capability"], "locality": body["locality"],
         "include_disabled": body["include_disabled"]}
    return hashlib.sha256(canonical(f).encode()).hexdigest()


def items_of(r):
    return ((r or {}).get("body") or {}).get("page", {}).get("items") or []


def ids_of(r):
    return [i.get("id") for i in items_of(r)]


def count(ctx, sql, *args):
    """A read-only count from the ledger, or None without --ledger."""
    c = ledger_ro(ctx)
    if c is None:
        return None
    try:
        return c.execute(sql, args).fetchone()[0]
    finally:
        c.close()


def d_list(F, ctx):
    shown = F.req("roster.list", list_body(include_disabled=True))
    F.check("include_disabled_true", is_result(shown) and DEPLOY_ID in ids_of(shown), f"ids={ids_of(shown)}")
    # A serve an earlier drive already ran roster.disable against holds the deploy record disabled
    # (re-enable is not an action): then the hidden listing must NOT carry it.
    retired = any(h.get("id") == DEPLOY_ID and h.get("disabled") is True for h in items_of(shown))
    r = F.req("roster.list", list_body())
    heads = items_of(r)
    shape = all(set(h) == {"id", "kind", "generation", "disabled", "capability", "locality"} for h in heads)
    F.check("success", is_result(r) and r.get("replayed") is False and (DEPLOY_ID in ids_of(r)) != retired and shape
            and (heads or retired) and all(h["disabled"] is False for h in heads), f"ids={ids_of(r)} shape_ok={shape} retired={retired}")
    for member, field in (("kinds", "/body/kinds"), ("capability", "/body/capability"), ("locality", "/body/locality"),
                          ("include_disabled", "/body/include_disabled"), ("page", "/body/page/limit")):
        body = list_body(); del body[member]
        F.refuse(f"missing_{member}", F.req("roster.list", body), "invalid_argument", field)
    F.refuse("limit_0", F.req("roster.list", list_body(limit=0)), "invalid_argument", "/body/page/limit")
    F.refuse("limit_101", F.req("roster.list", list_body(limit=101)), "invalid_argument", "/body/page/limit")
    body = list_body()
    stale = {"after_key": "a", "boot": 0, "filter_sha256": filter_digest(body)}
    F.refuse("cursor_other_boot", F.req("roster.list", list_body(cursor=stale)), "resync_required", "/body/page/cursor",
             because="epoch moved")
    F.refuse("kind_unknown", F.req("roster.list", list_body(kinds=["cpu"])), "invalid_argument", "/body/kinds")
    empty = F.req("roster.list", list_body(kinds=["runtime"]))
    F.check("empty_is_a_result", is_result(empty) and items_of(empty) == [] and empty["body"]["page"]["cursor"] is None, f"{empty}")


def d_inspect(F, ctx):
    r = F.req("roster.inspect", {"selector": {"record_id": DEPLOY_ID}})
    b = (r or {}).get("body") or {}
    rec, last = b.get("record") or {}, b.get("last_operation") or {}
    # retired: an earlier drive on this serve disabled the record (its last operation is that disable).
    retired = rec.get("disabled") is True
    want_action = "roster.disable" if retired else DEPLOY_ACTION
    F.check("by_id", is_result(r) and rec.get("id") == DEPLOY_ID and rec.get("kind") == "model"
            and (rec.get("generation") == 1 or retired) and isinstance(rec.get("definition"), dict)
            and isinstance(rec.get("updated_ts"), int) and set(last) == {"operation_id", "action", "idempotency_key", "ts"}, f"{r}")
    F.check("deploy_record_is_not_a_roster_update", is_result(r) and last.get("action") == want_action,
            f"last_operation.action={last.get('action')} want {want_action} (retired={retired})")
    ctx["roster_generation"] = rec.get("generation")
    # The E2E-12 half: no roster.update row about the deploy record, ever; on a fresh serve none at all.
    n = count(ctx, "select count(*) from operations where action = ? and (? or subject = ?)", "roster.update", not retired, DEPLOY_ID)
    if n is None:
        F.check("no_roster_update_row_after_install", False, NO_LEDGER_REASON, unmeasured=True)
    else:
        F.check("no_roster_update_row_after_install", n == 0,
                f"operations under roster.update {'in the ledger' if not retired else 'about ' + DEPLOY_ID}: {n}")
    d = count(ctx, "select count(*) from operations where action = ? and principal = 'deploy'", DEPLOY_ACTION)
    if d is None:
        F.check("one_deploy_install_row", False, NO_LEDGER_REASON, unmeasured=True)
    else:
        F.check("one_deploy_install_row", d == 1, f"operations under {DEPLOY_ACTION}: {d}")
    F.refuse("not_found", F.req("roster.inspect", {"selector": {"record_id": "model:no-such-record"}}), "not_found", "/body/selector")
    F.refuse("selector_both_forms", F.req("roster.inspect", {"selector": {"record_id": DEPLOY_ID, "source_action": "roster.update", "idempotency_key": "k"}}),
             "invalid_argument", "/body/selector")
    F.refuse("selector_missing", F.req("roster.inspect", {}), "invalid_argument", "/body/selector")


def d_update(F, ctx):
    k = str(uuid.uuid4())
    body = {"record_id": RECORD_ID, "definition": DEFINITION, "audit_reason": "drive create"}
    before = count(ctx, "select count(*) from roster_revisions where record_id = ?", RECORD_ID)
    r = F.req("roster.update", body, key=k)
    b = (r or {}).get("body") or {}
    ok = is_result(r) and r.get("replayed") is False and b.get("change") == "created" and (b.get("record") or {}).get("generation") == 1 \
        and isinstance(b.get("operation_id"), str)
    F.check("create", ok, f"{r}")
    if ok:
        ctx["roster"] = {"id": RECORD_ID, "key": k, "operation_id": b["operation_id"]}
    after = count(ctx, "select count(*) from roster_revisions where record_id = ?", RECORD_ID)
    if after is None:
        F.check("one_revision_row", False, NO_LEDGER_REASON, unmeasured=True)
    else:
        F.check("one_revision_row", after == (before or 0) + 1, f"revisions {before} -> {after}")
    rr = F.req("roster.update", body, key=k)
    F.check("replay", is_result(rr) and rr.get("replayed") is True and (rr.get("body") or {}).get("operation_id") == b.get("operation_id")
            and rr.get("body") == b, f"{rr}")
    again = count(ctx, "select count(*) from roster_revisions where record_id = ?", RECORD_ID)
    if again is None:
        F.check("replay_adds_no_revision", False, NO_LEDGER_REASON, unmeasured=True)
    else:
        F.check("replay_adds_no_revision", again == after, f"revisions after replay {again} want {after}")
    F.refuse("conflict", F.req("roster.update", dict(body, audit_reason="other bytes"), key=k), "conflict", "/idempotency_key")
    F.refuse("empty_definition", F.req("roster.update", dict(body, definition={}), key=str(uuid.uuid4())), "invalid_argument", "/body/definition")
    F.refuse("missing_key", F.req("roster.update", body), "invalid_argument", "/idempotency_key")
    F.refuse("bad_record_id", F.req("roster.update", dict(body, record_id="no colon"), key=str(uuid.uuid4())), "invalid_argument", "/body/record_id")
    rev = F.req("roster.update", dict(body, audit_reason="drive revise"), key=str(uuid.uuid4()),
                precondition={"resource": "roster", "id": RECORD_ID, "generation": 1})
    rb = (rev or {}).get("body") or {}
    F.check("revise_with_precondition", is_result(rev) and rb.get("change") == "revised" and (rb.get("record") or {}).get("generation") == 2, f"{rev}")
    F.refuse("stale_generation", F.req("roster.update", dict(body, audit_reason="stale"), key=str(uuid.uuid4()),
                                       precondition={"resource": "roster", "id": RECORD_ID, "generation": 1}),
             "stale_generation", "/precondition/generation", extra={"current_generation": 2})
    F.refuse("precondition_other_id", F.req("roster.update", body, key=str(uuid.uuid4()),
                                            precondition={"resource": "roster", "id": DEPLOY_ID, "generation": 2}),
             "invalid_argument", "/precondition/id")
    ins = F.req("roster.inspect", {"selector": {"source_action": "roster.update", "idempotency_key": k}})
    ib = (ins or {}).get("body") or {}
    F.check("readback_by_key", is_result(ins) and (ib.get("record") or {}).get("id") == RECORD_ID
            and (ib.get("record") or {}).get("generation") == 2 and (ib.get("last_operation") or {}).get("action") == "roster.update", f"{ins}")
    F.check("forbidden_operator_capability", False, "no grant exists; owner: grants slice (gates/features/README.md:67 UNWRITTEN)", unmeasured=True)


def d_disable(F, ctx):
    body = {"record_id": DEPLOY_ID, "active_attempt_policy": "let_finish", "audit_reason": "drive retire"}
    g = ctx.get("roster_generation") or 1
    F.refuse("missing_precondition", F.req("roster.disable", body, key=str(uuid.uuid4())), "invalid_argument", "/precondition")
    F.refuse("missing_key", F.req("roster.disable", body, precondition={"resource": "roster", "id": DEPLOY_ID, "generation": g}),
             "invalid_argument", "/idempotency_key")
    F.refuse("bad_policy", F.req("roster.disable", dict(body, active_attempt_policy="shrug"), key=str(uuid.uuid4()),
                                 precondition={"resource": "roster", "id": DEPLOY_ID, "generation": g}),
             "invalid_argument", "/body/active_attempt_policy")
    F.refuse("not_found", F.req("roster.disable", dict(body, record_id="model:no-such-record"), key=str(uuid.uuid4()),
                                precondition={"resource": "roster", "id": "model:no-such-record", "generation": 1}),
             "not_found", "/body/record_id")
    # The last mutating path: it disables the engine's only model record (disposable serve).
    k = str(uuid.uuid4())
    r = F.req("roster.disable", body, key=k, precondition={"resource": "roster", "id": DEPLOY_ID, "generation": g})
    b = (r or {}).get("body") or {}
    F.check("disable", is_result(r) and r.get("replayed") is False and (b.get("record") or {}).get("disabled") is True
            and (b.get("record") or {}).get("generation") == g + 1 and isinstance(b.get("operation_id"), str), f"{r}")
    F.check("empty_obligations", is_result(r) and b.get("active_attempts") == [] and b.get("cancellation_obligations") == [],
            f"active_attempts={b.get('active_attempts')} cancellation_obligations={b.get('cancellation_obligations')}")
    rr = F.req("roster.disable", body, key=k, precondition={"resource": "roster", "id": DEPLOY_ID, "generation": g})
    F.check("replay", is_result(rr) and rr.get("replayed") is True and (rr.get("body") or {}).get("record") == b.get("record"), f"{rr}")
    F.refuse("stale_generation", F.req("roster.disable", body, key=str(uuid.uuid4()),
                                       precondition={"resource": "roster", "id": DEPLOY_ID, "generation": g}),
             "stale_generation", "/precondition/generation", extra={"current_generation": g + 1})
    hidden = F.req("roster.list", list_body())
    F.check("list_hides_disabled", is_result(hidden) and DEPLOY_ID not in ids_of(hidden), f"ids={ids_of(hidden)}")
    shown = F.req("roster.list", list_body(include_disabled=True))
    marks = {i["id"]: i.get("disabled") for i in items_of(shown)}
    F.check("include_disabled_shows_it", is_result(shown) and marks.get(DEPLOY_ID) is True, f"{marks}")
    n = count(ctx, "select count(*) from operations where action = 'roster.disable' and idem_key = ? and subject = ?", k, DEPLOY_ID)
    if n is None:
        F.check("operations_row", False, NO_LEDGER_REASON, unmeasured=True)
    else:
        F.check("operations_row", n == 1, f"operations under roster.disable for this key (sent twice, replayed once): {n}")
    pv = F.req("task.preview", {"brief": BRIEF})
    pb = (pv or {}).get("body") or {}
    F.check("preview_excludes_disabled", is_result(pv) and pb.get("eligible") is False and DEPLOY_ID in (pb.get("exclusions") or []),
            f"{pv}")
    # The dispatcher must not route to the disabled model: a submitted task ends abandoned.
    s = F.req("task.submit", {"brief": BRIEF}, key=str(uuid.uuid4()))
    t = ((s or {}).get("body") or {}).get("task_id")
    ph, t0 = None, time.monotonic()
    while t and time.monotonic() - t0 < 5 and ph not in TERMINAL:
        g2 = F.req("task.get", {"task_id": t}); ph = ((g2 or {}).get("body") or {}).get("phase")
        if ph not in TERMINAL:
            time.sleep(0.2)
    F.check("dispatch_refuses_disabled_model", ph == "abandoned", f"task={t} phase={ph}")
    F.check("forbidden_operator_capability", False, "no grant exists; owner: grants slice (gates/features/README.md:67 UNWRITTEN)", unmeasured=True)


FEATURES = [("roster.list", d_list), ("roster.inspect", d_inspect), ("roster.update", d_update), ("roster.disable", d_disable)]
