//! T09 routing policy cases (`T09-RT-nn`). Every case constructs its inputs as
//! values, asserts the whole `Route` (arm, reason and the ordered explanation)
//! and names the acceptance obligation it covers. No model call, I/O or clock
//! is involved: the age of every availability observation is a value here.
use habitat_engine::contracts::roster::{Availability, Locality};
use habitat_engine::route::{
    Attempt, ConfigError, Declaration, DeclaredRecipe, EvidenceGap, Exclusion, Explanation,
    Failure, Fallback, Figure, Filter, Gap, Invalid, Key, MAX_CANDIDATES, MAX_QUALITY_BASIS_POINTS,
    MAX_RECIPES, MAX_STALENESS_MS, Observation, Policy, PrivacyClass, Ranked, Recipe, Refusal,
    Route, Routing, Rule, Step, Task, TieRule, evaluate_fallback, route,
};
use serde_json::{Value, json};
use sha2::Digest as _;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::Write as _;

type Outcome = Result<(), Box<dyn Error>>;

/// The reviewed configuration this battery pins; `Policy::load` is its only reader.
const CONFIG: &str = include_str!("../config/routes.toml");
const BASELINE_ID: &str = "09000000-0000-4000-8000-00000000000b";
const BASELINE_REVISION: &str = "1";
const ALPHA: &str = "09000000-0000-4000-8000-0000000000a1";
const BETA: &str = "09000000-0000-4000-8000-0000000000b2";
const GAMMA: &str = "09000000-0000-4000-8000-0000000000c3";
const DELTA: &str = "09000000-0000-4000-8000-0000000000d4";
const CAPS: &[&str] = &["final_output", "identity", "usage"];
const FINAL_ONLY: &[&str] = &["final_output"];
const BOUND_MS: u64 = 30_000;
/// The reviewed policy's revision, computed outside this code from the canonical rendering:
/// `printf 'hee3-route-policy\nschema_version=1\nfilters=required_capabilities,context_limit,`
/// `privacy_class,availability,cost_ceiling,deadline,quality_floor\nranking=cost,quality,latency`
/// `\ntie=baseline\nstaleness_bound_ms=30000\nbaseline=09000000-0000-4000-8000-00000000000b\n'`
/// `| sha256sum` (coreutils 9, 2026-09-24).
const REVIEWED_REVISION: &str =
    "sha256:1bbf286ff0085a0824be6b4e1b88a24096cf7387306e72111c89e0272bf94358";
/// The same rendering with the first two filters swapped, by the same independent command.
const SWAPPED_FILTERS_REVISION: &str =
    "sha256:4884a34fddd749823ced5cb413d2b05b6d533c56d67cbe822295250fe061b75c";
const FILTER_RULES: [Rule; 7] = [
    Rule::R02RequiredCapabilities,
    Rule::R03ContextLimit,
    Rule::R04PrivacyClass,
    Rule::R05Availability,
    Rule::R06CostCeiling,
    Rule::R07Deadline,
    Rule::R08QualityFloor,
];

fn observed(availability: Availability, age_ms: u64) -> Observation {
    Observation::Observed {
        availability,
        age_ms,
    }
}
fn fresh() -> Observation {
    observed(Availability::Available, 1_000)
}
fn baseline() -> Recipe<'static> {
    Recipe {
        id: BASELINE_ID,
        revision: BASELINE_REVISION,
        capabilities: CAPS,
        context_limit_tokens: Some(32_768),
        locality: Locality::Local,
        availability: fresh(),
        cost_microunits: Some(0),
        quality_basis_points: Some(5_000),
        latency_ms: Some(1_000),
    }
}
fn recipe(id: &'static str) -> Recipe<'static> {
    Recipe {
        id,
        revision: "7",
        capabilities: CAPS,
        context_limit_tokens: Some(32_768),
        locality: Locality::Local,
        availability: fresh(),
        cost_microunits: Some(100),
        quality_basis_points: Some(9_000),
        latency_ms: Some(500),
    }
}
fn task() -> Task<'static> {
    Task {
        required_capabilities: FINAL_ONLY,
        context_tokens: 8_192,
        privacy: PrivacyClass::LocalOnly,
        cost_ceiling_microunits: None,
        deadline_ms: None,
        quality_floor_basis_points: None,
    }
}
fn policy() -> Result<Policy, ConfigError> {
    Policy::load(CONFIG, &baseline())
}
fn revision() -> Result<String, ConfigError> {
    Ok(policy()?.revision().to_owned())
}
fn declaration() -> Declaration {
    Declaration {
        schema_version: 1,
        filters: Filter::ALL.iter().map(|f| f.name().to_owned()).collect(),
        ranking: Key::ALL.iter().map(|k| k.name().to_owned()).collect(),
        tie: "baseline".to_owned(),
        staleness_bound_ms: BOUND_MS,
        baseline: BASELINE_ID.to_owned(),
    }
}
fn declared(mutate: impl FnOnce(&mut Declaration)) -> Result<Policy, ConfigError> {
    let mut declaration = declaration();
    mutate(&mut declaration);
    Policy::declare(declaration, &baseline())
}
fn decide(candidates: &[Recipe<'static>]) -> Result<Route<'static>, Box<dyn Error>> {
    Ok(route(&policy()?, &task(), candidates, &baseline())?)
}
fn decide_task(
    task: &Task<'static>,
    candidates: &[Recipe<'static>],
) -> Result<Route<'static>, Box<dyn Error>> {
    Ok(route(&policy()?, task, candidates, &baseline())?)
}
fn chosen<'a>(route: &'a Route<'a>) -> Option<(&'a str, &'a str)> {
    match route {
        Route::Chosen {
            recipe, revision, ..
        } => Some((recipe, revision)),
        _ => None,
    }
}
fn fallback<'a>(route: &'a Route<'_>) -> Option<(&'a str, &'a str, &'a Fallback<'a>)> {
    match route {
        Route::Baseline {
            recipe,
            revision,
            reason,
            ..
        } => Some((recipe, revision, reason)),
        _ => None,
    }
}
fn refusal<'a>(route: &'a Route<'a>) -> Option<(Refusal<'a>, &'a Fallback<'a>)> {
    match route {
        Route::Refused {
            reason, fallback, ..
        } => Some((*reason, fallback)),
        _ => None,
    }
}
fn excluded_by<'a>(route: &'a Route<'a>, recipe: &str) -> Option<(Vec<Rule>, Rule, Exclusion<'a>)> {
    route
        .explanation()
        .steps
        .iter()
        .find_map(|step| match step {
            Step::Excluded {
                recipe: id,
                passed,
                rule,
                why,
            } if *id == recipe => Some((passed.clone(), *rule, *why)),
            _ => None,
        })
}
fn gap_of<'a>(route: &'a Route<'a>, recipe: &str) -> Option<(Vec<Rule>, Rule, Gap)> {
    route
        .explanation()
        .steps
        .iter()
        .find_map(|step| match step {
            Step::Gap {
                recipe: id,
                passed,
                rule,
                evidence,
            } if *id == recipe => Some((passed.clone(), *rule, *evidence)),
            _ => None,
        })
}
fn guarded() -> Step<'static> {
    Step::Guarded {
        recipe: BASELINE_ID,
        passed: FILTER_RULES.to_vec(),
    }
}
fn ranked(recipe: &Recipe<'static>) -> Ranked<'static> {
    Ranked {
        recipe: recipe.id,
        cost_microunits: recipe.cost_microunits.unwrap_or(u64::MAX),
        quality_basis_points: recipe.quality_basis_points.unwrap_or(0),
        latency_ms: recipe.latency_ms.unwrap_or(u64::MAX),
    }
}

// ---------------------------------------------------------------- configuration

/// T09-RT-01 · the reviewed `config/routes.toml` loads and every declared value is read back:
/// filter order, ranking order, the tie rule, the staleness bound and the baseline identity.
#[test]
fn reviewed_configuration_loads_with_every_declared_value() -> Outcome {
    let policy = policy()?;
    assert_eq!(policy.filters(), &Filter::ALL);
    assert_eq!(policy.ranking(), &[Key::Cost, Key::Quality, Key::Latency]);
    assert_eq!(policy.tie(), TieRule::Baseline);
    assert_eq!(policy.tie().name(), "baseline");
    assert_eq!(policy.staleness_bound_ms(), BOUND_MS);
    assert_eq!(policy.baseline(), BASELINE_ID);
    Ok(())
}

/// T09-RT-02 · an unknown top-level key is refused by name; an unknown key inside the
/// baseline table is refused by its dotted path.
#[test]
fn unknown_keys_are_refused_by_name() {
    let top = CONFIG.replace("[baseline]", "extra = 1\n[baseline]");
    assert_eq!(
        Policy::load(&top, &baseline()),
        Err(ConfigError::UnknownKey {
            key: "extra".to_owned()
        })
    );
    let nested = format!("{CONFIG}\nmodel = \"x\"\n");
    assert_eq!(
        Policy::load(&nested, &baseline()),
        Err(ConfigError::UnknownKey {
            key: "baseline.model".to_owned()
        })
    );
}

/// T09-RT-03 · a missing baseline is refused in each of its three shapes: no `[baseline]`
/// table, a table without `recipe`, and an empty `recipe`.
#[test]
fn missing_baseline_is_refused_in_every_shape() {
    let without_table = CONFIG.split("[baseline]").next().unwrap_or_default();
    assert_eq!(
        Policy::load(without_table, &baseline()),
        Err(ConfigError::MissingBaseline)
    );
    let without_recipe = format!("{without_table}[baseline]\n");
    assert_eq!(
        Policy::load(&without_recipe, &baseline()),
        Err(ConfigError::MissingBaseline)
    );
    assert_eq!(
        declared(|d| d.baseline = String::new()),
        Err(ConfigError::MissingBaseline)
    );
}

/// T09-RT-04 · filter declarations: a duplicate, a missing and an unknown filter are each
/// refused naming the filter; the world is `Filter::ALL`, not the author's list.
#[test]
fn filter_declaration_faults_are_refused_by_name() {
    assert_eq!(
        declared(|d| d.filters.push("availability".to_owned())),
        Err(ConfigError::DuplicateRule {
            name: "availability".to_owned()
        })
    );
    assert_eq!(
        declared(|d| {
            d.filters.retain(|name| name != "deadline");
        }),
        Err(ConfigError::MissingRule {
            name: "deadline".to_owned()
        })
    );
    assert_eq!(
        declared(|d| d.filters[0] = "budget".to_owned()),
        Err(ConfigError::UnknownRule {
            name: "budget".to_owned()
        })
    );
}

/// T09-RT-05 · ranking declarations: a duplicate, a missing and an unknown key are each
/// refused naming the key; the world is `Key::ALL`.
#[test]
fn ranking_declaration_faults_are_refused_by_name() {
    assert_eq!(
        declared(|d| d.ranking.push("cost".to_owned())),
        Err(ConfigError::DuplicateRankingKey {
            name: "cost".to_owned()
        })
    );
    assert_eq!(
        declared(|d| d.ranking = vec!["cost".to_owned(), "quality".to_owned()]),
        Err(ConfigError::MissingRankingKey {
            name: "latency".to_owned()
        })
    );
    assert_eq!(
        declared(|d| d.ranking[1] = "speed".to_owned()),
        Err(ConfigError::UnknownRankingKey {
            name: "speed".to_owned()
        })
    );
}

/// T09-RT-06 · the tie rule admits only `baseline`; any other spelling is refused by name.
#[test]
fn tie_rule_admits_only_baseline() {
    assert_eq!(
        declared(|d| d.tie = "lowest_id".to_owned()),
        Err(ConfigError::UnknownTieRule {
            name: "lowest_id".to_owned()
        })
    );
    assert_eq!(
        declared(|d| d.tie = "Baseline".to_owned()),
        Err(ConfigError::UnknownTieRule {
            name: "Baseline".to_owned()
        })
    );
}

/// T09-RT-07 · the staleness bound is accepted on `1..=60000` and refused at `0`, `60001` and
/// below zero, each naming the value; the ceiling is the roster's TTL ceiling.
#[test]
fn staleness_bound_is_refused_outside_the_roster_ttl_range() -> Outcome {
    assert_eq!(
        declared(|d| d.staleness_bound_ms = 0),
        Err(ConfigError::StalenessBound { value: 0 })
    );
    assert_eq!(
        declared(|d| d.staleness_bound_ms = MAX_STALENESS_MS + 1),
        Err(ConfigError::StalenessBound { value: 60_001 })
    );
    assert_eq!(
        declared(|d| d.staleness_bound_ms = MAX_STALENESS_MS)?.staleness_bound_ms(),
        60_000
    );
    assert_eq!(
        declared(|d| d.staleness_bound_ms = 1)?.staleness_bound_ms(),
        1
    );
    let negative = CONFIG.replace("staleness_bound_ms = 30000", "staleness_bound_ms = -5");
    assert_eq!(
        Policy::load(&negative, &baseline()),
        Err(ConfigError::StalenessBound { value: -5 })
    );
    Ok(())
}

/// T09-RT-08 · a baseline that is not local is ineligible by construction under the local-only
/// profile and is refused by name, for both `Remote` and `Hybrid`.
#[test]
fn baseline_that_is_not_local_is_refused_by_name() {
    for locality in [Locality::Remote, Locality::Hybrid] {
        let mut recipe = baseline();
        recipe.locality = locality;
        assert_eq!(
            Policy::load(CONFIG, &recipe),
            Err(ConfigError::BaselineNotLocal {
                recipe: BASELINE_ID.to_owned(),
                locality
            })
        );
    }
}

/// T09-RT-09 · a baseline lacking a figure the ranking reads is refused naming the first
/// missing figure in the declared key order (quality before latency when both are absent).
#[test]
fn baseline_missing_a_ranking_figure_is_refused_naming_the_first_missing() {
    let mut both = baseline();
    both.quality_basis_points = None;
    both.latency_ms = None;
    assert_eq!(
        Policy::load(CONFIG, &both),
        Err(ConfigError::BaselineMissingFigure {
            recipe: BASELINE_ID.to_owned(),
            figure: Figure::Quality
        })
    );
    let mut latency = baseline();
    latency.latency_ms = None;
    assert_eq!(
        Policy::load(CONFIG, &latency),
        Err(ConfigError::BaselineMissingFigure {
            recipe: BASELINE_ID.to_owned(),
            figure: Figure::Latency
        })
    );
    let mut cost = baseline();
    cost.cost_microunits = None;
    assert_eq!(
        Policy::load(CONFIG, &cost),
        Err(ConfigError::BaselineMissingFigure {
            recipe: BASELINE_ID.to_owned(),
            figure: Figure::Cost
        })
    );
}

/// T09-RT-10 · the declared baseline identity is bound to the supplied baseline value: a
/// different id is refused naming both sides; an invalid declared id is refused as identity.
#[test]
fn baseline_identity_is_bound_to_the_supplied_recipe() {
    let mut other = baseline();
    other.id = ALPHA;
    assert_eq!(
        Policy::load(CONFIG, &other),
        Err(ConfigError::BaselineMismatch {
            declared: BASELINE_ID.to_owned(),
            supplied: ALPHA.to_owned()
        })
    );
    assert_eq!(
        declared(|d| d.baseline = "caf\u{e9}".to_owned()),
        Err(ConfigError::BaselineIdentity {
            recipe: "caf\u{e9}".to_owned()
        })
    );
}

/// T09-RT-11 · malformed TOML, a mistyped value and an unsupported schema version are each
/// refused by their own name; a missing required key names the key.
#[test]
fn syntax_type_schema_and_missing_key_faults_are_distinct() {
    assert_eq!(
        Policy::load("filters = [", &baseline()),
        Err(ConfigError::Syntax)
    );
    let mistyped = CONFIG.replace("tie = \"baseline\"", "tie = 1");
    assert_eq!(
        Policy::load(&mistyped, &baseline()),
        Err(ConfigError::WrongType {
            key: "tie".to_owned()
        })
    );
    let mixed = CONFIG.replace(
        "ranking = [\"cost\", \"quality\", \"latency\"]",
        "ranking = [\"cost\", 2]",
    );
    assert_eq!(
        Policy::load(&mixed, &baseline()),
        Err(ConfigError::WrongType {
            key: "ranking".to_owned()
        })
    );
    let schema = CONFIG.replace("schema_version = 1", "schema_version = 2");
    assert_eq!(
        Policy::load(&schema, &baseline()),
        Err(ConfigError::SchemaVersion { found: 2 })
    );
    let missing = CONFIG.replace("tie = \"baseline\"", "");
    assert_eq!(
        Policy::load(&missing, &baseline()),
        Err(ConfigError::MissingKey {
            key: "tie".to_owned()
        })
    );
}

/// T09-RT-12 · the values door and the text door validate identically: a declaration built
/// from `Filter::ALL` and `Key::ALL` names yields the same policy as the reviewed file.
#[test]
fn declared_values_and_loaded_text_yield_the_same_policy() -> Outcome {
    assert_eq!(declared(|_| {})?, policy()?);
    assert_eq!(
        Filter::ALL.map(Filter::rule),
        FILTER_RULES,
        "filter rules R02..R08 in declared order"
    );
    Ok(())
}

// ------------------------------------------------------------ structural input

/// T09-RT-13 · the candidate set is bounded at the roster's record count: 256 route, 257 refuse
/// with both numbers.
#[test]
fn candidate_count_is_bounded_at_the_roster_limit() -> Outcome {
    let ids: Vec<String> = (0..=MAX_CANDIDATES)
        .map(|i| format!("cand-{i:03}"))
        .collect();
    let make = |n: usize| -> Vec<Recipe<'_>> {
        ids[..n]
            .iter()
            .map(|id| {
                let mut recipe = recipe(ALPHA);
                recipe.id = id.as_str();
                recipe
            })
            .collect()
    };
    let policy = policy()?;
    let full = make(MAX_CANDIDATES);
    assert!(route(&policy, &task(), &full, &baseline()).is_ok());
    let over = make(MAX_CANDIDATES + 1);
    assert_eq!(
        route(&policy, &task(), &over, &baseline()),
        Err(Invalid::TooManyCandidates {
            count: 257,
            limit: 256
        })
    );
    Ok(())
}

