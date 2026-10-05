# Resuming an interrupted workflow: keep cached agents byte-identical

**Seen 2026-10-05.** Wave 2 hit a usage limit: 4 of 12 agents finished and 8 failed. Then
the model changed, and with it the commit attribution line. The workflow cache keys on the
exact prompt, so changing the attribution text in every prompt would have re-run the
finished agents too.

**Rule.** Pass a list of the finished agent labels as an argument. Prompts for those labels
keep the old text, so they replay from cache. Prompts for the agents that re-run get the new
attribution line and a resume note: "the worktree may already hold an interrupted
predecessor's work; read `git log main..HEAD` and `git status` first". Failed builders leave
uncommitted edits behind, so the agent that replaces one must review those edits, never
assume a clean tree.
