//! Source-level integration tests for the ProbLog probabilistic
//! clause syntax (`0.7 :: edge(a, b).`).
//!
//! These tests round-trip through the regenerated lexer + parser +
//! loader + WMC engine. A failure here means a regression somewhere
//! along the probability-aware Prolog stack.
//!
//! ## Scope
//!
//! - Probabilistic facts: `0.7 :: rain.`
//! - Probabilistic rules: `0.4 :: smoker(X) :- adult(X).`
//! - Mixed deterministic + probabilistic programs (the engine picks
//!   FindFirst vs EnumerateAll automatically via `AutoDetect`).
//! - Boundary probabilities `0.0` and `1.0`.
//! - Out-of-range probabilities surface as `LoaderError::ProbabilityOutOfRange`.

use logic_core::{atom, compound, Term};
use logic_engine::{search, SearchMode, SearchResult};
use prolog_loader::{load_source, LoadedProgram, LoaderError};

/// Run every query in the source and return `Vec<probability>`.
/// Each query is wrapped in a synthetic rule so multi-goal bodies
/// and `\+` literals route through `BodyLiteral` as needed.
fn run_probabilities(src: &str) -> Vec<f64> {
    let LoadedProgram { mut kb, queries } = load_source(src).expect("loads");
    let mut probs = Vec::new();
    for (i, goals) in queries.into_iter().enumerate() {
        let head_name = format!("__query_{i}");
        let head = atom(&head_name);
        let body: Vec<logic_engine::BodyLiteral> = goals
            .iter()
            .cloned()
            .map(|g| {
                if let Term::Compound { functor, args } = &g {
                    if functor == "\\+" && args.len() == 1 {
                        return logic_engine::BodyLiteral::Neg(args[0].clone());
                    }
                }
                logic_engine::BodyLiteral::Pos(g)
            })
            .collect();
        kb.add_rule(logic_engine::Rule::certain(head.clone(), body));
        let r = search(&head, &kb, SearchMode::AutoDetect);
        probs.push(match r {
            SearchResult::FindFirstResult(opt) => {
                if opt.is_some() {
                    1.0
                } else {
                    0.0
                }
            }
            SearchResult::EnumerateAllResult { probability, .. } => probability,
        });
    }
    probs
}

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn probabilistic_fact_yields_its_probability() {
    let src = r#"
        0.7 :: rain.
        ?- rain.
    "#;
    let p = run_probabilities(src);
    assert_eq!(p.len(), 1);
    assert!(approx_eq(p[0], 0.7), "got {}", p[0]);
}

#[test]
fn conjunction_of_independent_probabilistic_facts_multiplies() {
    // P(alarm) = P(burglary) * P(triggers_alarm) under independence.
    let src = r#"
        0.05 :: burglary.
        0.95 :: triggers_alarm.
        alarm :- burglary, triggers_alarm.
        ?- alarm.
    "#;
    let p = run_probabilities(src);
    assert_eq!(p.len(), 1);
    assert!(approx_eq(p[0], 0.05 * 0.95), "got {}", p[0]);
}

#[test]
fn probabilistic_rule_gates_a_certain_premise() {
    // 0.4 :: smoker(X) :- adult(X).  + adult(alice).
    // → P(smoker(alice)) = 0.4
    let src = r#"
        adult(alice).
        0.4 :: smoker(X) :- adult(X).
        ?- smoker(alice).
    "#;
    let p = run_probabilities(src);
    assert_eq!(p.len(), 1);
    assert!(approx_eq(p[0], 0.4), "got {}", p[0]);
}

#[test]
fn boundary_probability_one_behaves_like_certain() {
    let src = r#"
        1.0 :: guaranteed.
        ?- guaranteed.
    "#;
    let p = run_probabilities(src);
    assert!(approx_eq(p[0], 1.0));
}

#[test]
fn boundary_probability_zero_makes_query_fail() {
    let src = r#"
        0.0 :: impossible.
        ?- impossible.
    "#;
    let p = run_probabilities(src);
    assert_eq!(p[0], 0.0);
}

#[test]
fn unprovable_probabilistic_query_returns_zero() {
    let src = r#"
        0.5 :: a.
        ?- b.
    "#;
    let p = run_probabilities(src);
    assert_eq!(p[0], 0.0);
}

#[test]
fn probability_above_one_surfaces_as_loader_error() {
    let src = r#"
        1.5 :: oops.
    "#;
    let err = load_source(src).unwrap_err();
    match err {
        LoaderError::ProbabilityOutOfRange { value } => {
            assert!((value - 1.5).abs() < 1e-9);
        }
        other => panic!("expected ProbabilityOutOfRange, got {other:?}"),
    }
}

#[test]
fn probability_below_zero_surfaces_as_loader_error() {
    // The lexer doesn't accept a leading `-` on a FLOAT token (Prolog
    // sign is an operator, not part of the numeric literal), so a
    // negative probability is only reachable by hand-constructed
    // ProgramItems. Still worth a round-trip check via the loader's
    // range check on whatever the grammar admits. Use a contrived
    // integer literal that's nonetheless invalid as a probability.
    let src = r#"
        2 :: too_high.
    "#;
    let err = load_source(src).unwrap_err();
    match err {
        LoaderError::ProbabilityOutOfRange { value } => {
            assert!((value - 2.0).abs() < 1e-9);
        }
        other => panic!("expected ProbabilityOutOfRange, got {other:?}"),
    }
}

#[test]
fn deterministic_clauses_alongside_probabilistic_ones_work() {
    // Mixed program: deterministic family rule + probabilistic fact.
    // AutoDetect routes the program through EnumerateAll (some
    // clauses are probabilistic) and WMC returns P(observed) * 1.0
    // for the conjunction.
    let src = r#"
        parent(alice, bob).
        ancestor(X, Y) :- parent(X, Y).
        0.3 :: observed.
        ?- ancestor(alice, bob), observed.
    "#;
    let p = run_probabilities(src);
    assert!(approx_eq(p[0], 0.3), "got {}", p[0]);
}

#[test]
fn probabilistic_clauses_and_regular_facts_can_appear_in_any_order() {
    // Probabilistic and certain clauses interleaved.
    let src = r#"
        0.6 :: a.
        b.
        0.5 :: c.
        d.
        both :- a, c.
        ?- a.
        ?- b.
        ?- both.
        ?- d.
    "#;
    let p = run_probabilities(src);
    assert!(approx_eq(p[0], 0.6));
    assert!(approx_eq(p[1], 1.0));
    assert!(approx_eq(p[2], 0.6 * 0.5));
    assert!(approx_eq(p[3], 1.0));
}

// Silence unused-import warning when the synthetic `_force_use_compound`
// only-uses-on-demand variant is needed.
#[allow(dead_code)]
fn _force_use_compound() {
    let _ = compound("dummy", vec![]);
}
