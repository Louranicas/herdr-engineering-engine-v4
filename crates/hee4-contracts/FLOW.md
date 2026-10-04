# hee4-contracts: flow

K0. Rung 1: a value of each type here is already legal. Sources: State and Transition Map §2c,
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
| `Brief` (I1) | all eleven fields present | `Brief::parse` (refuses a missing field by name, a duplicate); `check_restatement` refuses an empty RESTATEMENT |
| `Verdict`, `Reason`, `Decision` | plain data | K4 (policy is K4's) |
| `Receipt` (I4) | `seal(prev, ReceiptBody)` hashes decision + observed + `hash_prev` together over canonical JSON (keys sorted, no whitespace); read-only fields | `Receipt::seal`; `verify_chain` returns the first `ChainBreak{index, cause}` |
| `Refusal` | `#[non_exhaustive]`, named variants with typed fields, never strings | this crate |

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
| K6 host (admission, wire) | `Brief::parse` + `check_restatement`; parse `Observation`, ids and digests from the wire | pass a raw `String` where a newtype is required; admit a brief that failed either check |
