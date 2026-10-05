# analysis.get

A-21. Read one analysis: its state, dataset digest, decoded output (once validated) or error code, selected by id or by the request key. Scope **v4.2**, phase P9, owner `Owner::Numerical`, effect `Read`. Readback action for `analysis.request`. No D-row.

## Sub-features

- analysis-detail: `{analysis_id, …, state, dataset_sha256, report, error_code}` (field names as the API Map prints them).
- select-by-key: `selector{source_action:"analysis.request", idempotency_key}`.
- state-progression: `running → validated | failed | unknown` (and `cancelled`, whose writer is unnamed).
- v4.0-refusal: `unavailable` by name.

## How to get to it (user POV)

`hee4 analysis.get` (binary); wrapper "generated"; Pi `hee4_analysis_get` (PROPOSAL). At v4.0, `unavailable`. From v4.2 a caller polls it after `analysis.request` until `validated` or `failed`.

## Driving it with hee4

Preconditions (v4.2): a `Read` grant; an analysis id or request key.

```bash
hee4 analysis.get                                                                                   # v4.0: unavailable by name
hee4-sh analysis.get 'selector:={"analysis_id":"<id>"}'                                             # v4.2 (selector names UNWRITTEN)
hee4-sh analysis.get 'selector:={"source_action":"analysis.request","idempotency_key":"<uuid>"}'
```

Socket: request `body` `{selector: AnalysisSelectorV1}` (FACT required); result `body` `{analysis_id, …, state, dataset_sha256, report, error_code}` (API Map A-21). `UNWRITTEN: the AnalysisSelectorV1 field names, the decoded-output field's shape (decoded by K4 julia-decoders from one schema), the error_code domain, and the generated wrapper spelling.`

- v4.0 path: the `unavailable` refusal. Driven by `tools/drive` through `tools/drive.d/scoped.py`: the scope is read from `tools.inspect` (path `catalogued`), and the action, sent with a placeholder for each `Socket:` request member, must be refused `unavailable` at `/action` with exactly that scope's `because` from `Scope::because` (path `refused_by_scope`); the line is `verdict=PASS paths=2/2 scope=v4.2 (refused by release scope, as catalogued)`. Until `tools.inspect` carries `scope`, the line stays UNMEASURED `scope=unserved` naming the missing member.
- v4.2 success: a validated analysis returns its decoded output and `error_code=null`; a failed one returns `error_code` from the closed decoder set.
- Error: unknown id → `not_found`.
- Empty: a running analysis returns a null output field; still a `result`.
- Persistence: none written; the row it reads lives in the table API Map P-4 has yet to name.
- Side effect: none.

## Gotchas

- The reply's `…` in the API Map is literal: the full field list is not in any authority. Do not invent it in a test; pin only the fields named.
- The decoded output comes from julia records through a closed code set with one schema generating both sides (K4 julia-decoders); a decode failure is `failed` with an `error_code`, never a partial output.
- No D-row; v4.2 slice evidence only.
