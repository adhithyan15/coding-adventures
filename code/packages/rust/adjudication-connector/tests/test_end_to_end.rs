//! End-to-end integration tests for the adjudication-connector.
//!
//! These exercise complete IR-document → search-result flows using the
//! top-level `run_adjudication` API.

use adjudication_connector::run_adjudication;
use adjudication_ir::{
    DocumentId, IRDocument, IRNode, Modality, NodeId, NodeKind, Polarity, Span,
};
use logic_core::{atom, compound, var, Term};
use logic_engine::SearchResult;
use std::collections::HashMap;

fn doc_id() -> DocumentId {
    DocumentId::new("e2e-test")
}

fn span(start: usize, end: usize) -> Span {
    Span::new(doc_id(), start, end)
}

fn list_of(terms: Vec<Term>) -> Term {
    logic_core::logic_list(terms)
}

fn fact(id: &str, term: Term, polarity: Polarity, start: usize, end: usize) -> IRNode {
    IRNode {
        id: NodeId::new(id),
        kind: NodeKind::Fact,
        term,
        polarity,
        modality: Modality::Present,
        source_spans: vec![span(start, end)],
        confidence: 0.95,
        part_of: None,
        lowered_from: None,
        discard_reason: None,
        metadata: HashMap::new(),
    }
}

fn rule(id: &str, term: Term, start: usize, end: usize) -> IRNode {
    IRNode {
        id: NodeId::new(id),
        kind: NodeKind::Rule,
        term,
        polarity: Polarity::Affirmed,
        modality: Modality::Present,
        source_spans: vec![span(start, end)],
        confidence: 1.0,
        part_of: None,
        lowered_from: None,
        discard_reason: None,
        metadata: HashMap::new(),
    }
}

fn query(id: &str, term: Term, start: usize, end: usize) -> IRNode {
    IRNode {
        id: NodeId::new(id),
        kind: NodeKind::Query,
        term,
        polarity: Polarity::Affirmed,
        modality: Modality::Present,
        source_spans: vec![span(start, end)],
        confidence: 1.0,
        part_of: None,
        lowered_from: None,
        discard_reason: None,
        metadata: HashMap::new(),
    }
}

#[test]
fn deterministic_grandparent_adjudication() {
    // A small family-relations adjudication: every clause is Certain,
    // so the engine's AutoDetect picks FindFirst.
    let xv = var("X");
    let yv = var("Y");
    let zv = var("Z");

    // parent(X, Y) :- father(X, Y).
    let parent_rule_term = compound(
        "definitional",
        vec![
            compound("parent", vec![Term::Var(xv.clone()), Term::Var(yv.clone())]),
            list_of(vec![compound(
                "father",
                vec![Term::Var(xv.clone()), Term::Var(yv.clone())],
            )]),
        ],
    );

    // grandparent(X, Z) :- parent(X, Y), parent(Y, Z).
    let gp_rule_term = compound(
        "definitional",
        vec![
            compound(
                "grandparent",
                vec![Term::Var(xv.clone()), Term::Var(zv.clone())],
            ),
            list_of(vec![
                compound("parent", vec![Term::Var(xv.clone()), Term::Var(yv.clone())]),
                compound("parent", vec![Term::Var(yv.clone()), Term::Var(zv.clone())]),
            ]),
        ],
    );

    let doc = IRDocument {
        document_id: doc_id(),
        nodes: vec![
            fact(
                "F1",
                compound("father", vec![atom("homer"), atom("bart")]),
                Polarity::Affirmed,
                0,
                20,
            ),
            fact(
                "F2",
                compound("father", vec![atom("grandpa_abe"), atom("homer")]),
                Polarity::Affirmed,
                21,
                40,
            ),
            rule("R1", parent_rule_term, 41, 80),
            rule("R2", gp_rule_term, 81, 120),
            query(
                "Q1",
                compound("grandparent", vec![atom("grandpa_abe"), atom("bart")]),
                121,
                160,
            ),
        ],
    };

    let results = run_adjudication(&doc).unwrap();
    assert_eq!(results.len(), 1);

    match &results[0].result {
        SearchResult::FindFirstResult(Some(_)) => {
            // Success — grandparent(grandpa_abe, bart) was derived.
        }
        other => panic!("expected success, got {:?}", other),
    }
}

