# hee4-host flow

Three doors, no policy. Callers (K2/K4) decide what may run; this crate refuses only what contradicts the caller's own data.

```
admission --mint--> Permit{PermitId, ReceiptId, SpawnScope}
                       |
Command + NamespacePlan --> spawn::plan(&Permit, ..) --> SpawnPlan{argv}   or HostRefusal
                                                           |   (OutOfScope | UnlistedMount | RelativePath)
                                                           v
                                         spawn::run(SpawnPlan) --> SpawnOutcome{exit,stdout,stderr,elapsed}
                                                                   or SpawnError::TimedOut (child killed, reaped)
```

## spawn
- `Permit::mint(ReceiptId, SpawnScope)` is the only constructor; fields are private. No permit, no plan; the plan carries the receipt id, so the chain can name what actuated. TODO(S6): swap the local newtypes for `hee4_contracts` types and let only admission mint.
- argv: `--unshare-all --die-with-parent --new-session [--share-net if allow_loopback] --ro-bind p p ... --bind work work --chdir work -- program args`.
- Refused before any process starts: program not in `scope.programs`; a bind or work dir not in `listed_mounts`; a relative path.
- `run` polls `try_wait` every 10 ms, kills and reaps at `timeout`. Stdout and stderr drain in threads.
- Gap: `--share-net` shares the host network, not loopback only. A loopback-only netns needs a shim (K2). UNMEASURED.
- Gap: no cgroup accounting, no TERM-then-KILL grace, no env allowlist yet (K0h cgroup-io and later phases).

## clock
`Clock::now() -> Duration since epoch`; `SystemClock`; `TestClock::{new,set,advance}`. `spawn::run` still reads `Instant` for its own deadline.

## model
`OllamaClient::new(base)`, `.tags()` (`GET /api/tags`), `.generate(model, prompt, timeout)` (`POST /api/generate`, `stream:false`). Errors: `ModelUnreachable | ModelTimeout | ModelMalformed` (non-200 counts as malformed). Tested only against a std `TcpListener` mock on 127.0.0.1; no live daemon was contacted.
