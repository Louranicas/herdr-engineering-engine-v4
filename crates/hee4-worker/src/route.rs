//! Routing by capability floor with typed refusal.
//!
//! A model is picked by a floor it provably meets, or refused by name: when no model meets the
//! floor the answer is [`RouteRefusal::NoCapableModel`], never a default. Rules run in the order
//! `Route::rules` lists them; each rule has one name and one site (its arm in `Rule::apply`).
//! Shape after v3 `route.rs` (`Rule` R01..R13); the figures a rule reads (cost, latency,
//! quality) are declared on the roster entry, not measured.

use std::cmp::Reverse;

/// What a model can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// Largest context the model holds, in tokens.
    pub ctx_tokens: u32,
    /// Can answer in constrained JSON.
    pub json_mode: bool,
    /// Can call tools.
    pub tool_use: bool,
    /// Runs on this machine (no network).
    pub local: bool,
}

/// The least a task accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityFloor {
    /// Context the task needs, in tokens.
    pub ctx_tokens: u32,
    /// The task needs JSON mode.
    pub json_mode: bool,
    /// The task needs tool use.
    pub tool_use: bool,
    /// The task may only run on a local model.
    pub local_only: bool,
}

/// One floor requirement a model lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// Context too small.
    Context {
        /// Needed.
        need: u32,
        /// Offered.
        have: u32,
    },
    /// No JSON mode.
    JsonMode,
    /// No tool use.
    ToolUse,
    /// Not local.
    Local,
}

impl CapabilityFloor {
    /// Every requirement `caps` misses, in the order R02 (JSON, tools), R03 (context), R04 (local).
    #[must_use]
    pub fn missing(&self, caps: &Capabilities) -> Vec<Missing> {
        let mut out = Vec::new();
        if self.json_mode && !caps.json_mode {
            out.push(Missing::JsonMode);
        }
        if self.tool_use && !caps.tool_use {
            out.push(Missing::ToolUse);
        }
        if caps.ctx_tokens < self.ctx_tokens {
            out.push(Missing::Context {
                need: self.ctx_tokens,
                have: caps.ctx_tokens,
            });
        }
        if self.local_only && !caps.local {
            out.push(Missing::Local);
        }
        out
    }
}

/// What the last observation says about a model's availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Observed up.
    Up,
    /// Observed down.
    Down,
    /// No fresh observation: an evidence gap.
    Unknown,
}

/// One roster row. Figures are declared, not measured (UNMEASURED; J5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    /// The model's name as the daemon knows it.
    pub name: String,
    /// What it can do.
    pub caps: Capabilities,
    /// Last observed availability.
    pub availability: Availability,
    /// Declared cost, in milli-units.
    pub cost_milli: u32,
    /// Declared latency, in milliseconds.
    pub latency_ms: u32,
    /// Declared quality, 0..=1000.
    pub quality: u32,
}

/// The models a task may be routed to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roster {
    /// The rows, in declaration order.
    pub models: Vec<ModelEntry>,
}

/// What a task declares for routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteInput {
    /// The floor the model must meet.
    pub floor: CapabilityFloor,
    /// Cost ceiling, milli-units.
    pub cost_ceiling_milli: Option<u32>,
    /// Latency deadline, ms.
    pub deadline_ms: Option<u32>,
    /// Quality floor.
    pub quality_floor: Option<u32>,
    /// The task's declared baseline model, used when a rule routes to baseline. It must still meet
    /// the floor; there is no default.
    pub baseline: Option<String>,
    /// The model whose attempt just failed, never chosen again.
    pub previous_attempt: Option<String>,
}

/// The named rules, as in v3 (R01, the baseline guard, is folded into `baseline_or_refuse`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// R02: the model declares every capability the task needs (JSON mode, tool use).
    R02RequiredCapabilities,
    /// R03: the model's context holds the task's.
    R03ContextLimit,
    /// R04: a local-only task admits only local models.
    R04PrivacyClass,
    /// R05: a model observed down is excluded; an unknown one is excluded and marks an evidence gap.
    R05Availability,
    /// R06: under a cost ceiling the cost figure must not exceed it.
    R06CostCeiling,
    /// R07: under a deadline the latency figure must not exceed it.
    R07Deadline,
    /// R08: under a quality floor the quality figure must reach it.
    R08QualityFloor,
    /// R09: any evidence gap routes to the baseline.
    R09InsufficientEvidence,
    /// R10: an empty eligible set routes to the baseline.
    R10NoEligibleCandidate,
    /// R11: the eligible set is ordered by quality, then latency, then cost; a strict best wins.
    R11Ranking,
    /// R12: equal top keys route to the baseline.
    R12Tie,
    /// R13: a retry never re-chooses the model whose attempt failed.
    R13PreviousAttempt,
}

