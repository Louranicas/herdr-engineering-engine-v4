# Generated briefs must pass the brief door before anyone spawns

**Seen 2026-10-05 (V4-96).** Wave 1 landed `002_brief_link.sql`: `fm-db record spawn`
refuses a spawn unless the unit has a recorded brief that has all eleven fields as `FIELD:`
lines and the standing orders verbatim. Brief writers had produced `VERIFY (from the
worktree root …):` and `REPORT (≤300 words).`, and the STANDING field referred to the
orders without copying them. The door refused the captain's seven wave-2 spawns, so it was
working as designed.

**Rule.** Normalise every brief to the door's shape (`FIELD:` at the start of a line, the
standing orders pasted after `STANDING:`) and record it before the spawn. Run the door
against the brief files, not against what the writer claims to have written. A door that
refuses its own author is evidence that it works.
