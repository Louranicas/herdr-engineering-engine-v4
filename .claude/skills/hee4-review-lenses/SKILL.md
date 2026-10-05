---
name: hee4-review-lenses
description: Three focused review lenses for HEE v4 Rust diffs — silent failure, type design, Rust review — trimmed to what the lints and the gate do not already catch. Each finding must name the higher rung (type, door, check) that should have caught it. Use when reviewing a slice or a diff (hee4-reviewer, hee4-refuter, interrogate), or for "review this for silent failures / type design".
---

# HEE v4 review lenses

Assimilated from ECC (everything-claude-code) 2026-10-05: `agents/silent-failure-hunter.md`,
`agents/type-design-analyzer.md`, `agents/rust-reviewer.md`. Kept: the questions a reader still has
to ask here. Dropped: everything a door already answers, and ECC's prose about it.

**Review is rung 4, the floor** (`plan/STACK-MAP-2026-10-04.md` §0). A lens finding is therefore
also a defect in a higher rung. Every finding names: `rung=1|2|3` (where it should have been caught),
the door that would catch it (a type, an admission refusal, a lint, a test, a check under `tools/`
or `ops/checks/`), and whether a real past instance exists to prove that door fails on it (`correct`).

## Already caught above rung 4: do not review these
| Class | Door |
|---|---|
| `unsafe` | `unsafe_code = "forbid"` (rung 1) |
| `unwrap`/`expect`/`panic!` outside `crates/*/tests/` | clippy deny + `tools/lint-ratchet` R3 (rung 3) |
| a lint switched off by `allow`, `cfg_attr`, a crate opt-out, rustflags | `tools/lint-ratchet` R1–R5 (rung 3) |
| pedantic style, formatting, missing docs on `pub` | clippy pedantic, `cargo fmt --check`, `missing_docs` (rung 3) |
| a gate step that looked at nothing | `gate.toml` `expect`, the drive's UNMEASURED exit (rung 3) |

## Lens 1 — silent failure (from silent-failure-hunter)
- An error turned into a default: `unwrap_or_default()`, `ok()`, `.unwrap_or(…)`, `filter_map(Result::ok)`,
  `let _ =` on a `Result` or `#[must_use]` value. Ask: which caller can now not tell failure from empty?
- A fallback that looks graceful: a missing file read as "no rows", a timeout read as "nothing to do",
  a probe that could not run reported as PASS. Here that is AP-29/AP-33 (a gate that looked at nothing):
  the answer is UNMEASURED with a named reason, never zero.
- An error re-wrapped without the cause, or mapped to a generic variant that loses which door refused.
  HEE wants one named refusal per rule (the Error map); a catch-all variant is a second door.
- A side effect with no settle: a spawn, write or socket call whose failure leaves a task non-terminal
  (bad code (e), STACK-MAP §0). Ask which recovery rule R01–R14 owns the interrupted case.

## Lens 2 — type design (from type-design-analyzer)
- Can the state the design forbids be built? Look for `pub` fields on a type with an invariant
  (V4-96 named `SpawnPlan` pub fields bypassing the permit door), constructors that skip validation,
  `Default` on a type that has no valid default, `String`/`u64` where a branded type exists
  (`Sha256Hex`, `ToolId`).
- Is the transition whitelist the only writer? A second function that sets a `TaskState`, or a match
  arm `_ =>` over a contracts enum, hides the next variant from the compiler.
- Escape hatches: `From`/`TryFrom` impls that accept unchecked input, `serde(default)` on a field
  the invariant needs, `#[non_exhaustive]` missing on an enum callers must not exhaust.
- Score each type: encapsulation, invariant expressed, invariant useful (prevents a real past bug),
  enforcement (compiler, not comment). Anything scored low proposes the rung-1 change, not a comment.

## Lens 3 — Rust review (from rust-reviewer, minus what the lints own)
- Ownership: `.clone()` added to satisfy the borrow checker; `String`/`Vec` parameters where `&str`/`&[T]` do.
- Concurrency: unbounded channels; a lock held across I/O or a spawn; nested locks without one order;
  `PoisonError` dropped.
- Boundaries (`principle-boundary-discipline`): `std::process::Command` built outside the K0h spawn door;
  SQL built by `format!` instead of parameters, or a ledger write outside `transition`/`Store::operate`;
  paths from input without canonicalising under the allowed root; deserialising untrusted input without
  a size bound.
- Errors: `Box<dyn Error>` in a library crate (use the typed refusal); `todo!()`/`unreachable!()`
  in a production path (clippy `panic` does not cover these two).
- Tests (`principle-test-behavior-not-implementation`): a test that would pass if the function
  returned the default; a plant or mutant missing for a new door.

## Output
One line per finding: `lens=1|2|3 path:line rung=<1|2|3> door=<proposed door> past=<incident id|none> — <issue>`.
Last line, in the roster's form: `review-lenses verdict=PASS|PASS_WITH_GAPS|FAIL findings=n moved_up=k`.
