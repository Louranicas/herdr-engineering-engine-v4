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

Only an `Observation` (parsed I3, `deny_unknown_fields`) whose sealed bytes the adapter checked
against the bytes it sent, or a named skip. The task shape is one call,
`ddf::for_task(diff, &subject, &clock, budget) -> Result<TaskObservation, AdapterError>`, with
`diff: Diff::NoWorktree | Diff::Bytes(&[u8])` (the caller computes the diff; this crate never runs
git) and `TaskObservation::Observed(Observation) | Skipped(Skip)`:

- three skips, by wire word, none of which spawns or is a refusal: `no_worktree` when the task
  has no worktree (`Diff::NoWorktree`); `no_diff` when the bytes are empty (an empty diff must
  never reach `--require-files`, whose exit 7 would fail a task that merely changed nothing);
  `tool_absent` when the binary is not found at spawn (`io::ErrorKind::NotFound` only: a
  present-but-broken tool stays `AdapterError::Spawn`);
- otherwise runs `deep-diff-forge --stdin-patch --rank --json --require-files --require-hunks`
  as a local process (no network, no shell);
- exit 7 → `Observed` with `Outcome::Refused{reason}` (first stderr line; a fixed sentence when
  silent); any other non-zero exit → `AdapterError::Exit` (no observation); neither is a Pass;
- stdout must be `deep-diff-forge.rank.v0`; its `input_sha256` must equal
  `Sha256Hex::digest(diff)` or `AdapterError::SealMismatch`; an empty `ranked` is
  `AdapterError::LookedAtNothing`; `Exit`, `Malformed`, `SealMismatch` and `LookedAtNothing`
  come back as `Err`, refused by name, never as a skip;
- a run past `budget` is mapped inside to `timeout_observation` (`Observed`, outcome `error`,
  `elapsed_ms > budget_ms`, so `decide` yields `Refused(timeout)`);
- the Pass observation is bound to the subject's `input_sha256` (the VERIFY digest, V4-94) and
  carries two evidence items: `rank.v0` = digest of stdout, `diff` = digest of the bytes sent,
  which the adapter has checked equal the tool's sealed `input_sha256` (`SealMismatch`
  otherwise); `tool` comes from the JSON, `head_sha` from the subject, `advisory: false`,
  `budget_ms` = the `budget` argument.

`ddf::observe(diff, &subject, &clock, budget)` is the bytes-only call under it (no skips: an
empty diff reaches the tool and comes back exit 7, `Observed(Refused)`); `for_task_with` and
`observe_with` take an explicit binary for the stubs under `tests/fixtures/`.

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
- `ddf::for_task` has no caller until the dispatcher's settle (W4 `dispatcher-backups-ddf`);
  today the binary runs only as the gate's `ddf` step (gate.toml `[step.ddf]`).
- Whether W4 records an exit-7 `Observed(Refused{reason})` for a non-empty diff (and so fails
  the task on `Refused(invalid)`) or treats it as advisory is W4's FLOW row, not this crate's.
