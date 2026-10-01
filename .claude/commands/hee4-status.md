---
description: HEE v4 status - readiness, the P0 holds, and agent (roster) status. (Named hee4-status because /status is built in.)
allowed-tools: Bash(hee4db readiness), Bash(hee4db recipe blocks-phase P0), Bash(hee4db recipe agents), Bash(hee4db recipe last-runs:*), Bash(hee4db recipes), Bash(herdr agent list)
---
Compute state; never read it from a note (D-13). Run with the Bash tool:

1. `hee4db readiness` - quote the stderr verdict line (`measured=`, `deploy_ready=`, `ready_to_build=`).
2. `hee4db recipe blocks-phase P0` - list each open H-item as `H-n title (answer form)`.
3. `hee4db recipe agents` - each roster agent's schedule, last run, last verdict and 24 h spend
   (`hee4db recipe last-runs 4` for the individual runs). If the recipe is absent (exit 2), run
   `hee4db recipes` once and say which recipe you used, or UNMEASURED.
   Optionally `herdr agent list` for live panes (runtime state; may be empty).

Report ≤ 12 lines with every verdict line quoted. Unmeasured is reported as UNMEASURED, never inferred.
