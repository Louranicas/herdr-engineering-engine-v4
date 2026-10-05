# service.inspect

A-15. One managed habitat service: its owner, its systemd unit, the cached health from the last probe, and optionally one operation by id or by key. Scope **v4.2**, phase P9, owner `Owner::Service`, effect `Read`. Readback action for `service.probe` and `service.action`.

## Sub-features

- service-facts: `{service_id, owner_id, unit_id, cached_health, operation}` from `service_facts` (T-14) and `operations`.
- cached-health: the last committed `Observation` (K5 probe → K6 commits via K1, NF-SVC-OBS); rebuildable after restart.
- operation-readback: `operation{source_action, idempotency_key}` selects the probe or action whose reply was lost.
- v4.0-refusal: `unavailable` by name.

## How to get to it (user POV)

`hee4 service.inspect --body JSON` (binary, the generic client); Pi `hee4_service_inspect` (PROPOSAL). At v4.0 and v4.1, `unavailable` with `because` = v4.2. From v4.2 an operator reaches it after a probe or an action, or to read cached health.

## Driving it with hee4

Preconditions (v4.2): a `Read` grant; a `service_id`. Service ids are the rows `serve` seeds into `service_facts` at start (the service family's `on_serve_start`, `actions/service.rs` `SEEDS`): `self` (`hee4.service`), `model` (`ollama.service`), `drive` (`hee4-drive.service`), each `owner_id` `deploy`. There is no list action for services in the 22.

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive --only service.inspect`):

```bash
hee4 service.inspect --body '{"service_id":"drive","operation":null}'
hee4 service.inspect --body '{"service_id":"drive","operation":{"source_action":"service.probe","idempotency_key":"<key>"}}'
```

Socket: request `body` `{service_id, operation: null | {source_action, idempotency_key}}` (both members required; a missing `operation` is `invalid_argument` at `/body/operation`); result `body` `{service_id, owner_id, unit_id, owner_sha256, generation, cached_health, operation}`. `generation` and `owner_sha256` are additive to API Map A-15: they are what `service.action` names as its precondition and `expected_owner_sha256`. `cached_health` is `null` or the committed `Observation` (`hee4_contracts::Observation`: `{source, input_sha256, tool{name, version}, head_sha, outcome, evidence[{label, sha256}], advisory, elapsed_ms, budget_ms}`). `operation` is `null` or `{operation_id, action, idempotency_key, ts, result}`, selected through `Store::operation_by_key` with the engine principal; a row whose subject is another service is `not_found` at `/body/operation`.

- Seeded: `drive` reads `cached_health: null`, `generation: 1`, `owner_sha256 = sha256("deploy")`.
- After `service.probe`: `cached_health` equals the probe's `observation`; `operation` by key returns its `operation_id`.
- After `service.action`: `operation` by key returns that action's `operation_id` and stored `observed_state`; `generation` moved by one.
- Error: unknown `service_id` → `not_found` at `/body/service_id`.
- Persistence: `cached_health` survives restart; it is a read of committed facts, not a live probe.
- Side effect: none.

## Gotchas

- Inspect never probes. A stale `cached_health` is correct behaviour; run `service.probe` to refresh.
- `service_facts` is written only by the store via K6 from K5 (T-14); the v3 direct service→store write (AR row 6) is gone. A fact that appears without an operations row came in through a second door.
- No D-row (post-tag); the flow moves `l2` in P9 (E2E-10). S-7/S-4 read-backs for services are UNMEASURED before the tag (Socket map S-7).
