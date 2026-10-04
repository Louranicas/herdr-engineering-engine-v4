# service.action

A-17. A managed lifecycle act on a service's unit (start, stop, …) over S-4 D-Bus through K0h `cgroup-io`, guarded by the expected owner digest and a precondition, with the observed state and useful health read back. Scope **v4.2**, phase P9, owner `Owner::Service`, effect `ManagedLifecycle` (**mutating**, external effect). No D-row.

## Sub-features

- managed-lifecycle: `action` on `unit_id` for `service_id`, executed by K0h `cgroup-io` (`busctl` `StartTransientUnit`/`StopUnit`, V4-60) with K5 policy.
- owner-guard: `expected_owner_sha256` must match the current owner digest, else `conflict`.
- generation-precondition: required on the service.
- effect-unknown: when the D-Bus call's outcome cannot be confirmed by read-back, the reply is `effect_unknown` with the concrete read that settles it (Error map class 11).
- read-back-reply: `{operation_id, service_id, owner_job_id, observed_state, useful_health}`.
- idempotent-replay: readback `service.inspect`.
- v4.0-refusal: `unavailable` by name.

## How to get to it (user POV)

`hee4 service.action` (binary); wrapper "generated"; Pi `hee4_service_action` (PROPOSAL). At v4.0, `unavailable`. From v4.2 an operator reaches it to start or stop a managed unit through the engine instead of `systemctl`.

## Driving it with hee4

Preconditions (v4.2): a `ManagedLifecycle` grant; `service_id`, `unit_id`, current generation and owner digest from `service.inspect`; a fresh UUID.

```bash
K=$(uuidgen)
hee4 service.action                                                                                                      # v4.0: unavailable by name
hee4-sh service.action service_id=<id> unit_id=<unit> action=<act> expected_owner_sha256=<hex> @idempotency_key=$K '@precondition:={"resource":"service","id":"<id>","generation":"<g>"}'   # v4.2
systemctl --user show -p ActiveState,MainPID <unit>                                                                      # side effect
hee4-sh service.inspect service_id=<id> 'operation:={"source_action":"service.action","idempotency_key":"'$K'"}'       # readback
```

Socket: request `body` `{service_id, unit_id, action, expected_owner_sha256}` with `precondition{resource:"service", …}`; result `body` `{operation_id, service_id, owner_job_id, observed_state, useful_health}` (API Map A-17). `UNWRITTEN: the action value domain, observed_state and useful_health shapes, and the generated wrapper spelling.`

- v4.0 path: the `unavailable` refusal.
- v4.2 success: `observed_state` equals `systemctl --user show -p ActiveState`; `service_facts` and `operations` gain rows; the D-Bus property read-back equals the request (cgroup-io AP-49 rule).
- Error: `stale_generation`; `conflict` on an owner digest mismatch; `effect_unknown` with `retry=after_readback` and the settling read when the call's effect is uncertain; `busctl` digest mismatch → `unavailable` by name.
- Persistence: replay returns the stored result; after `kill -KILL` of `serve` mid-call the operation is either recorded with its effect or absent, never recorded as success without the read-back.
- Cancel: none; the act is bounded by its child deadline.
- Side effects to read: the unit's state on the host, `service_facts`, `operations`.

## Gotchas

- Transient units land **outside** `hee4.service`'s cgroup, under the user manager (ATLAS §3.1), so stopping `serve` does not reap them. A unit left running after a `serve` stop is expected; count it, do not call it a leak.
- `effect_unknown` on the wire reads `effect: unknown` in the envelope; it is the only code that does (Error map §0). It is not a failure; it is an obligation to read back.
- `expected_owner_sha256` is a CAS guard, not authentication; the grant is.
- Must not: every S-4 and `systemd-run` call is in K0h cgroup-io with K2/K5 supplying the plan (V4-60, DC-38); a `busctl` spawned anywhere else is a second door.