/// T09-RT-14 · a duplicate candidate identity and a baseline listed among the candidates are
/// refused by name before any rule runs.
#[test]
fn duplicate_and_baseline_among_candidates_are_refused() -> Outcome {
    let policy = policy()?;
    assert_eq!(
        route(
            &policy,
            &task(),
            &[recipe(ALPHA), recipe(ALPHA)],
            &baseline()
        ),
        Err(Invalid::DuplicateIdentity {
            recipe: ALPHA.to_owned()
        })
    );
    assert_eq!(
        route(&policy, &task(), &[recipe(ALPHA), baseline()], &baseline()),
        Err(Invalid::BaselineAmongCandidates {
            recipe: BASELINE_ID.to_owned()
        })
    );
    Ok(())
}

/// T09-RT-15 · at route time the supplied baseline must be the policy's: another id is
/// refused naming the declared and the supplied identities.
#[test]
fn route_time_baseline_must_match_the_policy() -> Outcome {
    let mut other = baseline();
    other.id = DELTA;
    assert_eq!(
        route(&policy()?, &task(), &[recipe(ALPHA)], &other),
        Err(Invalid::BaselineMismatch {
            declared: BASELINE_ID.to_owned(),
            supplied: DELTA.to_owned()
        })
    );
    Ok(())
}

/// T09-RT-16 · identity and capability text is bounded like the roster's: an empty id, a
/// control character, a 129-byte revision, a duplicate capability and an out-of-range quality
/// are each refused by their own name and recipe.
#[test]
fn identity_capability_and_quality_bounds_are_refused_by_name() -> Outcome {
    let policy = policy()?;
    let long = "r".repeat(129);
    let mut empty = recipe(ALPHA);
    empty.id = "";
    let mut control = recipe(ALPHA);
    control.revision = "1\n";
    let mut oversize = recipe(ALPHA);
    oversize.revision = long.as_str();
    for faulty in [empty, control, oversize] {
        assert_eq!(
            route(&policy, &task(), &[faulty], &baseline()),
            Err(Invalid::Identity {
                recipe: faulty.id.to_owned()
            })
        );
    }
    let mut duplicate = recipe(BETA);
    duplicate.capabilities = &["usage", "usage"];
    assert_eq!(
        route(&policy, &task(), &[duplicate], &baseline()),
        Err(Invalid::Capabilities {
            recipe: Some(BETA.to_owned())
        })
    );
    let mut over = recipe(GAMMA);
    over.quality_basis_points = Some(MAX_QUALITY_BASIS_POINTS + 1);
    assert_eq!(
        route(&policy, &task(), &[over], &baseline()),
        Err(Invalid::QualityRange {
            recipe: Some(GAMMA.to_owned())
        })
    );
    let mut floor = task();
    floor.quality_floor_basis_points = Some(10_001);
    assert_eq!(
        route(&policy, &floor, &[recipe(ALPHA)], &baseline()),
        Err(Invalid::QualityRange { recipe: None })
    );
    let mut caps = task();
    caps.required_capabilities = &["", "identity"];
    assert_eq!(
        route(&policy, &caps, &[recipe(ALPHA)], &baseline()),
        Err(Invalid::Capabilities { recipe: None })
    );
    Ok(())
}

// -------------------------------------------------------------- hard filters

/// T09-RT-17 · R02 required capabilities: a recipe lacking two required capabilities is
/// excluded naming the first missing one in the task's order, having passed nothing.
#[test]
fn missing_capability_excludes_naming_the_first_missing() -> Outcome {
    let mut task = task();
    task.required_capabilities = &["final_output", "identity", "usage"];
    let mut bare = recipe(ALPHA);
    bare.capabilities = FINAL_ONLY;
    let decision = decide_task(&task, &[bare])?;
    assert_eq!(
        excluded_by(&decision, ALPHA),
        Some((
            vec![],
            Rule::R02RequiredCapabilities,
            Exclusion::MissingCapability {
                capability: "identity"
            }
        ))
    );
    assert_eq!(
        fallback(&decision).map(|f| f.2),
        Some(&Fallback::NoEligibleCandidate)
    );
    Ok(())
}

/// T09-RT-18 · R03 context limit at the boundary: a limit equal to the task's context passes;
/// one token less excludes with both numbers.
#[test]
fn context_limit_boundary_is_inclusive() -> Outcome {
    let mut exact = recipe(ALPHA);
    exact.context_limit_tokens = Some(8_192);
    assert_eq!(chosen(&decide(&[exact])?), Some((ALPHA, "7")));
    let mut short = recipe(BETA);
    short.context_limit_tokens = Some(8_191);
    let decision = decide(&[short])?;
    assert_eq!(
        excluded_by(&decision, BETA),
        Some((
            vec![Rule::R02RequiredCapabilities],
            Rule::R03ContextLimit,
            Exclusion::ContextExceeded {
                limit_tokens: 8_191,
                required_tokens: 8_192
            }
        ))
    );
    Ok(())
}

/// T09-RT-62 · R03 with no known context limit. The RC03 roster definition declares no limit, so
/// the router must not invent one: the recipe is a gap at R03 naming the missing figure — never
/// eligible, never excluded with a made-up number — and that one gap routes to the baseline by R09
/// despite an eligible challenger, identically in every candidate order. A baseline with no known
/// limit still permits a strict choice, and is refused by R01 carrying the gap when a fallback
/// needs it. Route qualification gap route-G4 (docs/modules/route.md: "Missing measurements stay
/// unknown").
#[test]
fn unknown_context_limit_is_a_gap_at_r03_never_an_invented_number() -> Outcome {
    let mut unknown = recipe(BETA);
    unknown.context_limit_tokens = None;
    let missing = Gap::MissingFigure {
        figure: Figure::ContextLimit,
    };
    let mut remote = recipe(GAMMA);
    remote.locality = Locality::Hybrid;
    let mut worse = recipe(DELTA);
    worse.cost_microunits = Some(200);
    let decision = permutation_invariant(&[recipe(ALPHA), unknown, remote, worse])?;
    assert_eq!(
        decision,
        Route::Baseline {
            recipe: BASELINE_ID,
            revision: BASELINE_REVISION,
            reason: Fallback::InsufficientEvidence {
                gaps: vec![EvidenceGap {
                    recipe: BETA,
                    rule: Rule::R03ContextLimit,
                    evidence: missing,
                }]
            },
            explanation: Explanation {
                policy_revision: revision()?,
                steps: vec![
                    guarded(),
                    Step::Eligible {
                        recipe: ALPHA,
                        passed: FILTER_RULES.to_vec()
                    },
                    Step::Gap {
                        recipe: BETA,
                        passed: vec![Rule::R02RequiredCapabilities],
                        rule: Rule::R03ContextLimit,
                        evidence: missing
                    },
                    Step::Excluded {
                        recipe: GAMMA,
                        passed: FILTER_RULES[..2].to_vec(),
                        rule: Rule::R04PrivacyClass,
                        why: Exclusion::PrivacyViolated {
                            locality: Locality::Hybrid
                        }
                    },
                    Step::Eligible {
                        recipe: DELTA,
                        passed: FILTER_RULES.to_vec()
                    },
                    Step::Decided {
                        rule: Rule::R09InsufficientEvidence
                    },
                ]
            }
        }
    );
    let mut unbounded = baseline();
    unbounded.context_limit_tokens = None;
    let policy = Policy::load(CONFIG, &unbounded)?;
    let strict = route(&policy, &task(), &[recipe(ALPHA)], &unbounded)?;
    assert_eq!(chosen(&strict), Some((ALPHA, "7")));
    assert_eq!(
        strict.explanation().steps.first(),
        Some(&Step::Gap {
            recipe: BASELINE_ID,
            passed: vec![Rule::R02RequiredCapabilities],
            rule: Rule::R03ContextLimit,
            evidence: missing
        })
    );
    let refused = route(&policy, &task(), &[], &unbounded)?;
    assert_eq!(
        refusal(&refused),
        Some((
            Refusal::BaselineEvidence {
                rule: Rule::R03ContextLimit,
                evidence: missing
            },
            &Fallback::NoEligibleCandidate
        ))
    );
    assert_eq!(refused.dispatches(), None);
    Ok(())
}

/// T09-RT-19 · R04 privacy class: a local-only task excludes `Remote` and `Hybrid` recipes,
/// naming the locality; a remote-allowed task admits both.
#[test]
fn privacy_class_excludes_non_local_recipes_for_local_only_tasks() -> Outcome {
    for locality in [Locality::Remote, Locality::Hybrid] {
        let mut recipe = recipe(ALPHA);
        recipe.locality = locality;
        let decision = decide(&[recipe])?;
        assert_eq!(
            excluded_by(&decision, ALPHA),
            Some((
                vec![Rule::R02RequiredCapabilities, Rule::R03ContextLimit],
                Rule::R04PrivacyClass,
                Exclusion::PrivacyViolated { locality }
            ))
        );
        let mut open = task();
        open.privacy = PrivacyClass::RemoteAllowed;
        assert_eq!(chosen(&decide_task(&open, &[recipe])?), Some((ALPHA, "7")));
    }
    Ok(())
}

/// T09-RT-20 · R05 availability: a fresh `Unavailable` observation excludes, naming its age.
#[test]
fn fresh_unavailable_observation_excludes() -> Outcome {
    let mut down = recipe(ALPHA);
    down.availability = observed(Availability::Unavailable, 2_500);
    let decision = decide(&[down])?;
    assert_eq!(
        excluded_by(&decision, ALPHA),
        Some((
            vec![
                Rule::R02RequiredCapabilities,
                Rule::R03ContextLimit,
                Rule::R04PrivacyClass
            ],
            Rule::R05Availability,
            Exclusion::Unavailable { age_ms: 2_500 }
        ))
    );
    Ok(())
}

/// T09-RT-21 · R06 cost ceiling at the boundary: cost equal to the ceiling passes; one unit
/// above excludes with both numbers; without a ceiling the rule does not apply.
#[test]
fn cost_ceiling_boundary_is_inclusive() -> Outcome {
    let mut task = task();
    task.cost_ceiling_microunits = Some(100);
    assert_eq!(
        chosen(&decide_task(&task, &[recipe(ALPHA)])?),
        Some((ALPHA, "7"))
    );
    let mut over = recipe(BETA);
    over.cost_microunits = Some(101);
    let decision = decide_task(&task, &[over])?;
    assert_eq!(
        excluded_by(&decision, BETA),
        Some((
            FILTER_RULES[..4].to_vec(),
            Rule::R06CostCeiling,
            Exclusion::CostAboveCeiling {
                cost_microunits: 101,
                ceiling_microunits: 100
            }
        ))
    );
    assert_eq!(chosen(&decide(&[over])?), Some((BETA, "7")));
    Ok(())
}

/// T09-RT-22 · R07 deadline at the boundary: latency equal to the deadline passes; one
/// millisecond above excludes with both numbers.
#[test]
fn deadline_boundary_is_inclusive() -> Outcome {
    let mut task = task();
    task.deadline_ms = Some(500);
    assert_eq!(
        chosen(&decide_task(&task, &[recipe(ALPHA)])?),
        Some((ALPHA, "7"))
    );
    let mut slow = recipe(BETA);
    slow.latency_ms = Some(501);
    let decision = decide_task(&task, &[slow])?;
    assert_eq!(
        excluded_by(&decision, BETA),
        Some((
            FILTER_RULES[..5].to_vec(),
            Rule::R07Deadline,
            Exclusion::LatencyAboveDeadline {
                latency_ms: 501,
                deadline_ms: 500
            }
        ))
    );
    Ok(())
}

/// T09-RT-23 · R08 quality floor at the boundary: quality equal to the floor passes; one basis
/// point below excludes with both numbers.
#[test]
fn quality_floor_boundary_is_inclusive() -> Outcome {
    let mut task = task();
    task.quality_floor_basis_points = Some(9_000);
    assert_eq!(
        chosen(&decide_task(&task, &[recipe(ALPHA)])?),
        Some((ALPHA, "7"))
    );
    let mut weak = recipe(BETA);
    weak.quality_basis_points = Some(8_999);
    let decision = decide_task(&task, &[weak])?;
    assert_eq!(
        excluded_by(&decision, BETA),
        Some((
            FILTER_RULES[..6].to_vec(),
            Rule::R08QualityFloor,
            Exclusion::QualityBelowFloor {
                quality_basis_points: 8_999,
                floor_basis_points: 9_000
            }
        ))
    );
    Ok(())
}

/// T09-RT-24 · filters in combination: a recipe failing capability, privacy and availability
/// at once is excluded by the earliest declared rule only, having passed nothing.
#[test]
fn combined_failures_are_attributed_to_the_earliest_declared_filter() -> Outcome {
    let mut recipe = recipe(ALPHA);
    recipe.capabilities = &["usage"];
    recipe.locality = Locality::Remote;
    recipe.availability = observed(Availability::Unavailable, 10);
    let decision = decide(&[recipe])?;
    assert_eq!(
        excluded_by(&decision, ALPHA),
        Some((
            vec![],
            Rule::R02RequiredCapabilities,
            Exclusion::MissingCapability {
                capability: "final_output"
            }
        ))
    );
    assert_eq!(
        decision.explanation().exclusions(),
        vec![(ALPHA, Rule::R02RequiredCapabilities)]
    );
    Ok(())
}

/// T09-RT-25 · the filter order is the declaration's, not a fixed one: with `privacy_class`
/// declared first, the same doubly-failing recipe is excluded by R04 instead of R02.
#[test]
fn filter_order_is_taken_from_the_declaration() -> Outcome {
    let policy = declared(|d| d.filters.swap(0, 2))?;
    assert_eq!(policy.filters()[0], Filter::PrivacyClass);
    let mut recipe = recipe(ALPHA);
    recipe.capabilities = &["usage"];
    recipe.locality = Locality::Remote;
    let decision = route(&policy, &task(), &[recipe], &baseline())?;
    assert_eq!(
        excluded_by(&decision, ALPHA),
        Some((
            vec![],
            Rule::R04PrivacyClass,
            Exclusion::PrivacyViolated {
                locality: Locality::Remote
            }
        ))
    );
    Ok(())
}

/// T09-RT-26 · filters are never traded against ranking: a remote recipe with zero cost and
/// perfect quality is excluded while a dearer, weaker local recipe is chosen.
#[test]
fn a_remote_superstar_cannot_outrank_privacy() -> Outcome {
    let mut star = recipe(ALPHA);
    star.locality = Locality::Remote;
    star.cost_microunits = Some(0);
    star.quality_basis_points = Some(10_000);
    star.latency_ms = Some(1);
    let decision = decide(&[star, recipe(BETA)])?;
    assert_eq!(chosen(&decision), Some((BETA, "7")));
    assert_eq!(
        decision.explanation().exclusions(),
        vec![(ALPHA, Rule::R04PrivacyClass)]
    );
    Ok(())
}

// ------------------------------------------------------------ evidence gaps

/// T09-RT-27 · an unobserved recipe is an evidence gap at R05: the decision is the baseline,
/// naming the recipe, the rule and the gap; nothing was excluded.
#[test]
fn unobserved_recipe_routes_to_the_baseline() -> Outcome {
    let mut silent = recipe(ALPHA);
    silent.availability = Observation::Unobserved;
    let decision = decide(&[silent])?;
    assert_eq!(
        fallback(&decision),
        Some((
            BASELINE_ID,
            BASELINE_REVISION,
            &Fallback::InsufficientEvidence {
                gaps: vec![EvidenceGap {
                    recipe: ALPHA,
                    rule: Rule::R05Availability,
                    evidence: Gap::Unobserved
                }]
            }
        ))
    );
    assert_eq!(decision.explanation().exclusions(), vec![]);
    assert_eq!(
        decision.explanation().decided_by(),
        Some(Rule::R09InsufficientEvidence)
    );
    Ok(())
}

/// T09-RT-28 · a fresh `Unknown` availability is a gap, not an exclusion, and carries its age.
#[test]
fn unknown_availability_is_a_gap_with_its_age() -> Outcome {
    let mut unsure = recipe(ALPHA);
    unsure.availability = observed(Availability::Unknown, 40);
    let decision = decide(&[unsure])?;
    assert_eq!(
        gap_of(&decision, ALPHA),
        Some((
            vec![
                Rule::R02RequiredCapabilities,
                Rule::R03ContextLimit,
                Rule::R04PrivacyClass
            ],
            Rule::R05Availability,
            Gap::AvailabilityUnknown { age_ms: 40 }
        ))
    );
    assert!(matches!(decision, Route::Baseline { .. }));
    Ok(())
}

/// T09-RT-29 · staleness at the boundary: an age equal to the bound is stale (a gap naming age
/// and bound); one millisecond younger is fresh and the recipe is chosen.
#[test]
fn staleness_bound_is_reached_at_equality() -> Outcome {
    let mut stale = recipe(ALPHA);
    stale.availability = observed(Availability::Available, BOUND_MS);
    let decision = decide(&[stale])?;
    assert_eq!(
        gap_of(&decision, ALPHA).map(|g| (g.1, g.2)),
        Some((
            Rule::R05Availability,
            Gap::StaleAvailability {
                age_ms: 30_000,
                bound_ms: 30_000
            }
        ))
    );
    let mut fresh_enough = recipe(BETA);
    fresh_enough.availability = observed(Availability::Available, BOUND_MS - 1);
    assert_eq!(chosen(&decide(&[fresh_enough])?), Some((BETA, "7")));
    Ok(())
}

/// T09-RT-30 · the staleness bound is the declared one: under a 5000 ms bound an age of 5000
/// is stale and 4999 is fresh, with the declared bound reported in the gap.
#[test]
fn staleness_bound_is_the_declared_value() -> Outcome {
    let policy = declared(|d| d.staleness_bound_ms = 5_000)?;
    let mut stale = recipe(ALPHA);
    stale.availability = observed(Availability::Available, 5_000);
    let decision = route(&policy, &task(), &[stale], &baseline())?;
    assert_eq!(
        gap_of(&decision, ALPHA).map(|g| g.2),
        Some(Gap::StaleAvailability {
            age_ms: 5_000,
            bound_ms: 5_000
        })
    );
    let mut fresh_enough = recipe(BETA);
    fresh_enough.availability = observed(Availability::Available, 4_999);
    let decision = route(&policy, &task(), &[fresh_enough], &baseline())?;
    assert_eq!(chosen(&decision), Some((BETA, "7")));
    Ok(())
}

