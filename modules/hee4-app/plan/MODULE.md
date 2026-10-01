# hee4-app · plan
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 | UM:65 |
| v3 origin | `app` → `src/app/plan.rs` (candidate build plan: pinned toolchain, compiler version, compile/check; candidate receipt) | DP:103; RR:17 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › plan](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23plan): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | Content | Source |
|---|---|---|
| Purpose | Plan and describe the candidate build: pinned toolchain, compiler version, compile argv; record the receipt | DP:103; RR:17 |
| Build target | the U64 class builds for **musl + rust-lld**, self-contained, pins from `~/.rustup` (the host has no `cc`) | AT:82; AT:112; AT:127 |
| Owned state | None | UM:79 |
| Allowed deps | K0 (receipt types), K0h (spawn door for rustc) via K6 | UM:65 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3 musl + rust-lld class build | P2 | gate | none |
| P4 manifest records **linker, CRT, libgcc, link argv** | P3 + Tier-2 | `hee4 release install --control` *(rev 2026-10-01 V11-fix)*; host record | none |
Feeds D1 (RA7 obligations: linker, CRT, libgcc, link argv) — AT:58. Sources: AT:82-83, AT:127.

## 4 · v3 basis
- **Flag:** `app` PARTIAL (MA:19). **Recommendation:** `app` REFACTOR; `runtime` imports `plan` (cycle) (AR:7; EXX:544).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| receipt records compile argv and rustc exe, **not** link argv (rust-lld, link-self-contained) or toolchain files (build-04) | `plan.rs:437` | RR:17 | — |
| RA7 "cc versions and flags" gap → HO-01 PASS_WITH_GAPS | — | RT:26 | — |
| `plan.rs` spawns (rustc, mkfifo) are test-only | after `#[cfg(test)]` at `plan.rs:975` | IM:63 | — |
| pinned / compiler_version / check | `:362`, `:722`, `:862` | DP:103 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `pinned :362 · compiler_version :722 · check :862` · EXACT (DP:103) | — |
None JUDGMENT.

## 6 · Migrated inputs
None — `app` is REFACTOR (MIG:22).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-49 | the receipt must record measured link inputs, not what the script intended (RR build-04) |
| AP-27 | the toolchain-file set is enumerated from the world (what the link read), not a hand list |
| AP-10 | v3 test-only spawns stay `#[cfg(test)]` or leave (IM:63) |
| AP-08 | break `runtime → plan` import cycle (AR:7) |
| AP-06 | toolchain location is an input (sysroot path), not an env lookup inside the decision |
| EX-16 | re-hash what landed |
| EX-19 | one digest door for toolchain pins |
| D-09 | build policy stays in K6; no K2 → K6 edge |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| process (via K0h) | rustc / rust-lld from `~/.rustup` | AT:111 |
| outputs | candidate receipt; manifest fields for D1 | AT:58 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Receipt/manifest records linker, CRT, libgcc and link argv | D1 read-back line (AT:58); test asserting each field over two builds |
| 2 | Candidate builds on the host without `cc` (musl + rust-lld) | host probe rc=0 recorded (AT:111, AT:127) |
| 3 | No production spawn path outside the K0h door | review + grep census |
| 4 | Plants on a dropped link-argv field killed by a named test; scoped mutants, `CARGO_TARGET_DIR` unset | `plants=k/k killers=named` (AT:149) |

## 10 · Open decisions and risks
- A toolbox-built engine vs host glibc: re-measured at P4; a glibc mismatch is a stop (AT:128).
- Where plan ends and `u64-class` begins (both touch the class build) — decide at P3 flow contract (INTERP).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-39 (V4-61): `Class` port: home and seam shape. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
V4=~/herdr-engineering-engine-v4; EV=~/hee4-evidence; R=$EV/reference/v3-evidence-b5367bc
rg -n 'plan\.rs|link argv|rust-lld|musl' $EV/design/*.md $R/*.md
sed -n '53p;77,78p;106,107p;122,123p' $EV/design/DEPLOYMENT_ATLAS.md
sed -n '17p' $R/review-range-7ed3728-dfb32d5.md; sed -n '26p' $R/route-to-v010-b5367bc.md; sed -n 63p $R/interface-map-b5367bc.md
rg -n '^\| \*\*AP-(06|08|10|27|49)\*\*' $V4/docs/ANTIPATTERNS.md
```
