---
description: Regenerate derived blocks after a register edit (`just regen`), then `just verify`
allowed-tools: Bash(just regen), Bash(just verify)
---
1. Run `just regen`. It is a sequence: it stops at the first red step and prints
   `regen verdict=FAIL steps=k/N failed_at=<step> not_reached=M`. Steps after it were not reached,
   never "passed". On FAIL, report that line and the step's tail, and stop (do not run verify as if
   regen had succeeded, Mistakes #24/#36).
2. On `regen verdict=PASS`, run `just verify` and quote its `verify verdict=…` line verbatim.

Report both verdict lines, nothing paraphrased.
