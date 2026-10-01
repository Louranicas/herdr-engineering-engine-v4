# hee4-app · control-socket
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 | UM:79 |
| v3 origin | `app` → `src/app/control_socket.rs` | IM:7-10; UM:95-101 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › control-socket](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23control-socket): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
- **Purpose:** the engine's only listener: custody flock, bind 0700/0600, accept loop, SO_PEERCRED peer admission → `Principal`, per-principal rate bucket, connection cap, one frame per request (UM:95-101; IM:7-10).
- **Owned state:** runtime files only: `$XDG_RUNTIME_DIR/hee4/control.sock` 0600 in 0700 dir + `control.lock` flock (UM:146; V4-11 DEC:31). No durable state.
- **Allowed deps:** any (K6). Limits read from K0 (P5, UM:99).
- **v4 change:** `serve_connection` gains a **`Reply::Stream` arm** (loop writes N frames until close/cursor end) for `events.subscribe` (UM:101; V4-14 DEC:34).

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1 (serve + socket + health through main; RA10 fuzz from day one) | P0 green | t28-style binary test; fuzz plant killed under `--cap-lints=warn` (AT:80) | — |
| P5 (`Reply::Stream`) | P4 | events.subscribe gate test (AT:84) | — |
| P6 (RA10 security lens incl. socket) | P5 + P2 backup/restore | lens report 0 HIGH open; D3 read-back (AT:85) | H-6 (lane X lens), H-15 (socket not ingress, AT:200) |
Feeds D3 (AT:60).

## 4 · v3 basis
- **Flag:** app PARTIAL; serve/drain through main (MA:19). **Recommendation:** app REFACTOR overall (AR:7), but socket custody is **REUSE verbatim** (UM:95, UM:280).

| Finding | v3 file:line | Source |
|---|---|---|
| bind + custody flock | control_socket.rs:246,299 | UM:95 |
| custody code (verbatim reuse) | control_socket.rs:50-59,146,242,303 | UM:146 |
| idle 60 s, write 10 s | control_socket.rs:57-59 | IM:7 |
| peer uid / admit peer | :314, :326,:708 | UM:97-98 |
| rate bucket / connection cap | :355,:375 / :537,:751 | UM:99-100 |
| `Reply` is {Close, Frame}; streaming not expressible | actions/control.rs:66 | AR:34; migrated src/actions/control.rs:66 |
| v3 leftover `control.lock` in `habitat-engine/` — collision avoided by `hee4/` | — | AT:38; UM:146 |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| admit_peer :326 · EXACT · authority (DP:98) | not-Jev |
| Admission::admit :375 · THRESHOLD · 100/s, burst 32, cap 8 (DP:99) | not-Jev |

## 6 · Migrated inputs
None — control_socket.rs is app (REFACTOR) and not staged (MIG:22). `Reply` type lives in migrated `src/actions/control.rs:66` (actions card). Namespace dir renamed `hee4/` (V4-11).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-04 | frame read must be bounded at acquisition (≤ 1 MiB) |
| AP-01 | rate/cap/frame limits spelled once in K0 (UM:39) |
| AP-31 | stream loop and drain need budgets |
| AP-49 | read back socket mode and holder (D3) |
| EX-01 | bounded frame reader — the acquisition is the bound |
| EX-15 | `Principal` private fields + matcher for admission |
| D-06 | runtime namespace `hee4` is a fence against v3 leftovers |

## 8 · Interfaces
- Serves: all control-v1 actions via `actions::control::serve_composed` (UM:102); `Reply::Frame` or `Reply::Stream` (UM:126).
- Files: `$XDG_RUNTIME_DIR/hee4/control.sock`, `control.lock` (UM:146). No TCP listener (UM:147).

## 9 · Done criteria
| # | criterion | evidence |
|---|---|---|
| 1 | Socket 0600 in 0700, held by MainPID | `stat -c '%a %U' $XDG_RUNTIME_DIR/hee4/control.sock`; `/proc/<MainPID>/fd` (AT:60) |
| 2 | `hee4 health` → `ready=true recovery=complete database=ready socket=owned` | whole health line quoted (AT:60) |
| 3 | RA10 fuzz: seeded, budgeted; plant killed by named test | `--cap-lints=warn` battery line (AT:80; RT:46) |
| 4 | Stream arm delivers N frames then closes | events.subscribe gate test (AT:84) |
| 5 | second `serve` refused by custody flock | test through binary |
| 6 | Mutation | scoped mutants on admission/rate policy, `CARGO_TARGET_DIR` unset (AT:132) |

## 10 · Open decisions and risks
- Lane X cross-lineage lens (H-6) upgrades, never blocks the tag (AT:190).
- ~~Risk: stream arm reopens idle/write timeouts semantics — decide per-frame vs per-stream budget at P5 flow contract.~~ Settled (*rev 2026-10-01 ratified V4-65, DC-31*), one home in Workflow and Loop Map § RL-5. Each frame write has 10 s. A park ends at the publish wake, at shutdown, at the reused v3 60 s idle budget or at cursor expiry. `queue_limit` is 256 frames / 8 MiB. There is no whole-stream ceiling and **no per-principal stream sub-cap in v4.0**: the socket is 0600, so it has one principal. The trigger for a sub-cap is a second principal uid admitted by `admit_peer`.

## 11 · Pull commands
```bash
sed -n 90,114p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 141,149p ~/hee4-evidence/design/ULTRAMAP.md
rg -n 'control_socket' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md ~/hee4-evidence/reference/v3-evidence-b5367bc/interface-map-b5367bc.md
sed -n 60,80p ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/actions/control.rs   # Reply enum
sed -n '/### EX-01/,/### EX-02/p' ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
```