#[test]
fn probabilistic_alarm_adjudication() {
    // The textbook probabilistic example: alarm rings with probability
    // 0.95 given burglary; burglary occurs with probability 0.001.
    //
    //   0.001 :: burglary.
    //   0.95  :: alarm :- burglary.
    //   ?- alarm.
    //
    // Expected: P(alarm) = 0.001 * 0.95 = 0.00095.

    let doc = IRDocument {
        document_id: doc_id(),
        nodes: vec![
            // 0.001 :: burglary. — represented as a probabilistic rule
            // with empty body. (In a richer IR encoding we'd have a
            // probability field on Fact directly; for now we lower via
            // a `probabilistic` Rule with empty body.)
            rule(
                "R1",
                compound(
                    "probabilistic",
                    vec![
                        logic_core::float(0.001),
                        atom("burglary"),
                        list_of(vec![]),
                    ],
                ),
                0,
                30,
            ),
            // 0.95 :: alarm :- burglary.
            rule(
                "R2",
                compound(
                    "probabilistic",
                    vec![
                        logic_core::float(0.95),
                        atom("alarm"),
                        list_of(vec![atom("burglary")]),
                    ],
                ),
                31,
                70,
            ),
            query("Q1", atom("alarm"), 71, 100),
        ],
    };

    let results = run_adjudication(&doc).unwrap();
    assert_eq!(results.len(), 1);

    match &results[0].result {
        SearchResult::EnumerateAllResult { probability, .. } => {
            let expected = 0.001 * 0.95;
            assert!(
                (probability - expected).abs() < 1e-9,
                "expected P(alarm) = {} got {}",
                expected,
                probability
            );
        }
        other => panic!("expected probabilistic result, got {:?}", other),
    }
}

#[test]
fn mixed_deterministic_and_probabilistic_adjudication() {
    // A KB mixing deterministic facts and a probabilistic rule. The
    // engine's AutoDetect should switch to EnumerateAll because at
    // least one clause is probabilistic.
    //
    //   patient_has_fever.            % Certain
    //   patient_has_cough.            % Certain
    //   0.7 :: pneumonia :- patient_has_fever, patient_has_cough.
    //   ?- pneumonia.
    //
    // Expected: P(pneumonia) = 0.7 (both deterministic prerequisites
    // are satisfied; the probabilistic rule contributes its
    // probability).

    let doc = IRDocument {
        document_id: doc_id(),
        nodes: vec![
            fact("F1", atom("patient_has_fever"), Polarity::Affirmed, 0, 20),
            fact("F2", atom("patient_has_cough"), Polarity::Affirmed, 21, 40),
            rule(
                "R1",
                compound(
                    "probabilistic",
                    vec![
                        logic_core::float(0.7),
                        atom("pneumonia"),
                        list_of(vec![
                            atom("patient_has_fever"),
                            atom("patient_has_cough"),
                        ]),
                    ],
                ),
                41,
                80,
            ),
            query("Q1", atom("pneumonia"), 81, 100),
        ],
    };

    let results = run_adjudication(&doc).unwrap();
    assert_eq!(results.len(), 1);

    match &results[0].result {
        SearchResult::EnumerateAllResult { probability, .. } => {
            assert!(
                (probability - 0.7).abs() < 1e-9,
                "expected P(pneumonia) = 0.7, got {}",
                probability
            );
        }
        other => panic!("expected probabilistic result, got {:?}", other),
    }
}

#[test]
fn multiple_queries_each_get_their_own_result() {
    let doc = IRDocument {
        document_id: doc_id(),
        nodes: vec![
            fact("F1", atom("a"), Polarity::Affirmed, 0, 10),
            fact("F2", atom("b"), Polarity::Affirmed, 11, 20),
            query("Q1", atom("a"), 21, 30),
            query("Q2", atom("b"), 31, 40),
            query("Q3", atom("c"), 41, 50), // not in KB
        ],
    };

    let results = run_adjudication(&doc).unwrap();
    assert_eq!(results.len(), 3);

    let provable: Vec<bool> = results
        .iter()
        .map(|r| matches!(r.result, SearchResult::FindFirstResult(Some(_))))
        .collect();
    assert_eq!(provable, vec![true, true, false]);
}
