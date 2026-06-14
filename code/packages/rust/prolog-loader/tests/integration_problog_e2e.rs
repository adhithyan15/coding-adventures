//! End-to-end integration tests for the ProbLog (probabilistic
//! Prolog) pipeline.
//!
//! These tests exercise the engine + WMC backend through the
//! `ProblogProgram` builder API. They're the regression check that
//! a probabilistic Prolog program — facts and rules with explicit
//! `Probability::Value(p)` — produces the right WMC answers when
//! run through `logic_engine::search` under `EnumerateAll` mode.
//!
//! ## Why builder API instead of source text
//!
//! The ISO-Prolog grammar does not yet recognise the `0.7::fact.`
//! syntax (the lexer rejects a clause starting with a `FLOAT`
//! token). When the grammar gains a probabilistic-clause production,
//! `load_problog_source(src)` will be a thin wrapper that translates
//! parsed clauses into the same builder calls these tests use. The
//! engine half — what we test here — is already complete.
//!
//! ## Coverage
//!
//! - Single probabilistic fact: P(query) = annotation.
//! - Conjunctive rule over independent probabilistic facts:
//!   P(combined) = product (assumed-independent WMC).
//! - Probabilistic rule over a certain premise:
//!   P(head) = rule's annotation.
//! - Boundary probabilities 0.0 and 1.0.
//! - Multi-goal queries via the synthetic-head rewrite.
//! - Mixed deterministic and probabilistic clauses in one program.

use logic_core::{atom, compound, var, Term};
use logic_engine::{BodyLiteral, SearchMode};
use prolog_loader::ProblogProgram;

fn assert_close(actual: f64, expected: f64, tol: f64) {
    assert!(
        (actual - expected).abs() < tol,
        "expected {expected} ± {tol}, got {actual}",
    );
}

#[test]
fn rain_with_prob_0_7_yields_probability_0_7() {
    let (_kb, runs) = ProblogProgram::new()
        .with_prob_fact(0.7, atom("rain"))
        .with_query(vec![atom("rain")])
        .execute();
    assert_eq!(runs.len(), 1);
    assert_close(runs[0].probability(), 0.7, 1e-9);
}

#[test]
fn alarm_via_conjunctive_rule_multiplies_independent_factors() {
    // alarm :- burglary, triggers_alarm.
    // 0.05 :: burglary.
    // 0.95 :: triggers_alarm.
    // ?- alarm.
    // → 0.05 * 0.95 = 0.0475
    let (_kb, runs) = ProblogProgram::new()
        .with_prob_fact(0.05, atom("burglary"))
        .with_prob_fact(0.95, atom("triggers_alarm"))
        .with_rule(
            atom("alarm"),
            vec![
                BodyLiteral::Pos(atom("burglary")),
                BodyLiteral::Pos(atom("triggers_alarm")),
            ],
        )
        .with_query(vec![atom("alarm")])
        .execute();
    assert_close(runs[0].probability(), 0.05 * 0.95, 1e-9);
}

#[test]
fn probabilistic_rule_gates_a_certain_premise() {
    // 0.4 :: smoker(X) :- adult(X).
    // adult(alice).
    // ?- smoker(alice).
    // → 0.4
    let (_kb, runs) = ProblogProgram::new()
        .with_fact(compound("adult", vec![atom("alice")]))
        .with_prob_rule(
            0.4,
            compound("smoker", vec![Term::Var(var("X"))]),
            vec![BodyLiteral::Pos(compound("adult", vec![Term::Var(var("X"))]))],
        )
        .with_query(vec![compound("smoker", vec![atom("alice")])])
        .execute();
    assert_close(runs[0].probability(), 0.4, 1e-9);
}

#[test]
fn mixed_deterministic_and_probabilistic_program_works() {
    // Deterministic family rules + one probabilistic fact gating an
    // observation. Verifies AutoDetect routes the program through
    // EnumerateAll (some clauses are uncertain) and the WMC value
    // comes out right.
    let (_kb, runs) = ProblogProgram::new()
        .with_fact(compound(
            "parent",
            vec![atom("alice"), atom("bob")],
        ))
        .with_rule(
            compound(
                "ancestor",
                vec![Term::Var(var("X")), Term::Var(var("Y"))],
            ),
            vec![BodyLiteral::Pos(compound(
                "parent",
                vec![Term::Var(var("X")), Term::Var(var("Y"))],
            ))],
        )
        .with_prob_fact(0.3, atom("observed"))
        .with_query(vec![
            compound("ancestor", vec![atom("alice"), atom("bob")]),
            atom("observed"),
        ])
        .execute();

    // Two-goal query: ancestor(alice, bob) is certain, observed has
    // probability 0.3. The synthetic-head rewrite combines them in a
    // single rule; WMC returns 0.3.
    assert_close(runs[0].probability(), 0.3, 1e-9);
}

#[test]
fn unprovable_query_returns_zero_probability() {
    let (_kb, runs) = ProblogProgram::new()
        .with_prob_fact(0.5, atom("a"))
        .with_query(vec![atom("nonexistent")])
        .execute();
    assert!(!runs[0].succeeded());
    assert_eq!(runs[0].probability(), 0.0);
}

#[test]
fn probability_one_behaves_like_certain_in_enumerate_all_mode() {
    let (_kb, runs) = ProblogProgram::new()
        .with_prob_fact(1.0, atom("guaranteed"))
        .with_query(vec![atom("guaranteed")])
        .execute_with_mode(SearchMode::EnumerateAll);
    assert_close(runs[0].probability(), 1.0, 1e-12);
}

#[test]
fn probability_zero_makes_query_fail() {
    let (_kb, runs) = ProblogProgram::new()
        .with_prob_fact(0.0, atom("impossible"))
        .with_query(vec![atom("impossible")])
        .execute_with_mode(SearchMode::EnumerateAll);
    assert!(!runs[0].succeeded());
    assert_close(runs[0].probability(), 0.0, 1e-12);
}

#[test]
fn empty_program_has_no_queries() {
    let (_kb, runs) = ProblogProgram::new().execute();
    assert!(runs.is_empty());
}

#[test]
fn deterministic_program_via_builder_returns_probability_one() {
    // The ProbLog builder must handle deterministic programs gracefully:
    // a program with no probabilistic clauses should run on the
    // FindFirst path under AutoDetect and report probability 1.0.
    let (_kb, runs) = ProblogProgram::new()
        .with_fact(atom("sunny"))
        .with_query(vec![atom("sunny")])
        .execute();
    assert_eq!(runs[0].probability(), 1.0);
}
