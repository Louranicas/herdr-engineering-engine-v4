# HEE v4: project instructions (read before any action here)

**Fresh context?** Read `START.md` (one hop: what this is, where every home is on this machine, what to run, what to build first). Then `bash .claude/hooks/context-doctor.sh` (prints present|MISSING per home and tool; fix or route around every MISSING line before trusting a pointer), then `just verify`. The restart pointer `~/handoffs/HEE4_RESTART.md` and `hee4db recipe restart` are the deeper routes. Claude tooling for this repo: `.claude/README.md`; the ported pstack/brainmaxxing craft layer: `.claude/skills/README.md`.

**Meta goal (Luke, 2026-10-04): an environment that makes it hard to write bad code.** Prefer the highest rung for every door — impossible (types/architecture) > refused at admission > caught by a check > caught by review > caught in production. `plan/STACK-MAP-2026-10-04.md` §0.

**This machine (2026-10-04):** Arch/Omarchy, `$HOME=/home/louranicas`, no toolbox. Homes: repo `/mnt/storage-10tb/herdr-engineering-engine-v4`, evidence `/mnt/storage-10tb/hee4-evidence`, handoffs `/mnt/storage-10tb/handoffs`, vault `/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault`; `~/herdr-engineering-engine-v4`, `~/hee4-evidence`, `~/handoffs` are symlinks to them. Tooling reads `hee4.env`. The `/var/home/Louranicas` and `/var/mnt/STORAGE-10TB` paths in older rows are the previous Fedora layout.

## The fence (Luke 2026-10-01)
- **Work only in v4 homes:** this repo, `~/hee4-evidence`, and the vault `herdr-engineering-engine-v4.vault`.
- **Never work in, build, gate, deploy or edit HEE-v3:**
  - `/var/home/herdr-engineering-engine-v3` (from the toolbox: `/run/host/var/home/…`);
  - `~/.cache/hee3-worktrees`;
  - `~/.local/lib/herdr-engineering-engine-v3`;
  - `~/hee3-evidence`;
  - on this machine also: `/mnt/storage-10tb/herdr-engineering-engine-v3-t3-backups`, `/mnt/storage-10tb/HEE-v3-SECOND-COPY`, `/mnt/storage-10tb/hee3-backup`, `/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v3.vault`.
- **v3 is frozen and historical.** Reference it only through the read-only copies in `~/hee4-evidence/reference/v3-evidence-b5367bc/` and the staged `migrated/v3-b5367bc/`.
- **At session start:** `habitat-scope set --session "$HABITAT_SCOPE_SESSION" --charter "HEE v4" ~/herdr-engineering-engine-v4 ~/hee4-evidence "$HEE4_VAULT"` warned on writes outside the fence on the Fedora host. On this machine `habitat-scope` is not installed (its backup copy needs a `runtime` module that was not backed up); the fence is enforced by the `.claude/settings.json` deny rules and `hee4-v3-guard.sh` instead.
- **No v4 file or process may depend on a v3 path.** (Decision V4-9. The future check prints `v3_refs=0` over the tree, excluding `migrated/` and `.git/`.)

## The HOLD
No code until Luke says **"start coding"** (V4-0). Planning, maps, reviews and staged verbatim copies are allowed.

## Before touching a module
Read its card `modules/<crate>/<module>/MODULE.md`, plus the AP-/EX- entries it names in `docs/ANTIPATTERNS.md` and `docs/EXEMPLARS.md`, and `docs/DRIFT_AND_OVERENGINEERING.md`.
Run `just verify` before and after (one verdict over every check; `just` lists the other recipes).
Project commands: `/verify` `/restart` `/highway` `/module` `/regen` `/hee4-status` `/lessons`; skills `hee4-module-slice`, `hee4-brief`, `hee4-lessons`; map and proofs in `.claude/README.md`.

## Order of authority
`CHARTER.md` → `plan/DECISIONS.md` (append-only) → `gates/REQUIREMENTS.md` → the design files in `~/hee4-evidence/design/`. One topic, one home: the vault links here and never copies. Commit only when asked; push only on Luke's word.
