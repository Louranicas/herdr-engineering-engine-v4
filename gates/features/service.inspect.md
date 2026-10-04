# service.inspect

A-15. One managed habitat service: its owner, its systemd unit, the cached health from the last probe, and optionally one operation by id or by key. Scope **v4.2**, phase P9, owner `Owner::Service`, effect `Read`. Readback action for `service.probe` and `service.action`.

## Sub-features

- service-facts: `{service_id, owner_id, unit_id, cached_health, operation}` from `service_facts` (T-14) and `operations`.
- cached-health: the last committed `Observation` (K5 probe → K6 commits via K1, NF-SVC-OBS); rebuildable after restart.
- operation-readback: `operation{source_action, idempotency_key}` selects the probe or action whose reply was lost.
- v4.0-refusal: `unavailable` by name.

## How to get to it (user POV)

`hee4 service.inspect` (binary); wrapper "generated"; Pi `hee4_service_inspect` (PROPOSAL). At v4.0 and v4.1, `unavailable` with `because` = v4.2. From v4.2 an operator reaches it after a probe or an action, or to read cached health.

## Driving it with hee4

Preconditions (v4.2): a `Read` grant; a `service_id` (`UNWRITTEN: where service ids come from; no list action exists for services in the 22`).

```bash
hee4 service.inspect                                                                                   # v4.0: unavailable by name
hee4-sh service.inspect service_id=<id> operation:=null                                                # v4.2
hee4-sh service.inspect service_id=<id> 'operation:={"source_action":"service.probe","idempotency_key":"<uuid>"}'
```

Socket: request `body` `{service_id, operation}` (FACT required both); result `body` `{service_id, owner_id, unit_id, cached_health, operation}` (API Map A-15). `UNWRITTEN: the cached_health / Observation shape on the wire and the generated wrapper spelling.`

- v4.0 path: the `unavailable` refusal.
- v4.2 success: after `service.probe`, `cached_health` equals the probe's `observation`; after `service.action`, `operation` by key returns that action's `operation_id` and `observed_state`.
- Error: unknown `service_id` → `not_found`.
- Empty: `operation:=null` returns the facts without an operation.
- Persistence: `cached_health` survives restart (T-14 "health rebuildable after restart"); it is a read of committed facts, not a live probe.
- Side effect: none.

## Gotchas

- Inspect never probes. A stale `cached_health` is correct behaviour; run `service.probe` to refresh.
- `service_facts` is written only by the store via K6 from K5 (T-14); the v3 direct service→store write (AR row 6) is gone. A fact that appears without an operations row came in through a second door.
- No D-row (post-tag); the flow moves `l2` in P9 (E2E-10). S-7/S-4 read-backs for services are UNMEASURED before the tag (Socket map S-7).
