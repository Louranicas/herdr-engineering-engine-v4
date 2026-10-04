# START — HEE v4 in one hop

Read this first. Everything else is one link away. State is never asserted here; the commands print it.

## What this is
A local engineering engine for this machine. It admits a task through a control socket, dispatches it to the local model inside an isolated namespace, verifies it with **one** verdict authority (`decide`), settles it durably with recovery, and survives a crash, a restart and a restore. Planning only until Luke says **"start coding"** (`CHARTER.md` §2).

**Meta goal (Luke, 2026-10-04): an environment that makes it hard to write bad code.** Every door is ranked — impossible (types) > refused at admission > caught by a check > caught by review > caught in production — and the highest rung wins. `plan/STACK-MAP-2026-10-04.md` §0.

## Where things are on this machine
| Home | Path |
|---|---|
| This repo | `/mnt/storage-10tb/herdr-engineering-engine-v4` (also `~/herdr-engineering-engine-v4`) |
| Design + evidence | `/mnt/storage-10tb/hee4-evidence` (also `~/hee4-evidence`): `design/ULTRAMAP.md`, `design/DEPLOYMENT_ATLAS.md` (§5 = the held-for-Luke list) |
| Vault (navigation, module design, system maps) | `/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault` |
| Restart pointer | `/mnt/storage-10tb/handoffs/HEE4_RESTART.md` (also `~/handoffs/`) |
| Build memory | `brain/` (injected at session start; `/brain-reflect` writes to it) |
| Env for all tooling | `hee4.env` at the repo root (`HEE4_ROOT`, `HEE4_EVIDENCE`, `HEE4_VAULT`, `HEE4_HANDOFFS`, `HEE4_BRAIN`) |

## First five minutes
```bash
bash .claude/hooks/context-doctor.sh   # what you can reach; fix or route around every MISSING line first
just verify                            # every check, one verdict line; expect verify verdict=PASS steps=N/N
hee4db highway <module>                # everything about one module in one call
```
Then read, in order: `CHARTER.md` → `plan/STACK-MAP-2026-10-04.md` → `plan/INTEGRATION-MAP-2026-10-04.md` → the card `modules/<crate>/<module>/MODULE.md` you are working on → its feature file `gates/features/<action>.md`.

## What to build first (after "start coding")
The walking skeleton, 6 crates, 6–8 slices: `task.submit` → 3-table SQLite store written only through `transition` → synchronous dispatcher to the local model through the spawn door → bwrap candidate → `decide` → `kill -9` and restart drill. `plan/STACK-MAP-2026-10-04.md` §4. Nothing else lands before one task has gone end to end.

## How to work here
- Route the task with `/poteto-mode`; copy the playbook's steps verbatim as the todolist; a skipped step stays as `skip: <reason>`.
- Every claim carries its label: MEASURED, INFERRED or UNMEASURED.
- Before changing a behaviour, open its feature file; the change and the feature file land together.
- A repeated mistake goes **up** a rung via `/correct` (architecture → types → a lint that names the fix → test → docs), proven to fail on the real past instance.
- Commit only when asked; push only on Luke's word; nothing to Jev (`CLAUDE.md`).

## Glossary of prefixes
AP-nn anti-pattern (`docs/ANTIPATTERNS.md`) · EX-nn exemplar (`docs/EXEMPLARS.md`) · D-nn drift control (`docs/DRIFT_AND_OVERENGINEERING.md`) · V4-nn decision (`plan/DECISIONS.md`) · H-nn held for Luke (ATLAS §5) · DC-nn design conflict (vault Module Design Index) · Pn deployment phase (ATLAS §2) · Dn done-line criterion (ATLAS §1) · Kn crate cluster (ULTRAMAP §2) · RL-n runtime loop · DW-n dev workflow · E2E-nn flow trace (vault `16 System Maps`) · R01–R14 recovery policy · S-n socket · A-nn action (API Map) · F-n refusal family (Error map) · L-nn process lesson (`PROCESS-LEARNINGS.md`).
