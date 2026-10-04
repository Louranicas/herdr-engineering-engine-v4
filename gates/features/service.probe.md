# service.probe

A-16. Run one bounded probe against a managed service (a `busctl get-property` read over S-4, output bounded over S-11), validate it into an `Observation`, and commit it to `service_facts` as an operation. Scope **v4.2**, phase P9, owner `Owner::Service`, effect `BoundedProbe` (**mutating**). Trace E2E-10 (NF-SVC-OBS); loop RL-11. No D-row.

## Sub-features

- bounded-probe: `probe_id`/`probe_version` with `max_cost_microunits`; a spawned child under the K0h spawn door with a deadline.
- observation-commit: K5 returns a validated `Observation`; K6 commits it via K1 (`service_facts`, `operations`).
- external-effect-report: reply `{operation_id, service_id, observation, cost_microunits, external_effect}`.
- network-scope: under RC01 `network_scope` must be `none`; anything else is refused (CD RC03 §4).
- idempotent-replay: readback `service.inspect {service_id, operation:{source_action, idempotency_key}}`.
- v4.0-refusal: `unavailable` by name.

## How to get to it (user POV)

`hee4 service.probe` (binary); wrapper "generated"; Pi `hee4_service_probe` (PROPOSAL). At v4.0, `unavailable`. From v4.2 an operator probes to refresh `cached_health`.

## Driving it with hee4

Preconditions (v4.2): a `BoundedProbe` grant; a `service_id`; `/usr/bin/busctl` at the pinned digest (host and toolbox digests differ, ATLAS §3.1); a fresh UUID.

```bash
K=$(uuidgen)
hee4 service.probe                                                                                                           # v4.0: unavailable by name
hee4-sh service.probe service_id=<id> probe_id=<p> probe_version:=1 max_cost_microunits:=0 network_scope=none @idempotency_key=$K   # v4.2
hee4-sh service.inspect service_id=<id> 'operation:={"source_action":"service.probe","idempotency_key":"'$K'"}'           # readback
```

Socket: request `body` `{service_id, probe_id, probe_version, max_cost_microunits, network_scope}`; result `body` `{operation_id, service_id, observation, cost_microunits, external_effect}` (API Map A-16). `UNWRITTEN: the probe_id catalogue, the Observation and external_effect shapes, and the generated wrapper spelling.`

- v4.0 path: the `unavailable` refusal.
- v4.2 success: `service_facts` gains the observation; `operations` gains one row; `service.inspect` shows `cached_health` equal to the reply's `observation`.
- Error: `network_scope` ≠ `none` under RC01 → refused (`UNWRITTEN: as unavailable or forbidden; CD names both for RC01 profile refusals`); a child output over the S-11 bound → refusal by name with both numbers (UM-P8); a busctl digest mismatch → `unavailable` by name (Error map class 11).
- Persistence: replay returns the stored observation without re-probing; the committed fact survives restart.
- Cancel: a probe is a bounded child; its deadline is the cancel. There is no cancel action for it.
- Side effects to read: `service_facts`, `operations`, and the child's settled `ProcessReport` (no child outliving its report).

## Gotchas

- A `busctl` call is a spawned, digest-pinned child; a successful call is not a changed state (AP-49). The observation is what the property **read back**, and a read-back that differs from a request is a refusal.
- The probe's identity read-back for the model daemon is the LISTEN holder of :11434, never a unit/MainPID walk (the daemon is not a descendant of its unit's MainPID; Socket map S-3).
- S-7 and the service-side read-backs are UNMEASURED before the tag (Socket map S-7 row); nothing in v4.0 evidence covers this file.
- Must not: K5 never writes the store; only K6 commits via K1 (NF-SVC-OBS replaces AR row 6).