/// T09-RT-31 · a missing cost figure under a cost ceiling is a gap at R06 naming the figure.
#[test]
fn missing_cost_under_a_ceiling_is_a_gap() -> Outcome {
    let mut task = task();
    task.cost_ceiling_microunits = Some(1_000);
    let mut unpriced = recipe(ALPHA);
    unpriced.cost_microunits = None;
    let decision = decide_task(&task, &[unpriced])?;
    assert_eq!(
        gap_of(&decision, ALPHA),
        Some((
            FILTER_RULES[..4].to_vec(),
            Rule::R06CostCeiling,
            Gap::MissingFigure {
                figure: Figure::Cost
            }
        ))
    );
    Ok(())
}

/// T09-RT-32 · a missing latency under a deadline is a gap at R07; a missing quality under a
/// floor is a gap at R08; each names its figure and its rule.
#[test]
fn missing_latency_or_quality_under_a_bound_is_a_gap_at_that_rule() -> Outcome {
    let mut deadline = task();
    deadline.deadline_ms = Some(900);
    let mut untimed = recipe(ALPHA);
    untimed.latency_ms = None;
    let decision = decide_task(&deadline, &[untimed])?;
    assert_eq!(
        gap_of(&decision, ALPHA).map(|g| (g.1, g.2)),
        Some((
            Rule::R07Deadline,
            Gap::MissingFigure {
                figure: Figure::Latency
            }
        ))
    );
    let mut floor = task();
    floor.quality_floor_basis_points = Some(1);
    let mut unscored = recipe(BETA);
    unscored.quality_basis_points = None;
    let decision = decide_task(&floor, &[unscored])?;
    assert_eq!(
        gap_of(&decision, BETA).map(|g| (g.1, g.2)),
        Some((
            Rule::R08QualityFloor,
            Gap::MissingFigure {
                figure: Figure::Quality
            }
        ))
    );
    Ok(())
}

/// T09-RT-33 · without any task bound a missing ranking figure is still a gap, at R11, naming
/// the first missing figure in the declared key order after every filter passed.
#[test]
fn missing_ranking_figure_is_a_gap_at_ranking() -> Outcome {
    let mut recipe = recipe(ALPHA);
    recipe.quality_basis_points = None;
    recipe.latency_ms = None;
    let decision = decide(&[recipe])?;
    assert_eq!(
        gap_of(&decision, ALPHA),
        Some((
            FILTER_RULES.to_vec(),
            Rule::R11Ranking,
            Gap::MissingFigure {
                figure: Figure::Quality
            }
        ))
    );
    Ok(())
}

/// T09-RT-34 · the baseline survives an unevidenced challenger: one gapped recipe routes the
/// decision to the baseline even though another recipe is fully eligible, and the explanation
/// shows both the gap and the eligible recipe before R09 decides.
#[test]
fn one_gap_routes_to_the_baseline_despite_an_eligible_challenger() -> Outcome {
    let mut silent = recipe(BETA);
    silent.availability = Observation::Unobserved;
    let decision = decide(&[recipe(ALPHA), silent])?;
    assert_eq!(
        fallback(&decision).map(|f| f.2),
        Some(&Fallback::InsufficientEvidence {
            gaps: vec![EvidenceGap {
                recipe: BETA,
                rule: Rule::R05Availability,
                evidence: Gap::Unobserved
            }]
        })
    );
    assert_eq!(
        decision.explanation().steps,
        vec![
            guarded(),
            Step::Eligible {
                recipe: ALPHA,
                passed: FILTER_RULES.to_vec()
            },
            Step::Gap {
                recipe: BETA,
                passed: FILTER_RULES[..3].to_vec(),
                rule: Rule::R05Availability,
                evidence: Gap::Unobserved
            },
            Step::Decided {
                rule: Rule::R09InsufficientEvidence
            },
        ]
    );
    Ok(())
}

/// T09-RT-35 · a definite exclusion earlier in the order pre-empts a later gap: an unobserved
/// recipe that also lacks a capability is excluded by R02 and carries no gap, so the eligible
/// neighbour is chosen.
#[test]
fn an_earlier_exclusion_pre_empts_a_later_gap() -> Outcome {
    let mut both = recipe(BETA);
    both.capabilities = &["usage"];
    both.availability = Observation::Unobserved;
    let decision = decide(&[recipe(ALPHA), both])?;
    assert_eq!(chosen(&decision), Some((ALPHA, "7")));
    assert_eq!(gap_of(&decision, BETA), None);
    assert_eq!(
        decision.explanation().exclusions(),
        vec![(BETA, Rule::R02RequiredCapabilities)]
    );
    Ok(())
}

/// T09-RT-36 · several gaps are reported together, in identity order, whatever the input order.
#[test]
fn multiple_gaps_are_listed_in_identity_order() -> Outcome {
    let mut first = recipe(ALPHA);
    first.availability = observed(Availability::Unknown, 7);
    let mut second = recipe(GAMMA);
    second.cost_microunits = None;
    let expected = Fallback::InsufficientEvidence {
        gaps: vec![
            EvidenceGap {
                recipe: ALPHA,
                rule: Rule::R05Availability,
                evidence: Gap::AvailabilityUnknown { age_ms: 7 },
            },
            EvidenceGap {
                recipe: GAMMA,
                rule: Rule::R11Ranking,
                evidence: Gap::MissingFigure {
                    figure: Figure::Cost,
                },
            },
        ],
    };
    assert_eq!(
        fallback(&decide(&[second, first])?).map(|f| f.2),
        Some(&expected)
    );
    assert_eq!(
        fallback(&decide(&[first, second])?).map(|f| f.2),
        Some(&expected)
    );
    Ok(())
}

// ---------------------------------------------------------- empty eligible set

/// T09-RT-37 · every candidate excluded routes to the baseline by R10, and the explanation
/// carries each excluded recipe with the rule that excluded it.
#[test]
fn all_excluded_routes_to_the_baseline_with_every_exclusion_named() -> Outcome {
    let mut remote = recipe(ALPHA);
    remote.locality = Locality::Remote;
    let mut small = recipe(BETA);
    small.context_limit_tokens = Some(1);
    let decision = decide(&[small, remote])?;
    assert_eq!(
        fallback(&decision),
        Some((
            BASELINE_ID,
            BASELINE_REVISION,
            &Fallback::NoEligibleCandidate
        ))
    );
    assert_eq!(
        decision.explanation().exclusions(),
        vec![
            (ALPHA, Rule::R04PrivacyClass),
            (BETA, Rule::R03ContextLimit)
        ]
    );
    assert_eq!(
        decision.explanation().decided_by(),
        Some(Rule::R10NoEligibleCandidate)
    );
    Ok(())
}

/// T09-RT-38 · an empty candidate list routes to the baseline by R10 after the guard alone.
#[test]
fn empty_candidate_list_routes_to_the_baseline() -> Outcome {
    let decision = decide(&[])?;
    assert_eq!(
        decision,
        Route::Baseline {
            recipe: BASELINE_ID,
            revision: BASELINE_REVISION,
            reason: Fallback::NoEligibleCandidate,
            explanation: Explanation {
                policy_revision: revision()?,
                steps: vec![
                    guarded(),
                    Step::Decided {
                        rule: Rule::R10NoEligibleCandidate
                    }
                ]
            }
        }
    );
    Ok(())
}

// ------------------------------------------------------------------- ranking

/// T09-RT-39 · cost decides at the smallest margin: 100 beats 101, whichever comes first.
#[test]
fn cost_decides_at_one_microunit() -> Outcome {
    let mut dearer = recipe(BETA);
    dearer.cost_microunits = Some(101);
    let mut cheaper = recipe(ALPHA);
    cheaper.cost_microunits = Some(100);
    assert_eq!(chosen(&decide(&[dearer, cheaper])?), Some((ALPHA, "7")));
    cheaper.id = GAMMA;
    dearer.id = ALPHA;
    assert_eq!(chosen(&decide(&[dearer, cheaper])?), Some((GAMMA, "7")));
    Ok(())
}

/// T09-RT-40 · with equal cost, quality decides at one basis point: 9001 beats 9000.
#[test]
fn quality_decides_at_one_basis_point_when_cost_ties() -> Outcome {
    let mut better = recipe(BETA);
    better.quality_basis_points = Some(9_001);
    assert_eq!(
        chosen(&decide(&[recipe(ALPHA), better])?),
        Some((BETA, "7"))
    );
    Ok(())
}

/// T09-RT-41 · with equal cost and quality, latency decides at one millisecond: 500 beats 501.
#[test]
fn latency_decides_at_one_millisecond_when_cost_and_quality_tie() -> Outcome {
    let mut slower = recipe(ALPHA);
    slower.latency_ms = Some(501);
    assert_eq!(chosen(&decide(&[slower, recipe(BETA)])?), Some((BETA, "7")));
    Ok(())
}

/// T09-RT-42 · the declared key order decides which figure dominates: the same two recipes are
/// ranked differently under `cost, quality, latency` and under `quality, cost, latency`.
#[test]
fn declared_key_order_decides_dominance() -> Outcome {
    let mut cheap = recipe(ALPHA);
    cheap.cost_microunits = Some(10);
    cheap.quality_basis_points = Some(7_000);
    let mut strong = recipe(BETA);
    strong.cost_microunits = Some(11);
    strong.quality_basis_points = Some(7_001);
    assert_eq!(chosen(&decide(&[cheap, strong])?), Some((ALPHA, "7")));
    let quality_first = declared(|d| d.ranking.swap(0, 1))?;
    assert_eq!(
        quality_first.ranking(),
        &[Key::Quality, Key::Cost, Key::Latency]
    );
    let decision = route(&quality_first, &task(), &[cheap, strong], &baseline())?;
    assert_eq!(chosen(&decision), Some((BETA, "7")));
    let latency_first = declared(|d| d.ranking.rotate_right(1))?;
    assert_eq!(
        latency_first.ranking(),
        &[Key::Latency, Key::Cost, Key::Quality]
    );
    cheap.latency_ms = Some(500);
    strong.latency_ms = Some(499);
    let decision = route(&latency_first, &task(), &[cheap, strong], &baseline())?;
    assert_eq!(chosen(&decision), Some((BETA, "7")));
    Ok(())
}

/// T09-RT-43 · a tie on every key routes to the baseline by R12 naming the tied recipes in
/// identity order, and the ranking step lists them.
#[test]
fn a_full_tie_routes_to_the_baseline_naming_the_tied_recipes() -> Outcome {
    let decision = decide(&[recipe(BETA), recipe(ALPHA)])?;
    assert_eq!(
        fallback(&decision),
        Some((
            BASELINE_ID,
            BASELINE_REVISION,
            &Fallback::Tie {
                between: vec![ALPHA, BETA]
            }
        ))
    );
    assert_eq!(decision.explanation().decided_by(), Some(Rule::R12Tie));
    assert!(decision.explanation().steps.contains(&Step::Ranked {
        rule: Rule::R11Ranking,
        order: vec![ranked(&recipe(ALPHA)), ranked(&recipe(BETA))]
    }));
    Ok(())
}

/// T09-RT-44 · only the top group is a tie: two recipes equal at the top and a third strictly
/// worse name exactly the two; a third equal only on cost is not in the tie.
#[test]
fn only_the_top_group_forms_the_tie() -> Outcome {
    let mut worse = recipe(GAMMA);
    worse.latency_ms = Some(600);
    let decision = decide(&[worse, recipe(BETA), recipe(ALPHA)])?;
    assert_eq!(
        fallback(&decision).map(|f| f.2),
        Some(&Fallback::Tie {
            between: vec![ALPHA, BETA]
        })
    );
    let mut weaker = recipe(DELTA);
    weaker.quality_basis_points = Some(8_999);
    let decision = decide(&[weaker, recipe(ALPHA), recipe(BETA)])?;
    assert_eq!(
        fallback(&decision).map(|f| f.2),
        Some(&Fallback::Tie {
            between: vec![ALPHA, BETA]
        })
    );
    Ok(())
}

/// T09-RT-45 · permutation invariance: every ordering of four candidates (one excluded, one
/// worse, two nearly tied) yields the identical `Route`, explanation included.
#[test]
fn every_permutation_yields_the_identical_route() -> Outcome {
    let mut remote = recipe(GAMMA);
    remote.locality = Locality::Hybrid;
    let mut worse = recipe(DELTA);
    worse.cost_microunits = Some(200);
    let mut near = recipe(BETA);
    near.latency_ms = Some(499);
    let reference = permutation_invariant(&[recipe(ALPHA), near, remote, worse])?;
    assert_eq!(chosen(&reference), Some((BETA, "7")));
    Ok(())
}

