# tooling · deploy
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | none — **outside the crate DAG** (`deploy/`); the new `hee4 release …` verbs are served by the K6 binary (see §8) | UM:80, UM:150 |
| Cluster | outside (MIG corrected from K6, V1) | MIG:19; V1:37 |
| v3 origin | `deploy` (`install-release`, `worker.container`, `README.stub.md`, `herdr/README.stub.md`) | MA:22; MIG:19 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [T tooling › deploy](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/T%20tooling%23deploy): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Content-addressed release install (`.partial` → fsync → re-hash → rename → atomic `current`), rollback, prune; the `hee4.service` user unit; install-by-copy + read-back | UM:150, UM:183-184; AT:83, AT:85 |
| Owned files | `~/.local/lib/hee4/releases/<manifest-sha256>/` + `current`; `~/.config/systemd/user/hee4.service` | UM:183-184; DEC:31 (V4-11) |
| Manifest must record | linker, CRT, libgcc, link argv (RA7 "cc"), host tool pins, `seam_strings=0/0`, `retain=` | AT:58, AT:83; RR build-03/04 (RR:16-17) |
| Reads | K0's emitted schema; features/flags **measured from cargo**, not certified by the script's own values | UM:80; AR:27 |
| Roster install | an internal K1 operation kind `Owner::Deploy`, never the wire id `roster.update` | UM:135; AR:35 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P0 Scaffold (**INTERP**: AT's P0 row puts these under `tools/`, not `deploy/`; carried here as the deploy path's first gate): repo-owned `tools/cold-clone` + committed `tools/offline-bundle.lock` | "start coding" | gate + cold clone `steps=N matched=N tree= dirty=0` (AT:79) | **H-5**, H-1 (remote) |
| P4 Release + first host record: `hee4 release install\|rollback\|prune`, manifest with linker/CRT/libgcc/argv, `retain=` K=3 | P3 + Tier-2 | `hee4 release install --control cases=k/k` *(rev 2026-10-01 V11-fix)*; host record at a clean sha (AT:83) | none if H-9 re-affirmed |
| P6 Unit + security lens: `deploy/hee4.service`, login-only, no linger, `TimeoutStopSec` = 1,200 s + measured seal time | P5 + backup/2 + restore | D2, D3, D4 read-backs; lens 0 HIGH (AT:85) | **H-3** (enable), **H-4** (commissioning), **H-6** |
| P7 Operate + rehearse: runbooks 06-deploy / 07-recover executed; `hee4-backup` habitat unit | P6 | rehearsal record with per-step `rc=` (AT:86) | **H-7** |
| P8 Tier-3 + tag | P7 `deployed=9/9` (D1–D9; D10 at P9 entry, ATLAS P8 row) *(rev 2026-10-01 V11-fix)* | cold clone, push-scan `hits=0` (AT:87) | **H-1, H-2** |
Feeds D1, D2, D5(c), D7, D10 (AT:58-59, AT:62, AT:64, AT:67). v3 releases on disk are H-17's, not prune's (AT:202).

## 4 · v3 basis
Flag **PARTIAL** (MA:22): "RA7 manifest and content-addressed install; no systemd unit (HO-07), no linker/link argv". Recommendation **HARDEN** (AR:27): `worker.container` has 0 directives; 3 manifest fields certify the script's own values → real Quadlet or `.stub`; measure features/flags from cargo.

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Manifest omits linker, CRT, libgcc, ar, libstd | `deploy/install-release:536` | RR:16 (build-03) | — |
| Install partial → fsync → rehash → rename (sound, exemplar) | `install-release:162-217` | EX-16 | — |
| `target_refusal` is at `:121` | `install-release:121` | DP:134 | **E16** (was :44, the OWNED regex) |
| No restore/retention/prune; 9 v3 releases, 264 MB accumulated | `install-release:186-189` | AT:39 (A4) | — |
| Two cold-clone scripts, two bundles, inputs from the live checkout | — | AT:42 (A7); DEC:22 (V4-7) | — |
| `worker.container` has 0 directives | `deploy/worker.container` | AR:27; UM:273 | — |
| Persistent per-commit target dir: refuted as a finding | `install-release:462` | RR:34 (security-3) | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| deploy problems/quiescence/target_refusal :84/:112/:121 · EXACT (DP:134) | not-Jev |
| release retention K=3 and backup prune N=7 · THRESHOLD (PROPOSAL, pure policy) (AT:83, AT:225-226) | not-Jev |

