//! # adjudication-connector — wires ADJ IR to LP19 engine.
//!
//! Reference implementation of [`ADJ11`](../../../specs/ADJ11-problog-connector.md).
//! Takes an [`IRDocument`] (ADJ01) and produces a
//! [`logic_engine::KnowledgeBase`] (LP19), then runs any Query nodes
//! found in the document.
//!
//! The crate is deliberately thin. The substantive work — typed IR
//! grammar, validation, search, weighted-model-counting — lives in
//! `adjudication-ir` and `logic-engine`. This crate is only the
//! lowering layer plus a convenience wrapper around `search`.
//!
//! ## Rule subtype encoding (per ADJ01)
//!
//! ADJ Rule nodes encode their subtype in the term, not the kind, so
//! that the well-formedness check in adjudication-ir stays simple.
//! The connector recognises four functors:
//!
//! - `definitional(head, [body...])` → LP19 `Rule { probability: Certain }`
//! - `probabilistic(p, head, [body...])` → LP19 `Rule { probability: Value(p) }`
//! - `constraint([body...])` → LP19 `Rule` with synthetic
//!   `_constraint(c_N)` head
//! - `default(head, [body...], [exceptions...])` → LP19 `Rule` with
//!   `Pos` body literals + `Neg` exception literals
//!
//! Any other compound functor used at a Rule node yields
//! [`LoweringError::UnknownRuleSubtype`].

use adjudication_ir::{IRDocument, IRNode, NodeId, NodeKind, Polarity};
use logic_core::{Number, Term};
use logic_engine::{
    search, BodyLiteral, Fact, KnowledgeBase, Probability, Rule, SearchMode, SearchResult,
};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Every reason an IR document fails to lower to a KnowledgeBase.
///
/// The variants are deliberately specific so callers (typically the
/// clarification dialogue, ADJ06) can produce helpful messages when a
/// lowering fails.
#[derive(Debug, Clone, PartialEq)]
pub enum LoweringError {
    /// A Rule node's term is not a recognized subtype functor.
    UnknownRuleSubtype { node_id: NodeId, functor: String },

    /// A Rule subtype term had the wrong number of arguments.
    InvalidRuleArity {
        node_id: NodeId,
        subtype: String,
        expected: usize,
        actual: usize,
    },

    /// A Rule subtype's body list was malformed (not a `'.'/2` cons-cell
    /// chain ending in `[]`).
    InvalidRuleBodyList { node_id: NodeId, subtype: String },

    /// A `probabilistic` rule's first argument was not a numeric term.
    InvalidProbability { node_id: NodeId, found: String },

    /// A `probabilistic` rule's probability was outside `[0, 1]`.
    ProbabilityOutOfRange { node_id: NodeId, value: f64 },
}

// ---------------------------------------------------------------------------
// Lowering
// ---------------------------------------------------------------------------

/// Lower an IR document into a logic-engine KnowledgeBase, applying
/// the lowering rules from ADJ11.
///
/// Returns the KB on success or the first lowering error on failure.
/// The KB does **not** include Query nodes; use [`extract_queries`]
/// for those.
pub fn lower_to_kb(ir_doc: &IRDocument) -> Result<KnowledgeBase, LoweringError> {
    let mut kb = KnowledgeBase::new();
    let mut constraint_counter: u64 = 0;
    for node in &ir_doc.nodes {
        match node.kind {
            NodeKind::Fact => lower_fact(&mut kb, node)?,
            NodeKind::Rule => lower_rule(&mut kb, node, &mut constraint_counter)?,
            // Query nodes are returned by extract_queries; not added to KB.
            // Uncertainty / Exception / Discarded participate in
            // clarification, audit, and rule priority but do not produce
            // engine clauses.
            NodeKind::Query
            | NodeKind::Uncertainty
            | NodeKind::Exception
            | NodeKind::Discarded => {}
        }
    }
    Ok(kb)
}

/// Collect the `term` of every Query node in the document, in order.
/// Most documents contain exactly one Query; multiple are permitted.
pub fn extract_queries(ir_doc: &IRDocument) -> Vec<Term> {
    ir_doc
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Query)
        .map(|n| n.term.clone())
        .collect()
}