/// Decide over every ordering of four candidates, require each to equal the first decision
/// (explanation included) and return it. The permutation count is asserted, not assumed.
fn permutation_invariant(set: &[Recipe<'static>; 4]) -> Result<Route<'static>, Box<dyn Error>> {
    let reference = decide(set)?;
    let mut permutations = 0;
    for a in 0..4 {
        for b in (0..4).filter(|&b| b != a) {
            for c in (0..4).filter(|&c| c != a && c != b) {
                let d = 6 - a - b - c;
                let order = [set[a], set[b], set[c], set[d]];
                assert_eq!(decide(&order)?, reference, "permutation {a}{b}{c}{d}");
                permutations += 1;
            }
        }
    }
    assert_eq!(permutations, 24, "four candidates have 24 orderings");
    Ok(reference)
}

/// T09-RT-46 · a chosen route carries the chosen recipe's revision and a baseline route the
/// baseline's; `dispatches` names the recipe each would dispatch and none for a refusal.
#[test]
fn routes_carry_the_dispatched_revision() -> Outcome {
    let mut revised = recipe(ALPHA);
    revised.revision = "42";
    let chosen_route = decide(&[revised])?;
    assert_eq!(chosen(&chosen_route), Some((ALPHA, "42")));
    assert_eq!(chosen_route.dispatches(), Some(ALPHA));
    let baseline_route = decide(&[])?;
    assert_eq!(
        fallback(&baseline_route).map(|f| (f.0, f.1)),
        Some((BASELINE_ID, "1"))
    );
    assert_eq!(baseline_route.dispatches(), Some(BASELINE_ID));
    let mut task = task();
    task.required_capabilities = &["deltas"];
    let refused = decide_task(&task, &[revised])?;
    assert_eq!(refused.dispatches(), None);
    Ok(())
}

// --------------------------------------------------------- explanation stability

/// T09-RT-47 · the whole explanation of a chosen route, fixture one: three candidates, one
/// excluded by context, two eligible, the cheaper chosen by R11.
#[test]
fn whole_explanation_of_a_chosen_route() -> Outcome {
    let mut small = recipe(GAMMA);
    small.context_limit_tokens = Some(4_096);
    let mut dear = recipe(BETA);
    dear.cost_microunits = Some(150);
    let decision = decide(&[dear, small, recipe(ALPHA)])?;
    assert_eq!(
        decision,
        Route::Chosen {
            recipe: ALPHA,
            revision: "7",
            explanation: Explanation {
                policy_revision: revision()?,
                steps: vec![
                    guarded(),
                    Step::Eligible {
                        recipe: ALPHA,
                        passed: FILTER_RULES.to_vec()
                    },
                    Step::Eligible {
                        recipe: BETA,
                        passed: FILTER_RULES.to_vec()
                    },
                    Step::Excluded {
                        recipe: GAMMA,
                        passed: vec![Rule::R02RequiredCapabilities],
                        rule: Rule::R03ContextLimit,
                        why: Exclusion::ContextExceeded {
                            limit_tokens: 4_096,
                            required_tokens: 8_192
                        }
                    },
                    Step::Ranked {
                        rule: Rule::R11Ranking,
                        order: vec![
                            Ranked {
                                recipe: ALPHA,
                                cost_microunits: 100,
                                quality_basis_points: 9_000,
                                latency_ms: 500
                            },
                            Ranked {
                                recipe: BETA,
                                cost_microunits: 150,
                                quality_basis_points: 9_000,
                                latency_ms: 500
                            },
                        ]
                    },
                    Step::Decided {
                        rule: Rule::R11Ranking
                    },
                ]
            }
        }
    );
    Ok(())
}

/// T09-RT-48 · the whole explanation of a baseline route, fixture two, differing from fixture
/// one in every field: a remote-allowed task with a ceiling, a deadline and a floor the baseline
/// meets, one recipe excluded by the deadline, one gapped on quality, one eligible, R09 deciding.
#[test]
fn whole_explanation_of_a_baseline_route() -> Outcome {
    let task = Task {
        required_capabilities: &["identity"],
        context_tokens: 16_384,
        privacy: PrivacyClass::RemoteAllowed,
        cost_ceiling_microunits: Some(5_000),
        deadline_ms: Some(1_000),
        quality_floor_basis_points: Some(4_500),
    };
    let mut slow = recipe(DELTA);
    slow.revision = "3";
    slow.locality = Locality::Remote;
    slow.latency_ms = Some(1_001);
    let mut unscored = recipe(BETA);
    unscored.revision = "5";
    unscored.locality = Locality::Hybrid;
    unscored.quality_basis_points = None;
    let mut fine = recipe(GAMMA);
    fine.revision = "9";
    fine.cost_microunits = Some(4_999);
    fine.latency_ms = Some(999);
    let decision = decide_task(&task, &[fine, slow, unscored])?;
    assert_eq!(
        decision,
        Route::Baseline {
            recipe: BASELINE_ID,
            revision: BASELINE_REVISION,
            reason: Fallback::InsufficientEvidence {
                gaps: vec![EvidenceGap {
                    recipe: BETA,
                    rule: Rule::R08QualityFloor,
                    evidence: Gap::MissingFigure {
                        figure: Figure::Quality
                    }
                }]
            },
            explanation: Explanation {
                policy_revision: revision()?,
                steps: vec![
                    guarded(),
                    Step::Gap {
                        recipe: BETA,
                        passed: FILTER_RULES[..6].to_vec(),
                        rule: Rule::R08QualityFloor,
                        evidence: Gap::MissingFigure {
                            figure: Figure::Quality
                        }
                    },
                    Step::Eligible {
                        recipe: GAMMA,
                        passed: FILTER_RULES.to_vec()
                    },
                    Step::Excluded {
                        recipe: DELTA,
                        passed: FILTER_RULES[..5].to_vec(),
                        rule: Rule::R07Deadline,
                        why: Exclusion::LatencyAboveDeadline {
                            latency_ms: 1_001,
                            deadline_ms: 1_000
                        }
                    },
                    Step::Decided {
                        rule: Rule::R09InsufficientEvidence
                    },
                ]
            }
        }
    );
    Ok(())
}

/// T09-RT-49 · rule identities are stable and unique: R01 through R12 in evaluation order, and
/// R13, the fallback's previous-attempt exclusion.
#[test]
fn rule_identities_are_stable_and_unique() {
    let rules = [
        Rule::R01BaselineGuard,
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
        Rule::R13PreviousAttempt,
    ];
    let ids: Vec<&str> = rules.iter().map(|rule| rule.id()).collect();
    assert_eq!(
        ids,
        [
            "R01", "R02", "R03", "R04", "R05", "R06", "R07", "R08", "R09", "R10", "R11", "R12",
            "R13"
        ]
    );
    let names: Vec<&str> = Filter::ALL.iter().map(|f| f.name()).collect();
    assert_eq!(
        names,
        [
            "required_capabilities",
            "context_limit",
            "privacy_class",
            "availability",
            "cost_ceiling",
            "deadline",
            "quality_floor"
        ]
    );
    assert_eq!(Key::ALL.map(Key::name), ["cost", "quality", "latency"]);
    assert_eq!(
        Key::ALL.map(Key::figure),
        [Figure::Cost, Figure::Quality, Figure::Latency]
    );
}

/// T09-RT-50 · the serialized decision is a stable document: the whole JSON of a chosen route
/// over one excluded and one eligible recipe equals the literal a consumer would read.
#[test]
fn serialized_decision_is_a_stable_document() -> Outcome {
    let mut down = recipe(BETA);
    down.availability = observed(Availability::Unavailable, 12);
    let decision = decide(&[down, recipe(ALPHA)])?;
    let filters = json!([
        "R02RequiredCapabilities",
        "R03ContextLimit",
        "R04PrivacyClass",
        "R05Availability",
        "R06CostCeiling",
        "R07Deadline",
        "R08QualityFloor"
    ]);
    let expected: Value = json!({
        "route": "chosen",
        "recipe": ALPHA,
        "revision": "7",
        "explanation": {"policy_revision": REVIEWED_REVISION, "steps": [
            {"step": "guarded", "recipe": BASELINE_ID, "passed": filters},
            {"step": "eligible", "recipe": ALPHA, "passed": filters},
            {"step": "excluded", "recipe": BETA,
             "passed": ["R02RequiredCapabilities", "R03ContextLimit", "R04PrivacyClass"],
             "rule": "R05Availability", "why": {"why": "unavailable", "age_ms": 12}},
            {"step": "ranked", "rule": "R11Ranking", "order": [
                {"recipe": ALPHA, "cost_microunits": 100, "quality_basis_points": 9000, "latency_ms": 500}
            ]},
            {"step": "decided", "rule": "R11Ranking"}
        ]}
    });
    assert_eq!(serde_json::to_value(&decision)?, expected);
    Ok(())
}

/// T09-RT-63 · the admitted policy revision (route-G5; docs/modules/route.md: Op 1 takes the
/// "admitted policy revision" and "require deterministic replay inputs"). The reviewed policy's
/// revision and the swapped-filter policy's are pinned to digests computed outside this code; the
/// text door and the values door agree; every value a declaration can vary enters the revision, so
/// five policies differing in one value each have five revisions; and every decision names the
/// revision it was decided under — two policies whose steps are identical for these inputs yield
/// serialized decisions that differ in that one field and nowhere else.
#[test]
fn every_decision_names_the_admitted_policy_revision() -> Outcome {
    let reviewed = policy()?;
    assert_eq!(reviewed.revision(), REVIEWED_REVISION);
    assert_eq!(declared(|_| {})?.revision(), REVIEWED_REVISION);
    let swapped = declared(|d| d.filters.swap(0, 1))?;
    assert_eq!(swapped.revision(), SWAPPED_FILTERS_REVISION);
    let reranked = declared(|d| d.ranking.swap(1, 2))?;
    let tighter = declared(|d| d.staleness_bound_ms = BOUND_MS - 1)?;
    let mut other = recipe(ALPHA);
    other.revision = BASELINE_REVISION;
    let mut rebased = declaration();
    rebased.baseline = ALPHA.to_owned();
    let rebased = Policy::declare(rebased, &other)?;
    let revisions: BTreeSet<&str> = [&reviewed, &swapped, &reranked, &tighter, &rebased]
        .iter()
        .map(|policy| policy.revision())
        .collect();
    assert_eq!(
        revisions.len(),
        5,
        "one revision per distinct policy: {revisions:?}"
    );
    for revision in &revisions {
        let hex = revision.strip_prefix("sha256:").ok_or("sha256: prefix")?;
        assert!(
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "revision {revision} is not sha256: and 64 lowercase hex digits"
        );
    }
    let candidates = [recipe(ALPHA), recipe(BETA)];
    let under_reviewed =
        serde_json::to_value(route(&reviewed, &task(), &candidates, &baseline())?)?;
    let under_tighter = serde_json::to_value(route(&tighter, &task(), &candidates, &baseline())?)?;
    assert_eq!(
        under_reviewed["explanation"]["policy_revision"],
        REVIEWED_REVISION
    );
    assert_eq!(
        under_tighter["explanation"]["policy_revision"],
        tighter.revision()
    );
    assert_ne!(under_reviewed, under_tighter);
    let mut replayed = under_reviewed.clone();
    replayed["explanation"]["policy_revision"] = json!(tighter.revision());
    assert_eq!(
        replayed, under_tighter,
        "the revision is the only difference"
    );
    let under_swapped = route(&swapped, &task(), &candidates, &baseline())?;
    assert_eq!(
        under_swapped.explanation().policy_revision,
        SWAPPED_FILTERS_REVISION
    );
    Ok(())
}

// ------------------------------------------------------------- baseline guard

/// T09-RT-51 · the baseline lacking a capability the task requires is refused by R01 carrying
/// the R02 exclusion; the refusal dispatches nothing.
#[test]
fn baseline_missing_a_required_capability_is_refused() -> Outcome {
    let mut task = task();
    task.required_capabilities = &["final_output", "tool_proposals"];
    let decision = decide_task(&task, &[recipe(ALPHA)])?;
    assert_eq!(
        refusal(&decision),
        Some((
            Refusal::BaselineExcluded {
                rule: Rule::R02RequiredCapabilities,
                why: Exclusion::MissingCapability {
                    capability: "tool_proposals"
                }
            },
            &Fallback::NoEligibleCandidate
        ))
    );
    assert_eq!(decision.dispatches(), None);
    assert_eq!(
        decision.explanation().decided_by(),
        Some(Rule::R01BaselineGuard)
    );
    Ok(())
}

/// T09-RT-52 · a baseline whose availability is stale cannot honour a tie: the refusal carries
/// the R05 gap and the tie it could not resolve.
#[test]
fn baseline_with_stale_availability_cannot_honour_a_tie() -> Outcome {
    let mut stale = baseline();
    stale.availability = observed(Availability::Available, 31_000);
    let decision = route(&policy()?, &task(), &[recipe(BETA), recipe(ALPHA)], &stale)?;
    assert_eq!(
        refusal(&decision),
        Some((
            Refusal::BaselineEvidence {
                rule: Rule::R05Availability,
                evidence: Gap::StaleAvailability {
                    age_ms: 31_000,
                    bound_ms: BOUND_MS
                }
            },
            &Fallback::Tie {
                between: vec![ALPHA, BETA]
            }
        ))
    );
    assert_eq!(decision.dispatches(), None);
    Ok(())
}

/// T09-RT-53 · a baseline above the task's cost ceiling is refused by R01 carrying the R06
/// exclusion with both numbers.
#[test]
fn baseline_above_the_cost_ceiling_is_refused() -> Outcome {
    let mut task = task();
    task.cost_ceiling_microunits = Some(3);
    let mut dear = baseline();
    dear.cost_microunits = Some(4);
    let decision = route(&policy()?, &task, &[recipe(ALPHA)], &dear)?;
    assert_eq!(
        refusal(&decision),
        Some((
            Refusal::BaselineExcluded {
                rule: Rule::R06CostCeiling,
                why: Exclusion::CostAboveCeiling {
                    cost_microunits: 4,
                    ceiling_microunits: 3
                }
            },
            &Fallback::NoEligibleCandidate
        ))
    );
    Ok(())
}

/// T09-RT-54 · the guard is screened first but decides last: a baseline that cannot hold the
/// task's context leads the explanation as `Excluded`, and a strictly best candidate is still
/// chosen because no fallback was required.
#[test]
fn a_failed_guard_does_not_deny_a_strict_choice() -> Outcome {
    let mut task = task();
    task.context_tokens = 40_000;
    let mut roomy = recipe(ALPHA);
    roomy.context_limit_tokens = Some(65_536);
    let decision = decide_task(&task, &[roomy])?;
    assert_eq!(
        decision,
        Route::Chosen {
            recipe: ALPHA,
            revision: "7",
            explanation: Explanation {
                policy_revision: revision()?,
                steps: vec![
                    Step::Excluded {
                        recipe: BASELINE_ID,
                        passed: vec![Rule::R02RequiredCapabilities],
                        rule: Rule::R03ContextLimit,
                        why: Exclusion::ContextExceeded {
                            limit_tokens: 32_768,
                            required_tokens: 40_000
                        }
                    },
                    Step::Eligible {
                        recipe: ALPHA,
                        passed: FILTER_RULES.to_vec()
                    },
                    Step::Ranked {
                        rule: Rule::R11Ranking,
                        order: vec![ranked(&roomy)]
                    },
                    Step::Decided {
                        rule: Rule::R11Ranking
                    },
                ]
            }
        }
    );
    Ok(())
}

/// T09-RT-57 · the same baseline defect refuses the moment a fallback is required: with the
/// roomy candidate gapped, R09 requires the baseline and R01 refuses, carrying the gap fallback.
#[test]
fn a_failed_guard_refuses_when_a_fallback_is_required() -> Outcome {
    let mut task = task();
    task.context_tokens = 40_000;
    let mut roomy = recipe(ALPHA);
    roomy.context_limit_tokens = Some(65_536);
    roomy.availability = Observation::Unobserved;
    let decision = decide_task(&task, &[roomy])?;
    let gaps = vec![EvidenceGap {
        recipe: ALPHA,
        rule: Rule::R05Availability,
        evidence: Gap::Unobserved,
    }];
    assert_eq!(
        decision,
        Route::Refused {
            fallback: Fallback::InsufficientEvidence { gaps: gaps.clone() },
            reason: Refusal::BaselineExcluded {
                rule: Rule::R03ContextLimit,
                why: Exclusion::ContextExceeded {
                    limit_tokens: 32_768,
                    required_tokens: 40_000
                }
            },
            explanation: Explanation {
                policy_revision: revision()?,
                steps: vec![
                    Step::Excluded {
                        recipe: BASELINE_ID,
                        passed: vec![Rule::R02RequiredCapabilities],
                        rule: Rule::R03ContextLimit,
                        why: Exclusion::ContextExceeded {
                            limit_tokens: 32_768,
                            required_tokens: 40_000
                        }
                    },
                    Step::Gap {
                        recipe: ALPHA,
                        passed: FILTER_RULES[..3].to_vec(),
                        rule: Rule::R05Availability,
                        evidence: Gap::Unobserved
                    },
                    Step::Decided {
                        rule: Rule::R09InsufficientEvidence
                    },
                    Step::Decided {
                        rule: Rule::R01BaselineGuard
                    },
                ]
            }
        }
    );
    assert_eq!(decision.dispatches(), None);
    Ok(())
}

/// T09-RT-58 · a refusal never dispatches and a fallback never names an ineligible baseline:
/// over every task bound the baseline fails, the empty candidate list yields `Refused`, never
/// `Baseline`, and each refusal names the bound's own rule.
#[test]
fn an_ineligible_baseline_is_never_dispatched_by_a_fallback() -> Outcome {
    let mut floor = task();
    floor.quality_floor_basis_points = Some(5_001);
    let mut deadline = task();
    deadline.deadline_ms = Some(999);
    let mut ceiling = task();
    ceiling.cost_ceiling_microunits = Some(0);
    let mut dear = baseline();
    dear.cost_microunits = Some(1);
    for (task, baseline, rule) in [
        (floor, baseline(), Rule::R08QualityFloor),
        (deadline, baseline(), Rule::R07Deadline),
        (ceiling, dear, Rule::R06CostCeiling),
    ] {
        let decision = route(&policy()?, &task, &[], &baseline)?;
        assert_eq!(decision.dispatches(), None, "{rule:?}");
        assert_eq!(
            refusal(&decision).map(|r| r.1),
            Some(&Fallback::NoEligibleCandidate)
        );
        assert!(
            matches!(refusal(&decision), Some((Refusal::BaselineExcluded { rule: by, .. }, _)) if by == rule),
            "{rule:?}"
        );
    }
    Ok(())
}

// ------------------------------------------------------------ fallback (operation 3)

const FAILURES: [Failure; 4] = [
    Failure::Truncated,
    Failure::Refused,
    Failure::Failed,
    Failure::Cancelled,
];

fn after(
    previous: &'static str,
    failure: Failure,
    candidates: &[Recipe<'static>],
) -> Result<Route<'static>, Box<dyn Error>> {
    after_task(&task(), previous, failure, candidates)
}
fn after_task(
    task: &Task<'static>,
    previous: &'static str,
    failure: Failure,
    candidates: &[Recipe<'static>],
) -> Result<Route<'static>, Box<dyn Error>> {
    let attempt = Attempt {
        recipe: previous,
        failure,
    };
    Ok(evaluate_fallback(
        &policy()?,
        task,
        candidates,
        &baseline(),
        attempt,
    )?)
}
fn previous_step(recipe: &'static str, failure: Failure) -> Step<'static> {
    Step::Excluded {
        recipe,
        passed: Vec::new(),
        rule: Rule::R13PreviousAttempt,
        why: Exclusion::PreviousAttempt { failure },
    }
}
fn eligible(recipe: &'static str) -> Step<'static> {
    Step::Eligible {
        recipe,
        passed: FILTER_RULES.to_vec(),
    }
}
fn explained(steps: Vec<Step<'static>>) -> Result<Explanation<'static>, Box<dyn Error>> {
    Ok(Explanation {
        policy_revision: revision()?,
        steps,
    })
}
/// The per-recipe screening steps of a decision, keyed by recipe: what each filter did to it.
fn screenings<'a>(decision: &'a Route<'a>) -> Vec<(&'a str, &'a Step<'a>)> {
    decision
        .explanation()
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Guarded { recipe, .. }
            | Step::Eligible { recipe, .. }
            | Step::Excluded { recipe, .. }
            | Step::Gap { recipe, .. } => Some((*recipe, step)),
            Step::Ranked { .. } | Step::Decided { .. } => None,
        })
        .collect()
}

/// T09-RT-65 · operation 3 (route-G1; docs/modules/route.md "Evaluate fallback"): the recipe
/// whose attempt failed is never chosen again. The route chose ALPHA; after ALPHA's attempt fails
/// in each of the four categories, the fallback is BETA, and the whole explanation names ALPHA
/// excluded by R13 before any filter, carrying that category. A failed attempt on a recipe that was
/// not the choice leaves the choice as it was.
#[test]
fn a_fallback_never_rechooses_the_previous_recipe() -> Outcome {
    let mut worse = recipe(BETA);
    worse.cost_microunits = Some(200);
    worse.revision = "3";
    let candidates = [recipe(ALPHA), worse];
    assert_eq!(chosen(&decide(&candidates)?), Some((ALPHA, "7")));
    for failure in FAILURES {
        let expected = Route::Chosen {
            recipe: BETA,
            revision: "3",
            explanation: explained(vec![
                guarded(),
                previous_step(ALPHA, failure),
                eligible(BETA),
                Step::Ranked {
                    rule: Rule::R11Ranking,
                    order: vec![ranked(&worse)],
                },
                Step::Decided {
                    rule: Rule::R11Ranking,
                },
            ])?,
        };
        assert_eq!(after(ALPHA, failure, &candidates)?, expected, "{failure:?}");
    }
    let unmoved = after(BETA, Failure::Failed, &candidates)?;
    assert_eq!(chosen(&unmoved), Some((ALPHA, "7")));
    assert_eq!(
        excluded_by(&unmoved, BETA),
        Some((
            Vec::new(),
            Rule::R13PreviousAttempt,
            Exclusion::PreviousAttempt {
                failure: Failure::Failed
            }
        ))
    );
    Ok(())
}

/// T09-RT-66 · a fallback after the only candidate failed routes to the baseline because
/// nothing eligible remains (R10); the baseline is the permitted next recipe, not a relaxation.
#[test]
fn a_fallback_after_the_last_candidate_routes_to_the_baseline() -> Outcome {
    let decision = after(ALPHA, Failure::Truncated, &[recipe(ALPHA)])?;
    let expected = Route::Baseline {
        recipe: BASELINE_ID,
        revision: BASELINE_REVISION,
        reason: Fallback::NoEligibleCandidate,
        explanation: explained(vec![
            guarded(),
            previous_step(ALPHA, Failure::Truncated),
            Step::Decided {
                rule: Rule::R10NoEligibleCandidate,
            },
        ])?,
    };
    assert_eq!(decision, expected);
    Ok(())
}

/// T09-RT-67 · a truthful refusal carries the failure category. When the baseline's own attempt
/// failed, R13 excludes it; a fallback that then needs the baseline (nothing eligible, or a tie)
/// is refused by R01 with the R13 exclusion and the category, one per category. A strict choice
/// among the candidates is still made: the excluded baseline denies only what needs it.
#[test]
fn a_refused_fallback_carries_the_failure_category() -> Outcome {
    for failure in FAILURES {
        let why = Exclusion::PreviousAttempt { failure };
        let expected = Route::Refused {
            fallback: Fallback::NoEligibleCandidate,
            reason: Refusal::BaselineExcluded {
                rule: Rule::R13PreviousAttempt,
                why,
            },
            explanation: explained(vec![
                previous_step(BASELINE_ID, failure),
                Step::Decided {
                    rule: Rule::R10NoEligibleCandidate,
                },
                Step::Decided {
                    rule: Rule::R01BaselineGuard,
                },
            ])?,
        };
        assert_eq!(after(BASELINE_ID, failure, &[])?, expected, "{failure:?}");
    }
    let tied = after(
        BASELINE_ID,
        Failure::Refused,
        &[recipe(BETA), recipe(ALPHA)],
    )?;
    assert_eq!(
        refusal(&tied),
        Some((
            Refusal::BaselineExcluded {
                rule: Rule::R13PreviousAttempt,
                why: Exclusion::PreviousAttempt {
                    failure: Failure::Refused
                },
            },
            &Fallback::Tie {
                between: vec![ALPHA, BETA]
            }
        ))
    );
    let strict = after(BASELINE_ID, Failure::Failed, &[recipe(ALPHA)])?;
    assert_eq!(chosen(&strict), Some((ALPHA, "7")));
    assert_eq!(
        strict.explanation().steps.first(),
        Some(&previous_step(BASELINE_ID, Failure::Failed))
    );
    Ok(())
}

/// T09-RT-68 · a fallback relaxes no filter (T10: "fallback cannot cross privacy or tool
/// constraints"; D-3: route owns eligibility). Over a set with one recipe stopped by each of the
/// seven filters, one stale observation and two eligible recipes, under a task carrying all three
/// bounds, every recipe in turn (and the baseline) is the failed attempt: every OTHER recipe's
/// screening step is identical to the one `route` gave it, and only the failed recipe changes, to
/// the R13 exclusion. A fallback that skipped any filter would turn an excluded recipe eligible.
#[test]
fn a_fallback_relaxes_no_filter() -> Outcome {
    let bounded = Task {
        required_capabilities: FINAL_ONLY,
        context_tokens: 8_192,
        privacy: PrivacyClass::LocalOnly,
        cost_ceiling_microunits: Some(100),
        deadline_ms: Some(1_000),
        quality_floor_basis_points: Some(5_000),
    };
    let mut set = vec![recipe("e-0"), recipe("e-8")];
    let mut e1 = recipe("e-1");
    e1.capabilities = &["identity"];
    let mut e2 = recipe("e-2");
    e2.context_limit_tokens = Some(8_191);
    let mut e3 = recipe("e-3");
    e3.locality = Locality::Remote;
    let mut e4 = recipe("e-4");
    e4.availability = observed(Availability::Unavailable, 7);
    let mut e5 = recipe("e-5");
    e5.cost_microunits = Some(101);
    let mut e6 = recipe("e-6");
    e6.latency_ms = Some(1_001);
    let mut e7 = recipe("e-7");
    e7.quality_basis_points = Some(4_999);
    let mut e9 = recipe("e-9");
    e9.availability = observed(Availability::Available, BOUND_MS);
    set.extend([e1, e2, e3, e4, e5, e6, e7, e9]);
    let original = decide_task(&bounded, &set)?;
    let before = screenings(&original);
    let stopped: BTreeSet<&str> = original
        .explanation()
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Excluded { rule, .. } | Step::Gap { rule, .. } => Some(rule.id()),
            _ => None,
        })
        .collect();
    assert_eq!(
        stopped.len(),
        7,
        "seven filter rules stop a recipe: {stopped:?}"
    );
    let previous: Vec<&'static str> = set
        .iter()
        .map(|recipe| recipe.id)
        .chain([BASELINE_ID])
        .collect();
    let mut compared = 0;
    for failed in &previous {
        let decision = after_task(&bounded, failed, Failure::Failed, &set)?;
        let now = screenings(&decision);
        assert_eq!(now.len(), before.len(), "one screening per recipe");
        for ((id, step), (was_id, was)) in now.iter().zip(&before) {
            assert_eq!(id, was_id, "screening order");
            if id == failed {
                assert_eq!(*step, &previous_step(failed, Failure::Failed));
            } else {
                assert_eq!(step, was, "{id} screened differently after {failed} failed");
                compared += 1;
            }
        }
        assert_ne!(decision.dispatches(), Some(*failed));
    }
    assert_eq!(
        compared,
        11 * 10,
        "eleven failed attempts, ten other recipes each"
    );
    Ok(())
}