## 6 · Migrated inputs
| File | Lines | Anchor lines | Change when brought in |
|---|---|---|---|
| `migrated/v3-b5367bc/deploy/install-release` | 570 | 0 | replace the v3 install prefix (3 lines name it: `:3`, `:366`, `:445`; also `:53-54` write `~/.cache/hee3-release-*` and are stripped with them *(rev 2026-10-01 V11-fix)*; `:347` and `:350` name the v3 cargo package in cargo-tree test strings and change with the crate rename, not the prefix) and binary names `habitat-engine`/`hee-namespace-shim` (17 lines name the engine) with `hee4` / `hee4-namespace-shim` (DEC:27 V4-10 counts "8 v3 install paths"; DEC:31 V4-11; UM:82); route every subprocess through bounded capture (MIG:19); add rollback/prune (V4-15) |
| `…/deploy/worker.container` | 632 | 554 | strip anchors (V4-10); ship as `.stub` until a real Quadlet is needed (UM:273) |
| `…/deploy/README.stub.md` | 3,059 | 2,745 | strip anchors; rewrite |
| `…/deploy/herdr/README.stub.md` | 445 | 378 | strip anchors |
MIG:19 counts 4 files / 4,706 lines. Keep the `--control` cases (seam count, digest mismatch, quiescence; `install-release:252-371` in the copy).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-49 | a successful install is not a changed state: read back `current`, digests, `/proc/<MainPID>/exe` |
| AP-10 | seam strings in a release binary (`HEE4_TEST_*` count = 0 with a positive control, AT:161) |
| AP-27 | cold-clone inputs from the committed tree + digest lock, never the live checkout (AT:160) |
| AP-50 | unit enable / push grants preflighted at minute 1 (H-3, H-1) |
| AP-14 | prune must never silently drop `current` or the rollback target |
| AP-01 | two cold-clone scripts (A7) |
| EX-16 | imitate: partial → fsync → re-hash → rename → atomic `current` |
| EX-11 | imitate: device/prune rules pure over values (F95) |
| D-16 | this module is done only when the D1/D2 read-backs of §9 #1-#2 print (`current` digest, `is-enabled`, the MainPID exe digest equal to the manifest), never on an install exit code *(rev 2026-10-01 open-tasks CN-19: V5 generic row made module-specific)* |
| D-01 | release work names the flow it moves (D1/D2) |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| CLI | `hee4 release install \| rollback \| prune` (V4-15) | UM:150; DEC:35 |
| Unit | `hee4.service`, `ExecStart=%h/.local/lib/hee4/current/hee4 serve` | AT:59 |
| Files | releases + `current`; unit file; backups `/var/mnt/STORAGE-10TB/hee4-backups/` | UM:183-184; AT:85 |
| External | `systemctl --user` via `flatpak-spawn --host` | AT:58-59 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | D1 installed by digest | `readlink -f ~/.local/lib/hee4/current`; binary sha256 = manifest; line `release=… seam_strings=0/0 current=ok host_readback=ok` (AT:58) |
| 2 | D2 unit enabled + read back | `is-enabled`/`is-active`; `/proc/<MainPID>/exe` digest = manifest (AT:59) |
| 3 | Manifest records linker/CRT/libgcc/argv | manifest fields present, measured from cargo (RR:16-17; AT:83) |
| 4 | Rollback + prune | `readlink current` before/after; prune prints `keep=K prune=P bytes=B`, `--apply` needs a same-digest dry-run (AT:225) |
| 5 | D7 runbooks rehearsed | per-step `rc=` record (AT:64) |
| 6 | One cold-clone script | `tools/cold-clone` + `offline-bundle.lock` refusing on digest mismatch (AT:160; DEC:22) |
| 7 | Controls discriminate | `hee4 release install --control cases=k/k` *(rev 2026-10-01 V11-fix)*, each asserting its own diagnostic; prune policy under scoped `cargo mutants`/plants (AT:132, AT:163) |

## 10 · Open decisions and risks
- Retention K=3 and backup N=7 are PROPOSALs (AT:83, AT:226).
- Engine-state backup timer would need Luke's RC01 amendment (DEC:21 V4-6; H-12).
- UNMEASURED: whether `systemctl --user enable` through `flatpak-spawn --host` passes the classifier (AT:238).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-17 (V4-55): Home of the `hee4 release` code: a K6 Rust verb with no card, or the migrated Python script; DC-40 (V4-62): NF-ROSTER-INSTALL: phase and owner. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=$E/reference/v3-evidence-b5367bc; V=~/herdr-engineering-engine-v4; M=$V/migrated/v3-b5367bc
sed -n '80p;82p;135p;148p;181,182p;271p' $E/design/ULTRAMAP.md
sed -n '38p;41p;53,62p;74p;78,82p;155,156p;177,194p;202,203p;215p' $E/design/DEPLOYMENT_ATLAS.md
sed -n '134p' $E/design/DECISION_POINTS-b5367bc.md
sed -n '27p;35p' $R/architecture-review-b5367bc.md; sed -n '22p' $R/module-audit-b5367bc.md; sed -n '16,17p;34p' $R/review-range-7ed3728-dfb32d5.md
sed -n '21,22p;27p;31p;35p' $V/plan/DECISIONS.md
sed -n '19p' $M/MIGRATION.md; wc -l $M/deploy/install-release $M/deploy/worker.container $M/deploy/*.md $M/deploy/herdr/*
sed -n '162,217p' $M/deploy/install-release
grep -n 'EX-16' $V/docs/EXEMPLARS.md
```