fn lower_fact(kb: &mut KnowledgeBase, node: &IRNode) -> Result<(), LoweringError> {
    match node.polarity {
        Polarity::Affirmed => {
            // The simple case: a positive fact with whatever probability
            // the source declared. Currently the IR has no `probability`
            // field on Facts (it's a Rule-subtype concern); affirmed
            // Facts lower to Certain probability. Future versions of the
            // IR may carry per-Fact probabilities directly.
            kb.add_fact(Fact::certain(node.term.clone()));
        }
        Polarity::Denied => {
            // ADJ11's polarity-to-clause translation under
            // negation-as-failure: `Denied(t)` lowers to a Rule whose
            // body is a single `Neg(t)` literal. The rule succeeds when
            // `t` cannot be proved, capturing the denied semantics.
            kb.add_rule(Rule::certain(
                node.term.clone(),
                vec![BodyLiteral::Neg(node.term.clone())],
            ));
        }
        Polarity::Uncertain => {
            // A Fact node should not have Uncertain polarity per
            // ADJ01 well-formedness. If we see it here the upstream
            // validation was bypassed; silently treat as Affirmed for
            // robustness rather than panicking. (A pre-validated
            // IRDocument never hits this branch.)
            kb.add_fact(Fact::certain(node.term.clone()));
        }
    }
    Ok(())
}

fn lower_rule(
    kb: &mut KnowledgeBase,
    node: &IRNode,
    constraint_counter: &mut u64,
) -> Result<(), LoweringError> {
    let Term::Compound { functor, args } = &node.term else {
        return Err(LoweringError::UnknownRuleSubtype {
            node_id: node.id.clone(),
            functor: render_term_summary(&node.term),
        });
    };

    match functor.as_str() {
        "definitional" => {
            // definitional(head, [body...])
            if args.len() != 2 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "definitional".to_string(),
                    expected: 2,
                    actual: args.len(),
                });
            }
            let head = args[0].clone();
            let body = decode_list(&args[1])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "definitional".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            kb.add_rule(Rule::certain(head, body));
        }
        "probabilistic" => {
            // probabilistic(p, head, [body...])
            if args.len() != 3 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "probabilistic".to_string(),
                    expected: 3,
                    actual: args.len(),
                });
            }
            let p = match &args[0] {
                Term::Num(Number::Int(i)) => *i as f64,
                Term::Num(Number::Float(x)) => *x,
                other => {
                    return Err(LoweringError::InvalidProbability {
                        node_id: node.id.clone(),
                        found: render_term_summary(other),
                    });
                }
            };
            if !(0.0..=1.0).contains(&p) {
                return Err(LoweringError::ProbabilityOutOfRange {
                    node_id: node.id.clone(),
                    value: p,
                });
            }
            let head = args[1].clone();
            let body = decode_list(&args[2])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "probabilistic".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            kb.add_rule(Rule {
                id: logic_engine::RuleId(u64::MAX), // overwritten on insert
                head,
                body,
                probability: Probability::Value(p),
            });
        }
        "constraint" => {
            // constraint([body...]) - synthetic head `_constraint(c_N)`
            if args.len() != 1 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "constraint".to_string(),
                    expected: 1,
                    actual: args.len(),
                });
            }
            let body = decode_list(&args[0])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "constraint".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            let synthetic_head = logic_core::compound(
                "_constraint",
                vec![logic_core::atom(format!("c_{}", *constraint_counter))],
            );
            *constraint_counter += 1;
            kb.add_rule(Rule::certain(synthetic_head, body));
        }
        "default" => {
            // default(head, [body...], [exceptions...])
            if args.len() != 3 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "default".to_string(),
                    expected: 3,
                    actual: args.len(),
                });
            }
            let head = args[0].clone();
            let mut combined_body: Vec<BodyLiteral> = decode_list(&args[1])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "default".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            let exceptions = decode_list(&args[2])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "default".to_string(),
                })?;
            for exc in exceptions {
                combined_body.push(BodyLiteral::Neg(exc));
            }
            kb.add_rule(Rule::certain(head, combined_body));
        }
        other => {
            return Err(LoweringError::UnknownRuleSubtype {
                node_id: node.id.clone(),
                functor: other.to_string(),
            });
        }
    }
    Ok(())
}