/// T09-RT-69 · the DONE-route:28 fault/benign pair at library level, on the fallback path.
/// Stale evaluation: after ALPHA fails, a cheaper BETA observed exactly at the staleness bound
/// cannot be selected (an R05 gap routes to the baseline); one millisecond fresher it is chosen.
/// Forbidden data location: a cheaper GAMMA that is remote or hybrid is excluded by R04 for a
/// local-only task and the local DELTA is chosen; the same GAMMA declared local is chosen.
#[test]
fn stale_evaluation_and_forbidden_location_cannot_be_the_fallback() -> Outcome {
    let mut stale = recipe(BETA);
    stale.cost_microunits = Some(50);
    stale.availability = observed(Availability::Available, BOUND_MS);
    let fault = after(ALPHA, Failure::Failed, &[recipe(ALPHA), stale])?;
    assert_eq!(
        fallback(&fault),
        Some((
            BASELINE_ID,
            BASELINE_REVISION,
            &Fallback::InsufficientEvidence {
                gaps: vec![EvidenceGap {
                    recipe: BETA,
                    rule: Rule::R05Availability,
                    evidence: Gap::StaleAvailability {
                        age_ms: BOUND_MS,
                        bound_ms: BOUND_MS
                    },
                }]
            }
        ))
    );
    let mut fresh_enough = stale;
    fresh_enough.availability = observed(Availability::Available, BOUND_MS - 1);
    let benign = after(ALPHA, Failure::Failed, &[recipe(ALPHA), fresh_enough])?;
    assert_eq!(chosen(&benign), Some((BETA, "7")));

    let mut local = recipe(DELTA);
    local.cost_microunits = Some(150);
    for locality in [Locality::Remote, Locality::Hybrid] {
        let mut forbidden = recipe(GAMMA);
        forbidden.cost_microunits = Some(50);
        forbidden.locality = locality;
        let fault = after(
            ALPHA,
            Failure::Truncated,
            &[recipe(ALPHA), forbidden, local],
        )?;
        assert_eq!(chosen(&fault), Some((DELTA, "7")), "{locality:?}");
        assert_eq!(
            excluded_by(&fault, GAMMA),
            Some((
                vec![Rule::R02RequiredCapabilities, Rule::R03ContextLimit],
                Rule::R04PrivacyClass,
                Exclusion::PrivacyViolated { locality }
            ))
        );
    }
    let mut permitted = recipe(GAMMA);
    permitted.cost_microunits = Some(50);
    let benign = after(
        ALPHA,
        Failure::Truncated,
        &[recipe(ALPHA), permitted, local],
    )?;
    assert_eq!(chosen(&benign), Some((GAMMA, "7")));
    Ok(())
}

/// T09-RT-70 · a previous attempt naming a recipe that is neither a candidate nor the baseline is
/// structurally invalid and refused by name before any rule, so an attempt the caller cannot place
/// is never silently ignored; the ordinary structural refusals still come first.
#[test]
fn an_unknown_previous_attempt_is_refused_by_name() -> Outcome {
    assert_eq!(
        evaluate_fallback(
            &policy()?,
            &task(),
            &[recipe(ALPHA)],
            &baseline(),
            Attempt {
                recipe: GAMMA,
                failure: Failure::Failed
            }
        ),
        Err(Invalid::UnknownPreviousAttempt {
            recipe: GAMMA.to_owned()
        })
    );
    assert_eq!(
        evaluate_fallback(
            &policy()?,
            &task(),
            &[recipe(ALPHA), recipe(ALPHA)],
            &baseline(),
            Attempt {
                recipe: GAMMA,
                failure: Failure::Failed
            }
        ),
        Err(Invalid::DuplicateIdentity {
            recipe: ALPHA.to_owned()
        })
    );
    Ok(())
}

/// T09-RT-71 · a fallback is permutation invariant: every ordering of four candidates, the
/// previous attempt among them, yields the identical `Route`, explanation included.
#[test]
fn every_permutation_yields_the_identical_fallback() -> Outcome {
    let mut remote = recipe(GAMMA);
    remote.locality = Locality::Hybrid;
    let mut worse = recipe(DELTA);
    worse.cost_microunits = Some(200);
    let mut near = recipe(BETA);
    near.latency_ms = Some(499);
    let set = [recipe(ALPHA), near, remote, worse];
    let reference = after(BETA, Failure::Failed, &set)?;
    assert_eq!(chosen(&reference), Some((ALPHA, "7")));
    let mut permutations = 0;
    for a in 0..4 {
        for b in (0..4).filter(|&b| b != a) {
            for c in (0..4).filter(|&c| c != a && c != b) {
                let d = 6 - a - b - c;
                let order = [set[a], set[b], set[c], set[d]];
                assert_eq!(
                    after(BETA, Failure::Failed, &order)?,
                    reference,
                    "permutation {a}{b}{c}{d}"
                );
                permutations += 1;
            }
        }
    }
    assert_eq!(permutations, 24, "four candidates have 24 orderings");
    Ok(())
}

// -------------------------------------------------------- purity and known answers

/// T09-RT-55 · routing makes zero model calls, zero I/O and reads zero clocks. The policy body
/// after its anchor block may reference only the paths in `ROUTE_MAY_REFERENCE` — an allowlist
/// (a world, not a list of suspects): a helper, alias or third-party crate is refused because it is
/// not on it, where the first version of this case named 21 suspect tokens and passed anything else.
/// Every entry must be used, so the list cannot quietly grow into an open door, and the checker must
/// catch four planted violations and pass the same text inside a comment, a string and a raw string.
#[test]
fn policy_source_names_no_effectful_item() {
    let source = include_str!("../src/route.rs");
    let body = source.split("HEE3-ANCHORS-END").nth(1).unwrap_or_default();
    let used = referenced_paths(body);
    let outside: Vec<&String> = used.iter().filter(|path| !permitted(path)).collect();
    assert!(outside.is_empty(), "the policy body references {outside:?}");
    for entry in ROUTE_MAY_REFERENCE {
        assert!(
            used.iter().any(|path| under(path, entry)),
            "allowlist entry {entry} is used nowhere; remove it"
        );
    }
    assert!(body.contains("pub fn route<'a>("));
    for plant in [
        "fn f() { let _ = std::process::Command::new(\"x\"); }",
        "use crate::worker as w;\nfn f() { w::call(); }",
        "fn f() { crate::store::Store::open(p); }",
        "fn f() { reqwest::get(u); }",
        "fn f() { let _ = std::time::UNIX_EPOCH; }",
        "fn f() { let _ = std::fmtx::leak(); }",
    ] {
        let found = referenced_paths(plant);
        assert!(
            found.iter().any(|path| !permitted(path)),
            "planted source passed the allowlist: {plant} -> {found:?}"
        );
    }
    for benign in [
        "// std::process::Command in a comment\nfn f() {}",
        "/* std::fs /* nested */ std::net */ fn f() {}",
        "fn f() -> &'static str { \"std::net::TcpStream\" }",
        "fn f() -> &'static str { r#\"std::fs::read\"# }",
        "fn f<'a>(x: &'a str) -> char { let _ = x; 'x' }",
    ] {
        let found = referenced_paths(benign);
        assert!(
            found.iter().all(|path| permitted(path)),
            "benign source tripped the allowlist: {benign} -> {found:?}"
        );
    }
}

/// Everything `src/route.rs` may name: its declared build dependency (`contracts`), the derive it
/// serialises with, the pure `std` modules it uses, the TOML value types `Policy::load` parses a
/// `&str` into, the pure SHA-256 function that digests a policy's canonical rendering into its
/// revision (route-G5), and associated items of the primitive integers. Nothing here performs I/O.
const ROUTE_MAY_REFERENCE: &[&str] = &[
    "crate::contracts::roster",
    // B07a: the one UUIDv4 validator, a pure byte-shape check (`src/contracts.rs`), for a declared
    // recipe's adapter and roster record — the type only, not the rest of `crate::contracts`.
    "crate::contracts::UuidV4",
    "serde::Serialize",
    "sha2",
    "std::cmp",
    "std::collections",
    "std::fmt",
    "std::error",
    "fmt",
    "toml",
    "u64",
    "i64",
    "u16",
];

/// Segment-aware: an entry admits itself and anything below it, so `std::fmt` admits
/// `std::fmt::Display` but not a sibling spelled `std::fmtx`.
fn under(path: &str, entry: &str) -> bool {
    path == entry
        || path
            .strip_prefix(entry)
            .is_some_and(|rest| rest.starts_with("::"))
}

fn permitted(path: &str) -> bool {
    ROUTE_MAY_REFERENCE.iter().any(|entry| under(path, entry))
}

