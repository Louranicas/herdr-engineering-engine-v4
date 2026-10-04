# hee4-worker flow

K2. Route by floor, plan the namespace, drive one attempt. Never builds `TaskState` or `Verdict`, never writes the ledger. Uses `hee4-contracts` and `hee4-host` types; defines none of theirs.

```
RouteInput + Roster --route::select--> Selection{model, rule}  or RouteRefusal
NamespaceTask --namespace::plan_for--> NamespacePlan (hee4-host)
Attempt::run(permit, plan, OllamaClient, brief, playbook) --> AttemptOutcome{steps, stdout, stderr, exit, elapsed, observations, retries}
```

## route
`Route::default().rules` is the evaluation order, printed by `Route::order()`: `R13 R02 R03 R04 R05 R06 R07 R08 R09 R10 R11 R12`. R13 runs first as in v3 (the failed attempt is screened before any filter); the brief's `[R02..R13]` is the set, not the order. INFERRED: rule names and semantics come from the migrated v3 `Rule` enum (`migrated/v3-b5367bc/src/route.rs:335-`), R01 folded into `baseline_or_refuse`.

| Rule | Does | Refusal |
|---|---|---|
| R13 PreviousAttempt | drop the model whose attempt failed | `OnlyPreviousAttempt` if it empties the pool |
| R02 RequiredCapabilities | JSON mode, tool use | `NoCapableModel{floor, best}` |
| R03 ContextLimit | `ctx_tokens >= floor` | `NoCapableModel` |
| R04 PrivacyClass | local-only admits local | `NoCapableModel` |
| R05 Availability | drop Down and Unknown; Unknown sets the gap | none |
| R06 CostCeiling | `cost_milli <= ceiling` | none |
| R07 Deadline | `latency_ms <= deadline` | none |
| R08 QualityFloor | `quality >= floor` | none |
| R09 InsufficientEvidence | gap routes to baseline | `NoBaseline{rule}` |
| R10 NoEligibleCandidate | empty pool routes to baseline | `NoBaseline{rule}` |
| R11 Ranking | quality desc, latency, cost; strict best wins | none |
| R12 Tie | equal top keys route to baseline | `NoBaseline{rule}` |

A baseline is only the task's own declared model, and must meet the floor and not be Down. There is no default model: an unmet floor refuses, naming the closest model and what it lacks (`Missing::{Context, JsonMode, ToolUse, Local}`). Cost, latency and quality are declared figures, UNMEASURED (J5).

## namespace
`NamespaceTask::new(task_id, work_root, needs_model, timeout)` refuses a relative root or one inside a read-only path. `plan_for`: ro-binds `/usr /lib /lib64 /bin` (+ `/etc/alternatives` if present), one writable bind `<work_root>/<task_id>`, `allow_loopback = needs_model`. `work_root_from_env()` reads `HEE4_WORK`. Gap: host `--share-net` shares the whole host network, not loopback only (see hee4-host FLOW).

## native
Steps run in order. A step is `Done`, `Failed`, or `Skipped{reason}`; skipped steps stay in `steps`. Skips: step kind with no handler, a model step when the plan has no loopback, every step after a failure, and every step when `HEE4_LIVE_MODEL` is not `1` in `run_live` (prints `UNMEASURED`). Each model step yields one `Observation` (`advisory: false`, `input_sha256 = digest(prompt)`, tool `ollama` / model name, outcome Pass or Error). Model calls are made by this process; run steps go through `spawn::plan` and `spawn::run` in bwrap. Retry: `ModelUnreachable` only, `max_attempts = 3`, jittered exponential backoff (half to full), printed `attempt=k/n backoff_ms=`. Literals 3/200/2000 are stand-ins for K1 `budget` (UNMEASURED, DC proposal). Tested only against 127.0.0.1 std `TcpListener` mocks; no live daemon contacted.
