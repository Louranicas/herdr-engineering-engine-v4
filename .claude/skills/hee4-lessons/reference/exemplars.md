# Exemplars (EX-01…EX-20, "do instead") and anti-exemplars (A-1…A-10, "not this"): pointers

Index only. Each line: `trigger (reuse when) → the shape (≤ 15 words) → home`. The excerpts
(≤ 15 lines of v3 `b5367bc` code each) and the AP ids each prevents live in `docs/EXEMPLARS.md`;
the v3 originals are read-only reference (V4-9). Imitate the shape, never copy v3 code verbatim
into v4 without its strip step (`migrated/v3-b5367bc/MIGRATION.md`).

## Acquisition and bounds
- reading a frame or message off a socket → the acquisition itself is the bound → `EX-01`
- writing through the store's one door → deadline before COMMIT; poison on an uncertain commit → `EX-02`
- injecting a fault for a test → the fault path does not exist in the release build → `EX-03`
- bounding a collection from an iterator → `take(N+1)`, then refuse on length → `EX-13`

## Pure policy apart from I/O (F95)
- writing a decision the I/O feeds → a policy module with no I/O and named rules → `EX-04`
- parsing an enum from text → one spelling per variant; unknown text is `None`, never a default → `EX-05`
- writing a predicate over enum arms → it reads the values the arms carry → `EX-06`
- a settle or poll loop → the decision is a pure function of one poll → `EX-07`
- routing work → pure, explained, order-independent → `EX-10`
- a rule over mounts or devices → pure over (mount table, topology) values → `EX-11`
- resolving an endpoint → pure listener/holder policy plus one bounded acquisition → `EX-12`

## Types that refuse
- a counter or generation → `NonZeroU64` newtype; overflow refuses, zero unrepresentable → `EX-14`
- an identity value → private fields, validated constructor, a matcher instead of getters → `EX-15`
- combining check outcomes → fail-closed severity lattice with sealed output → `EX-17`

## Process lifecycle
- holding a pidfd → verify it before retaining; unverified is never a signal target → `EX-08`
- stopping a child → TERM and KILL each at most once, revalidated just before → `EX-09`
- a grace or timeout used by two enforcers → one constant, compile-time checked, rendered into the other → `EX-18`

## Durable install and one-door digests
- installing a release → `.partial` → fsync → re-hash what landed → rename → atomic `current` → `EX-16`
- computing a digest in two places → one door for it, with the reason it is one → `EX-19`
- a constant the contract text also states → check it against the contract text, not itself → `EX-20`

## Anti-exemplars (v3 code NOT to imitate)
- task state compared as text inside the store → state as text in the one door → `A-1`
- writing a generation or id as a literal → generation written as the literal "1" → `A-2`
- re-typing a grace "made one" elsewhere → a third door on one value → `A-3`
- re-typing a frame bound at a consumer → the bound re-spelt at the store → `A-4`
- one file owning many concerns → god-file with an intra-app cycle → `A-5`
- counting model-output fences as an exact rule → judgment parsed as exact → `A-6`
- a probe struct of many Options → option-soup state → `A-7`
- a docstring stating a property → the code did not have it → `A-8`
- a wrapper silently clamping a deadline → refuse instead of clamping → `A-9`
- a test-only module exported as pub → layering inverted by a test seam → `A-10`
