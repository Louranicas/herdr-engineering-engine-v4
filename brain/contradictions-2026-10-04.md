# Contradictions on record (2026-10-04), none resolved yet

From the feature map (`plan/STACK-MAP-2026-10-04.md` §8):
1. `modules/hee4-evidence/check/MODULE.md:22` says `Class` port in K6; `API Map.md:122` (PT-11, V4-61/DC-39) says `ClassFacts` value, no trait. Card stale.
2. `modules/hee4-app/actions/MODULE.md:24-25` vs `API Map.md:49-50`: which half of `task.*` is P2 vs P5 is ambiguous.
3. E2E-08 (`End-to-End Flow Traces.md:315`) assumes the unit restarts `serve`; ATLAS D7 says no `Restart=` exists.
4. Refusal names differ between API Map and Error map (`not_found` vs `unknown_action`; `stale_generation` vs `resync_required`).

From the morning review: ATLAS P0 cell cites V4-77, which does not exist; README's "blocks P0" list (H-1, H-5, H-27) is stale — H-1's P0 half is met (V4-76).

Each is a case of two homes disagreeing — the rung-2 door "one name per refusal / one phase per action" does not exist yet.

Related: [[verification-spine]], [[meta-goal-rungs]].
