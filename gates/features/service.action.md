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

`hee4 service.action --key K --precondition JSON --body JSON` (binary, the generic client); Pi `hee4_service_action` (PROPOSAL). At v4.0, `unavailable`. From v4.2 an operator reaches it to start or stop a managed unit through the engine instead of `systemctl`.

## Driving it with hee4

Preconditions (v4.2): a `ManagedLifecycle` grant; `service_id`, `unit_id`, the current `generation` and `owner_sha256` from `service.inspect`; a fresh key. The drive acts only on `drive` (`hee4-drive.service`, a transient `systemd-run --user --unit hee4-drive.service --collect /usr/bin/sleep 300`); it refuses, client-side, to send an action for `self` or `model`.

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive --only service.action`):

```bash
K=$(uuidgen)
hee4 service.action --key "$K" --precondition '{"resource":"service","id":"drive","generation":<g>}' --body '{"service_id":"drive","unit_id":"hee4-drive.service","action":"stop","expected_owner_sha256":"<hex>"}'
systemctl --user show -p ActiveState hee4-drive.service                                                  # side effect
hee4 service.inspect --body '{"service_id":"drive","operation":{"source_action":"service.action","idempotency_key":"'"$K"'"}}'   # readback
```

Socket: request `body` `{service_id, unit_id, action ∈ start|stop|restart, expected_owner_sha256}` + `idempotency_key` + `precondition{resource:"service", id: service_id, generation}`; result `body` `{operation_id, service_id, owner_job_id, observed_state, useful_health}`. `owner_job_id` is the manager's job path (`/org/freedesktop/systemd1/job/N`) from `busctl --user call … StartUnit|StopUnit|RestartUnit ss <unit> replace`; `observed_state` is the `ActiveState` read back (`active` for start/restart, `inactive` for stop) within the 5 s budget; `useful_health` is that read-back as an `Observation` (the shape in `service.probe.md`).

- Order of checks: body shape → unknown service `not_found` → `unit_id` ≠ the service's unit `invalid_argument` at `/body/unit_id` → precondition resource ≠ `service` or id ≠ `service_id` `invalid_argument` at `/precondition` → generation ≠ current `stale_generation` at `/precondition/generation` with `current_generation` → `expected_owner_sha256` ≠ the row's `conflict` at `/body/expected_owner_sha256` → the act. No precondition at all is dispatch's `invalid_argument` at `/precondition`.
- Success: `observed_state` equals `systemctl --user show -p ActiveState`; `service_facts.generation` moves by one; `operations` gains one row.
- Error: `effect_unknown` with `retry=after_readback`, `readback` `service.inspect`, `effect: "unknown"` when the call returned but the read-back did not settle; it writes **no** operations row and does not move the generation, so a retry with the same key re-executes the act; busctl digest mismatch → `unavailable` because `busctl digest`.
- Persistence: replay returns the stored result with `replayed: true` and runs no child; the commit re-checks generation and owner digest inside the transaction.
- Cancel: none; the act is bounded by its child deadline.
- Side effects to read: the unit's state on the host, `service_facts`, `operations`.

## Gotchas

- Transient units land **outside** `hee4.service`'s cgroup, under the user manager (ATLAS §3.1), so stopping `serve` does not reap them. A unit left running after a `serve` stop is expected; count it, do not call it a leak.
- `effect_unknown` on the wire reads `effect: unknown` in the envelope; it is the only code that does (Error map §0). It is not a failure; it is an obligation to read back.
- `expected_owner_sha256` is a CAS guard, not authentication; the grant is.
- Must not: every S-4 and `systemd-run` call is in K0h cgroup-io with K2/K5 supplying the plan (V4-60, DC-38); a `busctl` spawned anywhere else is a second door.
- An `effect_unknown` reply has no operations row by design (nothing is committed without a read-back), so `service.inspect` by its key is `not_found`; read the unit with `service.probe` and retry with the same key.
