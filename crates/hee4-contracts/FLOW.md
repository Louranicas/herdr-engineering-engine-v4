# hee4-contracts: flow

K0. Rung 1: each type here is built through its checked constructor (`Budgets`: see its row). Sources: State and Transition Map §2c,
STACK-MAP §2 (I1, I3, I4), `gates/features/crash-restart.md` (R01–R14).

## Types

| Type | What makes it legal | Built by |
|---|---|---|
| `TaskState` | opaque over `Phase`; the only values are those `transition` returned | `transition`, `TaskState::replay` (a fold of `transition`) |
| `Phase` | 11 states; `Blocked{cancel}` and `EffectUnknown{cancel}` hold cancellation (UM-P3); no `queued` | anyone (it is a read-only view) |
| `Event` | `Admit Dispatch Observe Settle(Settlement) Decide(Verdict) Accept Cancel Resolve(Resolution) Stop Recover(RecoveryRule)` | anyone |
| `Resolution` | `Quarantine(QuarantineReason)` or `Abandon(AbandonReason)`; the reason is payload, `transition` ignores it | anyone |
| `RecoveryRule` | `R01..R14`, the crash-restart table's names | anyone |
| `Sha256Hex` | 32 bytes; `FromStr` takes exactly 64 lowercase hex digits; `digest(bytes)`; `GENESIS` | parse or digest only |
| `GitSha` | 40 or 64 lowercase hex digits | `FromStr` only |
| `TaskId ReceiptId ObservationId SourceId ToolName ToolVersion EvidenceLabel` | 1..=128 bytes, no whitespace or control | `FromStr` only |
| `Outcome` | `#[non_exhaustive]`: `Pass`, `Fail`, `Error`, `Refused{reason: RefusalText}`; `RefusalText` is 1..=512 bytes, no control characters | serde, `FromStr` |
| `Observation` (I3) | every field is one of the parsed types above; `deny_unknown_fields` | serde at the boundary |
| `Brief` (I1) | all eleven fields present | `Brief::parse` (refuses a missing field by name, a duplicate); `check_restatement` refuses an empty RESTATEMENT; `check_verify` refuses a VERIFY that looks at nothing (`VacuousVerify{cause}`: empty, nothing runnable, only no-ops) |
| `Verdict`, `Reason`, `Decision` | plain data | K4 (policy is K4's) |
| `Receipt` (I4) | `seal(prev, ReceiptBody)` hashes decision + observed + `hash_prev` together over canonical JSON (keys sorted, no whitespace); read-only fields; `checkpoint(leaves)` is the RFC 6962 Merkle Tree Hash (`merkle_root`, leaf `0x00`, node `0x01`, built from `Sha256Hex::digest` only) over `hash_self` digests in order | `Receipt::seal`; `verify_chain` returns the first `ChainBreak{index, cause}`; `Receipt::checkpoint` (K1 stores it) |
| `Refusal` | `#[non_exhaustive]`, named variants with typed fields, never strings | this crate |
| `VerifyLine` | one VERIFY line after normalisation: Shell / Exec / Unsupported (`model:` is Unsupported{model}); the only home of the VERIFY line grammar (K6 playbook maps it) | `VerifyLine::parse_all` |
| `Budgets` | validated only through `Budgets::parse`, one streaming read of the text (never of a `serde_json::Value`, which has already collapsed a repeated key last-wins): the top value and every section an object (a positional array is refused, never read as the default), unknown key refused, a key named twice refused by name (`duplicate field`, never last-wins), every field non-zero, under its ceiling in budgets.rs, ordered. Neither `Budgets` nor a section has `Deserialize`, so `from_value`/`from_str` cannot build one past these checks (`lib.rs` `compile_fail` doctests). Fields are `pub` for reading; a literal or a field write after `parse` is not checked: rung 2, pending a DC row for private fields | `Budgets::DEFAULT`, `Budgets::parse` |
| `catalogue::{Action, Owner, Effect, Scope, CATALOGUE, find, revision}` | the 22 action ids as data: owner, effect (`mutates()`), scope (`because()`), readback, precondition rule; `revision()` is the content digest; `Judge`/`Deploy` owners are held | this crate (`CATALOGUE` const; tests compare it to `gates/features`) |

## Whitelist (`transition`)

`cr` = the source's cancel request (state `cancellation_requested`, or the field). Every pair not
listed is refused: `NotAdmitted`, `AlreadyAdmitted`, `Terminal`, `NoTaskEdge` (a Recover rule
with no task edge), or `Illegal`.

| From | Event | To |
|---|---|---|
| none | Admit | admitted |
| admitted, repair_pending | Dispatch | running |
| running | Settle ready | verifying |
| running | Settle not_ready | repair_pending |
| running, cancellation_requested | Settle unsettled; Recover R07, R08 | effect_unknown{cr} |
| verifying | Observe; Decide Pass | verifying |
| verifying | Decide Fail | repair_pending |
| verifying | Decide Refused(Invalid, Error, Timeout) | failed |
| verifying, cancellation_requested | Decide Refused(Unreconciled) | effect_unknown{cr} |
| verifying | Accept | accepted |
| cancellation_requested | Settle ready, not_ready; every other Decide | cancellation_requested |
| admitted, running, verifying, repair_pending, cancellation_requested | Cancel | cancellation_requested |
| blocked | any Settle | blocked (unchanged) |
| blocked / effect_unknown | Cancel | same variant, cancel = true |
| every non-terminal | Resolve Quarantine | blocked{cr} |
| every non-terminal | Resolve Abandon | cancelled if cr, else abandoned |
| admitted, repair_pending, verifying | Stop | failed |
| cancellation_requested | Stop | cancelled |

## Resolution reasons

`Event`, `Settlement`, `Resolution`, `RecoveryRule` derive `Deserialize` (round-trip over all 32
events: `tests/wire.rs`). Reasons are `#[non_exhaustive]` enums serialized as data, never strings.

| Resolution | Reason | Variants |
|---|---|---|
| `Abandon` | `AbandonReason` | `BriefUnreadable`, `RouteRefused{floor_unmet}`, `NamespaceRefused`, `WorkDirUnavailable`, `HeadUnknown`, `NoPermit`, `AttemptFailed` |
| `Quarantine` | `QuarantineReason` | `EffectUnknownPermanent{rule: RecoveryRule}` |

The whitelist is unchanged: every reason takes the same edge as its `Resolution`.

## `Outcome::Refused`

An adapter's exit 7 is a tier-0 observation `Refused{reason}`. `decide` MUST map it to
`Verdict::Refused(Reason::Invalid)` (never Pass, never Fail) and must not branch on the text.

Counted by `tests/transition.rs` over 14 sources × 32 events: `legal=65/65 illegal=383/383`.

## Who may construct what

| Crate | May | May not |
|---|---|---|
| K1 `hee4-core` (store, task) | call `transition`; persist `Phase::as_str`; rehydrate by `TaskState::replay` | build a `TaskState` any other way; write a state `transition` did not return |
| K4 `decide` | build `Verdict`, `Decision`, `ReceiptBody`; call `Receipt::seal` | mutate a sealed `Receipt`; attach `observed` after the seal |
| K6 host (admission, wire) | `Brief::parse` + `check_restatement` + `check_verify`; parse `Observation`, ids and digests from the wire; load `Budgets` by `parse` at serve start only; host and worker receive the validated value | pass a raw `String` where a newtype is required; admit a brief that failed any check; build a `Budgets` from a literal or write one of its fields after `parse` |

## What `check_verify` refuses beyond the exact table

`VerifyLine::is_no_op` reads a runnable line lexically (never the filesystem) and counts it as
a no-op when its exit status cannot be non-zero. Pinned as refused by
`tests/verify.rs::cannot_fail_lines_are_refused`; the refusal stays
`VacuousVerify{OnlyNoOps}` (every runnable line is a no-op), its text leads with `VERIFY`.

| VERIFY line | Why it cannot fail |
|---|---|
| `/usr/bin/env true`, `sh: "true"`, `sh: exit 0;`, `sh: true;`, `/usr/bin/../bin/true`, `//usr/bin/true`, `sh: true # comment` | a listed no-op after normalisation: `//`, `.`, `..` collapsed lexically; quotes removed; a trailing `;` or `# comment` dropped; `env <no-op>` read as the no-op |
| `/bin/sh -c true`, `sh: sh -c 'cargo test \|\| true'` | `sh -c <script>` (and `bash -c`) is read with the same rules |
| `sh: true && true`, `sh: true; :` | a compound whose every command is a listed no-op |
| `sh: cargo test \|\| true`, `sh: cargo test \|\| :`, `sh: cargo test \|\| exit 0` | a forced exit: a no-op after the last `\|\|` always ends at 0 |
| `sh: /usr/bin/false; exit 0`, `sh: cargo test; true`, `sh: exit 0; cargo test` | a forced exit: the last list is a no-op, or `exit 0` ends the shell |
| `sh: cargo test &` | a trailing `&` ends at 0 |
| `sh: cargo test \|\| true 2>/dev/null`, `sh: cargo test \|\| : >/dev/null 2>&1`, `sh: cargo test \|\| echo failed >&2`, `sh: cargo test \|\| echo failed >/dev/null`, `sh: cargo test \|\| true &>/dev/null`, `sh: cargo test \|\| true >&-`, `sh: cargo test \|\| : >&0`, `sh: cargo test \|\| echo failed 2>&-`, `sh: cargo test \|\| echo failed <&-`, `sh: cargo test \|\| echo failed 0<&0`, `sh: cargo test \|\| echo failed >&- >/dev/null`, `sh: cargo test \|\| echo failed >&2 2>&-`, `sh: cargo test \|\| echo -n >&-`, `sh: cargo test \|\| echo -n "" >&0` | a no-op whose redirections cannot fail is still a no-op. The reader applies a command's redirections in order, each to its own fd (default 1 for `>`, 0 for `<`; `&>` moves 1 and 2), starting from the fds the host gives a candidate: fd 0 from `/dev/null` read-only, fd 1 and fd 2 piped (`hee4-host/src/spawn.rs`). A redirection cannot fail when it opens `/dev/null`, closes an fd (`-`), or dups an fd that is open at that point. `true` and `:` write nothing, so that is all they need. `echo` writes to fd 1, so it also needs fd 1 to end open for writing; `echo -n` with no argument, or only empty ones, writes nothing and needs only the redirections. MEASURED for every `echo` line here by `tests/verify.rs::echo_redirections_match_the_shell` |
| `sh: cmd \| tail -1`, `sh: cmd \|& tail -1`, `sh: cmd \| head -n 5`, `sh: cmd \| true` | `\|&` is read as a pipe (bash pipes stderr too). No `pipefail`: a pipeline's status is its last command's. MEASURED: the dispatcher runs a `sh:` line as `/bin/sh -c <command>` (`hee4-app/src/dispatcher.rs:89-91`, `playbook`), no `-o pipefail`; `/bin/sh` is bash in POSIX mode on this host, `pipefail` off by default. fm-db refuses the same shape as `pipe_into_tail_head` (V4-105) |

A forced exit next to a real line is admitted like `sh: true` next to a real line: the rule is
still "every runnable line is a no-op" (`Brief::check_verify`, brief.rs). Refusing any one
cannot-fail line needs a `VerifyFault` variant raised there; not in this slice.

## What `check_verify` does not catch

`check_verify` is a rung-2 door: admission reads the text, not the effect; a real command that
can fail and proves nothing is the verdict's business and is deliberately a Pass. Pinned as `Ok`
by `tests/verify.rs::deliberately_not_caught`:

| VERIFY line | Why it admits |
|---|---|
| `sh: printf ok`, `sh: cat /dev/null` | real commands with a trivial effect |
| `sh: exit 1` | fails, which is not vacuous |
| `/usr/bin/test -d /usr` | the fixture command of the later waves: real, silent, in `RO_BINDS` |
| `sh: cargo test --workspace`, `/usr/bin/env cargo test` | real commands; `env <real>` is the real command |
| `sh: test -f out && true` | the no-op runs only on success, so the line can still fail |
| `sh: cargo test \|\| true; cargo clippy` | a masked command mid-line: the status is the last command's, which can fail |
| `sh: cargo test; exit` | a bare `exit` keeps the status before it |
| `sh: set -e; false; true`, `sh: cargo test \|\| exit 1; true` | a built-in that can end the shell (`set`, `exec`, `eval`, `exit N`, `trap`, ..) before the last command |
| `sh: (cargo test) \|\| true`, `sh: if cargo test; then :; fi`, `sh: test -n "$(cat f)" \|\| true` | text the reader does not follow (a subshell, a group, a compound keyword, `$(..)`, `${..}`, a backtick, a here-document) counts as a line that may fail: no false refusal, at the cost of these holes |
| `sh: echo a#b > f`, `sh: grep -q 'a \|\| true' f` | `#` inside a word is not a comment; a quoted operator is not an operator |
| `sh: cargo test \|\| true > out`, `sh: cargo test \|\| true <&3`, `sh: cargo test \|\| true 2>` | a redirection that can fail (a file that may not open, an fd that may be closed, no target) makes the no-op a command that can fail |
| `sh: cargo test \|\| echo failed >&-`, `sh: cargo test \|\| echo f >&0`, `sh: cargo test \|\| echo failed >/dev/null >&-`, `sh: cargo test \|\| echo failed 2>&- >&2`, `sh: cargo test \|\| true >&- 2>&1`, `sh: cargo test \|\| echo failed 1</dev/null`, `sh: cargo test \|\| echo "2">&-` | the redirections, in order, leave fd 1 closed or read-only before `echo` writes to it (a dup onto fd 0 copies the read-only `/dev/null` the host gives a candidate as stdin, `hee4-host/src/spawn.rs`), or a dup copies an fd closed before it (`true >&- 2>&1`). Quoted digits are an argument, not an fd: `echo "2">&-` closes fd 1. MEASURED by `tests/verify.rs::echo_redirections_match_the_shell`: each exits non-zero under `/bin/sh` with stdin from `/dev/null` and stdout piped. Not followed, each counted as can-fail, so admitted: a dup of an fd above 9, and an `echo` that prints nothing for a reason other than `-n` (a `\c` under a shell whose `echo` reads escapes; `/bin/sh` is bash here and does not) |

Vacuity stays at rung 2 because the brief's VERIFY is free text the worker wrote; a type cannot
refuse it before it is parsed, and parsing it is this check.
