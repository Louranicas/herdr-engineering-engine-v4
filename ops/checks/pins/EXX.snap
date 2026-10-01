# HEE v4 code exemplars (EX-01 … EX-20) and anti-exemplars

**Written:** 2026-10-01 · planning only (HOLD: no code until Luke says "start coding") · author Opus 5.5.
**Subject:** HEE-v3 at `b5367bc`. Every excerpt below was inserted mechanically from `git show b5367bc:<path>` in the v3 repository (read-only, at the time of writing; excerpts here are the durable copy) by line range, so it is **verbatim** (FACT); ranges are 1-based and inclusive. The first line of each block (`// b5367bc:<path>:<a>-<b>`) is a label added by the insertion, not source. Nothing was compiled or run; "sound" means sound by reading and by the cited review, not by a fresh measurement.
**Companions:** [ANTIPATTERNS.md](ANTIPATTERNS.md) (the AP ids each exemplar prevents) · [DRIFT_AND_OVERENGINEERING.md](DRIFT_AND_OVERENGINEERING.md).

**Why these were chosen.** The v3 architecture review found one module KEEP (`recovery`) and flagged the rest REFACTOR/HARDEN, but named sound parts inside them (`~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md` rows 13, 23; `~/hee4-evidence/design/DECISION_POINTS-b5367bc.md` rows marked EXACT/pure, e.g. :48, :50, :55, :69, :76). The discovery layer (independent review, plants, mutation) is where v3's real defects were found (SA:59); these files are the ones that layer left standing. **Imitate the shape, not the text** — v3 code is never copied into v4 (CLAUDE.local gate 2 applies to prototypes; v4 re-derives).

**How to read each entry.** *Range* is the whole construct; the excerpt is the load-bearing part (≤ 15 lines). *Prevents* names the AP ids. *Reuse when* is the trigger for reaching for the pattern.

**Index** (entries are grouped by theme; IDs are stable, not positional):

| ID | Pattern | v3 site (b5367bc) | Prevents |
|---|---|---|---|
| EX-01 | Bounded frame reader | `src/contracts/control.rs:102-175` | AP-04, AP-05, AP-14 |
| EX-02 | One write door, deadline before COMMIT | `src/store.rs:1180-1229` | AP-01, AP-31, AP-49 |
| EX-03 | Fault injection absent from release | `src/store.rs:541-556` | AP-10, AP-18 |
| EX-04 | Pure policy module with named rules | `src/recovery.rs:296-300`, `:866-911` | AP-06, AP-08, AP-18 |
| EX-05 | One spelling per enum, `parse → None` | `src/recovery.rs:305-406` | AP-02, AP-12, AP-14 |
| EX-06 | Predicate reads its arms' values | `src/recovery.rs:835-845` | AP-18, AP-19 |
| EX-07 | Pure settle step | `src/worker/process.rs:137-166` | AP-06, AP-31, AP-01 |
| EX-08 | Verify pidfd before retaining | `src/worker/process.rs:228-270` | AP-49, AP-03 |
| EX-09 | TERM/KILL once, revalidated | `src/worker/process.rs:353-390`, `:675-691` | AP-49, AP-41, AP-31 |
| EX-10 | Pure, explained, order-independent routing | `src/route.rs:300-310`, `:1923-1935`, `:2096-2165` | AP-06, AP-08, AP-16 |
| EX-11 | Device rule pure over values (F95) | `src/app/backup_target.rs:719-778` | AP-06, AP-29 |
| EX-12 | Pure listener/holder + bounded acquisition | `src/worker/native.rs:921-1085` | AP-04, AP-06, AP-29 |
| EX-13 | `take(N+1)` then refuse | `src/worker/native.rs:1026-1040` | AP-04, AP-14 |
| EX-14 | `Generation(NonZeroU64)` | `src/contracts.rs:278-306` | AP-02, AP-19, AP-17 |
| EX-15 | `Principal` private fields + matcher | `src/contracts/principal.rs:1-59` | AP-07, AP-01, AP-09 |
| EX-16 | Install partial → fsync → rehash → rename | `deploy/install-release:162-217` | AP-49, AP-13 |
| EX-17 | Fail-closed severity lattice, sealed output | `src/check/decision.rs:296-413` | AP-29, AP-22, AP-03 |
| EX-18 | One grace constant, compile-time checked | `src/worker/resources.rs:46-67` | AP-01, AP-19 |
| EX-19 | One door for a digest | `src/contracts/control.rs:176-190` | AP-01, AP-12, AP-15 |
| EX-20 | Constants checked against contract text | `src/app/dispatcher.rs:1506-1540` | AP-19, AP-21 |

---

## Acquisition and bounds

### EX-01 · Bounded frame reader: the acquisition itself is the bound
- **Range:** `src/contracts/control.rs:102-175` (`FrameReader`, `next_frame`, `close`).
````rust
// b5367bc:src/contracts/control.rs:148-158
            self.scanned = self.buffer.len();
            // Never hold more than one oversized byte: the acquisition itself is the bound.
            if self.buffer.len() > MAX_FRAME_BYTES {
                return Err(self.close(FrameFault::Oversize));
            }
            let room = (MAX_FRAME_BYTES + 1 - self.buffer.len()).min(READ_CHUNK);
            let read = match self.source.read(&mut chunk[..room]) {
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(ReadError::Io(error)),
            };
