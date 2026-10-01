# hee4-app · native-provider
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 | UM:65 |
| v3 origin | `app` → `src/app/native_provider.rs` (compose, install/open of the native provider config and roster rows) | DP:97 |
| Status | PLANNING — HOLD | DEC:4 |
| Config | `~/.config/hee4/native.toml` (v3 `native/native.toml`) | UM:182; IM:72 |

**Design section:** [K6 hee4-app › native-provider](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23native-provider): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | Content | Source |
|---|---|---|
| Purpose | Compose the native (ollama) provider from `native.toml`: curl client path, endpoint, profile; install/open the provider's roster record | DP:97; UM:169 |
| Owned state | None durable; roster rows persist in K1 via app | UM:75, UM:203 |
| Moves out | Roster install becomes a K1 internal operation kind `Owner::Deploy`, **not** the wire id `roster.update` | UM:135 |
| Moves out | `ProviderState` (`native.rs:508`) moves to K0 | UM:216 |
| Dropped | native.toml `[daemon] unit/scope`, `native::Systemd` (RT S1 N6d retirement) | UM:251; RT:43 |
| Allowed deps | K0 (config/state types), K0h (curl client pin), K1 (roster op), K2 (native execute) — K6 is the only crate allowed several clusters | UM:65 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3: native provider **resolves the daemon by the LISTEN holder of 127.0.0.1:11434**, no unit/MainPID walk; `Generated` decoder proven against a recorded live reply | P2 | gate; live-reply fixture committed from a recording (F113) | none (loopback ruled not-held by HO-04) |
| P4/P7: first host record; first real GPU task (`size_vram > 0`, pinned daemon exe `12ff8654…`) | P3 / P6 | host records F06/F07 (P4/P5 exit); D6 reads the P7 record F06u through the unit *(rev 2026-10-01 V11-fix)* | H-9, H-16 |
Feeds D6 (AT:63). Sources: AT:82, AT:86, AT:116-117, AT:137, AT:193, AT:201.

## 4 · v3 basis
- **Flag:** `app` PARTIAL (MA:19). **Recommendation:** `app` REFACTOR (AR:7).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Installer records roster revisions under the unserved `roster.update`, pre-populating that action's idempotency space (HIGH) | `native_provider.rs:273` | AR:35; UM:135; AP-12 | — |
| "curl pin at :505" is a `#[cfg(test)]` fixture `HOST` const; production curl path comes from `native.toml` | `native_provider.rs:491-505` | IM:39,59 | **E9** (ER:19) |
| compose / install / open | `:160`, `:315`, `:433` | DP:97 | — |
| Daemon is a grandchild under conmon, not a descendant of the unit's MainPID | — | AT:116; AT:137 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `compose :160` · EXACT (DP:97) | — |
| `install/open :315/:433` · EXACT (DP:97) | — |
No JUDGMENT sites in DP for this module.

## 6 · Migrated inputs
None — `app` is REFACTOR (MIG:22). Paths re-spelt under `hee4` (V4-11, DEC:31).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-12 | the roster install under a wire id is the v3 HIGH; v4 uses `Owner::Deploy` (UM:135) |
| AP-49 | install must read the roster row back; a curl/daemon identity is re-checked, not assumed |
| AP-21 / AP-18 | the reply fixture must be a recording, not hand-typed (a hand fixture missed `context`, AT:138) |
| AP-06 | endpoint resolution: pure listener/holder policy over values, I/O thin (F95) |
| AP-01 | curl timeout 180/5 s literal → one K0 constant (UM:169) |
| EX-12 / EX-13 | pure listener/holder + bounded acquisition; `take(N+1)` then refuse (EXX:26-27) |
| EX-19 | one door for a digest (pinned exe) |
| D-09 | only K6 composes worker + store; no cluster crate composes another |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| HTTP loopback (via K0h curl) | `127.0.0.1:11434/api/` GET version/tags/ps, POST generate | UM:169; IM:38-42 |
| identity | LISTEN holder from `/proc/net/tcp{,6}` + fd scan | IM:42; UM:170 |
| store (via K1) | roster install as `Owner::Deploy` operation | UM:135, UM:197 |
| config | `~/.config/hee4/native.toml` | UM:182 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | No roster write under `roster.update`; installer rows appear under `Owner::Deploy`. *(rev 2026-10-01 ratified V4-62, DC-40, reading (a))* At P3 this is only the native-provider's **own** roster record, composed as a K1 internal operation. The general installer (deploy writing roster rows) is v4.1 (T, E2E-12) | ledger read-back of `operations` rows; test asserting the owner value (UM:197) |
| 2 | Production curl path comes only from `native.toml`; no src pin literal | grep census `duplicate_sites=0` (UM:39) |
| 3 | Daemon resolved by LISTEN holder; refuses by name on digest ≠ pin or unreadable co-holder | host record line; HP-row (AT:230; RR security-1) |
| 4 | Decoder proven against a committed recording | fixture provenance line in the slice (AT:82, AT:138) |
| 5 | Plants on install owner and holder refusal killed by named tests under `--cap-lints=warn`; scoped mutants with `CARGO_TARGET_DIR` unset | `plants=k/k killers=named`; `mutants caught= survivors=` (AT:149) |

## 10 · Open decisions and risks
- Endpoint resolvable after a toolbox container rebuild? UNMEASURED(P3) (AT:240).
- H-16 permanent ollama topology (toolbox vs host-native) (AT:201). H-9 loopback-as-not-network for v4 (AT:193).
- RR security-1/-2 (holder unreadable co-holder; unbounded readlink bytes) land in K0h/K2 but surface here (RR:23-24).

## 11 · Pull commands
```bash
V4=~/herdr-engineering-engine-v4; EV=~/hee4-evidence; R=$EV/reference/v3-evidence-b5367bc
rg -n 'native_provider|native.toml|roster.update|Owner::Deploy|11434' $EV/design/*.md $R/*.md $EV/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '75p;135p;167,168p;214p;249p' $EV/design/ULTRAMAP.md
sed -n '58p;77p;111,112p;132,133p;185p;192p;217p' $EV/design/DEPLOYMENT_ATLAS.md
sed -n '35p' $R/architecture-review-b5367bc.md; sed -n '19p' $EV/reference/ERRATA-v3-evidence-b5367bc.md
rg -n '^\| \*\*AP-(01|06|12|18|21|49)\*\*' $V4/docs/ANTIPATTERNS.md
sed -n '263,283p' $V4/docs/EXEMPLARS.md
```