impl Rule {
    /// The printed id, `R02`..`R13`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::R02RequiredCapabilities => "R02",
            Self::R03ContextLimit => "R03",
            Self::R04PrivacyClass => "R04",
            Self::R05Availability => "R05",
            Self::R06CostCeiling => "R06",
            Self::R07Deadline => "R07",
            Self::R08QualityFloor => "R08",
            Self::R09InsufficientEvidence => "R09",
            Self::R10NoEligibleCandidate => "R10",
            Self::R11Ranking => "R11",
            Self::R12Tie => "R12",
            Self::R13PreviousAttempt => "R13",
        }
    }
}

/// The ordered rule list. Order is data, printed by [`Route::order`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// Rules in evaluation order.
    pub rules: [Rule; 12],
}

impl Default for Route {
    /// R13 first (as v3 screens the previous attempt before R02), then R02..R12.
    fn default() -> Self {
        Self {
            rules: [
                Rule::R13PreviousAttempt,
                Rule::R02RequiredCapabilities,
                Rule::R03ContextLimit,
                Rule::R04PrivacyClass,
                Rule::R05Availability,
                Rule::R06CostCeiling,
                Rule::R07Deadline,
                Rule::R08QualityFloor,
                Rule::R09InsufficientEvidence,
                Rule::R10NoEligibleCandidate,
                Rule::R11Ranking,
                Rule::R12Tie,
            ],
        }
    }
}

impl Route {
    /// The rule ids in evaluation order, e.g. `R13 R02 ...`.
    #[must_use]
    pub fn order(&self) -> String {
        self.rules.map(Rule::id).join(" ")
    }
}

/// The model that was chosen, and the rule that chose it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The chosen model's name.
    pub model: String,
    /// The deciding rule.
    pub rule: Rule,
}

/// The closest model to the floor, and what it lacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Best {
    /// Its name.
    pub name: String,
    /// What it lacks (never empty).
    pub missing: Vec<Missing>,
}

/// Why no model was selected. Typed, never a default model.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RouteRefusal {
    /// No model meets the floor.
    #[error("no model meets the capability floor (closest: {best:?})")]
    NoCapableModel {
        /// The floor that was asked.
        floor: CapabilityFloor,
        /// The model missing least, if the roster is not empty.
        best: Option<Best>,
    },
    /// The only candidates were the model whose attempt just failed.
    #[error("only the previous attempt's model remains: {0}")]
    OnlyPreviousAttempt(String),
    /// A rule routed to the baseline and there is no usable baseline.
    #[error(
        "{rule:?} routes to the baseline; baseline {baseline:?} is absent, below the floor or down"
    )]
    NoBaseline {
        /// The rule that asked for the baseline.
        rule: Rule,
        /// The declared baseline, if any.
        baseline: Option<String>,
    },
    /// The rule list ended without deciding (a custom `Route` missing a decision rule).
    #[error("rule list ended without a decision")]
    NoDecision,
}

struct State<'a> {
    pool: Vec<&'a ModelEntry>,
    gap: bool,
}

enum Step {
    Continue,
    Decide(Result<Selection, RouteRefusal>),
}

fn closest(input: &RouteInput, roster: &Roster) -> Option<Best> {
    roster
        .models
        .iter()
        .map(|m| (input.floor.missing(&m.caps), m))
        .min_by_key(|(miss, _)| miss.len())
        .map(|(missing, m)| Best {
            name: m.name.clone(),
            missing,
        })
}

fn floor_filter(
    input: &RouteInput,
    roster: &Roster,
    st: &mut State<'_>,
    keep: impl Fn(&ModelEntry) -> bool,
) -> Step {
    let had = !st.pool.is_empty();
    st.pool.retain(|m| keep(m));
    if had && st.pool.is_empty() {
        Step::Decide(Err(RouteRefusal::NoCapableModel {
            floor: input.floor,
            best: closest(input, roster),
        }))
    } else {
        Step::Continue
    }
}

