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
| tier-0, bound, in budget | outcome `refused{reason}` | `Refused(invalid)`; the reason is in the observation, whose content address is sealed in `observed` (the ddf adapter's exit 7 and its timeout are advisory, so they take the advisory row) |

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
  `tool_absent` when the binary is not found on `PATH` (a bare name) or at the given path; a
  present file that fails to exec (a missing `#!` interpreter or ELF loader, which `execve`
  also reports as `NotFound`; permissions, `PermissionDenied`) stays `AdapterError::Spawn`.
  The adapter checks the disk (`execvp`'s rule: a name with a `/` as given, else each `PATH`
  entry) before it calls a `NotFound` absent; `tests/fixtures/ddf-badshebang.sh` pins it;
- otherwise runs `deep-diff-forge --stdin-patch --rank --json --require-files --require-hunks`
  as a local process (no network, no shell);
- exit 7 → `Observed` with `Outcome::Refused{reason}` (first stderr line; a fixed sentence when
  silent), `advisory: true`: the tool declined to rank, which is not a defect of the candidate.
  Git emits diffs with files but no hunks for a mode-only change, a rename-only change and a
  binary add, and all three exit 7 `refused: 0 hunks in input (--require-hunks)` (MEASURED,
  deep-diff-forge 0.2.1; `tests/ddf.rs::exit_7_on_hunkless_diffs_is_advisory_and_a_tier0_pass_still_passes`),
  so a tier-0 refusal would fail a candidate that only renamed a file or added an image. ddf
  adds evidence, it does not gate: `decide` ignores the advisory row and the task's verdict
  comes from its tier-0 observations, while the reason is still sealed in `observed`. The
  dispatcher records it like any other observation, no branch. Any other non-zero exit →
  `AdapterError::Exit` (no observation); neither is a Pass;
- stdout must be `deep-diff-forge.rank.v0`; its `input_sha256` must equal
  `Sha256Hex::digest(diff)` or `AdapterError::SealMismatch`; an empty `ranked` is
  `AdapterError::LookedAtNothing`; `Exit`, `Malformed`, `SealMismatch` and `LookedAtNothing`
  come back as `Err`, refused by name, never as a skip;
- a run past `budget` is mapped inside to `timeout_observation` (`Observed`, outcome `error`,
  `elapsed_ms > budget_ms`, `advisory: true`): like exit 7 it adds evidence and does not gate,
  so `decide` ignores it and the task's verdict comes from its tier-0 observations;
- the Pass observation is bound to the subject's `input_sha256` (the VERIFY digest, V4-94) and
  carries two evidence items: `rank.v0` = digest of stdout, `diff` = digest of the bytes sent,
  which the adapter has checked equal the tool's sealed `input_sha256` (`SealMismatch`
  otherwise); `tool` comes from the JSON, `head_sha` from the subject, `advisory: false`,
  `budget_ms` = the `budget` argument.

`ddf::observe(diff, &subject, &clock, budget)` is the bytes-only call under it (no skips: an
empty diff reaches the tool and comes back exit 7, `Observed(Refused)`, advisory); `for_task_with` and
`observe_with` take an explicit binary for the stubs under `tests/fixtures/`.

## The deadline

The caller passes `budget: Duration`. The child is spawned in its own process group
(`CommandExt::process_group(0)`, so its pgid is its pid). Stdin is written on a detached thread
that is never joined; stdout and stderr are drained on detached threads that report over a
channel. Every 10 ms the adapter asks, without reaping (`waitid` with `WNOWAIT`, through `rustix`),
whether the child has exited, against a wall-clock `Instant`:

- At the deadline it sends `SIGKILL` to the whole group (`killpg`; the direct child alone if the
  group cannot be signalled), then `wait()`s the child and returns `AdapterError::Timeout{budget}`.
- On a normal exit the child is still a zombie, so its pid (the pgid) cannot be reused: the
  adapter sends `SIGKILL` to the group (whatever the tool left behind, such as a grandchild holding
  the pipes; `ESRCH` means nothing was left), then reaps the child. It then waits for the two
  drains only until the deadline; a pipe still held at the deadline is `AdapterError::Timeout`.

`ddf::timeout_observation(budget, diff, &subject)` turns a timeout into an advisory observation
(outcome `error`, `elapsed_ms = budget_ms + 1`, one `deadline` evidence item, the digest of the
diff). deep-diff-forge is never a second verdict authority (V4-81), so a hung tool does not gate:
beside a tier-0 Pass the task passes, alone the verdict is the floor `Refused(invalid)`, never
Pass, and the timeout is sealed in `observed`. Made tier-0, the same row would still be
`Refused(timeout)`: `decide` is unchanged.

Tests (`tests/ddf.rs`, stub `fixtures/ddf-grandchild.sh` in three modes): a `sleep 5` stub with a
300 ms budget returns well under 1 s; `a_grandchild_is_gone_after_the_timeout` finds the
`sleep 30` grandchild gone (`kill(pid, 0)` is `ESRCH`) after the deadline;
`a_grandchild_holding_the_pipes_after_a_normal_exit_neither_holds_the_call_nor_survives` (exit 0
at once, grandchild on the pipes, 20 s budget) returns in about 10 ms with the grandchild gone;
`an_escaped_grandchild_holding_the_pipes_cannot_hold_the_call_past_the_deadline` (the grandchild
called `setsid`) is a `Timeout` at the 700 ms budget. That escaped grandchild is not killed: it
left the group, and the adapter kills only by group. Its drain and stdin threads stay parked until
it closes the pipes.

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
- A Pass whose only tier-0 observations are `command` rows with an empty-stdout digest is a
  Pass. `decide` branches on no tool name or evidence label (they are K2's vocabulary); silence
  is the success output of `test`, `grep -q`, `cmp -s`. The vacuity door is at admission (K0
  `Brief::check_verify`, app `task.submit`/`task.preview`); a real command that proves nothing
  is the named gap. `tests/lattice.rs::silent_command_pass_stays_pass` pins it.
- `ddf::for_task`'s caller is `hee4-app` `settle_and_decide` (via `ddf_observation`); outside a task,
  today the binary runs only as the gate's `ddf` step (gate.toml `[step.ddf]`).
- Settled for W4: an exit-7 `Observed(Refused{reason})` is advisory at its construction site
  (`ddf::refused_observation`), so the dispatcher records it as it records any observation and
  `decide` cannot fail the task on it; `decide.rs` is unchanged. Settled for U-harden-05: the
  timeout observation is advisory too (`ddf::timeout_observation`, from V4-81: deep-diff-forge is
  never a second verdict authority), so a hung deep-diff-forge does not gate. The DECISIONS row
  is a hee4-scribe proposal for Luke to append; `decide.rs` is unchanged.
- Open (U-harden-05 AC9, third clause): the dispatcher's line for a timeout is `ddf=observed`
  like any observation (`hee4-app` `ddf_observation`), so the journal cannot tell a hung tool from
  a ranking. The fix is the app's: print `ddf=timeout budget_ms=<n>`, or give `TaskObservation`
  a typed timeout variant so the app cannot mislabel it. It belongs to a slice that owns
  `hee4-app` (h5-app-runtime-settle merges after this one).
