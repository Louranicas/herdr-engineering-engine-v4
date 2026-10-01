# Triggers: situation → which lessons to load

ONE table. Find the row for what you are about to do; load the ids from the reference files
(`antipatterns.md`, `exemplars.md`, `spells.md`, `mistakes.md`), then their homes. The last column
indexes `~/CLAUDE.md` §3–§5 rules by F-number only: that file is always loaded, so it is never copied.

| About to… | Anti-patterns / drift | Do instead (EX) | Loops, mistakes, spells | `~/CLAUDE.md` rule |
|---|---|---|---|---|
| write a **store door** / write path | `AP-01` `AP-02` `AP-04` `AP-31` `AP-49` | `EX-02` `EX-05` `EX-14` · not `A-1` `A-2` | `MM#6` `spell:Witness the Collapse` | `F132` `F90` |
| read **caller-controlled input** (socket, file, frame) | `AP-04` `AP-05` `AP-14` | `EX-01` `EX-13` `EX-12` · not `A-4` `A-9` | `MM#56` `MM#27` | `F103` |
| write a **pure policy / decision** fn | `AP-06` `AP-07` `AP-16` | `EX-04` `EX-07` `EX-10` `EX-11` | `P14` | `F95` |
| add a **state enum / transition** | `AP-02` `AP-03` `AP-13` | `EX-05` `EX-06` `EX-14` `EX-17` · not `A-7` | `MM#23` | `F89` |
| write a **test double** | `AP-18` `AP-19` `AP-20` | `EX-03` `EX-06` | `MM#54` `MM#9` | `F101` `F122` `F124` |
| write a **known-answer / property test** | `AP-21` `AP-19` `AP-28` | `EX-20` | `MM#25` `MM#27` | `F94` `F113` `F108` |
| write a **loop / await / shutdown path** | `AP-31` | `EX-07` `EX-09` | `MM#38` | `F102` |
| add a **constant / grace / bound** used twice | `AP-01` `AP-19` `AP-15` | `EX-18` `EX-19` `EX-20` · not `A-3` `A-8` | `L16` | `F122` |
| stop or signal a **child process** | `AP-38` `AP-49` | `EX-08` `EX-09` | `MM#37` `MM#55` `spell:Kill by PID, Not by Pattern` | `F137` |
| write or extend a **gate / check / detector** | `AP-22` `AP-24` `AP-25` `AP-27` `AP-29` `D-04` `D-10` | `EX-17` | `L1` `L6` `L13` `MM#39` `MM#40` `spell:The Welded Judge` | `F96` `F125` `F134` `F138` `F140` |
| run a **plant battery / mutants** | `AP-23` `AP-32` | `EX-03` | `L14` `L15` `MM#43` `K2` `K3` | `F133` · `memory:mutation-target-dir-trap` |
| **report a verdict** / write a receipt | `AP-29` `AP-30` `AP-33` `AP-34` | `EX-17` | `MM#28` `MM#29` `MM#30` `L28` `spell:Distrust the Evidence You Minted` | `F105` `F126` |
| write a **subagent brief** / fan out | `AP-34` `AP-35` `D-15` | — | `MM#35` `MM#53` `L19` `L21` `K10` | `F109` |
| write a **migration / schema / install** | `AP-45` `AP-49` `AP-13` | `EX-16` | `L10` `MM#20` | `F90` `F132` |
| add a **crate edge / pub item / file** | `AP-08` `AP-09` `AP-10` `AP-11` `D-09` | `EX-15` · not `A-5` `A-10` | `L27` | `F65` |
| write a **regex / census** over code or docs | `AP-26` `AP-27` | — | `MM#12` `MM#59` `S-4` | `F140` `F147` |
| edit a **planning doc / register / note** | `AP-15` `AP-44` `AP-46` `AP-47` `AP-48` `D-07` `D-13` | — | `MM#50` `MM#51` `MM#57` `S-2` | `F86` |
| run a **shell pipeline / blanket command** | `AP-29` `AP-39` `AP-41` | — | `MM#11` `MM#20` `MM#36` `L22` `L23` `K13` `S-5` | `F137` `F135` |
| start a **stack / design round** | `AP-36` `D-01` `D-02` `D-03` `D-11` `D-16` | — | `L12` `K4` `K5` `spell:Harden by the Frame You Didn't Take` | `F128` |
| need a **push / grant / permission** | `AP-50` `D-06` | — | `L20` `K14` `S-6` `S-8` | `F90` |
| a **scope / rule** feels wrong | `AP-42` `AP-17` | — | `L26` `MM#32` `MM#8` | `F121` |
