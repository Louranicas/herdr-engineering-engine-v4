# hee4-app · class-profile
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 (composition; the only crate that sees more than one cluster) | UM:65, UM:79 |
| v3 origin | `app` → `src/app/class_profile.rs` (screen, compose, `busctl_sha256` pin) | DP:96; IM:51 |
| Status | PLANNING — HOLD | DEC:4 |
| Config it reads | `~/.config/hee4/classes/…` (v3: the per-class config dir listed at IM:72) | UM:182; IM:72; DEC:31 (V4-11) |

**Design section:** [K6 hee4-app › class-profile](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23class-profile): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | Content | Source |
|---|---|---|
| Purpose | Load and screen a task class profile (the class's pinned tool digests, bounds and compose inputs) and compose it for dispatch | DP:96; IM:51 |
| Owned state | None durable. The profile is config; the app owns composition only | UM:79 |
| Pins | `busctl` digest pin (`busctl_sha256`); tool digests are **host-side** and differ per side (host busctl `1466a64c…` vs toolbox `85d8f9f0…`) | IM:51; UM:171; AT:114; AT:136 |
| Allowed deps | K6 may depend on K0, K0h, K0e, K1-K5 (UM:65). This module needs K0 (class/bounds types) and K0h (pinned-client digests) — INTERP from the pin it holds | UM:65, UM:72 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3 Worker + native + verdict (the U64 class builds for musl + rust-lld; pins from `~/.rustup`) | P2 green | gate; plants per rule | none in P3 (AT:82) |
| P4 Release + first host record (host pins re-read by HP-rows before every host run) | P3 + Tier-2 | host record committed; manifest stores **host** pins | H-9 (TH-DEV / loopback re-affirmed for v4) |
Feeds D1 (manifest digests), D6 (real task through the class) — AT:58, AT:63. Sources: AT:82-83, AT:136, AT:193.

## 4 · v3 basis
- **Flag:** `app` PARTIAL (MA:19). **Recommendation:** `app` REFACTOR — runtime is a 3,814-line hub; main holds policy (AR:7).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| `screen` admits `NotInstalled` (EXACT decision site) | `class_profile.rs:432` | DP:96 | — |
| `compose` | `class_profile.rs:482` | DP:96 | — |
| busctl digest pin | `class_profile.rs:166,541` | IM:51; UM:171 | — |
| A pin digest is valid on one side only (host vs toolbox) | — | AT:136 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `screen :432` (NotInstalled admits) · EXACT (DP:96) | — (EXACT never Jev, JM:22) |
| `compose :482` · EXACT (DP:96) | — |
| acquisition bound "profile" · THRESHOLD · safety limit, not calibration (DP:104) | not-Jev (JM:23) |

## 6 · Migrated inputs
None — `app` is REFACTOR, redesigned from findings, not carried verbatim (MIG:22). v4 names use the `hee4` namespace (V4-11, DEC:31).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | pins must have one definition site; the manifest's host pin and the profile pin must not be two doors (AT:136) |
| AP-49 | a pinned digest is a claim until the binary on the running side is re-hashed (HP-rows, AT:136) |
| AP-06 | "which side am I on" must be an input, not an environment branch |
| AP-19 | a test that pins one digest on a fresh profile pins nothing; assert two profiles that differ in every field |
| EX-19 | one door for a digest, with the reason it is one (EXX:449) |
| EX-16 | re-hash what landed, not what was intended (EXX:425) |
| EX-05 | class/state names: one spelling, unknown → `None` (EXX:152) |
| D-09 | K6 wiring only; class content stays behind a port, not inside K4 (UM:77) |
| D-01 | the class exists to move D6/D9; a slice that moves no flow names its excuse |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| config (read) | `~/.config/hee4/classes/` | UM:182 |
| external (via K0h) | `busctl` digest-pinned, StartTransientUnit/StopUnit/get-property | UM:171; IM:48-51 |
| consumers | runtime/dispatch (class chosen at admission), u64-class port | UM:77 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Profile screen refuses by name on every absent/mismatched pin; `NotInstalled` behaviour decided explicitly (admit or refuse) and recorded | unit tests over both fixtures + a row in DEC |
| 2 | Host pins in the manifest equal the digests re-read on the host before each host run | HP-row output quoted in the host record (AT:136, AT:151) |
| 3 | No pin literal outside the one definition site | one-door census `duplicate_sites=0` (UM:39; DEC:37 V4-17) |
| 4 | Plants: swap one pin digest → the named test fails under `--cap-lints=warn` | plant battery line `plants=k/k killers=named` (AT:149) |
| 5 | Scoped `cargo mutants` over screen/compose with `CARGO_TARGET_DIR` unset | `mutants caught=a survivors=b` each named (AT:132, AT:149) |

## 10 · Open decisions and risks
- Is `screen`'s `NotInstalled → admit` intended? DP:96 records it only as a site; UNMEASURED which way v4 should go.
- Host vs toolbox pin divergence after a toolbox rebuild (AT:114, AT:240).
- H-9: TH-DEV acknowledgment for v4 governs running model-generated candidates of this class (AT:193).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-39 (V4-61): `Class` port: home and seam shape. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
V4=~/herdr-engineering-engine-v4; EV=~/hee4-evidence; R=$EV/reference/v3-evidence-b5367bc
rg -n 'class_profile|class profile|classes/' $EV/design/*.md $R/interface-map-b5367bc.md
sed -n '79p;169p;180p' $EV/design/ULTRAMAP.md
sed -n '77,78p;109p;131p;185p' $EV/design/DEPLOYMENT_ATLAS.md
sed -n 19p $R/module-audit-b5367bc.md; sed -n 7p $R/architecture-review-b5367bc.md
rg -n '^\| \*\*AP-(01|06|19|49)\*\*' $V4/docs/ANTIPATTERNS.md
sed -n '425,472p' $V4/docs/EXEMPLARS.md
```
