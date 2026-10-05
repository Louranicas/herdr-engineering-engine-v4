# A content address needs its scope in the digest

**Seen 2026-10-05 (V4-96).** `observation_id` was `obs-` + sha256(canonical JSON of the
observation). The ledger binds each observation row to one task. Running the feature drive
twice on one ledger produced two tasks whose observations were byte-identical (same head, same
VERIFY digest, `/usr/bin/true` in 0 ms), so the second task's row collided with the first and
the store refused it: "already recorded with another body". The dispatch aborted between
`Settle(ready)` and `Observe`, and five live tasks were left in `verifying`.

**Rule.** If a row that is keyed by content also belongs to an owner (a task, a unit, a
receipt chain), put the owner in the digest. Two owners with identical content are two facts,
not one. The store's refusal was correct. The id was wrong.

**Carried.** No recovery rule yet re-decides or quarantines a task left in `verifying` with no
running attempt (R03/R09). The attempts ledger (U-stack-04 wave 2) and the dispatcher (wave 3)
own that rule. See [[meta-goal-rungs]].
