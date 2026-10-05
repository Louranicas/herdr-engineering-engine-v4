"""judge.inspect: held behind H-8. The drive measures the refusal and the catalogue entry only;
it sends nothing anywhere but the control socket (no judge endpoint, no egress)."""
from drive_d import NO_LEDGER_REASON, is_result, ledger_ro


def ops_count(ctx):
    c = ledger_ro(ctx)
    if c is None:
        return None
    try:
        return c.execute("select count(*) from operations").fetchone()[0]
    finally:
        c.close()


def d_judge(F, ctx):
    ops0 = ops_count(ctx)
    F.refuse("held", F.req("judge.inspect", {}), "unavailable", "/action", because="H-8")
    c = F.req("tools.inspect", {"action": "judge.inspect", "version": 1})
    F.check("catalogued", is_result(c) and (c.get("body") or {}).get("action") == "judge.inspect", f"{c}")
    lst = F.req("tools.list", {"query": "judge"})
    ids = [i.get("id") for i in ((lst or {}).get("body") or {}).get("page", {}).get("items") or []]
    F.check("listed", is_result(lst) and "judge.inspect" in ids, f"ids={ids}")
    if ops0 is None:
        return F.check("no_operations_row", False, NO_LEDGER_REASON, unmeasured=True)
    ops1 = ops_count(ctx)
    F.check("no_operations_row", ops0 == ops1, f"operations before={ops0} after={ops1}")


FEATURES = [("judge.inspect", d_judge)]
