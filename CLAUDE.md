# HEE v4: project instructions (read before any action here)

**Fresh context?** Read `~/handoffs/HEE4_RESTART.md` first, then run `hee4db recipe restart` and `just verify`. Claude tooling for this repo: `.claude/README.md`.

## The fence (Luke 2026-10-01)
- **Work only in v4 homes:** this repo, `~/hee4-evidence`, and the vault `herdr-engineering-engine-v4.vault`.
- **Never work in, build, gate, deploy or edit HEE-v3:**
  - `/var/home/herdr-engineering-engine-v3` (from the toolbox: `/run/host/var/home/…`);
  - `~/.cache/hee3-worktrees`;
  - `~/.local/lib/herdr-engineering-engine-v3`;
  - `~/hee3-evidence`.
- **v3 is frozen and historical.** Reference it only through the read-only copies in `~/hee4-evidence/reference/v3-evidence-b5367bc/` and the staged `migrated/v3-b5367bc/`.
- **At session start:** `habitat-scope set --session "$HABITAT_SCOPE_SESSION" --charter "HEE v4" ~/herdr-engineering-engine-v4 ~/hee4-evidence /var/mnt/STORAGE-10TB/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault`. It warns on writes outside the fence.
- **No v4 file or process may depend on a v3 path.** (Decision V4-9. The future check prints `v3_refs=0` over the tree, excluding `migrated/` and `.git/`.)

## The HOLD
No code until Luke says **"start coding"** (V4-0). Planning, maps, reviews and staged verbatim copies are allowed.

## Before touching a module
Read its card `modules/<crate>/<module>/MODULE.md`, plus the AP-/EX- entries it names in `docs/ANTIPATTERNS.md` and `docs/EXEMPLARS.md`, and `docs/DRIFT_AND_OVERENGINEERING.md`.
Run `just verify` before and after (one verdict over every check; `just` lists the other recipes).
Project commands: `/verify` `/restart` `/highway` `/module` `/regen` `/hee4-status` `/lessons`; skills `hee4-module-slice`, `hee4-brief`, `hee4-lessons`; map and proofs in `.claude/README.md`.

## Order of authority
`CHARTER.md` → `plan/DECISIONS.md` (append-only) → `gates/REQUIREMENTS.md` → the design files in `~/hee4-evidence/design/`. One topic, one home: the vault links here and never copies. Commit only when asked; push only on Luke's word.
