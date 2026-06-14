//! End-to-end integration tests for the probabilistic path of the
//! engine, using the top-level `search` API and the `SearchMode::AutoDetect`
//! short-circuit theorem from LP19.

use logic_core::{atom, compound, var, Term};
use logic_engine::{
    search, BodyLiteral, Fact, KnowledgeBase, Rule, SearchMode, SearchResult,
};

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn auto_detect_short_circuits_to_find_first_when_all_certain() {
    // A small all-Certain KB. AutoDetect should pick FindFirst and
    // return a single binding without materialising a proof DAG.
    let mut kb = KnowledgeBase::new();
    kb.add_fact(Fact::certain(compound(
        "father",
        vec![atom("homer"), atom("bart")],
    )));

    let x = var("X");
    let query = compound("father", vec![atom("homer"), Term::Var(x.clone())]);

    let result = search(&query, &kb, SearchMode::AutoDetect);
    match result {
        SearchResult::FindFirstResult(Some(subst)) => {
            assert_eq!(subst.walk_var(&x), atom("bart"));
        }
        other => panic!(
            "AutoDetect on all-Certain KB should yield FindFirstResult, got: {:?}",
            other
        ),
    }
}

#[test]
fn auto_detect_switches_to_enumerate_when_any_clause_is_probabilistic() {
    // Add a single probabilistic fact; AutoDetect should pick EnumerateAll
    // and the result should carry a proof DAG and a probability.
    let mut kb = KnowledgeBase::new();
    kb.add_fact(Fact::with_probability(atom("a"), 0.7));

    let result = search(&atom("a"), &kb, SearchMode::AutoDetect);
    match result {
        SearchResult::EnumerateAllResult { dag, probability } => {
            assert!(dag.has_proof());
            assert!(approx_eq(probability, 0.7));
        }
        other => panic!(
            "AutoDetect with a probabilistic fact should enumerate; got {:?}",
            other
        ),
    }
}

#[test]
fn probabilistic_graph_reachability_returns_0_86() {
    // The canonical LP19 worked example, run through the top-level API.
    let mut kb = KnowledgeBase::new();
    kb.add_fact(Fact::with_probability(
        compound("edge", vec![atom("a"), atom("b")]),
        0.9,
    ));
    kb.add_fact(Fact::with_probability(
        compound("edge", vec![atom("b"), atom("c")]),
        0.8,
    ));
    kb.add_fact(Fact::with_probability(
        compound("edge", vec![atom("a"), atom("c")]),
        0.5,
    ));

    let x = var("X");
    let y = var("Y");
    kb.add_rule(Rule::certain(
        compound("path", vec![Term::Var(x.clone()), Term::Var(y.clone())]),
        vec![BodyLiteral::Pos(compound(
            "edge",
            vec![Term::Var(x.clone()), Term::Var(y.clone())],
        ))],
    ));

    let x = var("X");
    let y = var("Y");
    let z = var("Z");
    kb.add_rule(Rule::certain(
        compound("path", vec![Term::Var(x.clone()), Term::Var(y.clone())]),
        vec![
            BodyLiteral::Pos(compound(
                "edge",
                vec![Term::Var(x.clone()), Term::Var(z.clone())],
            )),
            BodyLiteral::Pos(compound(
                "path",
                vec![Term::Var(z.clone()), Term::Var(y.clone())],
            )),
        ],
    ));

    let result = search(
        &compound("path", vec![atom("a"), atom("c")]),
        &kb,
        SearchMode::AutoDetect,
    );

    match result {
        SearchResult::EnumerateAllResult { dag, probability } => {
            assert_eq!(dag.proofs.len(), 2, "exactly two derivations");
            assert!(
                approx_eq(probability, 0.86),
                "expected 0.86, got {}",
                probability
            );
        }
        other => panic!(
            "AutoDetect with probabilistic edges should enumerate; got {:?}",
            other
        ),
    }
}

#[test]
fn forced_enumerate_all_on_certain_kb_returns_probability_one() {
    // A user can request EnumerateAll even on an all-Certain KB. The
    // result should still be consistent with the short-circuit:
    // probability is exactly 1.0 when at least one proof exists.
    let mut kb = KnowledgeBase::new();
    kb.add_fact(Fact::certain(atom("a")));
    kb.add_rule(Rule::certain(atom("b"), vec![BodyLiteral::Pos(atom("a"))]));

    let result = search(&atom("b"), &kb, SearchMode::EnumerateAll);
    match result {
        SearchResult::EnumerateAllResult { dag, probability } => {
            assert!(dag.has_proof());
            assert!(approx_eq(probability, 1.0));
        }
        other => panic!("expected enumerate result, got {:?}", other),
    }
}
