# analysis.request

A-20. Request a bounded numerical analysis of a dataset by a pinned recipe and runtime: a julia child (S-7) under the K0h spawn door with wall, memory and output limits, recorded as an operation and an analysis row with its own state. Scope **v4.2**, phase P9 (production needs O-15 and H-14), owner `Owner::Numerical`, effect `BoundedAnalysis` (**mutating**). No D-row.

## Sub-features

- bounded-analysis: `limits{wall_ms, memory_bytes, output_bytes ≤ 65,536}`; julia offline, 1 thread, `JULIA_*` allowlist, depot pinned.
- analysis-row: `{analysis_id, task_id, attempt_id, generation, dataset_sha256, state}` with `state ∈ {running, validated, failed, cancelled, unknown}` (`queued` dropped unless something writes it, API Map P-4).
- recipe-and-runtime-pins: `recipe_id`, `recipe_version`, `runtime_id`.
- dataset-digest: `dataset_sha256` of the input at `cutoff_unix_ms`.
- idempotent-replay: readback `analysis.get {selector:{source_action, idempotency_key}}`.
- production-gate: `unavailable` until O-15 / H-14 grant production use, even at v4.2.
- v4.0-refusal: `unavailable` by name.

## How to get to it (user POV)

`hee4 analysis.request` (binary); wrapper "generated"; Pi `hee4_analysis_request` (PROPOSAL). At v4.0, `unavailable` with `because` = v4.2; at v4.2 without the production grant, `unavailable` with a `because` naming O-15/H-14 (Error map class 10: distinct `because` strings, one site each).

## Driving it with hee4

Preconditions (v4.2): a `BoundedAnalysis` grant; a subject task/attempt; a dataset object; the julia depot pinned (Socket map S-7 "depot not pinned → refusal"); production grant for a non-gate run.

```bash
K=$(uuidgen)
hee4 analysis.request                                                                                                                   # v4.0: unavailable by name
hee4-sh analysis.request 'subject:={…}' 'dataset:={…}' cutoff_unix_ms=<ms> recipe_id=<r> recipe_version:=1 runtime_id=<rt> 'limits:={"wall_ms":…,"memory_bytes":…,"output_bytes":65536}' @idempotency_key=$K   # v4.2
hee4-sh analysis.get 'selector:={"source_action":"analysis.request","idempotency_key":"'$K'"}'                                        # readback
```

Socket: request `body` `{subject, dataset, cutoff_unix_ms, recipe_id, recipe_version, runtime_id, limits}`; result `body` `{analysis_id, task_id, attempt_id, generation, dataset_sha256, state}` (API Map A-20). `UNWRITTEN: the subject and dataset shapes, the K1 table that owns the analysis row (API Map P-4: no UM §5a table holds it), and the generated wrapper spelling.`

- v4.0 path: the `unavailable` refusal. Driven by `tools/drive` through `tools/drive.d/scoped.py`: the scope is read from `tools.inspect` (path `catalogued`), and the action, sent with a placeholder for each `Socket:` request member, must be refused `unavailable` at `/action` with exactly that scope's `because` from `Scope::because` (path `refused_by_scope`); the line is `verdict=PASS paths=2/2 scope=v4.2 (refused by release scope, as catalogued)`. Until `tools.inspect` carries `scope`, the line stays UNMEASURED `scope=unserved` naming the missing member.
- v4.2 success: the reply's `state` is `running` (or `validated` if synchronous: `UNWRITTEN: whether the request blocks on the julia child or returns before it settles`), then `analysis.get` reaches `validated` with its decoded output; `operations` has one row; the julia child's `ProcessReport` is settled.
- Error: `output_bytes` outside 1..65,536 or wall/memory outside bounds → `invalid_argument` naming the bound; depot not pinned → refusal by name; output over the bound → refusal with both numbers (UM-P8); without production grant → `unavailable` (O-15/H-14).
- Persistence: replay returns the stored row; `UNWRITTEN: whether a running analysis survives a serve restart as running or is reconciled to unknown (no recovery rule names analyses).`
- Cancel: `UNWRITTEN: no cancel action exists for an analysis; the state enum has cancelled but nothing in the 22 writes it.`
- Side effects to read: the analysis row (once its table is named), `operations`, objects under `objects/sha256/` for the decoded output.

## Gotchas

- The durable home of the analysis row is unnamed (API Map P-4); until the v4.2 slice names a K1 table, the side-effect check for this file cannot be written.
- Two `unavailable` meanings at v4.2 (scope vs production grant) must have distinct `because` strings; assert the string.
- v3 had test callers only for julia (Socket map S-7); nothing here is REUSE and S-7 is UNMEASURED before P9.
- Must not: K4 never spawns on its own; the only spawn is the K0h door (UM-P13), and K4 imports neither K1 nor K6 (card check D-09, the same rule for `numerical`).
