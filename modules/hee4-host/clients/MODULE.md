# hee4-host · clients
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-host` | UM:72 |
| Cluster | K0h | UM:72 |
| v3 origin | pinned-binary clients inside `worker` (native curl exchange, endpoint holder scan), `worker::aggregate`/`resources` (busctl, systemd-run), `worker::namespace` (bwrap), bounded readers | UM:72; IM:36-63 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K0h hee4-host › clients](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0h%20hee4-host%23clients): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Pinned-binary clients + bounded readers: curl (**loopback ollama only**), busctl, systemd-run, bwrap; endpoint-holder I/O (`/proc/net/tcp{,6}` + fd scan); **pins binary digests** | UM:72, UM:169-173 |
| Owned state | None durable | UM:72 |
| Allowed deps | K0 | UM:62, UM:72 |
| Not here | network egress off-machine (K0e only, V4-12); native provider policy (profile/daemon/transport/ollama) stays K2 | UM:73, UM:75; DEC:32 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P3 | P2 | endpoint resolved by the LISTEN holder of 127.0.0.1:11434 (no unit/MainPID walk); `Generated` decoder proven against a **recorded** live reply | loopback ruled not-held by HO-04 in v3; v4 re-affirmation **H-9** — AT:82, AT:193 |
| P4 | P3 + Tier-2 | host record; HP-rows re-read host pins before every host run | H-9 — AT:83, AT:136 |
| P7 | P6 | first GPU task: `/api/ps` `size_vram > 0`, daemon exe = pinned `12ff8654…` | H-16 (topology, only if changed) — AT:63, AT:201 |
Feeds D6 (AT:63) and HP-rows (AT:230).

## 4 · v3 basis
- **Flag:** worker FINISHED (MA:7) — unverified by any run (ER §2). **Recommendation:** worker REFACTOR — native.rs mixes ~7 concerns; curl 180/5 s literal (AR:10).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| ollama calls: base `native.rs:413`; GET version/tags/ps `:1364,1374,1425`; POST generate `:1328-1331` | as cited | IM:39-41 | — |
| Curl pin cited at `native_provider.rs:505` is a `#[cfg(test)]` fixture; production curl path comes from `native.toml` | `native_provider.rs:491-505` | UM:169 | **E9** |
| busctl StartTransientUnit/StopUnit/get-property; digest pin | `aggregate.rs:16-18,206,337-349`; `class_profile.rs:166` | IM:48-51; UM:171 | — |
| systemd-run scopes | `resources.rs:135` | IM:57; UM:172 | — |
| holder() accepts one readable holder while unreadable co-holders exist | `native.rs:956` | RR security-1 | — |
| fd readlink strings unbounded in bytes (count bounded only) | `native.rs:1074` | RR security-2; UM:42 | — |
| Pure listener/holder policy + one bounded acquisition | `native.rs:921-1085`, `:1026-1040` | EX-12, EX-13 | — |
| Tool digests differ host vs toolbox (busctl `1466a64c…`/`85d8f9f0…` etc.) | — | AT:114, AT:136 | — |
| Daemon is not a descendant of the ollama unit's MainPID | — | AT:116, AT:137 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| worker/native `exchange timeout :1300` · THRESHOLD · 180 s / 5 s literal (DP:45) | not-Jev |
| worker/native `clean :1266 · catalogue :1368 · resident :1430 · response :1606 · usage :1613 · done_reason :1618 · identity :1626 · cancel :1563` · EXACT · readbacks and bounds (DP:46) | not-Jev |
| worker/native `select_daemon :855 · listener :936 · holder :967 · hash_file :686 · daemon/resolve :758,:1145` · EXACT · identity, pid-reuse guard (DP:47) | not-Jev |
| worker/aggregate `unit_reply :589 · … · stop_empty :432` · EXACT (DP:54, **E16**) | not-Jev |

## 6 · Migrated inputs
None — worker is REFACTOR (MIG:22). Consumers in the migrated set: `src/numerical/process.rs` (julia spawn/hash; MIG:14). The v3 digest pins in `deploy/install-release` are install-time, not these runtime pins (UM:183).

## 7 · Quality guard
| id | why here |
|---|---|
| AP-04 | fd readlink bytes: the derived set takes its own bound (RR security-2) |
| AP-05 | endorse the bound at the caller (exchange truncates before parse, N6c) |
| AP-06 | holder/listener policy pure over values; I/O thin |
| AP-49 | read back identity (exe digest, `/api/ps`) after each call |
| AP-21 | reply fixtures only from recordings (AT:138) |
| AP-01 | one pin per binary per side; curl timeout from `bounds` |
| EX-12, EX-13, EX-19 | pure holder + bounded acquisition; `take(N+1)`; one digest door |
| D-09 | K0e, not K0h, owns off-machine egress |

## 8 · Interfaces
Uses (loopback/local only): `http://127.0.0.1:11434/api/` via pinned `/usr/bin/curl --noproxy` (AT:117); D-Bus user manager via `/usr/bin/busctl` (IM:45-51); `/usr/bin/systemd-run`; `/usr/bin/bwrap` + `hee4-namespace-shim` (UM:173). Dropped: systemd Manager unit/scope for the native daemon (UM:176).

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | Every client binary is invoked through `spawn` with a digest pin checked before use | refusal by name on a digest mismatch; HP-row read-back on the host (AT:136) |
| 2 | Holder scan byte-bounded and refuses an unreadable co-holder | named tests for RR security-1/-2; plants killed under `--cap-lints=warn` |
| 3 | The no-walk branch of resolve is exercised (v3 RR tests-1 gap) | test whose double returns `walks_descendants()==false` |
| 4 | `Generated` decoder fixture is a committed recording | fixture provenance line (AT:82) |
| 5 | curl never addresses a non-loopback host | type: host is a loopback-only newtype; compile-fail test |
| 6 | Scoped mutants on the holder policy, CARGO_TARGET_DIR unset | `survivors=` named (AT:132) |

## 10 · Open decisions and risks
- ~~Split line between K0h endpoint I/O and K2 `native/endpoint` policy: UM:170 puts holder identity in K0h, UM:75 lists `native/endpoint` in K2 — decide at P3 flow contract.~~ Settled (*rev 2026-10-01 ratified V4-60, DC-10*): the raw `/proc` holder scan (S-9) is K0h's, and the policy over its values is K2 `native/endpoint`.
- Endpoint survival across a toolbox rebuild: UNMEASURED → P3 (AT:240).
- H-9 loopback/TH-DEV re-affirmation for v4 (AT:193).

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 164,174p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 36,63p ~/hee4-evidence/reference/v3-evidence-b5367bc/interface-map-b5367bc.md
rg -n 'security-1|security-2|tests-1 ' ~/hee4-evidence/reference/v3-evidence-b5367bc/review-range-7ed3728-dfb32d5.md
rg -n '^\| E9 ' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
sed -n 45,47p ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 109,112p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md; sed -n 131,133p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
rg -n '### (EX-12|EX-13|EX-19)' docs/EXEMPLARS.md
```
