# Bash integration

Thin command entry points over actions the engine already admits. A wrapper, and nothing
more: it builds a request, hands it to the producer, and reports the producer's verdict
unchanged. It schedules nothing, duplicates no policy, retries nothing on its own authority,
and grants no action — an action the wrapper can name is one the engine already admits, and
one it cannot name simply does not run.

```
hee3 <action> [argument ...]       invoke one admitted action
hee3 chain <spec.json | ->         run a declared sequence of actions, in order
hee3 --check <action> [...]        print the request that would be sent; send nothing
hee3 --actions                     list the actions this wrapper can name
hee3 --inspect <action>            that action's prerequisites and the bounds in force
hee3 --version                     wrapper version, control protocol and producer diagnostics
hee3 --pin                         the digest of this wrapper and its catalogue
```

`HEE3_PRODUCER` names the producer executable; `HEE3_GRANT_ID` and `HEE3_SCOPE_SHA256` name the
authority every request is made under, and a request without them is refused by name. `HEE3_CATALOGUE` overrides where the action
vocabulary is read from; the default is `schemas/actions/control-v1.schema.json`, so an
action added to the engine is nameable here without anyone editing a list.

**Dependency/version mismatch.** The wrapper speaks `hee3.control/1`. A catalogue with any
`Request_*` definition whose `protocol` or `version` const says otherwise (including a version
of `true`, which Python would compare equal to 1) is refused with exit 3 and
`dependency/version mismatch: Request_<x> speaks "<p>"/<v>; this wrapper speaks "hee3.control"/1`
at every door — invoke, `--check`, `--actions`, `--inspect`, `--version` and so every chain
step. A catalogue with no `Request_*` definition at all is the same refusal
(`the catalogue declares no Request_* definition`): nothing was compared, and a comparison
that did not happen is not agreement. An unreadable catalogue is exit 3 at every door too, `--version` included (it once
printed `catalogue_actions: 0` and exited 0). A version query of the producer itself belongs to
the engine (T28) and is not made here.

**Environment.** The producer runs under `env -i` with only `PATH`, `HOME`, `XDG_RUNTIME_DIR`,
`LC_ALL`, `HEE3_CHAIN_STEP` and `HEE3_TIMEOUT_MS` of the caller's environment — what the engine
reads (`XDG_RUNTIME_DIR` for its socket, `HOME` for `serve`), what makes it runnable with stable
output, and what a chain hands its step. A name the caller did not set stays unset. The grant,
scope and catalogue path have done their work in the request by then. A producer path
containing `=` is refused (exit 3), because `env` would read it as an assignment, print its
environment and exit 0 without running anything.

## Requests

Every invocation sends one control-v1 request — the `Request_<action>` definition in the same
catalogue the action list comes from (review N1: the first version sent
`{protocol, version, action, arguments}`, which no definition accepts). The schema supplies
`protocol`, `version`, `kind` and `action_version`, and decides whether the action requires or
forbids an idempotency key or a precondition; the wrapper adds a fresh `request_id`, a
`deadline_unix_ms` of now plus `HEE3_TIMEOUT_MS` (default 30 s, never more than 60 s ahead) and
the authority. The caller supplies the rest:

| Argument | Becomes |
|---|---|
| `name=value` | body field `name`, this literal text |
| `name:=JSON` | body field `name`, this JSON value — parsed strictly: a duplicated key or `NaN` is refused |
| `@idempotency_key=UUID` | the envelope key; required by every effectful action, refused by name when missing |
| `@precondition:=JSON` | the envelope precondition; refused where the action takes none |

```
hee3 --check task.get 'selector:={"task_id":"123e4567-e89b-42d3-a456-000000000007"}' evidence=summary
```

The body is not validated here — that needs a schema engine this stdlib-only builder does not
have, and the engine validates every request it receives. `tests/bash_wrapper.py::Envelope`
feeds each of the 21 hand-authored requests in `tests/fixtures/native/control-v1/actions.json`
back through `--check` and requires the schema's definition, with body, authority, key and
precondition unchanged.

## Three rules, and why each is a mechanism

**No eval, and no re-parsing of untrusted strings.** Values travel in arrays from `argv` to
the producer, and the request is encoded by a JSON writer rather than built by
concatenation. A value containing a space, a quote, a backslash, a newline, `$(...)`,
backticks, `;`, `*`, braces, `${...}` or `$VAR` is a value, as text and inside a `:=` JSON
value alike. The cases assert on what *arrived* at the producer, and that its argv was exactly
the action and nothing more — a wrapper that dropped, split, expanded or executed a value, or
passed an extra argument, would pass a smoke test and fail them.

**A missing producer is a failure.** Unset, absent or non-executable each exit 3 with a
diagnostic. Printing nothing and exiting 0 because the wrapped thing was absent is the worst
outcome available, so it is not reachable.

