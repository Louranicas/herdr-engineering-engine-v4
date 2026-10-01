# Recurring mistakes, principles, v2/v3 process loops and prototype lessons: pointers

Index only. Each line: `trigger → the lesson (≤ 15 words) → home`. Homes:
- Mistakes (MM#n): `obsidian://open?vault=my-diary.vault&file=Reflections%2FMistakes%20I%20Made`
  = `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/my-diary.vault/Reflections/Mistakes I Made.md`
- Principles (Pn): `obsidian://open?vault=my-diary.vault&file=Principles%2F00%20-%20Principles`
  = `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/my-diary.vault/Principles/00 - Principles.md`
- v3 loops (Ln) and keeps (Kn): `~/hee4-evidence/learnings/PROCESS-LEARNINGS.md`; where each landed in
  the ATLAS: `~/hee4-evidence/learnings/FINAL-LESSONS-REVIEW.md#1 · PROCESS-LEARNINGS loops (L1–L27)`.
- v4 session lessons (S-n): `~/hee4-evidence/learnings/FINAL-LESSONS-REVIEW.md#0 · Session lessons (this session; ids S-1 … S-9)`.

## Recurring mistakes that bite in v4 coding (diary register)
- verdict from a pipe (six times) → `gate <cmd>` or `${PIPESTATUS[0]}`; a guard cannot reach every shell → `MM#11` `MM#19` `MM#21` `MM#44` `MM#48` `L23`
- killing a process → by pattern it hit my own shell, rule in front of me → `MM#37` `MM#55` `AP-38`
- a brief for an agent → it killed another session's run; the brief lacked the rule → `MM#35` `L21`
- chaining a commit after checks → committed a file that did not parse → `MM#36` `MM#24` `L22`
- a green on a warm target dir → declared clean; cold run found the defect → `MM#1` `P1`
- running a tool from the wrong layer → host vs toolbox, v2 vs v3: believed the output → `MM#2` `MM#49` `AP-40`
- writing a bound → check it bounds anything at the point of acquisition → `MM#6` `P3`
- reaching for `#[allow]` → went straight to the bottom rung → `MM#8` `P18`
- a test that passes when it does not run → assert it ran; count what it measured → `MM#9` `MM#17` `MM#18`
- designing a pattern-match over a corpus → sample 3-5 artifacts first → `MM#12`
- a guard or script reports success → assert the target state, not the guard → `MM#15` `AP-49`
- a mechanism that fits every observation → probe it before publishing → `MM#16`
- `rm -rf` on a directory → enumerate tracked files first → `MM#20` `K13`
- the lint law makes code ugly → suspect the representation, not the law → `MM#23` `P4`
- a red known-answer test → establish which side is wrong first → `MM#25` `AP-21`
- a scan range for probing → name the reachable domain from the signature first → `MM#27` `AP-05`
- quoting "zero warnings" → quote the printed N, never the caption → `MM#28` `MM#46` `AP-29`
- writing a claim into a record → quote the line the tree computes for it → `MM#29` `MM#30`
- a control meets the real world for the first time → run it before the gate does → `MM#31`
- widening a rule's scope → measure its cost; a workaround indicts the rule → `MM#32` `AP-42`
- writing a check against your own architecture → re-read the decisions it enforces → `MM#33`
- a negative control passes → it may measure the world it replaced → `MM#39`
- editing a control until the detector passes → never; that is narrowing the check → `MM#40`
- running a long job in another agent's repo → not your task, not your tree → `MM#41`
- truncate-and-commit in one command → guard every step a later step depends on → `MM#42`
- landing with a mutation sweep "owed" → the sweep found 21 survivors after landing → `MM#43` `L15`
- correcting a claim → check the claim, not the sentence; same overconfidence recurs → `MM#45`
- an instrument's output → seven times the instrument was wrong, reported as the world → `MM#47`
- editing published outputs → ask what tracks them; fix the source, not the projection → `MM#50` `MM#51` `AP-47`
- verifying with your own scratch gate → use the gate the repository owns → `MM#52` `L16`
- fanning out reviewers → check whether another session already answered → `MM#53`
- recording a path as "unobservable" → name the refusal it records and assert it → `MM#54` `AP-18`
- endorsing a refusal at a function → ask whether the flow reaches it → `MM#56` `S-3`
- appending to a handover → compute the newest target at write time → `MM#57` `S-2` `AP-37`
- a review verdict only in a message → write it where a reader looks → `MM#58`
- measuring module deps with a regex → parse, or state the regex's limits → `MM#59` `S-4`
- relaying agent or peer claims → spot-check before relaying → `MM#61` `S-1` `AP-34`

## Principles most likely to fire while building
- "remember to…" appears in a fix → it is a design problem → `P2`
- choosing tests to report → name the ones that would catch a regression → `P6`
- autonomous run goes quiet → silence is the dangerous failure mode → `P8`
- verifying your own work → self-verification has a ceiling → `P9`
- a plan with no contact → planning cannot substitute for contact → `P10`
- unsure which layer → know which layer you are standing on → `P12`
- a verdict → name the input that earns it → `P13`
- a signature could re-acquire its input → change the signature → `P14`
- your own RCA → take the broadest definition that covers you → `P15`
- a long autonomous run → checkpoint; drift is checkpoint-bound → `P16`
- a rule with no detector → it is a slogan → `P19`
- trusting a favourite instrument → the trusted ones are least tested → `P22`
- something is uncounted → what is not counted grows → `P28`

## v3 process loops (inefficiencies to design out) and keeps
- every slice paying gate + cold + publication → tier confirmation: commit / stack / cut → `L1` `L2`
- hand-bumped count literals in a gate → derive counts; relations only → `L3` `AP-28`
- first gate on a fresh worktree → it lacks generated inputs; provision hermetically → `L4`
- gate fails on leaked descendants → process hygiene is part of the gate → `L5`
- precount red on healthy trees → a check that cannot discriminate measures nothing → `L6`
- timing margins under load → load admission; no builds beside a gate → `L7`
- editing under a running gate → snapshot-subject gates; never edit a gated worktree → `L8`
- publication on the landing chain → no publisher; no reader, no row → `L9` `L11` `AP-44`
- corpus markers in product semantics → no generated blocks in source → `L10` `AP-45`
- design rounds in prose → ≤ 2 rounds, skeleton first → `L12` `D-02`
- detector survivors not falling → receding horizon: stop strengthening → `L13` `D-10`
- plants that planted nothing → a non-compiling plant is not a kill → `L14` `AP-23`
- mutation runs voided by a shared target dir → runner owns and unsets it → `L15` `AP-32`
- scratch gate passes, repo gate refuses → two doors on one rule → `L16` `AP-01`
- handover sprawl, wrong-file appends → one append-only decision log → `L17` `AP-37`
- relayed claims → witness per claim; `spot_checked=k/n` → `L18`
- fan-out cost exceeds yield → admit by severity; batch refuters → `L19` `AP-35`
- a classifier refuses late in a run → grant sweep at minute one → `L20` `AP-50`
- two sessions, one process → leases; kill only your own pid → `L21`
- built context on the wrong layer or codebase → resolve root and view before acting → `L24` `AP-40`
- stack branched before its base moved → rebase and re-gate → `L25`
- a rule scoped wider than its harm → record harm and scope per rule → `L26` `AP-42`
- planning layers growing where nothing counts → count apparatus vs product → `L27` `D-05`
- a roster run with no typed verdict → exit derives from the verdict line → `L28` `S-7`
- same-lineage verification → PASS_WITH_GAPS every time; get cross-lineage → `L29`
- a staged fix about to meet a new world → run the new world through it first → `L30`
- a bundle installed as a unit → test it as the unit, then on the installed paths → `L31`
- a cell or table labelled "derived" → find the generator or make it the query → `L32` `AP-48`
- a control's first green run → it tests the verdict precedence too; controls outrank → `L33`
- a "staged, not installed" line → measure applied state both ways first → `L34` `AP-46`
- a mutant or neuter that did not run → INVALID, never a kill or a survivor → `L35` `AP-32`
- editing a file a patch touched → its reverse-diff undo may break; keep a whole-file restore → `L36`
- keep: independent review of every slice on committed objects → `K1`
- keep: plant batteries with named killers under `--cap-lints=warn` → `K2`
- keep: scoped mutation on store and policy code → `K3`
- keep: the flow ratchet `l2=` before and after → `K4`
- keep: skeleton before review, ≤ 2 rounds → `K5`
- keep: enumerate before any blanket act → `K13`
- keep: push = explicit word + secret scan + `ls-remote` read-back → `K14`
- keep: gate inputs derived, not listed → `K15`
- keep: compiler/symbol-decided checks over source censuses → `K16`

## HEE-v2 lessons that transfer (habitat vault, read-only)
- evidence that cannot name its tree → a receipt without `tree=` is undetectably stale → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/herdr-fedora-habitat.vault/95 Genesis/Assimilation - Operational Learnings.md#1 · Evidence that cannot name its subject → **D15**`
- a gate admitted because it was written → show it catches a planted fault first → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/herdr-fedora-habitat.vault/95 Genesis/Assimilation - Operational Learnings.md#2 · A gate's coverage is a claim → **D16**`
- a destroy or cleanup step → enumerate; a true label hides tracked files → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/herdr-fedora-habitat.vault/95 Genesis/Assimilation - Operational Learnings.md#3 · Destructive operations must enumerate → **D17**`
- an agent roster or registry → registration is not activation; eligibility before preference → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/herdr-fedora-habitat.vault/90 Engineering Engine/IDD Learnings Register.md#✅ Learnings ADOPTED (as ideas, re-derived fresh in HEE-v2)`
- "does my change collide?" → ask from the other side's HEAD; dirty-pull refusal is not a conflict → `~/handoffs/CLAUDE.local-HEE-v2-history.md#§2 ⏯️ HEE-v2 — round 2 AND CB-46 landed and pushed (re-measured 2026-09-14)`

## Prototype and lineage lessons (distilled in the diary; the prototype itself is read-only)
- before any completion report → walk the false-pass family against it → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/my-diary.vault/Reflections/The Antipattern Registers.md#The false-pass family — aimed squarely at me`
- promoting a lesson to enforcement → detector, severity, phase, negative control, use-pattern mirror → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/my-diary.vault/Reflections/The Antipattern Registers.md#The contract — the actual gold`
- writing Rust, shell or method rules → the substrate-free transferable set → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/my-diary.vault/Reflections/The Antipattern Registers.md#The set that transfers regardless of substrate`
- a suppression → the ladder came from the ancestors → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/my-diary.vault/Reflections/What My Ancestors Knew That I Did Not.md#1 · The suppression ladder`
- reviewing your own lineage's work → cross-family review is the escape → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/my-diary.vault/Reflections/What My Ancestors Knew That I Did Not.md#4 · Cross-family review is the escape from self-verification`
- investigating a stale field → check the mtime before the field → `/var/mnt/STORAGE-10TB/fedora-obsidian-vaults/my-diary.vault/Reflections/What My Ancestors Knew That I Did Not.md#9 · Check the mtime before you investigate the field`
- a trap recorded where it does not travel → re-derive it into the Fedora vaults → `memory:claude-lineage-corpus`
