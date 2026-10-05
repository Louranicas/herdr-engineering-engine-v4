# judge.inspect

A-22. Read the judge admission state: question sets, admission state and shadow agreement, from the append-only `judgments` table (digests, not state). **Held**: catalogued under `Owner::Judge` (DC-24, V4-56) with no registry entry until Luke's Engine Data Grant (H-8), so every invocation is refused by name. Effect `Read` (PROPOSAL); no v3 schema exists; reply defined at P9 from judge-admission §8 (API Map P-5).

## Sub-features

- held-refusal: `unavailable`, retry `after_condition`, `because` naming the held grant (H-8) — the only behaviour that exists in any scope before the grant.
- catalogued-not-served: `tools.list` shows the id; `tools.inspect` returns its entry; `Registry::get(Owner::Judge)` misses.
- judgments-read (post-grant, PROPOSAL): question sets `QuestionSet{id, version, digest}`, admission state, shadow agreement (`Advised<T>{decision, advice, agreed}`).
- advice-never-authority: a judgment is advice; the code's EXACT/THRESHOLD decision stands, and an outage reads `UNMEASURED` (E2E-11).

## How to get to it (user POV)

`hee4 judge.inspect` (binary); wrapper "generated (held)"; no Pi tool. In every v4 scope before H-8 the reply is `unavailable` by name. There is no user path to a served reply; the gate that opens it is Luke's grant plus DPA/ZDR, the RC01 paid profile and egress (H-8, H-10, H-11, H-12).

## Driving it with hee4

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive`):

```bash
hee4 judge.inspect --body '{}'      # exits 1: unavailable, because names H-8
# raw: {"request_id":"r","action":"judge.inspect","action_version":1,"idempotency_key":null,"body":{}}
```

Paths (`tools/drive.d/judge.py` `d_judge`; it sends nothing anywhere but the control socket):

- `held`: error `unavailable` at `/action`, `because` names `H-8`.
- `catalogued`: `tools.inspect {action:"judge.inspect", version:1}` → a result with `action` `judge.inspect`.
- `listed`: `tools.list` query `judge` → items contain `judge.inspect`.
- `no_operations_row`: with `--ledger`, the operations count is unchanged.

Preconditions: README shared preconditions.

```bash
hee4 judge.inspect                                   # unavailable by name, because=H-8 (held)
hee4-sh tools.inspect action=judge.inspect version:=1   # catalogued: a result, not an error  # UNMEASURED: hee4-sh exists in no crate
hee4-sh --check judge.inspect                        # the wrapper can name it and print a request  # UNMEASURED: hee4-sh exists in no crate
```

Socket: request `body` not defined (no v3 schema; `UNWRITTEN: the request body, defined at P9`); reply Frame (PROPOSAL; `UNWRITTEN: the reply body from judge-admission §8`). The reachable reply today is the error frame `{code:"unavailable", retry:"after_condition", details:{…because…}}`.

- Held path (the only one): assert the code, the retry and the `because` string naming the held scope, distinct from the v4.1/v4.2 `because` strings (Error map class 10, F-4).
- Catalogue consistency: `tools.list` includes `judge.inspect` and `tools.inspect` succeeds for it while invocation is refused (card catalogue-data §10).
- Side effect: none; no `operations` row, no S-8 egress (the only egress is K0e, reachable only by K6 after the `DataClass` check, and nothing here reaches it).
- Post-grant paths: unreachable; blocked by H-8 (a Luke grant), named as such.

## Gotchas

- Do not confuse this with the judgment flow (E2E-11): that is an internal `Advised<T>` port with no action. This action only reads what was recorded.
- The owner is `Owner::Judge`, never `Owner::App` (K0 P-8 withdrawn): if `App` served it, the held action would be live by accident (API Map P-1). A test that finds it served under any owner has found the defect.
- `judgments` are digests, not state (T-15): nothing a judgment says can move a task state (PT-12 `decision` is always the code's value).
- Must not: no network egress exists except K0e over S-8 (DEC V4-12); `judge.inspect` must not open one.
- Never send anything to Jev from this file's drivers; the egress is held and the held register is ATLAS §5.