fn baseline_or_refuse(input: &RouteInput, roster: &Roster, rule: Rule) -> Step {
    let found = input.baseline.as_ref().and_then(|b| {
        roster.models.iter().find(|m| &m.name == b).filter(|m| {
            input.floor.missing(&m.caps).is_empty() && m.availability != Availability::Down
        })
    });
    Step::Decide(match found {
        Some(m) => Ok(Selection {
            model: m.name.clone(),
            rule,
        }),
        None => Err(RouteRefusal::NoBaseline {
            rule,
            baseline: input.baseline.clone(),
        }),
    })
}

impl Rule {
    /// The one site of each rule.
    fn apply(self, input: &RouteInput, roster: &Roster, st: &mut State<'_>) -> Step {
        let f = &input.floor;
        match self {
            Self::R13PreviousAttempt => {
                if let Some(prev) = &input.previous_attempt {
                    let had = !st.pool.is_empty();
                    st.pool.retain(|m| &m.name != prev);
                    if had && st.pool.is_empty() {
                        return Step::Decide(Err(RouteRefusal::OnlyPreviousAttempt(prev.clone())));
                    }
                }
                Step::Continue
            }
            Self::R02RequiredCapabilities => floor_filter(input, roster, st, |m| {
                (!f.json_mode || m.caps.json_mode) && (!f.tool_use || m.caps.tool_use)
            }),
            Self::R03ContextLimit => {
                floor_filter(input, roster, st, |m| m.caps.ctx_tokens >= f.ctx_tokens)
            }
            Self::R04PrivacyClass => {
                floor_filter(input, roster, st, |m| !f.local_only || m.caps.local)
            }
            Self::R05Availability => {
                st.gap = st
                    .pool
                    .iter()
                    .any(|m| m.availability == Availability::Unknown);
                st.pool.retain(|m| m.availability == Availability::Up);
                Step::Continue
            }
            Self::R06CostCeiling => {
                if let Some(c) = input.cost_ceiling_milli {
                    st.pool.retain(|m| m.cost_milli <= c);
                }
                Step::Continue
            }
            Self::R07Deadline => {
                if let Some(d) = input.deadline_ms {
                    st.pool.retain(|m| m.latency_ms <= d);
                }
                Step::Continue
            }
            Self::R08QualityFloor => {
                if let Some(q) = input.quality_floor {
                    st.pool.retain(|m| m.quality >= q);
                }
                Step::Continue
            }
            Self::R09InsufficientEvidence if st.gap => baseline_or_refuse(input, roster, self),
            Self::R10NoEligibleCandidate if st.pool.is_empty() => {
                baseline_or_refuse(input, roster, self)
            }
            Self::R11Ranking => {
                let key = |m: &ModelEntry| (Reverse(m.quality), m.latency_ms, m.cost_milli);
                let Some(top) = st.pool.iter().map(|m| key(m)).min() else {
                    return Step::Continue;
                };
                let mut best = st.pool.iter().filter(|m| key(m) == top);
                match (best.next(), best.next()) {
                    (Some(m), None) => Step::Decide(Ok(Selection {
                        model: m.name.clone(),
                        rule: self,
                    })),
                    _ => Step::Continue,
                }
            }
            Self::R12Tie if st.pool.len() > 1 => baseline_or_refuse(input, roster, self),
            Self::R09InsufficientEvidence | Self::R10NoEligibleCandidate | Self::R12Tie => {
                Step::Continue
            }
        }
    }
}

/// Pick a model for `task` from `roster`, or refuse by name.
///
/// # Errors
/// [`RouteRefusal::NoCapableModel`] when no model meets the floor; the other variants when a
/// rule cannot decide. A default model is never returned.
pub fn select(task: &RouteInput, roster: &Roster) -> Result<Selection, RouteRefusal> {
    select_with(&Route::default(), task, roster)
}