/// Every path in `code` whose first segment starts lowercase (`std::…`, `crate::…`, `serde::…`,
/// `w::…`), after comments, strings, raw strings and char literals are removed. Paths rooted at a
/// type (`Route::Chosen`, `Self::…`) name this module's own items and are not collected. A `use`
/// of `a::b as c` is collected as `a::b`, and a later `c::…` as `c::…` — refused unless listed.
fn referenced_paths(code: &str) -> BTreeSet<String> {
    let text = strip_comments_and_literals(code);
    let bytes = text.as_bytes();
    let mut found = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        let starts_word = index == 0 || !is_ident(bytes[index - 1]);
        if starts_word && bytes[index].is_ascii_lowercase() {
            let begin = index;
            let mut end = index;
            let mut segments = 1;
            loop {
                while end < bytes.len() && is_ident(bytes[end]) {
                    end += 1;
                }
                if end + 2 < bytes.len()
                    && &bytes[end..end + 2] == b"::"
                    && is_ident(bytes[end + 2])
                {
                    end += 2;
                    segments += 1;
                } else {
                    break;
                }
            }
            if segments > 1 {
                found.insert(text[begin..end].to_owned());
            }
            index = end.max(index + 1);
        } else {
            index += 1;
        }
    }
    found
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Replace comments and literal contents with spaces, keeping everything else byte-for-byte.
/// Handles nested block comments, escapes, raw strings of any hash count, byte strings, and the
/// difference between a char literal (`'x'`, `'\\n'`) and a lifetime (`'a`).
fn strip_comments_and_literals(code: &str) -> String {
    let bytes = code.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let rest = &bytes[i..];
        if rest.starts_with(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if rest.starts_with(b"/*") {
            let mut depth = 0usize;
            while i < bytes.len() {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            out.push(b' ');
        } else if (rest.starts_with(b"r#") || rest.starts_with(b"r\"") || rest.starts_with(b"br"))
            && (i == 0 || !is_ident(bytes[i - 1]))
        {
            let mut j = i + usize::from(rest[0] == b'b') + 1;
            let mut hashes = 0;
            while j < bytes.len() && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                let closing: Vec<u8> = std::iter::once(b'"')
                    .chain(std::iter::repeat_n(b'#', hashes))
                    .collect();
                j += 1;
                while j < bytes.len() && !bytes[j..].starts_with(&closing) {
                    j += 1;
                }
                i = (j + closing.len()).min(bytes.len());
                out.extend_from_slice(b"\"\"");
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        } else if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                i += if bytes[i] == b'\\' { 2 } else { 1 };
            }
            i += 1;
            out.extend_from_slice(b"\"\"");
        } else if bytes[i] == b'\'' {
            let char_literal = match rest.get(1) {
                Some(b'\\') => true,
                Some(_) => rest.get(2) == Some(&b'\''),
                None => false,
            };
            if char_literal {
                i += 1;
                while i < bytes.len() && bytes[i] != b'\'' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
                out.extend_from_slice(b"' '");
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// T09-RT-56 · known answers from an independent implementation: the retained table
/// `fixtures/route/known-answers.json`, computed by the Python oracle over
/// `fixtures/route/fixture.json`, equals this router's decisions over the same fixture, whole.
#[test]
fn known_answer_table_matches_the_independent_oracle() -> Outcome {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/route/fixture.json"))?;
    let expected: Value = serde_json::from_str(include_str!("fixtures/route/known-answers.json"))?;
    let policy = policy()?;
    let recipes = fixture["recipes"].as_array().ok_or("recipes")?;
    let capability_lists: Vec<Vec<&str>> = recipes
        .iter()
        .map(|r| {
            r["capabilities"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect()
        })
        .collect();
    let candidates: Vec<Recipe<'_>> = recipes
        .iter()
        .zip(&capability_lists)
        .map(|(r, caps)| fixture_recipe(r, caps))
        .collect::<Result<_, Box<dyn Error>>>()?;
    let tasks = fixture["tasks"].as_array().ok_or("tasks")?;
    let mut answers = Vec::new();
    for entry in tasks {
        let required = fixture_capabilities(entry);
        let task = fixture_task(entry, &required)?;
        let decision = route(&policy, &task, &candidates, &baseline())?;
        answers.push(json!({"task": entry["id"], "answer": project(&decision)}));
    }
    assert_eq!(tasks.len(), 13, "the fixture declares thirteen tasks");
    assert_eq!(Value::Array(answers), expected["answers"]);
    Ok(())
}

/// T09-RT-72 · fallback known answers from the same independent oracle (route-G1): for each
/// fixture row naming a task, the recipe whose attempt failed and its category, the oracle's
/// decision with that recipe excluded first equals `evaluate_fallback`'s over the same fixture,
/// whole. The rows reach a candidate, the baseline, a strict choice after a tie, and refusals by
/// R13 and by a filter; each row's category is echoed in the R13 step the router recorded.
#[test]
fn fallback_known_answers_match_the_independent_oracle() -> Outcome {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/route/fixture.json"))?;
    let expected: Value = serde_json::from_str(include_str!("fixtures/route/known-answers.json"))?;
    let policy = policy()?;
    let recipes = fixture["recipes"].as_array().ok_or("recipes")?;
    let capability_lists: Vec<Vec<&str>> = recipes.iter().map(fixture_capabilities).collect();
    let candidates: Vec<Recipe<'_>> = recipes
        .iter()
        .zip(&capability_lists)
        .map(|(r, caps)| fixture_recipe(r, caps))
        .collect::<Result<_, Box<dyn Error>>>()?;
    let tasks = fixture["tasks"].as_array().ok_or("tasks")?;
    let rows = fixture["fallbacks"].as_array().ok_or("fallbacks")?;
    let mut answers = Vec::new();
    let mut categories = BTreeSet::new();
    for row in rows {
        let entry = tasks
            .iter()
            .find(|task| task["id"] == row["task"])
            .ok_or("fallback row names an undeclared task")?;
        let required = fixture_capabilities(entry);
        let task = fixture_task(entry, &required)?;
        let named = row["failure"].as_str().ok_or("failure")?;
        let failure = FAILURES
            .into_iter()
            .find(|failure| serde_json::to_value(failure).ok() == Some(json!(named)))
            .ok_or("unknown failure category")?;
        categories.insert(named);
        let previous = row["previous"].as_str().ok_or("previous")?;
        let decision = evaluate_fallback(
            &policy,
            &task,
            &candidates,
            &baseline(),
            Attempt {
                recipe: previous,
                failure,
            },
        )?;
        assert_eq!(
            excluded_by(&decision, previous),
            Some((
                Vec::new(),
                Rule::R13PreviousAttempt,
                Exclusion::PreviousAttempt { failure }
            )),
            "row {row}"
        );
        answers.push(
            json!({"task": row["task"], "previous": previous, "failure": named,
                            "answer": project(&decision)}),
        );
    }
    assert_eq!(rows.len(), 11, "the fixture declares eleven fallback rows");
    assert_eq!(categories.len(), 4, "the rows reach every failure category");
    assert_eq!(Value::Array(answers), expected["fallback_answers"]);
    Ok(())
}

fn fixture_capabilities(row: &Value) -> Vec<&str> {
    let field = if row.get("capabilities").is_some() {
        "capabilities"
    } else {
        "required_capabilities"
    };
    row[field]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn fixture_task<'a>(entry: &Value, required: &'a [&'a str]) -> Result<Task<'a>, Box<dyn Error>> {
    Ok(Task {
        required_capabilities: required,
        context_tokens: entry["context_tokens"].as_u64().ok_or("context")?,
        privacy: if entry["privacy"] == "local_only" {
            PrivacyClass::LocalOnly
        } else {
            PrivacyClass::RemoteAllowed
        },
        cost_ceiling_microunits: entry["cost_ceiling_microunits"].as_u64(),
        deadline_ms: entry["deadline_ms"].as_u64(),
        quality_floor_basis_points: entry["quality_floor_basis_points"]
            .as_u64()
            .map(u16::try_from)
            .transpose()?,
    })
}

/// T09-RT-59 · the task's quality floor is in range at exactly `MAX_QUALITY_BASIS_POINTS`: the
/// boundary value screens by R08 and is not refused as structurally invalid, while one basis
/// point above it is refused. Both sides of the range comparison are named, not only the far
/// side — a value pinned only where the answer is the same on either side is pinned by nothing.
#[test]
fn a_task_quality_floor_at_the_maximum_is_in_range() -> Outcome {
    let mut at_bound = task();
    at_bound.quality_floor_basis_points = Some(MAX_QUALITY_BASIS_POINTS);
    let mut top = recipe(ALPHA);
    top.quality_basis_points = Some(MAX_QUALITY_BASIS_POINTS);
    let decision = decide_task(&at_bound, &[top])?;
    assert_eq!(chosen(&decision), Some((ALPHA, "7")));
    assert_eq!(decision.explanation().decided_by(), Some(Rule::R11Ranking));
    let mut just_over = task();
    just_over.quality_floor_basis_points = Some(MAX_QUALITY_BASIS_POINTS + 1);
    assert_eq!(
        route(&policy()?, &just_over, &[recipe(ALPHA)], &baseline()),
        Err(Invalid::QualityRange { recipe: None })
    );
    Ok(())
}

/// One rendered diagnostic: the variant it came from, what `Display` produced, what is expected.
type Rendering = (&'static str, String, &'static str);

/// The variant names an enum declares, taken from the module source rather than from a list
/// written here: an include list cannot see an omission, so the denominator comes from the
/// artefact that decides. `src/route.rs` is read at compile time; no test reads a file at run time.
/// The body is read with the T09-RT-55 lexer (comments and literals removed) and a variant is an
/// identifier at brace depth zero after the opening brace or a comma, so a struct variant's
/// fields, a tuple variant's types, an attribute and a generic header are never counted.
fn declared_variants(name: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let source = include_str!("../src/route.rs");
    let header = format!("pub enum {name}");
    let start = source
        .match_indices(&header)
        .map(|(at, _)| at + header.len())
        .find(|&after| source[after..].starts_with([' ', '<']))
        .ok_or("enum declaration")?;
    let open = start + source[start..].find('{').ok_or("enum body")? + 1;
    let body = strip_comments_and_literals(&source[open..]);
    let mut variants = Vec::new();
    let mut depth = 0usize;
    let mut expecting = true;
    let mut ident = String::new();
    for ch in body.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            ident.push(ch);
            continue;
        }
        if !ident.is_empty() && depth == 0 && expecting {
            variants.push(std::mem::take(&mut ident));
            expecting = false;
        }
        ident.clear();
        match ch {
            '{' | '(' | '[' => depth += 1,
            '}' if depth == 0 => return Ok(variants),
            '}' | ')' | ']' => depth = depth.checked_sub(1).ok_or("unbalanced enum body")?,
            ',' => expecting = true,
            _ => {}
        }
    }
    Err("enum body is not closed".into())
}

fn invalid_renderings_a() -> Vec<Rendering> {
    vec![
        (
            "TooManyCandidates",
            Invalid::TooManyCandidates {
                count: 257,
                limit: 256,
            }
            .to_string(),
            "257 candidates exceed the bound of 256",
        ),
        (
            "TooManyCandidates",
            Invalid::TooManyCandidates { count: 9, limit: 4 }.to_string(),
            "9 candidates exceed the bound of 4",
        ),
        (
            "Identity",
            Invalid::Identity {
                recipe: "alpha".to_owned(),
            }
            .to_string(),
            "invalid recipe identity \"alpha\"",
        ),
        (
            "Identity",
            Invalid::Identity {
                recipe: "beta-2".to_owned(),
            }
            .to_string(),
            "invalid recipe identity \"beta-2\"",
        ),
        (
            "DuplicateIdentity",
            Invalid::DuplicateIdentity {
                recipe: "gamma".to_owned(),
            }
            .to_string(),
            "duplicate recipe identity \"gamma\"",
        ),
        (
            "DuplicateIdentity",
            Invalid::DuplicateIdentity {
                recipe: "delta-4".to_owned(),
            }
            .to_string(),
            "duplicate recipe identity \"delta-4\"",
        ),
        (
            "BaselineAmongCandidates",
            Invalid::BaselineAmongCandidates {
                recipe: "epsilon".to_owned(),
            }
            .to_string(),
            "baseline \"epsilon\" is also a candidate",
        ),
        (
            "BaselineAmongCandidates",
            Invalid::BaselineAmongCandidates {
                recipe: "zeta-6".to_owned(),
            }
            .to_string(),
            "baseline \"zeta-6\" is also a candidate",
        ),
    ]
}

fn invalid_renderings_b() -> Vec<Rendering> {
    vec![
        (
            "BaselineMismatch",
            Invalid::BaselineMismatch {
                declared: "eta".to_owned(),
                supplied: "theta".to_owned(),
            }
            .to_string(),
            "policy declares baseline \"eta\", caller supplied \"theta\"",
        ),
        (
            "BaselineMismatch",
            Invalid::BaselineMismatch {
                declared: "iota-9".to_owned(),
                supplied: "kappa-10".to_owned(),
            }
            .to_string(),
            "policy declares baseline \"iota-9\", caller supplied \"kappa-10\"",
        ),
        (
            "Capabilities",
            Invalid::Capabilities {
                recipe: Some("lambda".to_owned()),
            }
            .to_string(),
            "invalid capabilities on recipe \"lambda\"",
        ),
        (
            "Capabilities",
            Invalid::Capabilities {
                recipe: Some("mu-12".to_owned()),
            }
            .to_string(),
            "invalid capabilities on recipe \"mu-12\"",
        ),
        (
            "Capabilities",
            Invalid::Capabilities { recipe: None }.to_string(),
            "invalid required capabilities on the task",
        ),
        (
            "QualityRange",
            Invalid::QualityRange {
                recipe: Some("nu".to_owned()),
            }
            .to_string(),
            "quality figure out of range on \"nu\"",
        ),
        (
            "QualityRange",
            Invalid::QualityRange {
                recipe: Some("xi-14".to_owned()),
            }
            .to_string(),
            "quality figure out of range on \"xi-14\"",
        ),
        (
            "QualityRange",
            Invalid::QualityRange { recipe: None }.to_string(),
            "quality floor out of range on the task",
        ),
        (
            "UnknownPreviousAttempt",
            Invalid::UnknownPreviousAttempt {
                recipe: "omicron".to_owned(),
            }
            .to_string(),
            "previous attempt recipe \"omicron\" is neither a candidate nor the baseline",
        ),
        (
            "UnknownPreviousAttempt",
            Invalid::UnknownPreviousAttempt {
                recipe: "pi-16".to_owned(),
            }
            .to_string(),
            "previous attempt recipe \"pi-16\" is neither a candidate nor the baseline",
        ),
    ]
}

/// T09-RT-60 · the whole rendered diagnostic of every `Invalid` variant, over two fixtures per
/// field-carrying variant that differ in every field. The variant list is taken from the module
/// source, so a variant added without a rendering fails here rather than passing unseen.
#[test]
fn every_invalid_refusal_renders_its_own_whole_diagnostic() -> Outcome {
    let mut table = invalid_renderings_a();
    table.extend(invalid_renderings_b());
    for (variant, rendered, expected) in &table {
        assert_eq!(rendered, expected, "{variant} rendered wrongly");
    }
    let declared = declared_variants("Invalid")?;
    assert_eq!(declared.len(), 8, "Invalid declares eight variants");
    for variant in &declared {
        assert!(
            table.iter().any(|(name, ..)| name == variant),
            "no rendering case for Invalid::{variant}"
        );
    }
    for (variant, ..) in &table {
        assert!(
            declared.iter().any(|name| name == variant),
            "rendering case names an undeclared variant Invalid::{variant}"
        );
    }
    let distinct: BTreeSet<&str> = table.iter().map(|(_, r, _)| r.as_str()).collect();
    assert_eq!(distinct.len(), table.len(), "two fixtures rendered alike");
    Ok(())
}

fn config_renderings_a() -> Vec<Rendering> {
    vec![
        (
            "Syntax",
            ConfigError::Syntax.to_string(),
            "route configuration is not valid TOML",
        ),
        (
            "UnknownKey",
            ConfigError::UnknownKey {
                key: "filters.extra".to_owned(),
            }
            .to_string(),
            "unknown key \"filters.extra\"",
        ),
        (
            "UnknownKey",
            ConfigError::UnknownKey {
                key: "tie.mode".to_owned(),
            }
            .to_string(),
            "unknown key \"tie.mode\"",
        ),
        (
            "MissingKey",
            ConfigError::MissingKey {
                key: "ranking".to_owned(),
            }
            .to_string(),
            "missing key \"ranking\"",
        ),
        (
            "MissingKey",
            ConfigError::MissingKey {
                key: "staleness_bound_ms".to_owned(),
            }
            .to_string(),
            "missing key \"staleness_bound_ms\"",
        ),
        (
            "WrongType",
            ConfigError::WrongType {
                key: "schema_version".to_owned(),
            }
            .to_string(),
            "wrong type at key \"schema_version\"",
        ),
        (
            "WrongType",
            ConfigError::WrongType {
                key: "baseline.recipe".to_owned(),
            }
            .to_string(),
            "wrong type at key \"baseline.recipe\"",
        ),
        (
            "SchemaVersion",
            ConfigError::SchemaVersion { found: 2 }.to_string(),
            "unsupported schema_version 2",
        ),
        (
            "SchemaVersion",
            ConfigError::SchemaVersion { found: -7 }.to_string(),
            "unsupported schema_version -7",
        ),
    ]
}

fn config_renderings_b() -> Vec<Rendering> {
    vec![
        (
            "UnknownRule",
            ConfigError::UnknownRule {
                name: "colour".to_owned(),
            }
            .to_string(),
            "unknown filter rule \"colour\"",
        ),
        (
            "UnknownRule",
            ConfigError::UnknownRule {
                name: "weather".to_owned(),
            }
            .to_string(),
            "unknown filter rule \"weather\"",
        ),
        (
            "DuplicateRule",
            ConfigError::DuplicateRule {
                name: "privacy_class".to_owned(),
            }
            .to_string(),
            "duplicate filter rule \"privacy_class\"",
        ),
        (
            "DuplicateRule",
            ConfigError::DuplicateRule {
                name: "context_limit".to_owned(),
            }
            .to_string(),
            "duplicate filter rule \"context_limit\"",
        ),
        (
            "MissingRule",
            ConfigError::MissingRule {
                name: "availability".to_owned(),
            }
            .to_string(),
            "missing filter rule \"availability\"",
        ),
        (
            "MissingRule",
            ConfigError::MissingRule {
                name: "cost_ceiling".to_owned(),
            }
            .to_string(),
            "missing filter rule \"cost_ceiling\"",
        ),
        (
            "UnknownRankingKey",
            ConfigError::UnknownRankingKey {
                name: "charisma".to_owned(),
            }
            .to_string(),
            "unknown ranking key \"charisma\"",
        ),
        (
            "UnknownRankingKey",
            ConfigError::UnknownRankingKey {
                name: "hue".to_owned(),
            }
            .to_string(),
            "unknown ranking key \"hue\"",
        ),
    ]
}

fn config_renderings_c() -> Vec<Rendering> {
    vec![
        (
            "DuplicateRankingKey",
            ConfigError::DuplicateRankingKey {
                name: "cost".to_owned(),
            }
            .to_string(),
            "duplicate ranking key \"cost\"",
        ),
        (
            "DuplicateRankingKey",
            ConfigError::DuplicateRankingKey {
                name: "latency".to_owned(),
            }
            .to_string(),
            "duplicate ranking key \"latency\"",
        ),
        (
            "MissingRankingKey",
            ConfigError::MissingRankingKey {
                name: "quality".to_owned(),
            }
            .to_string(),
            "missing ranking key \"quality\"",
        ),
        (
            "MissingRankingKey",
            ConfigError::MissingRankingKey {
                name: "identity".to_owned(),
            }
            .to_string(),
            "missing ranking key \"identity\"",
        ),
        (
            "UnknownTieRule",
            ConfigError::UnknownTieRule {
                name: "coin_flip".to_owned(),
            }
            .to_string(),
            "unknown tie rule \"coin_flip\"",
        ),
        (
            "UnknownTieRule",
            ConfigError::UnknownTieRule {
                name: "first_seen".to_owned(),
            }
            .to_string(),
            "unknown tie rule \"first_seen\"",
        ),
        (
            "StalenessBound",
            ConfigError::StalenessBound { value: 0 }.to_string(),
            "staleness_bound_ms 0 is outside 1..=60000",
        ),
        (
            "StalenessBound",
            ConfigError::StalenessBound { value: 60_001 }.to_string(),
            "staleness_bound_ms 60001 is outside 1..=60000",
        ),
        (
            "MissingBaseline",
            ConfigError::MissingBaseline.to_string(),
            "no baseline recipe is declared",
        ),
    ]
}

fn config_renderings_e() -> Vec<Rendering> {
    vec![
        (
            "TooManyRecipes",
            ConfigError::TooManyRecipes {
                count: 129,
                limit: 128,
            }
            .to_string(),
            "129 recipes exceed the bound of 128",
        ),
        (
            "TooManyRecipes",
            ConfigError::TooManyRecipes {
                count: 300,
                limit: 17,
            }
            .to_string(),
            "300 recipes exceed the bound of 17",
        ),
        (
            "RecipeIdentity",
            ConfigError::RecipeIdentity { index: 0 }.to_string(),
            "invalid recipe identity at recipes[0]",
        ),
        (
            "RecipeIdentity",
            ConfigError::RecipeIdentity { index: 41 }.to_string(),
            "invalid recipe identity at recipes[41]",
        ),
        (
            "DuplicateRecipe",
            ConfigError::DuplicateRecipe {
                recipe: "chi".to_owned(),
            }
            .to_string(),
            "duplicate recipe identity \"chi\"",
        ),
        (
            "DuplicateRecipe",
            ConfigError::DuplicateRecipe {
                recipe: "psi-23".to_owned(),
            }
            .to_string(),
            "duplicate recipe identity \"psi-23\"",
        ),
        (
            "RecipeVersion",
            ConfigError::RecipeVersion {
                recipe: "omega".to_owned(),
            }
            .to_string(),
            "recipe \"omega\" version is outside 1..=65535",
        ),
        (
            "RecipeVersion",
            ConfigError::RecipeVersion {
                recipe: "alef-2".to_owned(),
            }
            .to_string(),
            "recipe \"alef-2\" version is outside 1..=65535",
        ),
        (
            "RecipeAdapter",
            ConfigError::RecipeAdapter {
                recipe: "bet".to_owned(),
            }
            .to_string(),
            "recipe \"bet\" adapter is not a UUIDv4",
        ),
    ]
}

fn config_renderings_f() -> Vec<Rendering> {
    vec![
        (
            "RecipeAdapter",
            ConfigError::RecipeAdapter {
                recipe: "gimel-3".to_owned(),
            }
            .to_string(),
            "recipe \"gimel-3\" adapter is not a UUIDv4",
        ),
        (
            "RecipeRosterRecord",
            ConfigError::RecipeRosterRecord {
                recipe: "dalet".to_owned(),
            }
            .to_string(),
            "recipe \"dalet\" roster_record is not a UUIDv4",
        ),
        (
            "RecipeRosterRecord",
            ConfigError::RecipeRosterRecord {
                recipe: "he-5".to_owned(),
            }
            .to_string(),
            "recipe \"he-5\" roster_record is not a UUIDv4",
        ),
        (
            "RecipeServes",
            ConfigError::RecipeServes {
                recipe: "vav".to_owned(),
            }
            .to_string(),
            "recipe \"vav\" serves no class, repeats one, names an invalid one or more than 128",
        ),
        (
            "RecipeServes",
            ConfigError::RecipeServes {
                recipe: "zayin-7".to_owned(),
            }
            .to_string(),
            "recipe \"zayin-7\" serves no class, repeats one, names an invalid one or more than 128",
        ),
        (
            "RecipeQuality",
            ConfigError::RecipeQuality {
                recipe: "het".to_owned(),
            }
            .to_string(),
            "recipe \"het\" quality figure exceeds 10000",
        ),
        (
            "RecipeQuality",
            ConfigError::RecipeQuality {
                recipe: "tet-9".to_owned(),
            }
            .to_string(),
            "recipe \"tet-9\" quality figure exceeds 10000",
        ),
        (
            "RecipeFigure",
            ConfigError::RecipeFigure {
                recipe: "yod".to_owned(),
                key: "cost_microunits".to_owned(),
            }
            .to_string(),
            "recipe \"yod\" figure \"cost_microunits\" is negative",
        ),
        (
            "RecipeFigure",
            ConfigError::RecipeFigure {
                recipe: "kaf-11".to_owned(),
                key: "latency_ms".to_owned(),
            }
            .to_string(),
            "recipe \"kaf-11\" figure \"latency_ms\" is negative",
        ),
    ]
}

fn config_renderings_d() -> Vec<Rendering> {
    vec![
        (
            "BaselineIdentity",
            ConfigError::BaselineIdentity {
                recipe: "omicron".to_owned(),
            }
            .to_string(),
            "invalid baseline identity \"omicron\"",
        ),
        (
            "BaselineIdentity",
            ConfigError::BaselineIdentity {
                recipe: "pi-16".to_owned(),
            }
            .to_string(),
            "invalid baseline identity \"pi-16\"",
        ),
        (
            "BaselineMismatch",
            ConfigError::BaselineMismatch {
                declared: "rho".to_owned(),
                supplied: "sigma".to_owned(),
            }
            .to_string(),
            "declared baseline \"rho\" is not the supplied baseline \"sigma\"",
        ),
        (
            "BaselineMismatch",
            ConfigError::BaselineMismatch {
                declared: "tau-19".to_owned(),
                supplied: "upsilon-20".to_owned(),
            }
            .to_string(),
            "declared baseline \"tau-19\" is not the supplied baseline \"upsilon-20\"",
        ),
        (
            "BaselineNotLocal",
            ConfigError::BaselineNotLocal {
                recipe: "phi".to_owned(),
                locality: Locality::Remote,
            }
            .to_string(),
            "baseline \"phi\" is Remote, not local, and cannot serve a local-only task",
        ),
        (
            "BaselineNotLocal",
            ConfigError::BaselineNotLocal {
                recipe: "chi-22".to_owned(),
                locality: Locality::Hybrid,
            }
            .to_string(),
            "baseline \"chi-22\" is Hybrid, not local, and cannot serve a local-only task",
        ),
        (
            "BaselineMissingFigure",
            ConfigError::BaselineMissingFigure {
                recipe: "psi".to_owned(),
                figure: Figure::Cost,
            }
            .to_string(),
            "baseline \"psi\" carries no Cost figure and cannot be screened",
        ),
        (
            "BaselineMissingFigure",
            ConfigError::BaselineMissingFigure {
                recipe: "omega-24".to_owned(),
                figure: Figure::Latency,
            }
            .to_string(),
            "baseline \"omega-24\" carries no Latency figure and cannot be screened",
        ),
    ]
}

/// T09-RT-61 · the whole rendered diagnostic of every `ConfigError` variant, over two fixtures
/// per field-carrying variant that differ in every field. The loader refuses by name, and the
/// name a reader sees is pinned here; the variant list comes from the module source.
#[test]
fn every_config_refusal_renders_its_own_whole_diagnostic() -> Outcome {
    let mut table = config_renderings_a();
    table.extend(config_renderings_b());
    table.extend(config_renderings_c());
    table.extend(config_renderings_d());
    table.extend(config_renderings_e());
    table.extend(config_renderings_f());
    for (variant, rendered, expected) in &table {
        assert_eq!(rendered, expected, "{variant} rendered wrongly");
    }
    let declared = declared_variants("ConfigError")?;
    assert_eq!(
        declared.len(),
        27,
        "ConfigError declares twenty-seven variants"
    );
    for variant in &declared {
        assert!(
            table.iter().any(|(name, ..)| name == variant),
            "no rendering case for ConfigError::{variant}"
        );
    }
    for (variant, ..) in &table {
        assert!(
            declared.iter().any(|name| name == variant),
            "rendering case names an undeclared variant ConfigError::{variant}"
        );
    }
    let distinct: BTreeSet<&str> = table.iter().map(|(_, r, _)| r.as_str()).collect();
    assert_eq!(distinct.len(), table.len(), "two fixtures rendered alike");
    Ok(())
}

fn fixture_recipe<'a>(row: &'a Value, caps: &'a [&'a str]) -> Result<Recipe<'a>, Box<dyn Error>> {
    let availability = match row["availability"]["kind"].as_str() {
        Some("unobserved") => Observation::Unobserved,
        _ => Observation::Observed {
            availability: match row["availability"]["availability"].as_str() {
                Some("available") => Availability::Available,
                Some("unavailable") => Availability::Unavailable,
                _ => Availability::Unknown,
            },
            age_ms: row["availability"]["age_ms"].as_u64().ok_or("age")?,
        },
    };
    Ok(Recipe {
        id: row["id"].as_str().ok_or("id")?,
        revision: row["revision"].as_str().ok_or("revision")?,
        capabilities: caps,
        context_limit_tokens: row.get("context_limit_tokens").ok_or("limit")?.as_u64(),
        locality: match row["locality"].as_str() {
            Some("local") => Locality::Local,
            Some("remote") => Locality::Remote,
            _ => Locality::Hybrid,
        },
        availability,
        cost_microunits: row["cost_microunits"].as_u64(),
        quality_basis_points: row["quality_basis_points"]
            .as_u64()
            .map(u16::try_from)
            .transpose()?,
        latency_ms: row["latency_ms"].as_u64(),
    })
}

/// The projection both sides compare: arm, dispatched recipe, reason, ranking order, exclusions and gaps.
fn project(decision: &Route<'_>) -> Value {
    let explanation = decision.explanation();
    let order: Vec<&str> = explanation
        .steps
        .iter()
        .find_map(|step| match step {
            Step::Ranked { order, .. } => Some(order.iter().map(|r| r.recipe).collect()),
            _ => None,
        })
        .unwrap_or_default();
    let exclusions: Vec<Value> = explanation
        .exclusions()
        .iter()
        .map(|(recipe, rule)| json!([recipe, rule.id()]))
        .collect();
    let gaps: Vec<Value> = explanation
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Gap {
                recipe,
                rule,
                evidence,
                ..
            } => Some(json!([recipe, rule.id(), gap_name(*evidence)])),
            _ => None,
        })
        .collect();
    let (arm, reason) = match decision {
        Route::Chosen { .. } => ("chosen", Value::Null),
        Route::Baseline { reason, .. } => ("baseline", fallback_name(reason)),
        Route::Refused {
            reason, fallback, ..
        } => (
            "refused",
            match (reason, fallback) {
                (Refusal::BaselineExcluded { rule, .. }, required) => {
                    json!(["baseline_excluded", rule.id(), fallback_name(required)])
                }
                (Refusal::BaselineEvidence { rule, .. }, required) => {
                    json!(["baseline_evidence", rule.id(), fallback_name(required)])
                }
            },
        ),
    };
    json!({
        "route": arm,
        "recipe": decision.dispatches(),
        "reason": reason,
        "decided_by": explanation.decided_by().map(Rule::id),
        "order": order,
        "exclusions": exclusions,
        "gaps": gaps,
    })
}