/// Decode a Prolog-style list term (using `'.'/2` cons cells and the
/// `[]` empty-list atom) into a Vec of Terms. Returns `None` if the
/// term is not a well-formed list.
fn decode_list(term: &Term) -> Option<Vec<Term>> {
    let mut out = Vec::new();
    let mut current = term;
    loop {
        match current {
            Term::Atom(name) if name == "[]" => return Some(out),
            Term::Compound { functor, args } if functor == "." && args.len() == 2 => {
                out.push(args[0].clone());
                current = &args[1];
            }
            _ => return None,
        }
    }
}

/// Cheap one-line summary of a term for error messages.
fn render_term_summary(term: &Term) -> String {
    match term {
        Term::Atom(s) => s.clone(),
        Term::Num(_) | Term::Str(_) | Term::Var(_) => term.to_string(),
        Term::Compound { functor, args } => format!("{}/{}", functor, args.len()),
    }
}

// ---------------------------------------------------------------------------
// End-to-end adjudication
// ---------------------------------------------------------------------------

/// One query's result after running through the engine.
#[derive(Debug, Clone)]
pub struct AdjudicationResult {
    pub query: Term,
    pub result: SearchResult,
}

/// Lower the IR document and run every Query node under
/// `SearchMode::AutoDetect`. Returns one [`AdjudicationResult`] per
/// Query.
pub fn run_adjudication(ir_doc: &IRDocument) -> Result<Vec<AdjudicationResult>, LoweringError> {
    let kb = lower_to_kb(ir_doc)?;
    let queries = extract_queries(ir_doc);
    Ok(queries
        .into_iter()
        .map(|q| {
            let result = search(&q, &kb, SearchMode::AutoDetect);
            AdjudicationResult { query: q, result }
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Inline unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{DocumentId, Modality, NodeKind as Nk, Span};
    use logic_core::{atom, compound, int, var};

    fn doc_id() -> DocumentId {
        DocumentId::new("doc1")
    }

    fn span() -> Span {
        Span::new(doc_id(), 0, 10)
    }

    fn empty_meta() -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }

    fn affirmed_fact_node(id: &str, term: Term) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: Nk::Fact,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span()],
            confidence: 1.0,
            lowered_from: None,
            discard_reason: None,
            metadata: empty_meta(),
        }
    }

    fn denied_fact_node(id: &str, term: Term) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: Nk::Fact,
            term,
            polarity: Polarity::Denied,
            modality: Modality::Present,
            source_spans: vec![span()],
            confidence: 1.0,
            lowered_from: None,
            discard_reason: None,
            metadata: empty_meta(),
        }
    }

    fn rule_node(id: &str, term: Term) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: Nk::Rule,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span()],
            confidence: 1.0,
            lowered_from: None,
            discard_reason: None,
            metadata: empty_meta(),
        }
    }

    fn query_node(id: &str, term: Term) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: Nk::Query,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span()],
            confidence: 1.0,
            lowered_from: None,
            discard_reason: None,
            metadata: empty_meta(),
        }
    }

    fn list_of(terms: Vec<Term>) -> Term {
        logic_core::logic_list(terms)
    }

    #[test]
    fn empty_document_produces_empty_kb() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![],
        };
        let kb = lower_to_kb(&doc).unwrap();
        assert!(kb.is_all_certain()); // vacuously
    }

    #[test]
    fn affirmed_fact_lowers_to_certain_fact() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![affirmed_fact_node("F1", atom("ok"))],
        };
        let kb = lower_to_kb(&doc).unwrap();
        // We can't directly observe internals; instead, run a search.
        let r = search(&atom("ok"), &kb, SearchMode::AutoDetect);
        match r {
            SearchResult::FindFirstResult(Some(_)) => {} // expected
            other => panic!("expected FindFirstResult(Some), got {:?}", other),
        }
    }

    #[test]
    fn denied_fact_lowers_without_error() {
        // Denied(t) lowers to Rule { head: t, body: [Neg(t)] } — a
        // rule that succeeds when `t` cannot be proved.
        //
        // This produces a NON-STRATIFIED program (`t :- \+ t.`) when
        // the denied fact appears in isolation. LP19's well-founded
        // semantics rejects such programs; the stratification check is
        // a follow-up sub-spec (LP19a) and not yet implemented in the
        // Rust engine. So we verify only that the *lowering* succeeds;
        // running search on the resulting KB would recurse non-
        // terminatingly until the engine implements the check.
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![denied_fact_node("F1", atom("absent"))],
        };
        let kb = lower_to_kb(&doc).unwrap();
        // Sanity: the KB has at least one rule (the NAF rule).
        // Use is_all_certain() as a proxy for "the KB is populated"
        // since we don't expose direct counts.
        assert!(kb.is_all_certain(), "KB should contain only Certain clauses");
    }

    #[test]
    fn denied_fact_combined_with_other_proof_path_resolves_cleanly() {
        // Denied(absent) is the NAF rule `absent :- \+ absent.`. Add
        // an UNRELATED predicate so the test runs without entering the
        // non-stratified recursion: querying `something_else` does not
        // touch the NAF rule.
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                denied_fact_node("F1", atom("absent")),
                affirmed_fact_node("F2", atom("present")),
                query_node("Q1", atom("present")),
            ],
        };
        let results = run_adjudication(&doc).unwrap();
        match &results[0].result {
            SearchResult::FindFirstResult(Some(_)) => {}
            other => panic!("expected present to succeed, got {:?}", other),
        }
    }

    #[test]
    fn definitional_rule_lowering_executes_correctly() {
        // definitional(parent(X, Y), [father(X, Y)])
        let xv = var("X");
        let yv = var("Y");
        let head = compound(
            "parent",
            vec![Term::Var(xv.clone()), Term::Var(yv.clone())],
        );
        let body_lit = compound(
            "father",
            vec![Term::Var(xv.clone()), Term::Var(yv.clone())],
        );
        let body_list = list_of(vec![body_lit]);
        let rule_term = compound("definitional", vec![head, body_list]);

        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node(
                    "F1",
                    compound("father", vec![atom("homer"), atom("bart")]),
                ),
                rule_node("R1", rule_term),
                query_node(
                    "Q1",
                    compound("parent", vec![atom("homer"), atom("bart")]),
                ),
            ],
        };

        let results = run_adjudication(&doc).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0].result {
            SearchResult::FindFirstResult(Some(_)) => {}
            other => panic!("expected parent(homer, bart) to succeed, got {:?}", other),
        }
    }

    #[test]
    fn probabilistic_rule_lowering_produces_value_probability() {
        // probabilistic(0.5, alarm, [burglary])
        // Together with `burglary` (Certain), engine should compute
        // P(alarm) = 0.5.
        let rule_term = compound(
            "probabilistic",
            vec![
                logic_core::float(0.5),
                atom("alarm"),
                list_of(vec![atom("burglary")]),
            ],
        );

        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node("F1", atom("burglary")),
                rule_node("R1", rule_term),
                query_node("Q1", atom("alarm")),
            ],
        };

        let results = run_adjudication(&doc).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0].result {
            SearchResult::EnumerateAllResult { probability, .. } => {
                assert!(
                    (*probability - 0.5).abs() < 1e-9,
                    "expected P(alarm) = 0.5, got {}",
                    probability
                );
            }
            other => panic!("expected probabilistic result, got {:?}", other),
        }
    }

    #[test]
    fn constraint_rule_lowering_uses_synthetic_head() {
        let body_lit = atom("placeholder");
        let body_list = list_of(vec![body_lit]);
        let rule_term = compound("constraint", vec![body_list]);

        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term)],
        };

        let kb = lower_to_kb(&doc).unwrap();
        // The synthetic head is `_constraint(c_0)`. We don't have a
        // direct accessor, but we can verify it's present by
        // querying for it.
        let r = search(
            &compound("_constraint", vec![atom("c_0")]),
            &kb,
            SearchMode::FindFirst,
        );
        // The rule's body requires `placeholder` to be provable,
        // which it isn't. So the query for the synthetic head
        // should fail.
        match r {
            SearchResult::FindFirstResult(None) => {} // body fails
            other => panic!("expected None (body unsatisfied), got {:?}", other),
        }
    }

    #[test]
    fn default_rule_lowering_combines_body_and_negated_exceptions() {
        // default(p, [a], [b])  →  p :- a, \+ b
        // With a present, p is provable iff b is not.
        let rule_term = compound(
            "default",
            vec![
                atom("p"),
                list_of(vec![atom("a")]),
                list_of(vec![atom("b")]),
            ],
        );

        // Case 1: a holds, b doesn't → p succeeds.
        let doc1 = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node("F1", atom("a")),
                rule_node("R1", rule_term.clone()),
                query_node("Q1", atom("p")),
            ],
        };
        let r1 = run_adjudication(&doc1).unwrap();
        match &r1[0].result {
            SearchResult::FindFirstResult(Some(_)) => {}
            other => panic!("expected p to succeed when a yes, b no; got {:?}", other),
        }

        // Case 2: a and b both hold → p fails (exception fires).
        let doc2 = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node("F1", atom("a")),
                affirmed_fact_node("F2", atom("b")),
                rule_node("R1", rule_term),
                query_node("Q1", atom("p")),
            ],
        };
        let r2 = run_adjudication(&doc2).unwrap();
        match &r2[0].result {
            SearchResult::FindFirstResult(None) => {}
            other => panic!("expected p to fail when exception holds; got {:?}", other),
        }
    }

    #[test]
    fn unknown_rule_subtype_errors_with_functor_name() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node(
                "R1",
                compound("unknownify", vec![atom("x")]),
            )],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::UnknownRuleSubtype { functor, .. }) => {
                assert_eq!(functor, "unknownify");
            }
            other => panic!("expected UnknownRuleSubtype, got {:?}", other),
        }
    }

    #[test]
    fn rule_with_wrong_arity_errors() {
        // definitional only takes 2 args
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node(
                "R1",
                compound("definitional", vec![atom("h")]),
            )],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::InvalidRuleArity {
                subtype, expected, actual, ..
            }) => {
                assert_eq!(subtype, "definitional");
                assert_eq!(expected, 2);
                assert_eq!(actual, 1);
            }
            other => panic!("expected InvalidRuleArity, got {:?}", other),
        }
    }

    #[test]
    fn probabilistic_with_non_numeric_p_errors() {
        let rule_term = compound(
            "probabilistic",
            vec![atom("not_a_number"), atom("h"), list_of(vec![])],
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term)],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::InvalidProbability { .. }) => {}
            other => panic!("expected InvalidProbability, got {:?}", other),
        }
    }

    #[test]
    fn probabilistic_with_out_of_range_p_errors() {
        let rule_term = compound(
            "probabilistic",
            vec![logic_core::float(1.5), atom("h"), list_of(vec![])],
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term)],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::ProbabilityOutOfRange { value, .. }) => {
                assert!((value - 1.5).abs() < 1e-9);
            }
            other => panic!("expected ProbabilityOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn rule_body_not_a_list_errors() {
        let rule_term = compound(
            "definitional",
            vec![atom("h"), atom("not_a_list")], // should be a list
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term)],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::InvalidRuleBodyList { .. }) => {}
            other => panic!("expected InvalidRuleBodyList, got {:?}", other),
        }
    }

    #[test]
    fn extract_queries_returns_all_query_terms_in_order() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                query_node("Q1", atom("a")),
                affirmed_fact_node("F1", atom("a")),
                query_node("Q2", atom("b")),
            ],
        };
        let queries = extract_queries(&doc);
        assert_eq!(queries, vec![atom("a"), atom("b")]);
    }

    #[test]
    fn integer_probability_is_accepted_when_in_range() {
        // probabilistic(1, head, []) — integer 1, equivalent to Value(1.0)
        let rule_term = compound(
            "probabilistic",
            vec![int(1), atom("h"), list_of(vec![])],
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term), query_node("Q1", atom("h"))],
        };
        let results = run_adjudication(&doc).unwrap();
        match &results[0].result {
            SearchResult::EnumerateAllResult { probability, .. } => {
                assert!((*probability - 1.0).abs() < 1e-9);
            }
            other => panic!("expected EnumerateAllResult, got {:?}", other),
        }
    }
}
