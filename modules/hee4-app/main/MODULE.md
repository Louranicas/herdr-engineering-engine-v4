# hee4-app · main
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 | UM:79 |
| v3 origin | `app` + `src/main.rs` (binary `habitat-engine`) | AR:7; IM:30-32 |
| Binary | `hee4` (V4-11) | UM:82; DEC:31 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › main](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23main): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
- **Purpose:** `main` = argv → `app::serve(Config)`; CLI client connects to the socket, sends one frame, prints (UM:79, UM:94; IM:9).
- **Owned state:** none. Reads `HOME`, `XDG_RUNTIME_DIR` only (IM:66).
- **Allowed deps:** `hee4-app` internals. **Must hold no policy**: `compose_native` (main.rs:459) and the duplicated Dispatcher literal move into app (UM:79).
- **Verbs:** `serve`, `commission <s>`, `<action>`, `restore --into`, `release install|rollback|prune` (UM:148-150; V4-15). *(rev 2026-10-01 ratified V4-55, DC-17)* **Scope:** `main` owns these verbs as K6 Rust code over the `deploy` card's pure policy. The migrated Python `install-release` is reference input only, and nothing runs it.

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P0 (skeleton binary in workspace) | H-5 | gate + cold clone `steps=N matched=N` (AT:79) | H-5 |
| P1 (serve + health through main) | P0 | t28-style binary test (AT:80) | — |
| P4 (`hee4 release install|rollback|prune`) | P3 + Tier-2 | `hee4 release install --control cases=k/k` *(rev 2026-10-01 V11-fix)*; D1 (AT:83) | — |
| P6 (unit `ExecStart=%h/.local/lib/hee4/current/hee4 serve`) | P5 | D2 read-back (AT:59, AT:85) | H-3 |
Feeds D1-D3 (AT:58-60).

## 4 · v3 basis
- **Flag:** app PARTIAL (MA:19). **Recommendation:** REFACTOR — main.rs holds policy (AR:7).

| Finding | v3 file:line | Source |
|---|---|---|
| `compose_native` policy in root | main.rs:459 | UM:79; V2:38 |
| duplicated `Dispatcher` literal | main.rs:867/:880 | AP-08 |
| CLI client connect / print | main.rs:1147,1175 | UM:94; IM:9 |
| serve / commission entry | main.rs:261-267 | IM:30-31 |
| 10 feature-gated test seams (`headroom-seam`) | main.rs | IM:66 |

## 5 · Decision points
None in DP for `main` (DP:77-110 list none). EXACT CLI parsing only.

## 6 · Migrated inputs
None — app REFACTOR (MIG:22). Binary rename `habitat-engine` → `hee4` (V4-11).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-08 | policy in the composition root is the v3 defect here |
| AP-10 | feature-flag seams in the product binary — v4 uses `#[cfg(test)]` only (AT:161) |
| AP-01 | one Dispatcher construction site |
| AP-49 | release install reads back what landed |
| EX-16 | install partial → fsync → rehash → rename |
| A-5 | root/runtime god-file shape |
| D-09 | main assembles; never defines cluster policy |

## 8 · Interfaces
CLI → `$XDG_RUNTIME_DIR/hee4/control.sock` (UM:146); release dirs `~/.local/lib/hee4/releases/<sha>/` + `current` (UM:183); unit `hee4.service` (UM:184).

## 9 · Done criteria
| # | criterion | evidence |
|---|---|---|
| 1 | `main.rs` contains no policy (only argv → Config → serve) | review + `rg compose_native crates/hee4-app/src/main.rs` → 0 |
| 2 | 0 test-seam strings in release | `seam_strings=0/0` with positive control (AT:58, AT:161) |
| 3 | Installed release by digest | `release=… engine=… shim=… seam_strings=0/0 current=ok host_readback=ok` (AT:58) |
| 4 | Running unit = installed bytes | `sha256sum /proc/<MainPID>/exe` = manifest (AT:59) |

## 10 · Open decisions and risks
- H-3 unit enable if classifier refuses (AT:187). Manifest must record linker/CRT/libgcc/link argv (RR build-03/04; AT:83).

## 11 · Pull commands
```bash
rg -n 'main\.rs|hee4 (serve|release|restore)' ~/hee4-evidence/design/ULTRAMAP.md
sed -n 53,56p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
sed -n 27,34p ~/hee4-evidence/reference/v3-evidence-b5367bc/interface-map-b5367bc.md
rg -n 'V4-11|V4-15' ~/herdr-engineering-engine-v4/plan/DECISIONS.md
```