````
- **Why exemplary:** the reader never requests more than `MAX_FRAME_BYTES + 1 - buffer.len()` bytes, so no input can make it hold more than one byte past the limit; the refusal is by name (`FrameFault::Oversize`) and **sticky** — after a fault every later call returns the same fault (`:133-135`), so an attacker cannot resynchronise by scanning chosen content (doc `:104-106`). `Interrupted` is retried, EOF inside a record is `Truncated`, a clean EOF is `Ok(None)`: three outcomes, three names.
- **Prevents:** AP-04 (limit after acquisition), AP-05 (the bound sits where the input arrives, not in a later parser), AP-14 (no silent truncation).
- **Reuse when:** any stream, pipe, socket or child output is read. Caveat (MM #56): a caller that truncates *before* this reader makes its refusal unreachable — bound once, at the first acquisition.

### EX-02 · The one write door: deadline before COMMIT, poison on an uncertain commit
- **Range:** `src/store.rs:1180-1229` (`Store::transaction`).
````rust
// b5367bc:src/store.rs:1198-1204
        let result = require_normal(&tx, &self.epoch, self.inspection_only)
            .and_then(|()| action(&tx))
            .and_then(|value| {
                remaining(deadline)?;
                cut_point!(fault, CutPoint::BeforeCommit);
                Ok(value)
            });
````
````rust
// b5367bc:src/store.rs:1218-1222
        // A failed COMMIT may or may not be durable; say so, as every later write will.
        if tx.commit().is_err() {
            self.poisoned = true;
            return Err(Error::UncertainCommit);
        }
````
- **Why exemplary:** every store mutation goes through one generic door that (1) refuses when poisoned or inspection-only (`:1185-1190`), (2) checks the caller's deadline **after** the action and **before** COMMIT, so a write never commits past its budget, (3) rolls back on any error and poisons the store if the rollback itself fails, and (4) treats a failed COMMIT as *uncertain* — it neither claims success nor failure, and every later write refuses with `UncertainCommit`. The test-only cut point sits inside the same door.
- **Prevents:** AP-01 (one door for the transaction rule), AP-31 (budget checked on the path that commits), AP-49 (never reports a state it cannot vouch for).
- **Reuse when:** any durable mutation. In v4 the *body* passed to the door must use typed state (AP-02) — v3's bodies are the anti-exemplar below.

### EX-03 · Fault injection that does not exist in the release build
- **Range:** `src/store.rs:541-556` (`Fault`, `NO_FAULT`).
````rust
// b5367bc:src/store.rs:543-555
#[cfg(test)]
pub(crate) type Fault = Option<CutPoint>;
/// Injected fault state: a cut point in a test build; in a production build a zero-sized value, so the
/// store holds no injectable state.
#[cfg(not(test))]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fault;
/// No injected fault, in either build.
#[cfg(test)]
pub(crate) const NO_FAULT: Fault = None;
/// No injected fault, in either build.
#[cfg(not(test))]
pub(crate) const NO_FAULT: Fault = Fault;
````
- **Why exemplary:** tests get an injectable `Option<CutPoint>`; the production build gets a zero-sized `Fault` with no state, so there is nothing to inject and nothing to misuse. The `AfterCommit` cut point is itself `#[cfg(test)]` (`:532-535`) — "production reports what COMMIT returned".
- **Prevents:** AP-10 (test-only seams leaking to release), AP-18 (the seam is a real path through the real door, not a double).
- **Reuse when:** crash-point testing of any writer (F132 demands the real writer under a real kill; this is how the real writer carries its cut points without shipping them).

### EX-13 · `take(N+1)`, then refuse on length
- **Range:** `src/worker/native.rs:1026-1040` (`socket_table`); same idiom `src/app/startup.rs:259-268` (`read_stat`).
````rust
// b5367bc:src/worker/native.rs:1032-1039
    let mut bytes = Vec::new();
    file.take(MAX_SOCKET_TABLE + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Endpoint(EndpointWhy::Table))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_SOCKET_TABLE {
        return Err(Error::Endpoint(EndpointWhy::Table));
    }
    String::from_utf8(bytes).map_err(|_| Error::Endpoint(EndpointWhy::Table))
````
- **Why exemplary:** reading `N+1` distinguishes "exactly at the bound" from "over it" without allocating past it; the refusal is a named error, never a truncation. A missing table (`NotFound`) is an explicit empty, not a failure (`:1029`).
- **Prevents:** AP-04, AP-14.
- **Reuse when:** a whole-file read of a bounded file (`/proc`, manifests, configs). Do not ban `read_to_end` wholesale — it is correct under `take` (diary *The Antipattern Registers* (`my-diary.vault/Reflections/The Antipattern Registers.md`) lines 142-145; re-pointed from `AR:` 2026-10-01 V7/V8/V9/V10-fix, unpinned).

---

## Pure policy apart from I/O (F95)

### EX-04 · A policy module that performs no I/O, with named rules
- **Range:** `src/recovery.rs:296-300` (module contract), `:866-911` (`reconcile`), rules R01–R14 `:874-1297` (DP:50).
````rust
// b5367bc:src/recovery.rs:296-300
//! Pure attempt reconciliation policy over durable inventory values and physical
//! observations. Every input is a value the caller already holds; every output is
//! a report the app consumes. This file performs no I/O, reads no clock, opens no
//! `/proc` entry, sends no signal and dispatches nothing: a decision here never
//! resumes, settles, cleans up or replays anything by itself.
````
````rust
// b5367bc:src/recovery.rs:866-873
/// Reconcile one attempt from its durable facts and the caller's observations.
#[must_use]
pub fn reconcile(
    ledger: &LedgerFacts<'_>,
    task: &TaskFacts<'_>,
    attempt: &AttemptFacts<'_>,
    observed: &Observations<'_>,
) -> Decision {
````
- **Why exemplary:** every input is a value the caller already holds; every output is a `Decision { rule, reconciliation }` naming the rule that decided (e.g. `Rule::R01StaleObservationEpoch` at `:877`). The module states what it does *not* do (no clock, no `/proc`, no signal, no dispatch), which makes the claim checkable by grep. It is the only v3 module the architecture review rated **KEEP** (ARV row 23).
- **Prevents:** AP-06 (environment-dependent branches), AP-08 (a policy that cannot grow I/O concerns), AP-18 (tests call the real function with values; no double).
- **Reuse when:** any decision about durable state vs observations: recovery, settlement, admission. v4: its state enums move to `hee4-contracts` (UM §2) so the store uses them too (fixes AP-02).

### EX-05 · One spelling per enum; unknown text parses to `None`, never a default
- **Range:** `src/recovery.rs:305-406` (state enums and the `spelled!` macro).
````rust
// b5367bc:src/recovery.rs:378-385
            pub fn name(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
            /// Parse the ledger's spelling; any other text is `None`, never a default.
            #[must_use]
            pub fn parse(text: &str) -> Option<Self> {
                match text { $($text => Some(Self::$variant),)+ _ => None }
            }
````
- **Why exemplary:** the wire/ledger spelling and the Rust variant are declared once, side by side, and generated into both directions; `parse` returns `None` for anything else, so a corrupt row cannot silently become a default state.
- **Prevents:** AP-02 (the enum is the value, the string is only its rendering), AP-12 (one list, not three that must agree), AP-14 (no default on unknown input).
- **Reuse when:** any closed set that crosses a boundary (SQL text, JSON, CLI). Pair with a round-trip test over `ALL` variants, asserted against an independent spelling list (AP-21).

### EX-06 · A predicate that reads the values its arms carry
- **Range:** `src/recovery.rs:835-845` (`Reconciliation::permits_execution`).
````rust
// b5367bc:src/recovery.rs:835-845
    /// Whether this decision permits any execution, replay or redispatch: it never does.
    /// Reattach carries `redispatch`, cursor snapshot carries `replay`; both are read here
    /// so a consumer that trusts this method is bound to the values the arms carry.
    #[must_use]
    pub fn permits_execution(&self) -> bool {
        match self {
            Self::ReattachObservationOnly { redispatch, .. } => *redispatch,
            Self::CursorSnapshotOnly { replay, .. } => *replay,
            _ => false,
        }
    }
````
- **Why exemplary:** instead of returning a constant `false` (which a test could not tell from the real answer), the predicate reads the `redispatch`/`replay` fields of the arms, so a consumer that trusts it is bound to those values, and a test that sets them observes a change. The doc says why.
- **Prevents:** AP-18 (a computed value that is observable), AP-19 (not pinned only at the identity element `false`).
- **Reuse when:** any `bool` authority method over an enum — derive it from the data, never from the variant name alone.

### EX-07 · Settle loop: the decision is a pure function of one poll
- **Range:** `src/worker/process.rs:137-166` (`SettleStep`, `settle_step`, `SETTLE_PAUSE`).
````rust
// b5367bc:src/worker/process.rs:151-162
/// The one home of the settled predicate (R21 N18): every settle loop — the plan's probe, a
/// source's retained children — decides its turn here.
#[must_use]
pub fn settle_step(poll: &CleanupPoll, now: Instant, deadline: Instant) -> SettleStep {
    if poll.leader_terminal && poll.group == GroupState::Empty {
        SettleStep::Settled
    } else if poll.ownership == WaitOwnership::Lost || now >= deadline {
        SettleStep::Refused
    } else {
        SettleStep::Wait
    }
}
````
- **Why exemplary:** "settled" has one home (R21 N18, `:151`). The I/O (`poll_cleanup`) produces a `CleanupPoll` value; `settle_step` decides over it and the caller's clock values, so every branch — settled, refused (ownership lost or deadline), wait — is reachable by choosing arguments. Pacing is a named constant documented as "pacing, never a limit" (`:164-166`).
- **Prevents:** AP-06 (F95), AP-31 (the loop's exit includes the deadline), AP-01 (one settled predicate for every loop).
- **Reuse when:** any poll/wait loop (process, unit, file, socket readiness).

### EX-10 · Routing: pure, explained, and order-independent
- **Range:** module contract `src/route.rs:300-310`; `compare` `:1923-1935`; `decide` `:2096-2165` (ties at `:2128-2140`).
````rust
// b5367bc:src/route.rs:308-309
//! no I/O and reads no clock: the age of every observation is an input value,
//! and the only place a route configuration is read is [`Routing::parse`] (which
````
````rust
// b5367bc:src/route.rs:1923-1935
/// The declared total preorder: keys in declared order; lower cost, higher
/// quality and lower latency first. Equal on every key is a tie.
fn compare(ranking: &[Key], left: &Ranked<'_>, right: &Ranked<'_>) -> Ordering {
    ranking
        .iter()
        .map(|key| match key {
            Key::Cost => left.cost_microunits.cmp(&right.cost_microunits),
            Key::Quality => right.quality_basis_points.cmp(&left.quality_basis_points),
            Key::Latency => left.latency_ms.cmp(&right.latency_ms),
        })
        .find(|ordering| ordering.is_ne())
        .unwrap_or(Ordering::Equal)
}
````
- **Why exemplary:** no model call, no I/O, no clock — observation age is an input (`:308-309`). Ranking is a declared total preorder over declared keys; when more than one candidate is equal on every key, `decide` does **not** pick by input order: it records `R12Tie` and falls back to the baseline (`:2138-2140`), so the result cannot depend on the order candidates arrived. Every decision carries an `Explanation` naming the policy revision and each applied rule (`:2103-2106`). Config revisions hash a canonical rendering in which "row order and formatting do not enter it" (`:1208-1209`).
- **Prevents:** AP-06, AP-08, AP-16 (a ranking that could have been a JUDGMENT is exact and explained; DP:57 notes the figures are hand-declared — measure them, do not Jev them).
- **Reuse when:** any selection among candidates. Harden per ARV row 13: typed rule per step kind; split `route/config.rs` (the file is 2,165 lines — AP-08 risk).

### EX-11 · Device rule pure over (mount table, topology)
- **Range:** `src/app/backup_target.rs:719-778` (`device_decision`); acquisitions `MountTable::read` `:357`, `Topology::read` `:528`.
````rust
// b5367bc:src/app/backup_target.rs:731-738
pub fn device_decision(
    state: Option<&Mount>,
    destination: Option<&Mount>,
    topology: &Result<Topology, TopologyWhy>,
) -> Result<(), DeviceWhy> {
    let state = state.ok_or(DeviceWhy::Unresolved(Side::State))?;
    let destination = destination.ok_or(DeviceWhy::Unresolved(Side::Destination))?;
    for (side, mount) in [(Side::State, state), (Side::Destination, destination)] {
````
````rust
// b5367bc:src/app/backup_target.rs:766-771
    if !state_disks.is_disjoint(&destination_disks) {
        return Err(DeviceWhy::SameDisk {
            state: mounts.0,
            destination: mounts.1,
        });
    }
````
- **Why exemplary:** the "separate device surviving a disk failure" rule (RC02) is decided over parsed values, so every refusal (`Unresolved`, `NoDevice`, `SameSource`, `SameFilesystem`, `Topology`, `DiskUnresolved`, `SameDisk`) is provable by constructing mounts — no test needs two real disks (module doc `:28`: "never by arranging the machine (F95)"). The topology is consulted only at the disk step, so a refusal that needs no topology is never masked by one that could not be read (`:724-725`). The topology arrives as `&Result<Topology, TopologyWhy>`: the acquisition's failure is a value the policy orders.
- **Prevents:** AP-06 (F95 by name), AP-29 (each refusal names which side and why).
- **Reuse when:** any host-shape rule (filesystems, mounts, units, devices).

### EX-12 · Endpoint resolution: pure listener/holder policy + one bounded acquisition
- **Range:** `listener` `src/worker/native.rs:921-948`; `holder` `:950-972`; `within_descriptor_bound` `:974-985`; `endpoint_holder` acquisition `:1042-1085`.
````rust
// b5367bc:src/worker/native.rs:962-971
    let holders: Vec<u32> = held
        .iter()
        .filter(|(_, targets)| targets.contains(&target))
        .map(|(pid, _)| *pid)
        .collect();
    match holders.as_slice() {
        [pid] => Ok(*pid),
        [] => Err(EndpointWhy::Unobserved { unreadable }),
        several => Err(EndpointWhy::Owners(several.len())),
    }
````
- **Why exemplary:** "exactly one" is a slice pattern with three named outcomes (`[pid]`, `[]` with how many processes were unreadable, `several` with the count). The acquisition reads `/proc/net/tcp{,6}` through EX-13, bounds the descriptor walk *before each next read* (`:1071`), checks the deadline/cancellation per process (`:1056`), and verifies the holder shares the resolver's network namespace (`:1080-1084`). The doc names the split: "The policy is [`listener`] and [`holder`]" (`:1044`).
- **Prevents:** AP-04 (per-read bound on a derived set), AP-06, AP-29 (`Unobserved { unreadable }` separates "none" from "could not look").
- **Reuse when:** resolving any live identity from the kernel (pid by socket, unit by cgroup).

---

## Types that refuse

### EX-14 · `Generation(NonZeroU64)`: overflow refuses, zero is unrepresentable
- **Range:** `src/contracts.rs:278-306`.
````rust
// b5367bc:src/contracts.rs:278-280
/// Nonzero generation. Overflow refuses advancement instead of wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Generation(NonZeroU64);
````
````rust
// b5367bc:src/contracts.rs:290-296
    pub fn next(self) -> Result<Self, ScalarError> {
        let next = self.value().checked_add(1).ok_or(ScalarError::Overflow)?;
        // checked_add on a nonzero value cannot produce zero.
        NonZeroU64::new(next)
            .map(Self)
            .ok_or(ScalarError::ZeroGeneration)
    }
````
- **Why exemplary:** zero cannot be constructed; `next` uses `checked_add` and returns `Overflow` rather than wrapping; parsing refuses `0` by name (`:303`). The comment explains why the final `ok_or` is unreachable instead of `unwrap`ping it.
- **Prevents:** AP-02 (a value type, not a string), AP-19 (a `FIRST` constant can be asserted off the origin — ARV row 15 proposes `Generation::FIRST`), AP-17 (no `unwrap` needed to be total).
- **Reuse when:** any counter, epoch, sequence or version that must be monotonic and nonzero. v4: the store writes this type, never the text `"1"` (anti-exemplar A-2).

### EX-15 · `Principal`: private fields, validated constructor, a matcher instead of getters
- **Range:** `src/contracts/principal.rs:1-59`.
````rust
// b5367bc:src/contracts/principal.rs:5-9
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Principal {
    uid: u32,
    role: String,
}
````
````rust
// b5367bc:src/contracts/principal.rs:35-40
    /// Whether this is the principal with `uid` and the configured `role`: how a record naming
    /// its principal (a grant) is matched without exposing either field.
    #[must_use]
    pub fn is(&self, uid: u32, role: &str) -> bool {
        self.uid == uid && self.role == role
    }
````
- **Why exemplary:** a principal is "never decoded from a body" (`:4`); the only constructor validates the role (`:20-34`); fields are private and read only `pub(crate)`; outside the crate a record is matched with `is(uid, role)` without exposing either field. The operator role has one spelling below both doors (`OPERATOR_ROLE`, `:56-59`).
- **Prevents:** AP-07 (a function holding a `Principal` cannot re-derive it from input), AP-01 (one spelling of the operator role), AP-09 (narrow surface).
- **Reuse when:** any authority-bearing identity (principal, grant, lease holder).

### EX-17 · Fail-closed severity lattice with sealed output
- **Range:** `src/check/decision.rs:296-413` (`Decision` with private fields, `Severity`, `decide`).
````rust
// b5367bc:src/check/decision.rs:398-410
    let state = if let Some(state) = causal {
        state
    } else if a.classes.contains(&Severity::Invalid)
        || (a.classes.contains(&Severity::Derivative) && !a.classes.contains(&Severity::Error))
    {
        VerdictV1State::Invalid
    } else if a.classes.contains(&Severity::Error) {
        VerdictV1State::Error
    } else if a.classes.contains(&Severity::Fail) {
        VerdictV1State::Fail
    } else {
        VerdictV1State::PassCandidate
    };
````
- **Why exemplary:** every detector adds a reason with a severity; the verdict is derived from the *set* of severities in a fixed precedence, so missing evidence (`CheckerFact::Unavailable` → `Invalid`, `:384-388`) can never read as a pass, and the best outcome is only `PassCandidate`. All observed reasons stay in the result (`:367-368`). `Decision`'s fields are private with `const` getters (`:296-323`); its doc states the reason: "a verdict that did not come from the frozen facts is unrepresentable" (`:296-298`).
- **Prevents:** AP-29 (a verdict carries its reasons and detectors), AP-22 (absence is not success — AX-2), AP-03 (the lattice replaces flag soup).
- **Reuse when:** any aggregate verdict. v4 caveat (ARV row 3): v3 had a *second* verdict authority beside this one (AP-01); v4 keeps exactly one `verdict_of(decide)`.

---

## Process lifecycle

### EX-08 · Verify the pidfd before retaining it; an unverified descriptor is never a signal target
- **Range:** `src/worker/process.rs:228-270` (`PendingChild`, `new`).
````rust
// b5367bc:src/worker/process.rs:255-269
            Ok(fd) => match waitid(WaitId::PidFd(fd.as_fd()), WAIT_FLAGS) {
                Ok(status) => {
                    owner.terminal = status.is_some();
                    owner.pidfd = Some(fd);
                    true
                }
                Err(rustix::io::Errno::CHILD) => {
                    owner.lose_wait();
                    false
                }
                Err(_) => false,
            },
            Err(_) => false,
        };
        // An unverified descriptor is never retained or used as a signal target.
````
- **Why exemplary:** stable Rust cannot acquire a pidfd atomically at spawn (module doc `:3-6`), so the owner verifies the post-spawn pidfd with `waitid` before keeping it; `ECHILD` revokes authority (`lose_wait`) and nothing restores it. Ownership is a typed state (`WaitOwnership`, `GroupState`), not flags.
- **Prevents:** AP-49 (a successful `pidfd_open` is not proof of ownership — read it back), AP-03.
- **Reuse when:** any spawned child the engine must later signal or reap.

### EX-09 · TERM and KILL each sent at most once, revalidated immediately before
- **Range:** `src/worker/process.rs:353-390` (`send`); escalation `:675-681` (TERM, then KILL after `TERM_GRACE`, stop after `CLEANUP`); the leader is kept until its group is empty (`:155`, `:684-691`).
````rust
// b5367bc:src/worker/process.rs:353-367
    fn send(&mut self, signal: Signal) {
        let previous = if signal == Signal::TERM {
            self.signals.group_term
        } else {
            self.signals.group_kill
        };
        if previous == SignalOutcome::NotAttempted {
            // Revalidate immediately before every numeric group signal, including
            // Drop. ECHILD removes authority; a retained pidfd never restores it.
            let outcome = match self.observe_wait() {
                Ok(()) if self.ownership == WaitOwnership::Waitable => {
                    signal_outcome(kill_process_group(self.pid, signal))
                }
                Err(rustix::io::Errno::CHILD) | Ok(()) => SignalOutcome::NotOwned,
                Err(error) => SignalOutcome::Failed(error.raw_os_error()),
````
- **Why exemplary:** each signal class records its outcome (`NotAttempted`, `NotOwned`, `Failed(errno)`, …), so a repeat call is a no-op and the record says exactly what was attempted. Group authority is revalidated with `observe_wait` right before every numeric group signal, including on `Drop`, so a recycled pid is never signalled. The loop exits only when the leader is terminal **and** the group is empty **and** both pipes are drained (`:684-691`).
- **Prevents:** AP-49, AP-41 (pid-reuse and `$!`-style races are designed out, not remembered), AP-31 (escalation is time-bounded).
- **Reuse when:** any stop path for a process tree. Pair with EX-18 for the grace constant.

### EX-18 · One grace constant, compile-time checked, rendered into the other enforcer
- **Range:** `src/worker/resources.rs:46-67`.
````rust
// b5367bc:src/worker/resources.rs:46-52
/// The grace between SIGTERM and SIGKILL, for both enforcers: systemd's `TimeoutStopSec=` on the
/// scope and the process owner's own escalation (`worker::process`) — one value, two doors made one.
pub const TERM_GRACE: Duration = Duration::from_secs(5);

// systemd's `TimeoutStopSec=` is rendered in whole seconds; a sub-second grace would make the two
// enforcers disagree silently (B14a-2b-i review LOW-4).
const _: () = assert!(TERM_GRACE.subsec_nanos() == 0);
````
- **Why exemplary:** the grace between SIGTERM and SIGKILL is declared once for both enforcers (systemd `TimeoutStopSec=` and the process owner); a `const` assertion refuses a sub-second value that systemd would render differently; the systemd property is **rendered from** the constant (`:66`), not typed again.
- **Prevents:** AP-01 (two doors made one), AP-19 (the value is derived, not re-typed).
- **Reuse when:** any limit enforced by two mechanisms. **v3 broke its own exemplar** at `src/worker/namespace.rs:1364` (anti-exemplar A-3) — so the v4 mechanism is a grep/census for the literal, not the comment.

---

## Durable install and one-door digests

### EX-16 · Install: `.partial` → fsync → re-hash what landed → rename → atomic `current`
- **Range:** `deploy/install-release:162-217` (`sha256_file`, `_fsync`, `install`).
````python
# b5367bc:deploy/install-release:190-204
    if not final.exists():
        partial = releases / (name + ".partial")
        partial.mkdir(mode=0o700)
        for binary in BINARIES:
            shutil.copyfile(sources[binary], partial / binary)
            os.chmod(partial / binary, 0o500)
            _fsync(partial / binary)
        (partial / "manifest.json").write_bytes(manifest_bytes)
        os.chmod(partial / "manifest.json", 0o400)
        _fsync(partial / "manifest.json")
        _fsync(partial)
        refused = digest_refusal(expected, {binary: sha256_file(partial / binary) for binary in BINARIES})
        if refused is not None:
            raise ValueError(refused)
        os.rename(partial, final)
````
- **Why exemplary:** the release directory is built under a name `current` can never point at, every file and the directory are fsynced, then the bytes **that landed** are re-hashed against the manifest before the rename; the parent is fsynced after the rename; `current` moves by writing `current.new` and renaming it (`:211-216`). A re-hash that differs refuses before `current` moves. Stale partials from a failed install are named and removed first (`:186-189`). It re-checks the final directory even when it already existed (`:206-209`).
- **Prevents:** AP-49 (read back what the syscall claimed), AP-13 (every write has a reader — the re-hash).
- **Reuse when:** any artifact publication: releases, backups, state snapshots, evidence bundles.

### EX-19 · The one door for a digest, with the reason it is one
- **Range:** `src/contracts/control.rs:176-190` (`criteria_digest`).
````rust
// b5367bc:src/contracts/control.rs:176-190
/// The digest a task binds its acceptance criteria by: `request_sha256` of the criteria's compact
/// JSON array, strings in the order given, duplicates kept. The one door for it — submit records it
/// and the dispatcher compares a class's criteria with it (B14-P2a) — so the two cannot encode the
/// same list differently. It is over the re-serialised strings, so two wire spellings of one string
/// bind the same.
#[must_use]
pub fn criteria_digest<S: AsRef<str>>(criteria: &[S]) -> String {
    let array = serde_json::Value::Array(
        criteria
            .iter()
            .map(|criterion| serde_json::Value::String(criterion.as_ref().to_owned()))
            .collect(),
    );
    request_sha256(array.to_string().as_bytes())
}
````
- **Why exemplary:** both sides that must agree (submit records it; the dispatcher compares a class's criteria with it) call one function; it hashes a re-serialisation, so two wire spellings of one string bind the same. The doc names both callers — a claim a reviewer can check by grep (contrast AP-15, where the doc's "only" was false).
- **Prevents:** AP-01, AP-12, AP-15.
- **Reuse when:** any identity that two components compute (digests, keys, revisions).

### EX-20 · Constants checked against the contract text, not against themselves
- **Range:** `src/app/dispatcher.rs:1506-1540` (`rc01_s_numbers_are_the_contract_s`).
````rust
// b5367bc:src/app/dispatcher.rs:1506-1519
    /// OPS-2 · the independent source for RC01's constants (F122): the published contract text,
    /// read at run time from the repository (not compiled in, so a missing document fails this case
    /// by name rather than the build). Each number is parsed out of its row and compared with the
    /// constant the gate enforces.
    #[test]
    fn rc01_s_numbers_are_the_contract_s() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/contract-decisions.md");
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !text.is_empty(),
            "the RC01 contract text is unreadable at {}",
            path.display()
        );
````
- **Why exemplary:** the RC01 numbers are read at run time from the published contract document, parsed out of their rows and compared with the constants the code enforces — the F122 fix ("a constant every test compares against is pinned by nothing"). A missing document fails this case by name, not the build.
- **Prevents:** AP-19, AP-21.
- **Caveat (INTERP):** the contract doc was written by the same lineage as the code, so per F113 it is independent of the *code*, not of the *head*. v4 should prefer a source the world produced where one exists.
- **Reuse when:** any policy constant with a written contract.

---

## Anti-exemplars: v3 code NOT to imitate

Each is quoted verbatim from `b5367bc` (same mechanical insertion). The AP id is the rule it breaks; the right-hand column is the exemplar to use instead.

### A-1 · Task state as text inside the one door (AP-02, AP-01)
`src/store.rs:1983-1984`
````rust
// b5367bc:src/store.rs:1983-1984
            if head.state!="verifying" {return Err(Error::Outstanding);}
            let generation:Generation=head.generation.parse().map_err(|_|Error::Corrupt)?;
````
The state is compared as a string and the generation is parsed back from text inside a transaction body that rustfmt left unformatted (ARV row 1: "rustfmt the transaction bodies"; reason INTERP). The enum `TaskState` already existed at `src/recovery.rs:338`. **Use instead:** EX-05, EX-14, and one pure `transition(State, Event)`.

### A-2 · Generation written as the literal `"1"` (AP-02, AP-19)
`src/store.rs:1252-1253`
````rust
// b5367bc:src/store.rs:1252-1253
            let sequence = event(tx,input.event.as_str(),input.task.as_str(),"1","admitted")?;
            let result = Admission { task:input.task.as_str().to_owned(),generation:"1".to_owned(),epoch,sequence };
````
**Use instead:** EX-14 (`Generation::FIRST`, asserted off the origin).

### A-3 · A third door on a grace that was "made one" (AP-01)
`src/worker/namespace.rs:1363-1365`
````rust
// b5367bc:src/worker/namespace.rs:1363-1365
        let kill = observation.stopping_since.is_some_and(|start| {
            observation.now.saturating_duration_since(start) >= Duration::from_secs(5)
        }) || observation
````
`TERM_GRACE` is 5 s at `src/worker/resources.rs:48`; this literal agrees today by coincidence (DP:33). **Use instead:** EX-18.

### A-4 · The frame bound re-typed at the store (AP-01)
`src/store.rs:1236` (also `:1617`, `:1705`, `src/app/coordinator.rs:53`)
````rust
// b5367bc:src/store.rs:1234-1238
    pub fn submit(&mut self, input: Submission<'_>, deadline: Instant) -> Result<Admission> {
        if input.request_bytes.is_empty()
            || input.request_bytes.len() > 1_048_576
            || !input.allocation.valid()
        {
````
`MAX_FRAME_BYTES` is `src/contracts/control.rs:23`. **Use instead:** every bound names its contract constant (ARV sys-rec 5).

### A-5 · God-file with an intra-app cycle (AP-08)
`src/app/runtime.rs` is 3,814 lines; it imports `live_verifier` and `plan`, and both `live_verifier` and `candidates` import it back:
````rust
// b5367bc:src/app/runtime.rs:16-17
use super::live_verifier::{Cleanup, checked};
use super::plan;
````
````rust
// b5367bc:src/app/live_verifier.rs:22-22
use super::runtime::{CheckPlan, Observed, Resources, Unlaunched, Verifier, declared_criteria};
````
**Use instead:** `runtime::{ports, lifecycle, settle, publish}` (ARV row 2); in v4 the app crate assembles clusters that cannot import each other (UM §2 dependency law; cited by section, *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F22)*).

### A-6 · Model-output fence counting treated as an exact rule (AP-16)
`src/app/candidates.rs:219-236`
````rust
// b5367bc:src/app/candidates.rs:219-229
    let mut start = 0;
    while start < text.len() {
        let end = text[start..]
            .find('\n')
            .map_or(text.len(), |offset| start + offset);
        if text[start..end].starts_with("```") {
            fences.push((start, end));
        }
        start = end + 1;
    }
    let body = match fences.as_slice() {
````
Counting column-0 ```` ``` ```` lines is a judgment about unstructured text (DP:19, J1): a reply with a nested fence, an indented fence or prose around one file is refused or misread. **Use instead:** declare the site's kind (JUDGMENT) in its doc, give it a labelled fixture set, and keep it behind the advisory port (UM, Jev held).

### A-7 · `LocalProbe` option-soup (AP-03, AP-09)
`src/service/local_probe.rs:47-57`
````rust
// b5367bc:src/service/local_probe.rs:47-57
pub struct LocalProbe {
    preparation: ProbePreparation,
    pub recipe: LocalRecipe,
    pub process: Option<ProcessReport>,
    pub facts: Option<ProbeObservation>,
    pub error: Option<LocalProbeError>,
    pub postflight_error: Option<LocalProbeError>,
    pub wall_elapsed: Duration,
    pub cpu_elapsed: Option<Duration>,
    recorded: Option<Recorded>,
}
````
Six `Option`s and public fields describe one lifecycle; which combinations are legal is unwritten, and any caller can assert a Useful health (ARV row 6). **Use instead:** a phase enum carrying only the data valid in that phase, returned as a validated observation for app to commit (ARV row 6); EX-08, EX-17.

### A-8 · A docstring claiming a property the code does not have (AP-15, AP-09)
`src/budget.rs:301-304`
````rust
// b5367bc:src/budget.rs:301-304
//! One owner for every counter. `docs/modules/budget.md` sets the complexity boundary
//! explicitly — *"Do not create independent budget counters in each wrapper or omit
//! checker/context cost"* — so [`Ledger`] is the only thing in the engine that adds or
//! subtracts an allocation, and every wrapper reports **through** it.
````
`Ledger` has 0 production callers (ARV row 4); the store adds at `src/store.rs:2019`. **Use instead:** EX-19 (a doc that names both callers of the one door, checkable by grep); `habitat-unused-pub`.

### A-9 · Silent deadline clamp in the wrapper (AP-14)
`integrations/bash/hee3:263-264` and `:275`
````python
# b5367bc:integrations/bash/hee3:263-264
budget = os.environ.get("HEE3_TIMEOUT_MS", "")
ahead = int(budget) if budget.isdigit() and int(budget) > 0 else DEFAULT_MS
````
````python
# b5367bc:integrations/bash/hee3:275-275
    "deadline_unix_ms": str(now + min(ahead, MAX_AHEAD_MS)),
````
A non-numeric budget silently becomes 30 s, and any budget over 60 s is silently cut, where the engine refuses (ARV row 21). **Use instead:** refuse by name with both numbers (EX-01's style).

### A-10 · A test-only module shipped as public API, inverting the layers (AP-10, AP-11)
`src/worker/mod.rs:1005` and `src/worker/tools.rs:620`
````rust
// b5367bc:src/worker/mod.rs:1005-1005
pub mod tools;
````
````rust
// b5367bc:src/worker/tools.rs:620-620
use crate::actions::{Action, Caller, Catalogue, Effect, Owner};
````
Its only importer is `tests/t28_actions.rs:1050`; the import is the K2→K6 inversion (CMAP:16). **Use instead:** EX-03's pattern (test-only code compiled out), projection in `actions::tools` (ARV row 11).

---

**Counter-evidence locator.** (1) No exemplar was compiled, mutated or run for this document; "sound" rests on the cited reviews and on reading. (2) Line numbers hold at `b5367bc` only. (3) EX-10 and EX-12 sit in files the review marks HARDEN; imitate the functions named, not the files. (4) EX-20's independence is partial (see its caveat). (5) Anti-exemplar reasons marked INTERP (A-1's rustfmt cause) were not measured.
