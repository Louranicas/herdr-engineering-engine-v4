# hee4-host flow

Four doors, no policy. Callers (K2/K4) decide what may run; this crate refuses only what contradicts the caller's own data.

```
admission --mint--> Permit{PermitId, ReceiptId, SpawnScope}
                       |
Command + NamespacePlan --> spawn::plan(&Permit, ..) --> SpawnPlan{argv}   or HostRefusal
                                                           |   (OutOfScope | UnlistedMount | RelativePath | DoorNotSocket)
                                                           v
                                         spawn::run(SpawnPlan) --> SpawnOutcome{exit,stdout,stderr,elapsed}
                                                                   or SpawnError::TimedOut (child killed, reaped)

candidate (no network) --HTTP/1.1 over $HEE4_MODEL_SOCKET--> model_door::serve --TCP--> 127.0.0.1:11434
                                                                  |  503 {"refused":"model unreachable"} if upstream refuses
                                                                  v
                                                     DoorRequest{bytes, sha256(body), fate} per request
```

## spawn
- `Permit::mint(ReceiptId, SpawnScope)` is the only constructor; fields are private. No permit, no plan; the plan carries the receipt id, so the chain can name what actuated. TODO(S6): swap the local newtypes for `hee4_contracts` types and let only admission mint.
- argv: `--unshare-all --unshare-net --die-with-parent --new-session --ro-bind p p ... --bind work work [--bind door door --setenv HEE4_MODEL_SOCKET door] --chdir work -- program args`. There is no network option: the candidate never has a network, and the model door is its only path to the model.
- Writable binds: the work dir and, when `model_door` is set, the door socket. Nothing else can be writable.
- Refused before any process starts: program not in `scope.programs`; a bind, work dir or door not in `listed_mounts`; a relative path; a door that is not a unix socket at plan time (a directory, file or missing path would be a second writable bind) — `DoorNotSocket`.
- `run` polls `try_wait` every 10 ms, kills and reaps at `timeout`. Stdout and stderr drain in threads.
- Gap: the candidate inherits the worker's environment (bwrap does not clear it); no env allowlist yet.
- Gap: no cgroup accounting, no TERM-then-KILL grace, no env allowlist yet (K0h cgroup-io and later phases).

## clock
`Clock::now() -> Duration since epoch`; `SystemClock`; `TestClock::{new,set,advance}`. `spawn::run` still reads `Instant` for its own deadline.

## model_door
`serve(socket_path, Upstream, DoorBudget) -> Door`. `Upstream::parse` accepts only `http://<loopback-ip>:<port>`. One HTTP/1.1 request per connection, each connection on its own thread (pool of 8; the accept loop waits for a slot). The request line must be `GET|POST SP /api/... SP HTTP/1.1`; anything else is refused and never forwarded. Header deadline 2 s, body deadline 10 s. Drops hop headers, sends `Host: <upstream>` and `Connection: close`, relays the response to EOF. Every request is logged as `DoorRequest{bytes, sha256, fate, label, reason}`; a refused row carries the bytes the door actually read and `label: "refused"`. `Door::close` stops accepting, joins the connection threads, removes the socket and returns the log.

Refusal table (JSON body `{"refused":"<why>"}`):
- `400` bad request: bad request line (HTTP/0.9, CONNECT, other method, path outside `/api/`, binary garbage), non-utf-8 header, bad `Content-Length`, early EOF or short body. The reply body is always `bad request`; the log row's `reason` names which.
- `408` header timeout / body timeout: the client did not finish its header in 2 s or its body in 10 s.
- `411` chunked bodies are not forwarded: `Transfer-Encoding` present; send `Content-Length`.
- `413` body over budget (one body over `max_body_bytes`) or byte budget exhausted (sum of bodies over `max_total_bytes`, 8 MiB, for the whole attempt).
- `429` door request budget exhausted: more than `max_requests` forwarded.
- `503` model unreachable: the upstream did not accept or write failed.

(`431` header too large also exists: header block over 64 KiB.) Budget literals (64 requests, 1 MiB per body, 8 MiB total, 2 s, 10 s, 120 s) are K1 `budget` stand-ins, UNMEASURED. Tested against std `TcpListener` mocks; no live daemon contacted.

## model
Used by the dispatcher's availability probe and doctor only; the attempt path makes no in-process model call. `OllamaClient::new(base)`, `.tags()` (`GET /api/tags`), `.generate(model, prompt, timeout)` (`POST /api/generate`, `stream:false`). Errors: `ModelUnreachable | ModelTimeout | ModelMalformed` (non-200 counts as malformed). Tested only against a std `TcpListener` mock on 127.0.0.1; no live daemon was contacted.