/// [`select`] under an explicit rule order.
///
/// # Errors
/// As [`select`].
pub fn select_with(
    route: &Route,
    task: &RouteInput,
    roster: &Roster,
) -> Result<Selection, RouteRefusal> {
    if roster.models.is_empty() {
        return Err(RouteRefusal::NoCapableModel {
            floor: task.floor,
            best: None,
        });
    }
    let mut st = State {
        pool: roster.models.iter().collect(),
        gap: false,
    };
    for rule in route.rules {
        if let Step::Decide(d) = rule.apply(task, roster, &mut st) {
            return d;
        }
    }
    Err(RouteRefusal::NoDecision)
}

#[cfg(test)]
mod tests {
    type R = Result<(), Box<dyn std::error::Error>>;
    use super::*;

    fn caps(ctx: u32, json: bool, tools: bool, local: bool) -> Capabilities {
        Capabilities {
            ctx_tokens: ctx,
            json_mode: json,
            tool_use: tools,
            local,
        }
    }

    fn m(name: &str, c: Capabilities, q: u32, lat: u32, cost: u32) -> ModelEntry {
        ModelEntry {
            name: name.into(),
            caps: c,
            availability: Availability::Up,
            cost_milli: cost,
            latency_ms: lat,
            quality: q,
        }
    }

    fn floor() -> CapabilityFloor {
        CapabilityFloor {
            ctx_tokens: 8000,
            json_mode: false,
            tool_use: false,
            local_only: false,
        }
    }

    fn input() -> RouteInput {
        RouteInput {
            floor: floor(),
            cost_ceiling_milli: None,
            deadline_ms: None,
            quality_floor: None,
            baseline: None,
            previous_attempt: None,
        }
    }

    fn ok() -> Capabilities {
        caps(32000, true, true, true)
    }

    fn roster(models: Vec<ModelEntry>) -> Roster {
        Roster { models }
    }

    fn refusal_best(
        r: Result<Selection, RouteRefusal>,
    ) -> Result<Option<Best>, Box<dyn std::error::Error>> {
        match r {
            Err(RouteRefusal::NoCapableModel { best, .. }) => Ok(best),
            other => Err(format!("expected NoCapableModel, got {other:?}").into()),
        }
    }

    #[test]
    fn default_order_prints_all_twelve_once() {
        assert_eq!(
            Route::default().order(),
            "R13 R02 R03 R04 R05 R06 R07 R08 R09 R10 R11 R12"
        );
    }

    #[test]
    fn r02_required_capabilities_refuses_naming_the_missing_one() -> R {
        let mut t = input();
        t.floor.tool_use = true;
        let r = roster(vec![m("a", caps(32000, true, false, true), 5, 5, 5)]);
        let best = refusal_best(select(&t, &r))?.ok_or("closest model named")?;
        assert_eq!(best.name, "a");
        assert_eq!(best.missing, vec![Missing::ToolUse]);
        Ok(())
    }

    #[test]
    fn r03_context_limit_names_need_and_have() -> R {
        let r = roster(vec![m("small", caps(4000, true, true, true), 5, 5, 5)]);
        let best = refusal_best(select(&input(), &r))?.ok_or("closest model named")?;
        assert_eq!(
            best.missing,
            vec![Missing::Context {
                need: 8000,
                have: 4000
            }]
        );
        Ok(())
    }

    #[test]
    fn r04_privacy_class_excludes_remote() -> R {
        let mut t = input();
        t.floor.local_only = true;
        let r = roster(vec![
            m("cloud", caps(32000, true, true, false), 9, 1, 1),
            m("loc", ok(), 1, 9, 9),
        ]);
        let s = select(&t, &r)?;
        assert_eq!((s.model.as_str(), s.rule), ("loc", Rule::R11Ranking));
        let only_remote = roster(vec![m("cloud", caps(32000, true, true, false), 9, 1, 1)]);
        let best = refusal_best(select(&t, &only_remote))?.ok_or("closest model named")?;
        assert_eq!(best.missing, vec![Missing::Local]);
        Ok(())
    }

    #[test]
    fn r05_availability_excludes_down() -> R {
        let mut down = m("down", ok(), 9, 1, 1);
        down.availability = Availability::Down;
        let r = roster(vec![down, m("up", ok(), 1, 9, 9)]);
        assert_eq!(select(&input(), &r)?.model, "up");
        Ok(())
    }

