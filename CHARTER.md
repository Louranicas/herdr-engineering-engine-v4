# HEE v4 charter (drafted 2026-10-01 by louranicas-2c; Luke owns every item marked ⚑)

## 1 · Purpose
A deployed, used engine on this machine. It admits a task through the control socket, dispatches it to the live local model inside an isolated candidate namespace, verifies it with one verdict authority, settles it durably with recovery, and survives a crash, a restart and a restore. v4 rebuilds on the v3 findings: 11 of 23 modules REFACTOR, 11 HARDEN, 1 KEEP (`~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md`). It carries the mechanisms that were missing (`~/hee4-evidence/learnings/PROCESS-LEARNINGS.md`).

## 2 · The HOLD ⚑
No code (Rust, Python, shell under `src/` or crates, build files) until Luke says **"start coding"**. Planning, maps, reviews and staged verbatim copies are allowed.

## 3 · Scope
- **In scope:**
  - the 9-crate layout (ULTRAMAP §2; V4-12, superseding D-U3) *(rev 2026-10-01, V4-12)*;
  - the release-scope actions (ULTRAMAP §4);
  - the deployment phases (DEPLOYMENT_ATLAS);
  - Jev as an advisory port designed but held (`jev-decision-map-b5367bc.md`).
- **Out of scope until decided:** live Jev engine calls ⚑ (grant, DPA/ZDR, cost mode); off-machine custody ⚑; DG1-b/d/e ⚑. These are scope boundaries, not a second held-for-Luke list: each is an H-list row (H-8, H-10a/H-10b, H-11, H-15) *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F07)*.

## 4 · Done-line
"Deployed" as defined in `~/hee4-evidence/design/DEPLOYMENT_ATLAS.md` §1. Each criterion is a read-back command, not a claim. A version cut is an annotated tag; its message fields are listed once, at ATLAS §1 row D10 *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F21)*.

## 5 · How work is done (from PROCESS-LEARNINGS §2-§3; adopted as intent, enforced once code exists)
The ten requirements in `gates/REQUIREMENTS.md`: a derived tiered gate with one runner; no publisher or generated blocks; a fan-out kernel; skeleton-first slices (≤ 2 design rounds); hermetic provisioning; snapshot-subject gates; one plant/mutation harness; leases plus a charter-time permission manifest; one door per rule; state as a query plus an append-only decision log.

## 6 · Agent roster ⚑
Unattended Sonnet 5.5 agents, one runner (`ops/roster/run-agent.sh`). `hee4-curator` keeps the corpus equal to measured state; `hee4-workflow-curator` records workflows and loops and proposes optimisations. Each escalates judgment to Opus or Fable, writes only its own vault blocks and reports, and exits on its typed verdict. Schedule: `ops/roster/README.md` (installation status in `plan/DECISIONS.md`).

## 7 · Authority ⚑
Every item held for Luke has one home: the H-list in `~/hee4-evidence/design/DEPLOYMENT_ATLAS.md` §5 — the H-list (ATLAS §5), with no range quoted here so it cannot go stale *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F08)*. This charter lists none itself *(rev 2026-10-01, V4-16)*. Decisions within this charter are recorded in `plan/DECISIONS.md` (decision · evidence · reversible? · re-measure trigger).
