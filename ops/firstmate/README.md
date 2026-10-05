# Firstmate database adapter

This directory contains HEEv4's `fm-db` adapter, its schema migrations and its controls. The [HEEv4 entry point](../../README.md) supplies the wider engine context. The [Firstmate source repository](</mnt/storage-10tb/firstmate/README.md>) owns its supervisor implementation and instructions.

[Poteto Weave](</mnt/storage-10tb/hee4-evidence/prototypes/turso-tool-context/assimilation-20261005/README.md>) links back here as the framework's Firstmate integration reference. It retrieves source-grounded context and records its own experiments separately. Firstmate keeps its existing orchestration database ownership, and HEE keeps verdict authority.

Read [fm-db](fm-db) for current command behavior and [the control suite](tests/control.py) for demonstrated contracts. This navigation link does not install the candidate into Firstmate or change adapter behavior.

## Doors and schema level

`fm-db record spawn` requires `--brief-sha`: a spawn that names no brief is refused as `brief_sha_missing` (exit 20), and `brief_sha_unknown` and `brief_sha_unit_mismatch` still apply. `schema/003_spawn_brief_required.sql` refuses the same NULL in SQLite, so raw SQL cannot go around the door. Rows recorded before `002_brief_link.sql` keep a NULL `brief_sha`.

`fm-db record brief` refuses a VERIFY line whose exit status cannot fail (`verify_line_cannot_fail line=<n> shape=<shape>`, exit 20). The command string of a shell's `-c` (`bash`, `sh`, `dash`, `zsh`, `ksh`, `mksh`, `ash`) is judged as a line of its own, also when transparent command wrappers come first. A word's command name is read with its quotes and backslashes removed, so `"bash"` and `\bash` are `bash`. The shell's own options that take the next word (`-o`, `+o`, `-O`, `+O`, `--rcfile`, `--init-file`) are skipped with it. Leading `VAR=v` words are skipped, then any chain of `env` and the wrappers below. Each wrapper has its own options and operands, and the chain may nest in any order (`nice timeout 60 env X=1 bash -c '...'`):

| Wrapper | Options that take the next word | Operand before the command (skipped only when the word has its form) |
|---|---|---|
| `env` | `-u`, `-C`, `-S`, `--unset`, `--chdir`, `--split-string` (`-S STRING` is read as a command line) | none |
| `nice` | `-n`, `--adjustment` (`-N` takes none) | none |
| `timeout` | `-s`, `-k`, `--signal`, `--kill-after` | DURATION (`60`, `1.5m`) |
| `nohup` | none | none |
| `stdbuf` | `-i`, `-o`, `-e`, `--input`, `--output`, `--error` | none |
| `setsid` | none (`-c`, `-f`, `-w`) | none |
| `ionice` | `-c`, `-n`, `--class`, `--classdata` (`-p` runs no command) | none |
| `taskset` | none (`-c` makes the operand a list; `-p` runs no command) | MASK or LIST (`0x3`, `0-3,5`) |
| `chrt` | `-T`, `-P`, `-D` and their long forms (`-p`, `-m` run no command) | PRIORITY (digits; optional, and left out under `-o`, `-b`, `-i`) |
| `sudo` | `-u`, `-g`, `-p`, `-C`, `-h`, `-D`, `-r`, `-t`, `-U`, `-T`, `-R`, `-a` and their long forms; `VAR=v` words follow | none |
| `doas` | `-u`, `-C` | none |
| `command` | none (`-p`; `-v`, `-V` run no command) | none |
| `exec` | `-a` | none |
| `time` | `-o`, `-f`, `--output`, `--format` | none |
| `busybox` | none (the applet, such as `sh`, follows) | none |

Commands that run a string of their own are judged the same way: `eval` (its arguments joined), `su -c`/`-C`/`--command`/`--session-command`, `runuser` with the same options, `script -c`/`--command` and `flock FILE -c`/`--command`.

The door fails closed. When the word it reaches is not a shell, it scans every word after the leading `VAR=v` words, and judges the string of each one that names a shell followed by a `-c` cluster, or a string runner. That word may follow an unknown wrapper (`mywrap --opt bash -c '... | tail -1'`), a wrapper option that runs no command, or an operand the table misread. When a shell's options hide its string, the first non-option word after any later `-c` cluster is judged. Reading an unknown wrapper as transparent can only refuse more lines whose string cannot fail. It never admits one. `verify_cannot_fail` and `_shell_c` in [fm-db](fm-db) hold the rule. The `brief-verify-*` cases in [the control suite](tests/control.py) demonstrate it.

Every verb except `init` compares `schema/*.sql` with `schema_migrations` before it writes or reads back, and refuses `schema_behind=<file>` (exit 3). After a new migration merges, run `FM_HOME=<home> ops/firstmate/fm-db init` on each home before its orchestrator records anything. The [orchestration plan](../../plan/FIRSTMATE-ORCHESTRATION-2026-10-05.md) lists every door.
