# hee4-evidence: flow

K4. `decide` is the one verdict authority. Nothing else produces a verdict.

```
Identities{collector, locks, standards} --+
Observation[] (I3, from the ledger) ------+--> decide --> Decision{verdict}
Subject{task_id, head_sha, input_sha256} -+          \
                                                      +--> decide_and_seal(prev, id, ..)
                                                             = Receipt::seal(prev, ReceiptBody{
                                                                 decision: decide(..),
                                                                 observed: sorted, deduplicated
                                                                           observation_id(o) })
```

## The lattice

Severity is totally ordered: `Pass < Fail < Refused(invalid) < Refused(timeout) <
Refused(error) < Refused(cancelled) < Refused(unreconciled)`. The verdict is the worst
contribution, folded from a floor.

| Step | Condition | Contribution |
|---|---|---|
| 0 (before the lattice) | any identity `Unavailable(_)` | verdict is `Refused(error)` (AP-22) |
| floor | no tier-0 (`advisory: false`) observation | `Refused(invalid)`: a gate that looked at nothing |
| floor | at least one tier-0 observation | `Pass` |
| any observation | `head_sha` ≠ subject's, or `input_sha256` ≠ subject's | `Refused(unreconciled)` |
| advisory, bound | anything | nothing (it can never raise to Pass, nor lower) |
| tier-0, bound | `evidence` empty | `Refused(invalid)` |
| tier-0, bound | `elapsed_ms > budget_ms` | `Refused(timeout)` |
| tier-0, bound | outcome `error` / `fail` / `pass` | `Refused(error)` / `Fail` / `Pass` |
| tier-0, bound, in budget | outcome `refused{reason}` (exit 7) | `Refused(invalid)`; the reason is in the observation, whose content address is sealed in `observed` |

`decide` never emits `Refused(cancelled)`; it is ranked only so the order is total.
A missing `input_sha256` or `head_sha` cannot reach `decide`: the I3 type requires both (rung 1).

Tests: `tests/lattice.rs` (24 single rows, 24 paired rows, 13824 ordered triples checked for
order-independence, advisory-only-refuses, Pass-needs-tier-0, monotonicity),
`tests/seal.rs` (order gives the same `hash_self`; a changed set changes it).

## What K6's adapter may hand in

Only an `Observation` (parsed I3, `deny_unknown_fields`) whose `input_sha256` the adapter
checked against the bytes it sent. `ddf::observe(diff, &subject, &clock, budget)` is the template:

- runs `deep-diff-forge --stdin-patch --rank --json --require-files --require-hunks` as a local
  process (no network, no shell);
- exit 7 → a tier-0 observation with `Outcome::Refused{reason}` (first stderr line); any other
  non-zero exit → `AdapterError::Exit` (no observation); neither can be a Pass;
- stdout must be `deep-diff-forge.rank.v0`; its `input_sha256` must equal
  `Sha256Hex::digest(diff)` or `AdapterError::SealMismatch`; an empty `ranked` is
  `AdapterError::LookedAtNothing`;
- fills `tool` from the JSON, `head_sha` from the subject, `advisory: false`, outcome `pass`,
  one evidence item `rank.v0` = digest of stdout, `budget_ms` = the `budget` argument.

## The deadline

The caller passes `budget: Duration`. The child is spawned; stdin is written and stdout/stderr
drained on detached threads; the adapter polls `try_wait` every 10 ms against a wall-clock
`Instant`. At the deadline it calls `kill()` then `wait()` and returns
`AdapterError::Timeout{budget}`. `ddf::timeout_observation(budget, diff, &subject)` turns that
into a tier-0 observation (outcome `error`, `elapsed_ms = budget_ms + 1`, one `deadline`
evidence item, the digest of the diff). `decide` tests `elapsed_ms > budget_ms` before the outcome,
so it yields `Refused(timeout)`, never Pass. Test: a `sleep 5` stub with a 300 ms budget returns
well under 1 s (`tests/ddf.rs`). Detached threads mean a grandchild holding a pipe cannot hold the
adapter past the deadline; the grandchild itself is not killed (UNMEASURED beyond the stub).

## The seal door (census)

`decide_and_seal` is the only sealing path in this crate. `ReceiptBody`'s fields are public in
`hee4-contracts`, so a type cannot stop another crate sealing a Pass over invented `observed` ids;
`tests/one_sealer.rs` scans `crates/*/src/**` and fails on `Receipt::seal(` or `ReceiptBody {`
outside `hee4-evidence/src/decide.rs` and `hee4-contracts/src/receipt.rs`. It scans `src/` only:
test files (e.g. `hee4-core/tests/store.rs`) are not covered. Proposed type fix: DC to K0.

An adapter may not hand in a verdict, a `Decision`, or an observation it built from a caption
or exit code alone (AP-29).

## Gaps

- `ObservationId` is a content address (`obs-` + SHA-256 of canonical JSON) because I3 carries
  no id; if the ledger (K1) assigns ids, `decide_and_seal` must take them instead.
- Exit 7 yields no observation, because I3 `Outcome` has no `refused` variant.
