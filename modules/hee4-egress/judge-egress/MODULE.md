# hee4-egress · judge-egress
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub. **Doubly held:** no call leaves the machine without Luke's grant (H-8, H-10, H-11, H-12).

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-egress` | UM:73; DEC:32 (V4-12) |
| Cluster | K0e | UM:58, UM:73 |
| v3 origin | **none (new)**; designed in JM §4.2 (placed "K2 worker" there, superseded) | JM:59-65; UM:58 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K0e hee4-egress › judge-egress](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0e%20hee4-egress%23judge-egress): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | **The only network egress**: the Jev/TypeSafe client, `POST api.typesafe.ai/v1/systemone` | UM:73, UM:175 |
| Owned state | None durable (ledgering is K1 `judgments`) | UM:73, UM:207 |
| Allowed deps | K0 only | UM:64 |
| Dependants | **K6 only** — so no cluster can call out without K6's `DataClass` check | UM:58, UM:64, UM:66 |
| Contract | model pinned `jev-1.13.0` (never `jev-latest`); request/response bounded (32k state; 255 choices; 2-10 levels); caller's deadline passed through; 429/529 backoff inside it; typed decode; API key in a 0600 custody file; egress only from `serve`; candidate namespace stays network-less | JM:59-65; UM:73 |
| Supersedes | D-U4 (egress in K0h) — V2 showed K2/K4/K5 → K0h could reach it | DEC:15, DEC:32; UM:295 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P0 | H-5 | crate skeleton; planted `use hee4_egress` outside K6 fails E0432/E0433, asserted on the diagnostic | H-5 — AT:79, UM:84 |
| P9 | tag pushed + grant | Jev ports in shadow mode, calling out only through K0e from K6 | **H-8, H-10, H-11, H-12** — AT:88, AT:192-197 |
No D-row (v4.0 ships with egress dark; RC01 external request cap 0, AT:119).

## 4 · v3 basis
- **Flag / recommendation:** n/a (new). v3 engine had no network egress: "Network egress from `serve` (none today)" (JM:90); IM lists no non-loopback call (IM:36-43).
- The habitat-side precedent (`jev-boundary`) leaked report text through the Stop verifier — suspected, Luke's ruling (JQ:30, F-B): the engine door must be a type, not a path check (JM:69).

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| None in DP (new). Egress serves the grant-dispositioned JUDGMENT sites | J1, J2, J3, J7, J8, J9, J10, J12, J13 — grant (JM:44) |

## 6 · Migrated inputs
None (new; MIG lists no egress module).

## 7 · Quality guard
| id | why here |
|---|---|
| AP-01 | one egress door; D-U4 shows two candidate homes existed |
| AP-04 | request/response bounded at acquisition (32k state) |
| AP-16 | advice never authority; outage ⇒ `UNMEASURED` |
| AP-49 | read back the model id in each response (pinned `jev-1.13.0`) |
| AP-50 | the grant is a charter-time permission; preflight at minute 1 |
| EX-01, EX-19 | bounded reader; one door |
| D-09 | the compiler (not a check) keeps K1-K5 off this crate |

## 8 · Interfaces
Outbound: TypeSafe `POST api.typesafe.ai/v1/systemone` — **HELD** (UM:175). Inbound: called only by K6 after its `DataClass` check against `~/.config/hee4/judge/` grant record (UM:133, UM:182).

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | Only `hee4-app` depends on `hee4-egress` | planted control in every other crate fails with E0432/E0433 naming the path (UM:84) |
| 2 | Refuses to send with RC01 cap 0 or with no grant record | named refusal; control case |
| 3 | Deadline passed through; no created time limit | test with a caller deadline at 3 values (off origin) |
| 4 | Response decode bounded; oversize refused by name with both numbers | fixture from a recorded response (F113), not hand-typed |
| 5 | API key file mode checked (0600) before read | refusal on 0644 fixture |

## 10 · Open decisions and risks
- All activation is Luke's (JM:86-91; H-8/H-10/H-11/H-12). Build only the P0 skeleton + compiler control before then (D-01 brake).
- ~~HTTP client choice (reuse pinned curl via `spawn`, or a Rust client): undecided; spawning curl would make K0e depend on K0h, which UM:64 does not allow — decide at P9.~~ Decided (*rev 2026-10-01 ratified V4-66, DC-11*): K0e owns its own minimal in-process HTTPS client, the only lawful option under UM §2. The P9 slice measures its dependency closure against the Anti-Bloat budget, and an overrun reopens this. RC01's request cap is charged at K1 budget, the one charge door; K0e refuses only on an absent grant (DC-12). `ask` takes `Cleared`, and K0e's `clear` is its only constructor (DC-13).

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 58p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 64,66p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 73p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 84p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 173p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 59,69p ~/hee4-evidence/reference/v3-evidence-b5367bc/jev-decision-map-b5367bc.md; sed -n 86,91p ~/hee4-evidence/reference/v3-evidence-b5367bc/jev-decision-map-b5367bc.md
rg -n 'V4-12|D-U4' plan/DECISIONS.md
sed -n 184,188p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
```