fn fallback_name(fallback: &Fallback<'_>) -> Value {
    match fallback {
        Fallback::Tie { between } => json!(["tie", between]),
        Fallback::InsufficientEvidence { .. } => json!(["insufficient_evidence"]),
        Fallback::NoEligibleCandidate => json!(["no_eligible_candidate"]),
    }
}

fn gap_name(gap: Gap) -> &'static str {
    match gap {
        Gap::Unobserved => "unobserved",
        Gap::AvailabilityUnknown { .. } => "availability_unknown",
        Gap::StaleAvailability { .. } => "stale_availability",
        Gap::MissingFigure {
            figure: Figure::Cost,
        } => "missing_cost",
        Gap::MissingFigure {
            figure: Figure::Quality,
        } => "missing_quality",
        Gap::MissingFigure {
            figure: Figure::Latency,
        } => "missing_latency",
        Gap::MissingFigure {
            figure: Figure::ContextLimit,
        } => "missing_context_limit",
    }
}

// ------------------------------------------------------------- decision-enum census

/// One named census case: the decision it produced and the whole serialized form it must equal.
type CensusCase = (&'static str, Route<'static>, Value);

/// Serialized names the census reads for each decision enum; each comes from the enum's own
/// `serde` tag, so a variant is counted only where a decision actually carried it.
const CENSUS_ENUMS: [&str; 5] = ["Exclusion", "Gap", "Step", "Fallback", "Refusal"];

/// Every failure mode of a hard filter, each excluding one candidate, under a task whose three
/// bounds the baseline meets exactly: seven exclusions and no eligible candidate (R10).
fn census_exclusions() -> Result<CensusCase, Box<dyn Error>> {
    let task = Task {
        required_capabilities: FINAL_ONLY,
        context_tokens: 8_192,
        privacy: PrivacyClass::LocalOnly,
        cost_ceiling_microunits: Some(100),
        deadline_ms: Some(1_000),
        quality_floor_basis_points: Some(5_000),
    };
    let mut e1 = recipe("e-1");
    e1.capabilities = &["identity"];
    let mut e2 = recipe("e-2");
    e2.context_limit_tokens = Some(8_191);
    let mut e3 = recipe("e-3");
    e3.locality = Locality::Remote;
    let mut e4 = recipe("e-4");
    e4.availability = observed(Availability::Unavailable, 7);
    let mut e5 = recipe("e-5");
    e5.cost_microunits = Some(101);
    let mut e6 = recipe("e-6");
    e6.latency_ms = Some(1_001);
    let mut e7 = recipe("e-7");
    e7.quality_basis_points = Some(4_999);
    let decision = decide_task(&task, &[e7, e6, e5, e4, e3, e2, e1])?;
    let expected = json!({
        "route": "baseline", "recipe": BASELINE_ID, "revision": BASELINE_REVISION,
        "reason": {"reason": "no_eligible_candidate"},
        "explanation": {"policy_revision": REVIEWED_REVISION, "steps": [
            {"step": "guarded", "recipe": BASELINE_ID, "passed": census_rules(7)},
            {"step": "excluded", "recipe": "e-1", "passed": census_rules(0), "rule": "R02RequiredCapabilities",
             "why": {"why": "missing_capability", "capability": "final_output"}},
            {"step": "excluded", "recipe": "e-2", "passed": census_rules(1), "rule": "R03ContextLimit",
             "why": {"why": "context_exceeded", "limit_tokens": 8191, "required_tokens": 8192}},
            {"step": "excluded", "recipe": "e-3", "passed": census_rules(2), "rule": "R04PrivacyClass",
             "why": {"why": "privacy_violated", "locality": "remote"}},
            {"step": "excluded", "recipe": "e-4", "passed": census_rules(3), "rule": "R05Availability",
             "why": {"why": "unavailable", "age_ms": 7}},
            {"step": "excluded", "recipe": "e-5", "passed": census_rules(4), "rule": "R06CostCeiling",
             "why": {"why": "cost_above_ceiling", "cost_microunits": 101, "ceiling_microunits": 100}},
            {"step": "excluded", "recipe": "e-6", "passed": census_rules(5), "rule": "R07Deadline",
             "why": {"why": "latency_above_deadline", "latency_ms": 1001, "deadline_ms": 1000}},
            {"step": "excluded", "recipe": "e-7", "passed": census_rules(6), "rule": "R08QualityFloor",
             "why": {"why": "quality_below_floor", "quality_basis_points": 4999, "floor_basis_points": 5000}},
            {"step": "decided", "rule": "R10NoEligibleCandidate"}
        ]}
    });
    Ok((
        "seven exclusions, no eligible candidate",
        decision,
        expected,
    ))
}

/// Every kind of evidence gap beside one eligible candidate: the gaps route to the baseline by
/// R09 and no ranking is reported.
fn census_gaps() -> Result<CensusCase, Box<dyn Error>> {
    let mut g1 = recipe("g-1");
    g1.availability = Observation::Unobserved;
    let mut g2 = recipe("g-2");
    g2.availability = observed(Availability::Unknown, 5);
    let mut g3 = recipe("g-3");
    g3.availability = observed(Availability::Available, BOUND_MS);
    let mut g4 = recipe("g-4");
    g4.context_limit_tokens = None;
    let decision = decide(&[recipe("g-5"), g4, g3, g2, g1])?;
    let gap = |recipe: &str, rule: &str, evidence: Value| json!({"recipe": recipe, "rule": rule, "evidence": evidence});
    let gaps = [
        gap("g-1", "R05Availability", json!({"evidence": "unobserved"})),
        gap(
            "g-2",
            "R05Availability",
            json!({"evidence": "availability_unknown", "age_ms": 5}),
        ),
        gap(
            "g-3",
            "R05Availability",
            json!({"evidence": "stale_availability", "age_ms": 30000, "bound_ms": 30000}),
        ),
        gap(
            "g-4",
            "R03ContextLimit",
            json!({"evidence": "missing_figure", "figure": "context_limit"}),
        ),
    ];
    let passed = [3, 3, 3, 1];
    let mut steps =
        vec![json!({"step": "guarded", "recipe": BASELINE_ID, "passed": census_rules(7)})];
    for (entry, count) in gaps.iter().zip(passed) {
        let mut step = entry.clone();
        step["step"] = json!("gap");
        step["passed"] = census_rules(count);
        steps.push(step);
    }
    steps.push(json!({"step": "eligible", "recipe": "g-5", "passed": census_rules(7)}));
    steps.push(json!({"step": "decided", "rule": "R09InsufficientEvidence"}));
    let expected = json!({
        "route": "baseline", "recipe": BASELINE_ID, "revision": BASELINE_REVISION,
        "reason": {"reason": "insufficient_evidence", "gaps": gaps},
        "explanation": {"policy_revision": REVIEWED_REVISION, "steps": steps}
    });
    Ok(("four gaps beside an eligible candidate", decision, expected))
}

/// Two candidates equal on every key: ranked, then a tie routed to the baseline by R12.
fn census_tie() -> Result<CensusCase, Box<dyn Error>> {
    let decision = decide(&[recipe("t-2"), recipe("t-1")])?;
    let row = |id: &str| json!({"recipe": id, "cost_microunits": 100, "quality_basis_points": 9000, "latency_ms": 500});
    let expected = json!({
        "route": "baseline", "recipe": BASELINE_ID, "revision": BASELINE_REVISION,
        "reason": {"reason": "tie", "between": ["t-1", "t-2"]},
        "explanation": {"policy_revision": REVIEWED_REVISION, "steps": [
            {"step": "guarded", "recipe": BASELINE_ID, "passed": census_rules(7)},
            {"step": "eligible", "recipe": "t-1", "passed": census_rules(7)},
            {"step": "eligible", "recipe": "t-2", "passed": census_rules(7)},
            {"step": "ranked", "rule": "R11Ranking", "order": [row("t-1"), row("t-2")]},
            {"step": "decided", "rule": "R12Tie"}
        ]}
    });
    Ok(("an exact tie", decision, expected))
}

/// A baseline lacking a capability the task requires, with nothing else to route to: the
/// required fallback is refused by R01 carrying the R02 exclusion.
fn census_refused_excluded() -> Result<CensusCase, Box<dyn Error>> {
    let mut task = task();
    task.required_capabilities = &["final_output", "vision"];
    let decision = decide_task(&task, &[])?;
    let why = json!({"why": "missing_capability", "capability": "vision"});
    let expected = json!({
        "route": "refused",
        "fallback": {"reason": "no_eligible_candidate"},
        "reason": {"reason": "baseline_excluded", "rule": "R02RequiredCapabilities", "why": why},
        "explanation": {"policy_revision": REVIEWED_REVISION, "steps": [
            {"step": "excluded", "recipe": BASELINE_ID, "passed": census_rules(0),
             "rule": "R02RequiredCapabilities", "why": why},
            {"step": "decided", "rule": "R10NoEligibleCandidate"},
            {"step": "decided", "rule": "R01BaselineGuard"}
        ]}
    });
    Ok(("a baseline excluded when needed", decision, expected))
}

/// A baseline whose availability observation is stale, with nothing else to route to: the
/// required fallback is refused by R01 carrying the R05 gap.
fn census_refused_evidence() -> Result<CensusCase, Box<dyn Error>> {
    let mut stale = baseline();
    stale.availability = observed(Availability::Available, 40_000);
    let decision = route(&Policy::load(CONFIG, &stale)?, &task(), &[], &stale)?;
    let evidence = json!({"evidence": "stale_availability", "age_ms": 40000, "bound_ms": 30000});
    let expected = json!({
        "route": "refused",
        "fallback": {"reason": "no_eligible_candidate"},
        "reason": {"reason": "baseline_evidence", "rule": "R05Availability", "evidence": evidence},
        "explanation": {"policy_revision": REVIEWED_REVISION, "steps": [
            {"step": "gap", "recipe": BASELINE_ID, "passed": census_rules(3),
             "rule": "R05Availability", "evidence": evidence},
            {"step": "decided", "rule": "R10NoEligibleCandidate"},
            {"step": "decided", "rule": "R01BaselineGuard"}
        ]}
    });
    Ok((
        "a baseline without evidence when needed",
        decision,
        expected,
    ))
}

