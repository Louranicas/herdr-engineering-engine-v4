"""The tools family: tools.list and tools.inspect (gates/features/tools.*.md).

The expected id set is read from gates/features (every *.md but README and the two cross-cutting
files), never written here; the page bound and the query bound are read from the refusal
messages, never written here. Both actions are reads: with --ledger the operations count is
unchanged, without it that path is UNMEASURED naming --ledger.
"""
import os
import re

from drive_d import NO_LEDGER_REASON, is_result, ledger_ro

FEATURES_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), "gates", "features")
CROSS_CUTTING = {"README", "crash-restart", "multi-surface-journeys"}


def expected_ids():
    return {f[:-3] for f in os.listdir(FEATURES_DIR) if f.endswith(".md")} - CROSS_CUTTING


def body_of(r):
    return ((r or {}).get("body") or {}) if is_result(r) else {}


def page_of(r):
    return body_of(r).get("page") or {}


def tools_list(F, query=None, page=None):
    body = {"query": query}
    if page is not None:
        body["page"] = page
    return F.req("tools.list", body)


def walk(F, query=None, limit=None):
    """Every page until the cursor is null: (pages of item lists, last cursor seen non-null, replies)."""
    pages, cursor, seen, guard = [], None, None, 0
    while True:
        page = None if limit is None and cursor is None else {"limit": limit, "cursor": cursor}
        r = tools_list(F, query, page)
        if not is_result(r):
            return pages, seen, r
        p = page_of(r)
        pages.append(p.get("items") or [])
        cursor = p.get("cursor")
        guard += 1
        if cursor is None or guard > len(expected_ids()) + 1:
            return pages, seen, r
        seen = cursor


def ints(msg):
    return [int(x) for x in re.findall(r"\d+", str(msg))]


def ops_count(ctx):
    c = ledger_ro(ctx)
    if c is None:
        return None
    try:
        return c.execute("select count(*) from operations").fetchone()[0]
    finally:
        c.close()


def no_ops_row(F, ctx, before):
    if before is None:
        return F.check("no_operations_row", False, NO_LEDGER_REASON, unmeasured=True)
    after = ops_count(ctx)
    F.check("no_operations_row", before == after, f"operations before={before} after={after}")


def d_tools_list(F, ctx):
    ops0 = ops_count(ctx)
    r = tools_list(F)
    rev = body_of(r).get("catalogue_revision")
    pages, _, last = walk(F)
    items = [i for p in pages for i in p]
    ids = {i.get("id") for i in items}
    want = expected_ids()
    shaped = all({"id", "version", "purpose", "effect"} <= set(i) for i in items)
    F.check("success", is_result(r) and re.fullmatch(r"[0-9a-f]{64}", str(rev)) is not None and is_result(last)
            and ids == want and shaped, f"revision={rev} missing={sorted(want - ids)} extra={sorted(ids - want)} shaped={shaped}")
    q = [i for p in walk(F, "task")[0] for i in p]
    F.check("query_subset", bool(q) and all("task" in i.get("id", "") or "task" in i.get("purpose", "") for i in q)
            and "task.submit" in {i.get("id") for i in q}, f"ids={[i.get('id') for i in q]}")
    e = tools_list(F, "zz-no-such-id")
    F.check("query_empty", is_result(e) and page_of(e).get("items") == [] and page_of(e).get("cursor") is None, f"{e}")
    long = tools_list(F, "x" * 257)
    F.refuse("query_too_long", long, "invalid_argument", "/body/query")
    F.check("query_too_long_names_both", len(ints((long or {}).get("message"))) >= 2, f"message={(long or {}).get('message')}")
    for name, limit in (("limit_zero", 0), ("limit_over", 1000000)):
        x = tools_list(F, page={"limit": limit, "cursor": None})
        F.refuse(name, x, "invalid_argument", "/body/page/limit")
        F.check(f"{name}_names_both", len(ints((x or {}).get("message"))) >= 2, f"message={(x or {}).get('message')}")
    small, cur, _ = walk(F, limit=5)
    flat = [i.get("id") for p in small for i in p]
    F.check("paging", len(small) > 1 and all(len(p) <= 5 for p in small) and len(flat) == len(set(flat)) and set(flat) == ids,
            f"pages={len(small)} sizes={[len(p) for p in small]}")
    r2 = tools_list(F)
    F.check("revision_stable", is_result(r2) and body_of(r2).get("catalogue_revision") == rev, f"{rev} vs {body_of(r2).get('catalogue_revision')}")
    if not isinstance(cur, dict) or not isinstance(cur.get("boot"), int):
        F.check("stale_cursor", False, f"no cursor object from the limit-5 walk: {cur}")
    else:
        moved = dict(cur, boot=cur["boot"] + 1)
        F.refuse("stale_cursor", tools_list(F, page={"limit": 5, "cursor": moved}), "resync_required", "/body/page/cursor",
                 because="epoch moved")
    no_ops_row(F, ctx, ops0)


def inspect(F, body):
    return F.req("tools.inspect", body)


def d_tools_inspect(F, ctx):
    ops0 = ops_count(ctx)
    r = inspect(F, {"action": "task.submit", "version": 1})
    b = body_of(r)
    hexes = all(re.fullmatch(r"[0-9a-f]{64}", str(b.get(k))) for k in ("request_schema_sha256", "result_schema_sha256", "error_schema_sha256"))
    pos = all(isinstance(b.get(k), int) and b[k] > 0 for k in ("max_request_bytes", "max_deadline_ms"))
    F.check("task_submit", b.get("effect") == "durable_admission" and b.get("readback_action") == "task.get" and hexes and pos, f"{r}")
    h = inspect(F, {"action": "health", "version": 1})
    F.check("health", is_result(h) and "readback_action" in body_of(h) and body_of(h)["readback_action"] is None, f"{h}")
    j = inspect(F, {"action": "judge.inspect", "version": 1})
    F.check("judge_inspect", body_of(j).get("action") == "judge.inspect", f"{j}")
    F.refuse("unknown", inspect(F, {"action": "no.such.action", "version": 1}), "unknown_action", "/body/action")
    F.refuse("wrong_version", inspect(F, {"action": "task.submit", "version": 2}), "unsupported_action_version", "/body/version")
    F.refuse("missing_action", inspect(F, {}), "invalid_argument", "/body/action")
    no_ops_row(F, ctx, ops0)


FEATURES = [("tools.list", d_tools_list), ("tools.inspect", d_tools_inspect)]
