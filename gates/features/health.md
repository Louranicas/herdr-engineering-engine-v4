# health

A-01. The readiness read: one frame that reports protocol and engine version, whether the engine is ready, the recovery state, the ledger state and socket custody. Scope v4.0, phase P1, owner `Owner::App`, effect `Read`, not mutating. It is the read-back that closes `serve` start (E2E-05) and the third row of `doctor`.

## Sub-features

- readiness-line: `ready`, `recovery`, `database`, `socket` in one reply; ATLAS D3 reads it as `ready=true recovery=complete database=ready socket=owned`.
- version-fields: `protocol_version` (`hee4.control` 1) and `engine_version` (the installed release) in the body.
- custody-read: `socket=owned` means the answering process holds the S-2 flock and the S-1 inode, not merely that a file exists.
- recovery-read: `recovery` reflects RL-7's result for the active generation (after `serve` start or `hee4 restore --into`).
- ledger-read: `database` reflects whether the active generation opened (`ledger_meta`, `migration_history`).
- timestamp: `checked_unix_ms`, the receiver's clock at the read.
- empty-body: the request `body` is `{}`; no filter, no page.

## How to get to it (user POV)

`hee4 health` from any shell as the socket owner, after the unit is active (or after `hee4 serve --until-stdin-closes` in the gate world). ATLAS D3 and card `hee4-app/control-socket` §9 #2 quote the whole line. The wrapper form is `hee4-sh health`; there is no Pi tool for it (v3 `tool: None`; API Map Conflicts §1).

Not reachable: before `serve` has bound S-1 (no socket; the client fails to connect, an IO/host refusal with no frame), or while recovery is not `complete` (the socket is bound only after recovery completes and the start backup verified, K6 PR-6, E2E-05). So a caller never sees `recovery` other than `complete` through the socket in the designed order.

## Driving it with hee4

Preconditions: README shared preconditions; a grant covering `Read`.

```bash
hee4 health
hee4-sh health
hee4-sh --check health          # prints the request frame, sends nothing
```

Socket: request `body` `{}`; result `body` `{protocol_version, engine_version, ready, recovery, database, socket, checked_unix_ms}` (API Map A-01, CD:357 shape).

- Success: assert the four fields `ready=true recovery=complete database=ready socket=owned` and that `engine_version` equals the manifest of `readlink -f ~/.local/lib/hee4/current`.
- Side effect: none. Assert no `operations` row was written (`sqlite3 … mode=ro 'select count(*) from operations'` before and after is equal).
- Persistence: repeat with a new `request_id`; `replayed=false` both times (not mutating; API Map §3 "readback none").
- Error: an expired deadline (`deadline_unix_ms` in the past) is `deadline_exceeded`, retry `never`, even for a read (K1 §4.3 order: expiry decided at receipt; an expired read is refused, State map §3).
- Empty: there is no empty variant; the body has no fields.
- Custody negative: with a second `hee4 serve` attempted, the second one is refused by name at the CLI (card control-socket §9 #5) and `health` on the first still reads `socket=owned`.
- `UNWRITTEN: the exact value set of recovery and database (which strings other than complete / ready exist) and the printed text form of the CLI line versus the JSON body.`

Concrete, deployed frame (rev 2026-10-05 drive) (`crates/hee4-app/FLOW.md`; run all of it with `tools/drive`):

```bash
hee4 health     # ready=true recovery=complete database=ready socket=owned head=<sha12> uptime_s=<n>
printf '%s\n' '{"request_id":"r1","action":"health","action_version":1,"idempotency_key":null,"body":{}}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/hee4/control.sock
```

- Success: `kind=result`, `replayed=false`, body `{ok:true, head_sha, recovery_complete:true, uptime_s}`.
- Malformed frame (`{not json`): `invalid_argument` at `/`; unknown action: `unknown_action` at `/action`; `action_version` 2: `unsupported_action_version` at `/action_version`; each retry `never`.
- Perms: `stat -c '%a'` of the socket dir `700` and of `control.sock` `600`.

## Gotchas

- `health` has no model field. "Model reachable" is a separate `doctor` row (README); a green health line says nothing about `127.0.0.1:11434`.
- `socket=owned` is custody, not permissions. Read `stat -c '%a %U'` separately for the 0700/0600 claim (D3 lists both).
- v3 raised `unavailable` from `fn health` for unserved owners (Error map class 5, F-4); v4 has no such site, so a `health` reply never lists unserved actions. Use `tools.list` for that.
- The health line is only evidence for the process that answered: pair it with `sha256sum /proc/<MainPID>/exe` or a stale unit passes `doctor` by accident.
- Must not: `main` holds no policy (card main §9 #1), so `health` content comes from startup-coordinator and control-socket, never from a CLI-side computation.
- No D-row other than D3 reads this action; it does not count as "use" (D9 counts accepted tasks only).