    #[test]
    fn r06_cost_ceiling() -> R {
        let mut t = input();
        t.cost_ceiling_milli = Some(10);
        let r = roster(vec![m("dear", ok(), 9, 1, 50), m("cheap", ok(), 1, 9, 5)]);
        assert_eq!(select(&t, &r)?.model, "cheap");
        Ok(())
    }

    #[test]
    fn r07_deadline() -> R {
        let mut t = input();
        t.deadline_ms = Some(100);
        let r = roster(vec![m("slow", ok(), 9, 500, 1), m("fast", ok(), 1, 50, 1)]);
        assert_eq!(select(&t, &r)?.model, "fast");
        Ok(())
    }

    #[test]
    fn r08_quality_floor() -> R {
        let mut t = input();
        t.quality_floor = Some(500);
        let r = roster(vec![
            m("weak", ok(), 100, 1, 1),
            m("strong", ok(), 600, 99, 99),
        ]);
        assert_eq!(select(&t, &r)?.model, "strong");
        Ok(())
    }

    #[test]
    fn r09_unknown_availability_routes_to_baseline() -> R {
        let mut t = input();
        t.baseline = Some("base".into());
        let mut unknown = m("unk", ok(), 9, 1, 1);
        unknown.availability = Availability::Unknown;
        let r = roster(vec![unknown, m("base", ok(), 1, 9, 9)]);
        let s = select(&t, &r)?;
        assert_eq!(
            (s.model.as_str(), s.rule),
            ("base", Rule::R09InsufficientEvidence)
        );
        Ok(())
    }

    #[test]
    fn r10_empty_eligible_set_routes_to_baseline_and_refuses_without_one() -> R {
        let mut t = input();
        t.quality_floor = Some(999);
        let r = roster(vec![m("base", ok(), 1, 1, 1)]);
        assert_eq!(
            select(&t, &r),
            Err(RouteRefusal::NoBaseline {
                rule: Rule::R10NoEligibleCandidate,
                baseline: None
            })
        );
        t.baseline = Some("base".into());
        let s = select(&t, &r)?;
        assert_eq!(s.rule, Rule::R10NoEligibleCandidate);
        Ok(())
    }

    #[test]
    fn r11_ranking_quality_then_latency_then_cost() -> R {
        let r = roster(vec![
            m("q5-slow", ok(), 5, 90, 1),
            m("q5-fast", ok(), 5, 10, 1),
            m("q1", ok(), 1, 1, 1),
        ]);
        let s = select(&input(), &r)?;
        assert_eq!((s.model.as_str(), s.rule), ("q5-fast", Rule::R11Ranking));
        Ok(())
    }

    #[test]
    fn r12_tie_routes_to_baseline() -> R {
        let mut t = input();
        t.baseline = Some("a".into());
        let r = roster(vec![m("a", ok(), 5, 5, 5), m("b", ok(), 5, 5, 5)]);
        let s = select(&t, &r)?;
        assert_eq!((s.model.as_str(), s.rule), ("a", Rule::R12Tie));
        t.baseline = None;
        assert!(matches!(
            select(&t, &r),
            Err(RouteRefusal::NoBaseline {
                rule: Rule::R12Tie,
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn r13_previous_attempt_never_rechosen() -> R {
        let mut t = input();
        t.previous_attempt = Some("best".into());
        let r = roster(vec![m("best", ok(), 9, 1, 1), m("next", ok(), 1, 9, 9)]);
        assert_eq!(select(&t, &r)?.model, "next");
        let only = roster(vec![m("best", ok(), 9, 1, 1)]);
        assert_eq!(
            select(&t, &only),
            Err(RouteRefusal::OnlyPreviousAttempt("best".into()))
        );
        Ok(())
    }

    #[test]
    fn empty_roster_refuses_with_no_best() -> R {
        assert_eq!(refusal_best(select(&input(), &Roster::default()))?, None);
        Ok(())
    }

    #[test]
    fn unmet_floor_never_falls_back_to_baseline() -> R {
        let mut t = input();
        t.floor.json_mode = true;
        t.baseline = Some("a".into());
        let r = roster(vec![m("a", caps(32000, false, true, true), 5, 5, 5)]);
        assert!(refusal_best(select(&t, &r))?.is_some());
        Ok(())
    }
}