/// A fallback after the baseline's own attempt failed, with nothing else to route to: R13
/// excludes the baseline, and the required fallback is refused by R01 carrying that exclusion
/// and its failure category.
fn census_refused_previous() -> Result<CensusCase, Box<dyn Error>> {
    let decision = after(BASELINE_ID, Failure::Cancelled, &[])?;
    let why = json!({"why": "previous_attempt", "failure": "cancelled"});
    let expected = json!({
        "route": "refused",
        "fallback": {"reason": "no_eligible_candidate"},
        "reason": {"reason": "baseline_excluded", "rule": "R13PreviousAttempt", "why": why},
        "explanation": {"policy_revision": REVIEWED_REVISION, "steps": [
            {"step": "excluded", "recipe": BASELINE_ID, "passed": [], "rule": "R13PreviousAttempt", "why": why},
            {"step": "decided", "rule": "R10NoEligibleCandidate"},
            {"step": "decided", "rule": "R01BaselineGuard"}
        ]}
    });
    Ok(("a baseline whose own attempt failed", decision, expected))
}

/// The first `count` filter rules as their serialized names.
fn census_rules(count: usize) -> Value {
    json!(
        FILTER_RULES[..count]
            .iter()
            .map(|rule| format!("{rule:?}"))
            .collect::<Vec<_>>()
    )
}

/// The serde `snake_case` spelling of a declared variant name.
fn snake(name: &str) -> String {
    let mut out = String::new();
    for (at, ch) in name.char_indices() {
        if ch.is_ascii_uppercase() && at > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

/// Every (enum, tag) a serialized decision carries, read by the position each enum occupies.
fn census_tags(decision: &Value, into: &mut BTreeSet<(&'static str, String)>) {
    let mut note = |name: &'static str, value: &Value| {
        if let Some(tag) = value.as_str() {
            into.insert((name, tag.to_owned()));
        }
    };
    let mut fallbacks = Vec::new();
    match decision["route"].as_str() {
        Some("baseline") => fallbacks.push(&decision["reason"]),
        Some("refused") => {
            fallbacks.push(&decision["fallback"]);
            let refusal = &decision["reason"];
            note("Refusal", &refusal["reason"]);
            note("Exclusion", &refusal["why"]["why"]);
            note("Gap", &refusal["evidence"]["evidence"]);
        }
        _ => {}
    }
    for fallback in fallbacks {
        note("Fallback", &fallback["reason"]);
        for gap in fallback["gaps"].as_array().into_iter().flatten() {
            note("Gap", &gap["evidence"]["evidence"]);
        }
    }
    for step in decision["explanation"]["steps"]
        .as_array()
        .into_iter()
        .flatten()
    {
        note("Step", &step["step"]);
        note("Exclusion", &step["why"]["why"]);
        note("Gap", &step["evidence"]["evidence"]);
    }
}

/// T09-RT-64 · census of the five decision enums (route-G10; T09 closure W3). Every variant of
/// `Exclusion`, `Gap`, `Step`, `Fallback` and `Refusal` — enumerated from `src/route.rs`, not listed
/// here — is produced by at least one named case whose whole serialized decision equals its
/// literal; a produced tag no declared variant spells is refused too. Prints `variants=N/N`.
#[test]
fn every_decision_enum_variant_is_produced_by_a_named_whole_case() -> Outcome {
    let cases = [
        census_exclusions()?,
        census_gaps()?,
        census_tie()?,
        census_refused_excluded()?,
        census_refused_evidence()?,
        census_refused_previous()?,
    ];
    let mut produced = BTreeSet::new();
    for (name, decision, expected) in &cases {
        let serialized = serde_json::to_value(decision)?;
        assert_eq!(&serialized, expected, "census case {name:?}");
        census_tags(&serialized, &mut produced);
    }
    let mut declared = BTreeSet::new();
    for name in CENSUS_ENUMS {
        for variant in declared_variants(name)? {
            declared.insert((name, snake(&variant)));
        }
    }
    let missing: Vec<_> = declared.difference(&produced).collect();
    let undeclared: Vec<_> = produced.difference(&declared).collect();
    let covered = declared.intersection(&produced).count();
    println!(
        "variants={covered}/{} cases={}",
        declared.len(),
        cases.len()
    );
    assert!(missing.is_empty(), "no named case produces {missing:?}");
    assert!(
        undeclared.is_empty(),
        "cases produce undeclared tags {undeclared:?}"
    );
    assert_eq!(
        declared.len(),
        23,
        "8 exclusions, 4 gaps, 6 steps, 3 fallbacks, 2 refusals"
    );
    Ok(())
}

// ---- B07a · declared recipes (route owns them; `config/routes.toml` `recipes`) ----------------

/// `sha256sum` of `printf 'hee3-route-recipes\nschema_version=1\ncount=0\n'`, computed outside Rust.
const EMPTY_RECIPES_REVISION: &str =
    "sha256:c53ff16b5668c96e25cd51ecb33bb7d452d7448d25482a092362eb102e0966b4";
/// `sha256sum` of the literal two-recipe rendering written out in T09-RT-74, computed outside Rust.
const TWO_RECIPES_REVISION: &str =
    "sha256:c65951dd32cb5596a9cc2c5547467144060d262e34738ae8bbe84670d0b713de";
const ROUTINE: &str = r#"{ id = "routine-code", version = 3, adapter = "09000000-0000-4000-8000-0000000000e1", actual_model_required = true, roster_record = "09000000-0000-4000-8000-0000000000f1", serves = ["rust-library-change/1"], context_limit_tokens = 32768, cost_microunits = 0, quality_basis_points = 7000, latency_ms = 900 }"#;
const PRIVATE: &str = r#"{ id = "private-local", version = 1, adapter = "09000000-0000-4000-8000-0000000000e2", actual_model_required = false, roster_record = "09000000-0000-4000-8000-0000000000f2", serves = ["b-class", "a-class"] }"#;

/// The shipped configuration with its empty `recipes = []` replaced by `rows`.
fn with_recipes(rows: &[&str]) -> String {
    assert_eq!(
        CONFIG.matches("recipes = []").count(),
        1,
        "one empty recipes key ships"
    );
    CONFIG.replacen(
        "recipes = []",
        &format!("recipes = [\n  {},\n]", rows.join(",\n  ")),
        1,
    )
}

/// A `ROUTINE` row with `member` replaced (or added) as `key = value`.
fn routine_with(key: &str, value: &str) -> String {
    let mut members: Vec<String> = ROUTINE
        .trim_start_matches("{ ")
        .trim_end_matches(" }")
        .split(", ")
        .filter(|member| !member.starts_with(&format!("{key} =")))
        .map(str::to_owned)
        .collect();
    if !value.is_empty() {
        members.push(format!("{key} = {value}"));
    }
    format!("{{ {} }}", members.join(", "))
}

/// T09-RT-73 · the shipped `config/routes.toml` declares no recipes (commissioning populates them,
/// RC02/T18): one parse reads the declaration and the empty recipe set, whose revision is the
/// digest of the empty rendering; the policy it yields is the one `Policy::load` yields (C1); the
/// `[baseline]` token and the `recipes = []` key each appear exactly once (C4).
#[test]
fn the_shipped_configuration_declares_no_recipes_and_one_policy() -> Outcome {
    let routing = Routing::parse(CONFIG)?;
    assert_eq!(routing.recipes(), &[] as &[DeclaredRecipe]);
    assert_eq!(routing.baseline(), BASELINE_ID);
    assert_eq!(routing.recipes_revision(), EMPTY_RECIPES_REVISION);
    assert_eq!(routing.policy(&baseline())?, policy()?);
    assert_eq!(routing.policy(&baseline())?.revision(), REVIEWED_REVISION);
    assert_eq!(CONFIG.matches("[baseline]").count(), 1);
    assert_eq!(CONFIG.matches("recipes = []").count(), 1);
    Ok(())
}

/// T09-RT-74 · two declared recipes that differ in every field read back whole, in identity order
/// with `serves` sorted; an absent figure is `None` (unknown, never invented). The recipe-set
/// revision is `sha256:` over the canonical rendering written out below; its digest was computed
/// by `sha256sum` over that literal, so the rendering is pinned byte for byte. Row order in the
/// source does not enter it; recipes do not enter the policy's own revision (C3).
#[test]
fn declared_recipes_read_back_whole_with_their_own_revision() -> Outcome {
    let source = with_recipes(&[ROUTINE, PRIVATE]);
    let routing = Routing::parse(&source)?;
    let local = DeclaredRecipe {
        id: "private-local".to_owned(),
        version: 1,
        adapter: "09000000-0000-4000-8000-0000000000e2".to_owned(),
        actual_model_required: false,
        roster_record: "09000000-0000-4000-8000-0000000000f2".to_owned(),
        serves: vec!["a-class".to_owned(), "b-class".to_owned()],
        context_limit_tokens: None,
        cost_microunits: None,
        quality_basis_points: None,
        latency_ms: None,
    };
    let coding = DeclaredRecipe {
        id: "routine-code".to_owned(),
        version: 3,
        adapter: "09000000-0000-4000-8000-0000000000e1".to_owned(),
        actual_model_required: true,
        roster_record: "09000000-0000-4000-8000-0000000000f1".to_owned(),
        serves: vec!["rust-library-change/1".to_owned()],
        context_limit_tokens: Some(32_768),
        cost_microunits: Some(0),
        quality_basis_points: Some(7_000),
        latency_ms: Some(900),
    };
    assert_eq!(routing.recipes(), &[local.clone(), coding.clone()]);
    // The rendering, written out whole (review G3): `sha256sum` over exactly these bytes is
    // TWO_RECIPES_REVISION, so the code's revision equal to it pins its rendering byte for byte.
    let rendering = "hee3-route-recipes\nschema_version=1\ncount=2\n\
        recipe.id=private-local\nrecipe.version=1\n\
        recipe.adapter=09000000-0000-4000-8000-0000000000e2\nrecipe.actual_model_required=false\n\
        recipe.roster_record=09000000-0000-4000-8000-0000000000f2\nrecipe.serves.count=2\n\
        recipe.serves=a-class\nrecipe.serves=b-class\nrecipe.context_limit_tokens=unknown\n\
        recipe.cost_microunits=unknown\nrecipe.quality_basis_points=unknown\n\
        recipe.latency_ms=unknown\n\
        recipe.id=routine-code\nrecipe.version=3\n\
        recipe.adapter=09000000-0000-4000-8000-0000000000e1\nrecipe.actual_model_required=true\n\
        recipe.roster_record=09000000-0000-4000-8000-0000000000f1\nrecipe.serves.count=1\n\
        recipe.serves=rust-library-change/1\nrecipe.context_limit_tokens=32768\n\
        recipe.cost_microunits=0\nrecipe.quality_basis_points=7000\nrecipe.latency_ms=900\n";
    let digest = sha2::Sha256::digest(rendering.as_bytes()).iter().fold(
        String::from("sha256:"),
        |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        },
    );
    assert_eq!(
        digest, TWO_RECIPES_REVISION,
        "the literal is what sha256sum hashed"
    );
    assert_eq!(routing.recipes_revision(), TWO_RECIPES_REVISION);
    let swapped = Routing::parse(&with_recipes(&[PRIVATE, ROUTINE]))?;
    assert_eq!(swapped.recipes(), &[local, coding]);
    assert_eq!(swapped.recipes_revision(), TWO_RECIPES_REVISION);
    assert_eq!(routing.policy(&baseline())?.revision(), REVIEWED_REVISION);
    Ok(())
}

/// The recipe refusal cases of T09-RT-75: the key and type rules.
fn recipe_key_cases() -> Vec<(String, ConfigError)> {
    let routine = |key: &str, value: &str| routine_with(key, value);
    vec![
        (
            CONFIG.replacen("recipes = []", "recipes = 5", 1),
            ConfigError::WrongType {
                key: "recipes".to_owned(),
            },
        ),
        (
            with_recipes(&["7"]),
            ConfigError::WrongType {
                key: "recipes[0]".to_owned(),
            },
        ),
        (
            with_recipes(&[PRIVATE, &routine("model", "\"x\"")]),
            ConfigError::UnknownKey {
                key: "recipes[1].model".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("id", "")]),
            ConfigError::MissingKey {
                key: "recipes[0].id".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("serves", "")]),
            ConfigError::MissingKey {
                key: "recipes[0].serves".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("version", "\"3\"")]),
            ConfigError::WrongType {
                key: "recipes[0].version".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("actual_model_required", "1")]),
            ConfigError::WrongType {
                key: "recipes[0].actual_model_required".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("latency_ms", "\"900\"")]),
            ConfigError::WrongType {
                key: "recipes[0].latency_ms".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("serves", "[1]")]),
            ConfigError::WrongType {
                key: "recipes[0].serves".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("id", "\"\"")]),
            ConfigError::RecipeIdentity { index: 0 },
        ),
        (
            with_recipes(&[PRIVATE, &routine("id", "\"a\\u0001b\"")]),
            ConfigError::RecipeIdentity { index: 1 },
        ),
    ]
}

/// The recipe refusal cases of T09-RT-75: the value rules.
fn recipe_value_cases() -> Vec<(String, ConfigError)> {
    let routine = |key: &str, value: &str| routine_with(key, value);
    // 129 distinct valid classes: over route's capability bound (review N7); 128 is admitted.
    let classes = |count: usize| {
        let names: Vec<String> = (0..count)
            .map(|index| format!("\"c-{index:03}\""))
            .collect();
        format!("[{}]", names.join(", "))
    };
    assert!(Routing::parse(&with_recipes(&[&routine("serves", &classes(128))])).is_ok());
    vec![
        (
            with_recipes(&[&routine("serves", &classes(129))]),
            ConfigError::RecipeServes {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[ROUTINE, ROUTINE]),
            ConfigError::DuplicateRecipe {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("version", "0")]),
            ConfigError::RecipeVersion {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("version", "65536")]),
            ConfigError::RecipeVersion {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine(
                "adapter",
                "\"09000000-0000-1000-8000-0000000000e1\"",
            )]),
            ConfigError::RecipeAdapter {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("roster_record", "\"f1\"")]),
            ConfigError::RecipeRosterRecord {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("serves", "[]")]),
            ConfigError::RecipeServes {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("serves", "[\"a\", \"a\"]")]),
            ConfigError::RecipeServes {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("serves", "[\"a\\u0007\"]")]),
            ConfigError::RecipeServes {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("quality_basis_points", "10001")]),
            ConfigError::RecipeQuality {
                recipe: "routine-code".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("cost_microunits", "-1")]),
            ConfigError::RecipeFigure {
                recipe: "routine-code".to_owned(),
                key: "cost_microunits".to_owned(),
            },
        ),
        (
            with_recipes(&[&routine("context_limit_tokens", "-5")]),
            ConfigError::RecipeFigure {
                recipe: "routine-code".to_owned(),
                key: "context_limit_tokens".to_owned(),
            },
        ),
    ]
}

/// T09-RT-75 · every recipe refusal by name, one mutation of one valid row each.
#[test]
fn each_recipe_refusal_names_its_row_and_rule() {
    let mut cases = recipe_key_cases();
    cases.extend(recipe_value_cases());
    for (source, expected) in cases {
        assert_eq!(Routing::parse(&source), Err(expected.clone()), "{expected}");
        assert_eq!(Policy::load(&source, &baseline()), Err(expected));
    }
    // The quality bound is inclusive: 10000 is admitted.
    let at_bound = routine_with("quality_basis_points", "10000");
    assert!(Routing::parse(&with_recipes(&[&at_bound])).is_ok());
}

/// T09-RT-76 · the declaration is checked before any recipe (C2): a declaration defect wins over a
/// recipe defect in the same source, and a missing `recipes` key is refused after every
/// declaration key (R3.1), so no existing refusal moved.
#[test]
fn declaration_refusals_come_before_recipe_refusals() {
    let bad_recipe = with_recipes(&[&routine_with("version", "0")]);
    let bad_both = bad_recipe.replacen("\"deadline\"", "\"dead_line\"", 1);
    assert_eq!(
        Routing::parse(&bad_both),
        Err(ConfigError::UnknownRule {
            name: "dead_line".to_owned()
        })
    );
    let unknown_tie = bad_recipe.replacen("tie = \"baseline\"", "tie = \"coin\"", 1);
    assert_eq!(
        Routing::parse(&unknown_tie),
        Err(ConfigError::UnknownTieRule {
            name: "coin".to_owned()
        })
    );
    let bad_identity = bad_recipe.replacen(BASELINE_ID, "caf\u{e9}", 1);
    assert_eq!(
        Routing::parse(&bad_identity),
        Err(ConfigError::BaselineIdentity {
            recipe: "caf\u{e9}".to_owned()
        })
    );
    let without = CONFIG.replacen("recipes = []", "", 1);
    assert_eq!(
        Routing::parse(&without),
        Err(ConfigError::MissingKey {
            key: "recipes".to_owned()
        })
    );
    let without_and_bad = without.replacen("\"deadline\"", "\"dead_line\"", 1);
    assert_eq!(
        Routing::parse(&without_and_bad),
        Err(ConfigError::UnknownRule {
            name: "dead_line".to_owned()
        })
    );
}

/// T09-RT-77 · the recipe count is bounded at acquisition by `MAX_RECIPES` (128, the preview
/// result's array bound): 128 rows parse, 129 are refused by count with both numbers.
#[test]
fn the_recipe_set_is_bounded_by_count() -> Outcome {
    assert_eq!(MAX_RECIPES, 128);
    let rows: Vec<String> = (0..=MAX_RECIPES)
        .map(|index| routine_with("id", &format!("\"r-{index:03}\"")))
        .collect();
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    assert_eq!(
        Routing::parse(&with_recipes(&refs[..MAX_RECIPES]))?
            .recipes()
            .len(),
        128
    );
    assert_eq!(
        Routing::parse(&with_recipes(&refs)),
        Err(ConfigError::TooManyRecipes {
            count: 129,
            limit: 128
        })
    );
    Ok(())
}