**The producer's exit status survives every pipe.** This is the rule the file exists for, and
it is where the first version was wrong.

## The bug the tests found

A pipeline reports its **last** element's status. `producer | head -c N` is therefore green
whenever `head` succeeds — which is always — so a producer that printed a partial result and
exited 7 looked like a success.

The first version of `invoke` got this wrong twice over:

```bash
output=$(printf '%s' "$request" | "$bin" "$action" | head -c "$MAX") || true
local -a stages=("${PIPESTATUS[@]}")
```

`$(...)` is a subshell, so the parent's `PIPESTATUS` describes the *assignment*, not the
pipeline. And `|| true` makes `true` the last command, which resets `PIPESTATUS` to `(0)`.
Ten of the fifty-two cases went red, including `a failing producer fails the wrapper through
the pipe` — the one written for exactly this.

The pipeline then ran in this shell, with its output going to a file, and `stages[1]` was
read before anything else touched `PIPESTATUS`. That left one hole (review 7.3, confirmed by a
case that went red): bash runs a trap only after a *foreground* command ends, so a `TERM` sent
to the wrapper's pid alone — not its group — never reached the producer, which ran on until its
own timeout. The producer now runs in the background, writing into a FIFO that `head` bounds,
and the wrapper `wait`s on the producer's own pid; its status is `wait`'s, never a pipeline's
last element. A cancelling signal interrupts the `wait`, the trap forwards `TERM` to the
producer (an asynchronous command ignores `INT`), waits for it, and exits 128+n. That wait has
no budget of its own: a producer that ignores `TERM` is ended by whoever supervises the
wrapper, as the chain does with `KILL` on the step's group. Every producer code from 1 to 127
is passed through unchanged, a producer killed by a signal is not a success, and a producer
that overruns the output bound dies of `SIGPIPE` rather than being quietly truncated into one.

A second, smaller edit came from the same direction: `output=$(cat file)` strips trailing
newlines, so a producer emitting `{}\n\n\n` had one silently removed. The wrapper now `cat`s
the file and adds a final newline only when the producer did not end with one. A wrapper that
edits its producer's bytes is not reporting its producer's result.

## Chains

`hee3 chain <spec.json | ->` runs a declared, ordered list of actions. Each step is one
ordinary invocation of this same file, so every rule above — bounds, admission, literal
encoding, `PIPESTATUS` — reaches a chained step through the same code, not a copy of it.

```json
{"protocol": "hee3.chain", "version": 1, "timeout_ms": 20000,
 "steps": [
   {"id": "find", "action": "task.list",
    "arguments": {"states": ["running"], "task_class": null, "parent_task_id": null,
                  "page": {"limit": 10, "cursor": null}},
    "output": "json", "provides": ["task_id"]},
   {"id": "stop", "action": "task.cancel",
    "arguments": {"reason": "operator_request",
                  "@idempotency_key": "123e4567-e89b-42d3-a456-000000000008",
                  "@precondition": {"resource": "task", "generation": "1",
                                    "id": "123e4567-e89b-42d3-a456-000000000007"}},
    "inputs": {"note": {"step": "find", "field": "task_id"}}, "timeout_ms": 5000}]}
```

**Declared contracts, checked twice.** Before the first step runs, the whole spec is
validated — unknown keys, duplicate keys, a boolean where a budget belongs, an input that reads
a later step or a field its source does not declare in `provides` — and every step is put
through `hee3 --check`, the same door `invoke` uses. A refusal at step 3 therefore cannot
arrive after step 1 has had its effect. After each step, its declared output is checked
against what it actually produced: a `json` step must print one JSON document, and every
field it `provides` must be present and a string. Coercing `5` to `"5"` would be the wrapper
deciding what the producer meant, so it refuses instead.

**Fields and targets may be JSON Pointers (RFC 6901).** Engine results nest under `body`, and
typed request members nest too, so a `provides` entry or an input name that begins with `/` is
a pointer: `"provides": ["/body/task/task_id"]` reads that string out of the step's result, and
`"inputs": {"/selector/task_id": {"step": "submit", "field": "/body/task/task_id"}}` writes it
into the next request's `selector` object. `~1` is `/` and `~0` is `~`, unescaped in that order;
an array index is decimal without a leading zero, and `-` or an index past the end names
nothing, which is the same contract failure as an absent field, named by its pointer. Targets
under one member are sent together as one `member:=JSON` argument; a target whose member is also
a literal argument, or two targets where one lies inside the other, are refused before any step
runs. A name without a leading `/` is the top-level member it names, as before.

