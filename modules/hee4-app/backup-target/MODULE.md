# hee4-app · backup-target
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 | UM:79 |
| v3 origin | `app` → `src/app/backup_target.rs` | DP:95; IM:72 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › backup-target](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23backup-target): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
- **Purpose:** decide whether a backup destination is admissible: `device_decision` pure over (mount table, topology) — separate physical disk, not tmpfs (DP:95; EX-11; AT:62, AT:130).
- **Owned state:** reads `~/.config/hee4/backup.json` (UM:182); no durable state.
- **Allowed deps:** any (K6). Decision must stay pure (P7 F95, UM:41).
- **v4 policy:** backup prune as a pure policy over a listing with a dry-run; never removes the only verified backup (AT:81, AT:226).

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P2 (backup/2, prune policy, dry-run) | P1 | gate + restore drill (AT:81) | — |
| P6 (security lens over backup target; real destination `/var/mnt/STORAGE-10TB/hee4-backups/`) | P5 + P2 | lens 0 HIGH (AT:85) | H-4, H-15 (off-site) |
| P7 (prune under 128 GiB rehearsed) | P6 | rehearsal record (AT:86) | — |
Feeds D5 (AT:62), D7 (AT:64).

## 4 · v3 basis
- **Flag:** app PARTIAL (backup gate through main, MA:19). **Recommendation:** app REFACTOR (AR:7); this function is an exemplar (EX-11).

| Finding | v3 file:line | Source |
|---|---|---|
| Device rule pure over values | backup_target.rs:719-778 | EX-11 |
| config paths | backup_target.rs:43,45 | IM:72 |
| test seams `HEE3_TEST_MOUNTINFO`, `HEE3_TEST_BLOCKTOPO` behind feature `headroom-seam` | main.rs gates | IM:66 |
| prune "by hand" in v3; no retention | — | AT:39, AT:226 |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| device_decision :731 · EXACT · pure over mountinfo and topology (DP:95) | not-Jev |

## 6 · Migrated inputs
None — app REFACTOR (MIG:22). Imitate EX-11.

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-06 | the policy must be provable by arguments, not by mounting a disk |
| AP-10 | v3 used feature-gated env seams; v4 seams `#[cfg(test)]` only (AT:161) |
| AP-29 | print the device facts the decision looked at |
| EX-11 | the pattern itself |
| EX-03 | fault injection absent from release |
| D-16 | D5 needs the drill, not just the decision |

## 8 · Interfaces
Reads `/proc/self/mountinfo` + block topology (via K0h bounded readers); `~/.config/hee4/backup.json` (UM:182). Consumed by dispatcher's `backup_due` path.

## 9 · Done criteria
| # | criterion | evidence |
|---|---|---|
| 1 | Refuses same-disk and tmpfs destinations by name | table test over recorded mountinfo fixtures (recordings, F113 AT:138) |
| 2 | Prune dry-run digest required for `--apply`; only verified backup never pruned | test prints `keep=K prune=P bytes=B` (AT:225-226) |
| 3 | 0 `HEE4_TEST_` strings in release binary | D1/D2 `seam_strings=0/0` (AT:58-59) |
| 4 | Mutation | scoped mutants on device_decision, `CARGO_TARGET_DIR` unset (AT:132) |

## 10 · Open decisions and risks
- Prune N=7 days is a PROPOSAL (AT:226); this is the "N days" retention the roster card once carried (V5 LOW, moved here). Off-site copy none — H-15.
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-22 (V4-58): three. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
sed -n '/### EX-11/,/### EX-12/p' ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
rg -n 'backup' ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
rg -n 'backup_target' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md ~/hee4-evidence/reference/v3-evidence-b5367bc/interface-map-b5367bc.md
```
