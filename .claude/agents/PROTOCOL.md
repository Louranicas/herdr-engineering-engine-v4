# Agent communication and collaboration protocol (HEE v4 roster)

Drafted 2026-10-05. Binding on every `hee4-*` agent. Where this protocol and an agent's own file disagree, this file wins; where it and `CHARTER.md`/`plan/DECISIONS.md` disagree, those win.

## 1 · Every message is a claim with its evidence
A report, a hand-back, a ledger row, a brain note: each sentence that asserts a fact carries one of **MEASURED** (a command ran; its output is quoted or its path given), **INFERRED** (reasoned from measured facts; the facts are named), **UNMEASURED** (not known; never written as zero). A coordinator or refuter drops any report missing a label, a `head_sha`, or the witness command — and respawns the sender once, fresh. A dropped report is not a pass (swarm rule: "a gap is never a pass").

## 2 · The brief is the only way work starts
No agent acts on a chat sentence. The coordinator writes the brief (I1, eleven fields: GOAL, SCOPE, CONTEXT, ACCEPTANCE, VERIFY, TIMEBOX, FORBIDDEN, REPORT, STANDING, RECON, RESTATEMENT). The receiving agent's first output is its RESTATEMENT in its own words; a RESTATEMENT that conflicts with ACCEPTANCE is refused back to the coordinator before any work (narrative principle 13). STANDING orders (`agents/standing-orders.md`) are pasted verbatim into every brief, never summarised.

**RECON through Poteto Weave, when it is fresh.** Poteto Weave
(`/mnt/storage-10tb/hee4-evidence/prototypes/turso-tool-context/assimilation-20261005/`) is the
habitat's generation-pinned, read-only context layer over HEE v4, pstack, deep-diff-forge,
LoomLattice and Firstmate (`context.search`, `context.read`, `hee4.catalogue`, `hee4.inspect`). A
coordinator or builder first runs its `snapshots.py ... status`; when it prints `"fresh":true,
"qualified":true`, cross-codebase lookups go through it and the brief or report cites the
generation id beside each fact it supplied. When it refuses (`stale_source`,
`catalogue_contract_changed`, or not fresh), the report records `poteto-weave=UNMEASURED(<refusal>)`
and reads the sources directly. A retrieval is context, never a verdict: Poteto Weave's own
contract says a successful plan is not an HEE result. It is used read-only; its runtime root
and active pointer belong to its owner, and a captain builds any generation of its own in a
separate root under `~/.cache`.

## 3 · Fresh, bounded, counted
- Every spawn is a fresh agent with consolidated scope; resuming is allowed only to answer a refuter's question about the agent's own prior output.
- `planned_agents=N` is written to the ledger **before** any fan-out; a fan-out beyond N is a STOP.
- Each brief carries a command budget and a TIMEBOX; an agent at 70% of either stops spawning and reports what remains UNMEASURED.
- Build output (every `CARGO_TARGET_DIR`, export or work tree) goes under `~/.cache/hee4-*`, never /tmp or a session scratchpad: /tmp is a RAM tmpfs (`tools/doctor` row `tmp_usage`, brain `tmp-is-ram-keep-builds-in-cache`).
- Nested fan-out (an agent spawning agents) requires the coordinator's line in the ledger; each nested layer re-pays orientation, so the default is flat.

## 4 · One writer per artefact
| Artefact | Single writer | Everyone else |
|---|---|---|
| `$FM_HOME/data/firstmate.db` (units, briefs, spawns, claims, verifications, receipts, exits, andon) — the Firstmate home's one orchestration DB, Luke 2026-10-05 | the first mate, through `ops/firstmate/fm-db record <kind>` only (flock, BEGIN IMMEDIATE); crew append claims through the same verb, never raw SQL | `fm-db q 'SELECT …'` / `fm-db status` (tursodb read-only) |
| `agents/ledger.tsv` | superseded by `firstmate.db`; kept as the offline fallback when no Firstmate home exists | read |
| `agents/standing-orders.md` | Luke (proposals via `hee4-scribe`) | read, paste verbatim |
| `brain/*.md` | `hee4-scribe` (and Luke) | propose in reports |
| `plan/DECISIONS.md` | Luke (append-only) | propose rows in reports |
| cards, feature files, maps | the facet's builder, one facet per change | propose design conflicts (DC-nn) |
| the subject under review | nobody during review | — |
| `hee4db record claim` / `record verify` | claimant / a different agent | the claimant cannot verify its own claim |

## 5 · Typed exit, read by machines
The last non-empty line of every report is the verdict, in the roster runner's form: `<agent without hee4-> verdict=(PASS|PASS_WITH_GAPS|FAIL|BLOCKED|STOP) cases=k/n [reason=…] [head=<sha12>]`. `BLOCKED` names what blocks (an H-row, a missing input, a grant). `STOP` is the andon: the coordinator halts all dispatch in the unit and reports to Luke; only a watcher or refuter may raise it, and the reason must be MEASURED.

## 6 · Channels
- **In-session:** `SendMessage` to a named agent for questions that need its context; a reply is still a labelled claim.
- **Across sessions:** files only — the ledger, reports under `$HEE4_EVIDENCE/reviews/` and `$HEE4_EVIDENCE/roster/`, `brain/`, `hee4db record claim|verify`. No agent relies on another agent's memory.
- **To Luke:** the coordinator's report, one per unit, with the open asks in a list headed `Luke:`; held items go to ATLAS §5 by proposal only.
- **Outward:** nothing. No agent sends v4 text to Jev or any external service (H-10a); `hee4-watch-fence` measures it.

## 7 · Disagreement
Two agents disagreeing on a fact run the witness command again, together, and record which measurement stands; disagreeing on a design write a DC-nn row and stop — the authority order (`CLAUDE.md`) decides, not the louder report. A refuter's refutation outranks the executor's green until re-measured.

## 8 · What a watcher may do
Read, measure, report, raise STOP. Never edit, never fix, never spawn a builder. A watcher that finds the same class twice proposes the next rung up (`/correct` ladder) in its report, with the past instance named so the new door can be proven against it.