**The admitted tuple is pinned.** Each step is a fresh process that reads the catalogue and
the wrapper again, so once every check has passed the chain takes `hee3 --pin` — the SHA-256 of
the catalogue and of the wrapper — and hands it to every step as `HEE3_PIN_SHA256`. The request
door refuses a step whose files no longer match (exit 3, `the catalogue or the wrapper changed
since the chain was checked`), so no step runs under a tuple the checks did not admit.
Two residuals, stated rather than closed: a file changed between a step's pin check and its
read of that file is not seen; and the check is made by the wrapper file it pins, so a
wrapper replaced by one without the check is not refused by it. The runner does not re-digest
before spawning a step; closing both needs the runner to run the bytes it verified.

**Inputs stay literal.** A step's input is a named string field of an earlier step's output,
passed on as one `name=value` argument — never spliced into a command, and bounded by the same
8 KiB rule when it is sent. An input fills a top-level body field only; it cannot reach inside an
object such as `task.get`'s `selector` (a stated limit, not an oversight). A literal argument may
be any JSON value and travels as `name:=JSON`; an argument name ending in `:` is refused, since
it would reach the request door as the typed form of another name.

**Ordered results.** One JSON record per step, printed as each finishes, then a `summary`:

```
{"kind": "step", "index": 1, "step": "get", "action": "task.get", "status": 0, "outcome": "ok", "stdout": "..."}
{"kind": "summary", "outcome": "failed", "status": 7, "failed_step": "get", "ran": ["find", "get"], "not_run": ["close"], "detail": "producer exited 7"}
```

The first step that does not succeed ends the chain. The steps after it are **named** in
`not_run`, rather than left for a reader to infer from their absence.

**Timeout and cancellation.** `timeout_ms` bounds the whole chain; a step may declare a
smaller budget. The budget in force is handed to the step as `HEE3_TIMEOUT_MS`. Each step
runs in its own process group, so a timeout or a `SIGINT`/`SIGTERM`/`SIGHUP` to the chain
stops the producer *and anything it started*. The group gets `TERM`, then `KILL` after a
2-second grace, and a step that still will not settle is marked `"settled": false` rather
than waited on forever. A signal outranks what the step did after it: a step that died of the
`TERM` the chain forwarded is reported as cancelled, not as a failing producer.

| Outcome | Exit | Meaning |
|---|---|---|
| `ok` | 0 | every step succeeded and met its declared output |
| `failed` | the step's own status | a producer failed; its code is passed through unchanged — `7` when the engine answered with a typed error record (the record is the step's `stdout`) |
| `timeout` | 5 | a step or the chain outran its budget (`limit` says which) |
| `contract` | 6 | a step succeeded but did not produce what it declared |
| `cancelled` | 128+n | the chain received signal n |

An exit code alone cannot tell a producer's own `5` from a chain timeout. The `summary`
record is the decisive verdict; the code is a convenience.

`hee3 --check <action> [argument ...]` prints the request that `invoke` would send and sends
nothing, and needs no producer.

## Bounds

| Bound | Value | On overrun |
|---|---|---|
| arguments | 64 | exit 4 |
| bytes per argument | 8192 | exit 4 |
| producer stdout | 1 MiB | bounded in the stream; the producer's own status is reported |
| chain steps | 16 | exit 2, before any step runs |
| chain spec | 64 KiB | exit 2; read one byte past the bound, never the whole input first |
| chain budget | 1 ms – 1 h | exit 2 when out of range; exit 5 when spent |

Each bound is checked from both sides: 64 arguments must be accepted and 65 refused, or only
the refusing half was ever tested.

Exit codes: `0` producer succeeded · `2` usage · `3` missing producer or catalogue, or a
dependency/version mismatch · `4` bounds ·
`5` chain timeout · `6` chain output contract · `128+n` cancelled by signal n ·
otherwise the producer's own code, unchanged.

## Scope

`shellcheck` reports nothing on the wrapper (measured 2026-09-24, before and after the
dependency/environment/cancellation/pin slice); `bash -n` remains in the suite. The cases in
`tests/bash_wrapper.py` drive the real script end to end against real producer fixtures.
`tools/check-bash-sites` neuters every `refuse`/`finish` site of the chain runner and applies
hand-named plants, each required to fail the test named for it (`sites=37 plants=42 killed=79
survived=0` on 2026-09-24; `sites=39 plants=45 killed=84 survived=0 drifted=0` on 2026-09-25,
after JSON-pointer inputs, whose rewrite first left two plants `drifted` until they were
re-pointed at the new spelling of the same rule). The plants were enumerated by the author, so they are a floor,
not a census; dropping env's `--` is recorded there as an equivalent mutant, with its reason.

The suite invokes the wrapper as `bash integrations/bash/hee3`, not by executing it. The
corpus publisher owns this file's mode — it is listed in `corpus/publication-outputs.json`
and normalised to 0644 on every generation — so depending on the executable bit made 54 cases
fail with `PermissionError` after a routine publish. `tools/check-quality` is invoked through
`python3` for the same reason. The shebang is still pinned by a case, because it is what makes
the file runnable for anyone who does install it with the bit.

No transport, scheduler, grant, acceptance or admission is implemented here.
