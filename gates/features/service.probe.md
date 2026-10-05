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

`hee4 service.probe --key K --body JSON` (binary, the generic client); Pi `hee4_service_probe` (PROPOSAL). At v4.0, `unavailable`. From v4.2 an operator probes to refresh `cached_health`.

## Driving it with hee4

Preconditions (v4.2): a `BoundedProbe` grant; a seeded `service_id` (`self`, `model`, `drive`); `/usr/bin/busctl` at the pinned digest (`HEE4_BUSCTL_SHA256` when set, else the digest `serve` measured at start; the serve log prints `busctl_sha256=<hex> source=env|measured`); `$XDG_RUNTIME_DIR/bus` present; a fresh key.

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive --only service.probe`):

```bash
K=$(uuidgen)
hee4 service.probe --key "$K" --body '{"service_id":"drive","probe_id":"active_state","probe_version":1,"max_cost_microunits":0,"network_scope":"none"}'
hee4 service.inspect --body '{"service_id":"drive","operation":{"source_action":"service.probe","idempotency_key":"'"$K"'"}}'   # readback
```

Socket: request `body` `{service_id, probe_id, probe_version, max_cost_microunits, network_scope}` + `idempotency_key`; result `body` `{operation_id, service_id, observation, cost_microunits: 0, external_effect: "none"}`. The probe catalogue is `probe_id` ∈ `active_state` (`Unit.ActiveState`), `sub_state` (`Unit.SubState`), `main_pid` (`Service.MainPID`); `probe_version` is 1. The observation is `hee4_contracts::Observation` with `source` `service.probe`, `input_sha256` = sha256 of the request body bytes, `tool` `{busctl, <first line of busctl --version as one token>}`, `outcome` `pass` (exit 0 and the property parsed) / `fail` / `error` (spawn io), one evidence `{label: <probe_id>, sha256 of stdout}`, `advisory: false`. The child is `busctl --user get-property org.freedesktop.systemd1 /org/freedesktop/systemd1/unit/<escaped unit> <iface> <prop>` through `hee4_host::spawn::plan/run` with the user bus as the one listed rw socket, no network, a 5 s deadline and a 4096 B stdout bound (`service_runner.rs` `PROBE_TIMEOUT`, `PROBE_STDOUT_MAX`).

- Success: `service_facts.cached_health_json` gains the observation; `operations` gains one row (subject = the service id); `service.inspect` shows `cached_health` equal to the reply's `observation`.
- Replay: the same key and body returns the stored body with `replayed: true` and runs no child.
- Error: `network_scope` ≠ `none` → `forbidden` at `/body/network_scope` (resolved: forbidden, not unavailable; the peer-credential `forbidden` is at `/`, so the field tells them apart); `probe_version` ≠ 1 → `unsupported_action_version` at `/body/probe_version`; unknown `probe_id` → `invalid_argument` at `/body/probe_id`; `max_cost_microunits` ≠ 0 → `invalid_argument` naming the bound 0; unknown service → `not_found`; stdout over the bound → `resource_exhausted` naming both numbers; a busctl digest mismatch → `unavailable` with `because` `busctl digest`; a build without a git head → `unavailable` because `head unknown`; no user bus → `unavailable` because `user bus absent`.
- Cancel: a probe is a bounded child; its deadline is the cancel.
- Side effects to read: `service_facts`, `operations`.

## Gotchas

- A `busctl` call is a spawned, digest-pinned child; a successful call is not a changed state (AP-49). The observation is what the property **read back**, and a read-back that differs from a request is a refusal.
- The probe's identity read-back for the model daemon is the LISTEN holder of :11434, never a unit/MainPID walk (the daemon is not a descendant of its unit's MainPID; Socket map S-3).
- S-7 and the service-side read-backs are UNMEASURED before the tag (Socket map S-7 row); nothing in v4.0 evidence covers this file.
- Must not: K5 never writes the store; only K6 commits via K1 (NF-SVC-OBS replaces AR row 6).
