# Anti-patterns (AP-01…AP-50) and drift controls (D-01…D-16): pointers

Index only. Each line: `trigger → lesson (≤ 15 words) → home`. The home is the row in
`docs/ANTIPATTERNS.md` (Definition, Tell, Detector/TRIGGER, v3 instance, Mirror EX) or the section in
`docs/DRIFT_AND_OVERENGINEERING.md`. Read the home before acting; this file never holds the detail.
Where an AP landed in the ATLAS: `~/hee4-evidence/learnings/FINAL-LESSONS-REVIEW.md#3 · Anti-patterns (AP-01 … AP-50)`.

## A · Code design
- writing a literal, predicate or verdict → `git grep` its value first; one rule, one enforcing site → `AP-01`
- a field whose valid values are a closed list → type it as an enum with one pure transition fn → `AP-02`
- a struct gains a fourth `Option` for one lifecycle → phase enum carrying only that phase's data → `AP-03`
- reading caller-controlled input (read_to_string, collect, split) → bound at acquisition; bound the derived set too → `AP-04`
- endorsing a refusal or bound at a function → list callers; name the reachable domain first → `AP-05`
- a branch only reachable by arranging the world → split pure policy from thin I/O → `AP-06`
- a signature that could re-acquire what it was given → pass the value or a proof type → `AP-07`
- a file crosses 1,500 lines or a second concern → split; print `largest_file=` at each cut → `AP-08`
- adding `pub` → something must reach it; run `habitat-unused-pub` → `AP-09`
- a new `pub` item with only test importers → `#[cfg(test)]` seam, never release API → `AP-10`
- a cross-cluster `use` → crates are clusters; let the compiler refuse the edge → `AP-11`
- a second list of the same string ids → dispatch through one typed registry → `AP-12`
- a new table, variant or field → name its writer AND its reader in the FLOW page → `AP-13`
- `min`/`clamp`/default on a caller-supplied bound → refuse by name with both numbers instead → `AP-14`
- a comment saying only / never / every / one door → grep the claim; anchor it or delete it → `AP-15`
- parsing model or human judgment as an exact rule → declare it JUDGMENT; labelled fixtures first → `AP-16`
- reaching for `#[allow]` → climb the suppression ladder one rung at a time → `AP-17`

## B · Verification and testing
- writing a test double → record what it was handed; `_`-prefixed params are unassertable → `AP-18`
- an expected literal of 0, 1, GENESIS, "" or <prefix>-1 → move the assertion off the origin → `AP-19`
- asserting `.contains(` on a rendered line → assert the whole line over two fixtures → `AP-20`
- writing a known-answer test → name the independent source; on red, find which side is wrong → `AP-21`
- a control or gate goes red/green → assert on the rule's own diagnostic, not the exit → `AP-22`
- a plant is killed → re-run under `--cap-lints=warn`; require the named test → `AP-23`
- a negative control fires → neuter each rule in turn; print `clauses=N/N` → `AP-24`
- a detector's survivors are not falling per round → stop; ask what the compiler can refuse → `AP-25`
- matching declarations with a regex, or counting reason names → parse; count sites, not names → `AP-26`
- a check over a set that can grow → enumerate the world; exclusions only with reasons → `AP-27`
- a test or gate pins a count literal → assert relations, never hand-bumped counts → `AP-28`
- quoting a verdict → from the command's exit and printed number, not a pipe or caption → `AP-29`
- reporting a gate result → name the tree: `tree=<sha> dirty=N`, gate on a snapshot → `AP-30`
- writing any loop or exit-path await → name its budget; assert with both numbers → `AP-31`
- running mutants or plants → runner owns its target dir; a precheck plant must go red → `AP-32`

## C · Process and agents
- about to report your own work green → independent review on committed objects → `AP-33`
- relaying an agent's or peer's claim → spot-check it; unwitnessed is UNVERIFIED → `AP-34`
- launching a fan-out → `planned_agents=` first; batch refuters; command-budget briefs → `AP-35`
- a third design round in prose → land the buildable cut after two rounds → `AP-36`
- appending to a record → compute the append target at write time; check a reader exists → `AP-37`
- stopping a process → kill by PID from a pid file, never by pattern → `AP-38`
- chaining a commit after checks → one tool ANDs typed lines, commits only on PASS → `AP-39`
- before acting on a path → confirm the layer and target (host vs toolbox, v3 vs v4) → `AP-40`
- writing a helper script → check `~/agent-harness` first; do not re-derive primitives → `AP-41`
- a receipt says "workaround:" → the rule's scope is the suspect, not the code → `AP-42`
- scripted insert before `fn x(` → insert after the previous item's closing brace → `AP-43`

## D · Documentation and corpus
- generating a corpus or status page → no reader, no row; no publisher on the landing chain → `AP-44`
- generated `BEGIN…END` spans in source → refuse them in src, tests, migrations, manifests → `AP-45`
- writing state (sha, counts) into a note → state is a query; compute it on read → `AP-46`
- editing an output of record or a projection → find the owner; fix the source → `AP-47`
- the same topic in two files → one home plus a crosswalk; link, never copy → `AP-48`

## E · Deployment and operations
- setting state through a syscall, daemon or service → read it back; fsync before the ack → `AP-49`
- a push, grant or prerequisite needed late → preflight every grant at minute one → `AP-50`

## Drift and over-engineering controls
- starting a stack → name the flow it moves; `l2` must rise → `D-01`
- a third design round → land the buildable cut → `D-02`
- asking for review → only once it compiles → `D-03`
- proposing a new detector → admission header: trigger, budget, reader → `D-04`
- at every cut → print the apparatus ratio (scripts vs product) → `D-05`
- at charter time → declare the scope fence (`habitat-scope set`) → `D-06`
- adding to an always-loaded file → one topic, one home; budget the bytes → `D-07`
- adding a corpus row → no reader, no row → `D-08`
- a dependency rule → let cargo's workspace edges enforce it → `D-09`
- strengthening a check again → survivors per round must fall, else stop → `D-10`
- two zero-delta stacks in a row → stop the line and ask why → `D-11`
- ~3 phase boundaries run autonomously → checkpoint: re-assert the standard, audit drift → `D-12`
- writing status into a file → state is a query, never a field → `D-13`
- admitting apparatus → prune something of equal weight → `D-14`
- sizing an agent fan-out → budget agents by severity and yield → `D-15`
- calling something done → "deployed" is the only done → `D-16`
