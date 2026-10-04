# Verification, tuned to this stack

Seven tunings (`plan/STACK-MAP-2026-10-04.md` §3):
1. Deterministic decides; models advise (tier-0 only feeds the lattice).
2. Bind every observation to its subject: `head_sha` + `input_sha256`; gate runs on a `git archive` at a sha.
3. Refuse to look at nothing (zero files / zero tests / zero features = refused, not green).
4. Doctor first: a result against a stale binary is not evidence.
5. Observations are ledgered before the verdict; recovery replays them.
6. Tiers derived from cost and printed (`elapsed/budget`, `margin=`); no count literals.
7. Verification grows only by the correct ladder.

First concrete rung-2 door: `git diff | deep-diff-forge --stdin-patch --rank --json --require-files --require-hunks` — exit 7 on empty input, `input_sha256` + `tool{name,version}` in the output (verified 2026-10-04 against `sha256sum`; byte-identical across runs).

The feature map `gates/features/` is the rung-3 substrate: one file per release action, four H2s, sweep order, same-change rule.

Related: [[meta-goal-rungs]], [[contradictions-2026-10-04]].
