//! ADJ10 — TSA carry-on adjudication, end-to-end through the
//! pipeline orchestrator.
//!
//! This is the third of the three E2E test goals (Prolog, ProbLog,
//! semantic source map). The fixture builds a TSA-style IR
//! programmatically, feeds it through
//! [`adjudication_pipeline::run`], and asserts:
//!
//! - The pipeline returns `Verdict::Resolved`.
//! - Every checker pass is recorded in the audit trail (ADJ02
//!   passed, ADJ03 passed, ADJ04 + ADJ05 currently recorded as
//!   `Skipped` — they slot in via a follow-up that adds an LLM
//!   gateway argument to the pipeline).
//! - The engine returned an answer.
//! - The trail round-trips through `serde_json`.
//!
//! ## Why programmatic rather than file-based at v0.1
//!
//! The full ADJ10 spec includes 7 facts + 5 rules + 1 query plus a
//! clarification dialogue. v0.1 ships a *minimal credible TSA
//! scenario* (one carry-on item + one prohibited item + one query)
//! to verify the pipeline composes end-to-end. The richer fixture
//! file (`code/specs/fixtures/adj10-tsa/`) lands when
//! adjudication-ir gets `serde::Deserialize` and we can ship a
//! JSON fixture loaded at test time.

use adjudication_audit_trail::{AdjudicationId, AdjudicationOutcome, PassName, PassOutcome};
use adjudication_ir::{
    DocumentId, IRDocument, IRNode, Modality, NodeId, NodeKind, Polarity, Span,
};
use adjudication_pipeline::{run, PipelineDocument, PipelineInput, Verdict};
use logic_core::{atom, compound};

/// Build the TSA fixture document.
///
/// Text: `"1 carry-on bag, matches."` (24 bytes).
fn tsa_document() -> PipelineDocument {
    PipelineDocument {
        id: "tsa-2026-05-11-001".into(),
        name: "tsa_declaration".into(),
        received_at: "2026-05-11T08:00:00Z".into(),
        normalized_text: "1 carry-on bag, matches.".into(),
        normalization_pipeline: "plain-text-v1".into(),
        normalization_version: "1.0.0".into(),
    }
}

/// Build a TSA-shape IR document with two Facts and one Query.
///
/// Source text: `"1 carry-on bag, matches."` (24 bytes).
/// Spans tile the entire document so ADJ02 coverage passes:
///
/// - F1: `carry_on(1)` — affirmed, spans bytes 0..16 (`"1 carry-on bag, "`)
/// - F2: `prohibited(matches)` — affirmed, spans bytes 16..24 (`"matches."`)
/// - Q1: `compliant(passenger_a)` — query asking whether the passenger
///   is compliant. The engine returns `FindFirstResult(None)` because
///   the KB has no `compliant/1` clause; that's a valid engine answer
///   for v0.1 and still routes through the pipeline cleanly.
fn tsa_ir_document() -> IRDocument {
    let doc_id = DocumentId::new("tsa-2026-05-11-001");

    // F1 uses the typed-quantity shape per ADJ21/ADJ22: the bag
    // count is wrapped in `quantity(1, count)` rather than left as
    // a bare atom. This is the same shape ADJ22 enforces for every
    // numerical literal, and matches the v5 decompose_text contract.
    let f1 = IRNode {
        id: NodeId::new("F1"),
        kind: NodeKind::Fact,
        term: compound(
            "carry_on",
            vec![compound("quantity", vec![atom("1"), atom("count")])],
        ),
        polarity: Polarity::Affirmed,
        modality: Modality::Present,
        source_spans: vec![Span::new(doc_id.clone(), 0, 16)],
        confidence: 1.0,
        discard_reason: None,
        metadata: Default::default(),
    };

    let f2 = IRNode {
        id: NodeId::new("F2"),
        kind: NodeKind::Fact,
        term: compound("prohibited", vec![atom("matches")]),
        polarity: Polarity::Affirmed,
        modality: Modality::Present,
        source_spans: vec![Span::new(doc_id.clone(), 16, 24)],
        confidence: 1.0,
        discard_reason: None,
        metadata: Default::default(),
    };

    let q1 = IRNode {
        id: NodeId::new("Q1"),
        kind: NodeKind::Query,
        term: compound("compliant", vec![atom("passenger_a")]),
        polarity: Polarity::Affirmed,
        modality: Modality::Present,
        // Query node has zero spans (it's synthesized, not extracted
        // from source). Coverage check allows zero-span queries.
        source_spans: vec![],
        confidence: 1.0,
        discard_reason: None,
        metadata: Default::default(),
    };

    IRDocument {
        document_id: doc_id,
        nodes: vec![f1, f2, q1],
        edges: vec![],
    }
}

/// Counter-backed deterministic clock for the audit trail's timestamps.
fn deterministic_clock() -> impl Fn() -> String {
    let tick = std::cell::Cell::new(0u32);
    move || {
        let t = tick.get();
        tick.set(t + 1);
        format!("2026-05-11T08:00:{:02}Z", t.min(59))
    }
}

#[test]
fn tsa_fixture_runs_end_to_end_through_the_pipeline() {
    let input = PipelineInput {
        document: tsa_document(),
        ir_document: tsa_ir_document(),
    };
    let out = run(
        input,
        AdjudicationId::new("adj-tsa-001"),
        deterministic_clock(),
    );

    // The pipeline should reach the engine — coverage + propagation
    // pass on the fixture. The engine itself returns whatever the
    // KB says about `compliant(passenger_a)` (no clauses → no
    // proof, but that's still a Resolved verdict with an empty
    // answer in the engine artifacts).
    match &out.verdict {
        Verdict::Resolved { answers } => {
            assert_eq!(
                answers.len(),
                1,
                "expected one engine answer for one Query node"
            );
        }
        other => panic!("expected Resolved, got {other:?}"),
    }

    // Audit trail must have 5 checker results (ADJ02, ADJ22, ADJ03,
    // ADJ04-Skipped, ADJ05-Skipped) plus engine artifacts. ADJ22
    // was wired in per ADJ24 between ADJ02 and ADJ03 — see spec.
    let trail = &out.audit_trail;
    assert_eq!(trail.checker_results.len(), 5);
    assert_eq!(trail.checker_results[0].pass_name, PassName::Adj02Coverage);
    assert!(matches!(
        trail.checker_results[0].outcome,
        PassOutcome::Passed
    ));
    assert_eq!(
        trail.checker_results[1].pass_name,
        PassName::Adj22TypedQuantity
    );
    assert!(matches!(
        trail.checker_results[1].outcome,
        PassOutcome::Passed
    ));
    assert_eq!(
        trail.checker_results[2].pass_name,
        PassName::Adj03PolarityModality
    );
    assert!(matches!(
        trail.checker_results[2].outcome,
        PassOutcome::Passed
    ));
    assert_eq!(trail.checker_results[3].pass_name, PassName::Adj04RoundTrip);
    assert!(matches!(
        trail.checker_results[3].outcome,
        PassOutcome::Skipped
    ));
    assert_eq!(trail.checker_results[4].pass_name, PassName::Adj05Adversarial);
    assert!(matches!(
        trail.checker_results[4].outcome,
        PassOutcome::Skipped
    ));

    // Engine artifacts present (engine ran).
    assert!(trail.engine_artifacts.is_some());

    // Outcome is Resolved (not ClarificationExhausted / Aborted).
    assert!(matches!(trail.outcome, AdjudicationOutcome::Resolved { .. }));
}

#[test]
fn tsa_audit_trail_round_trips_through_serde_json() {
    let input = PipelineInput {
        document: tsa_document(),
        ir_document: tsa_ir_document(),
    };
    let out = run(
        input,
        AdjudicationId::new("adj-tsa-json"),
        deterministic_clock(),
    );

    let json = serde_json::to_string(&out.audit_trail).expect("AuditTrail serializes");
    let back: adjudication_audit_trail::AuditTrail =
        serde_json::from_str(&json).expect("AuditTrail deserializes");
    assert_eq!(back, out.audit_trail);
}

#[test]
fn tsa_audit_trail_mirrors_input_document_and_all_nodes() {
    let input = PipelineInput {
        document: tsa_document(),
        ir_document: tsa_ir_document(),
    };
    let out = run(
        input,
        AdjudicationId::new("adj-tsa-mirror"),
        deterministic_clock(),
    );

    let trail = &out.audit_trail;
    assert_eq!(trail.documents.len(), 1);
    assert_eq!(trail.documents[0].id.0, "tsa-2026-05-11-001");
    assert_eq!(trail.documents[0].name, "tsa_declaration");
    // 3 IR nodes (2 facts + 1 query).
    assert_eq!(trail.ir_nodes.len(), 3);
    assert_eq!(trail.ir_nodes[0].id.0, "F1");
    assert_eq!(trail.ir_nodes[1].id.0, "F2");
    assert_eq!(trail.ir_nodes[2].id.0, "Q1");
}

#[test]
fn tsa_pipeline_blocks_on_out_of_bounds_span() {
    // Demonstrate the Blocked path: an IR node whose source span
    // exceeds the document length surfaces as a coverage violation
    // and the pipeline returns `Verdict::Blocked` without invoking
    // the engine.
    let doc_id = DocumentId::new("tsa-2026-05-11-001");
    let bad_fact = IRNode {
        id: NodeId::new("F-bad"),
        kind: NodeKind::Fact,
        term: atom("oops"),
        polarity: Polarity::Affirmed,
        modality: Modality::Present,
        // Span 100..200 — way past the 24-byte document.
        source_spans: vec![Span::new(doc_id.clone(), 100, 200)],
        confidence: 1.0,
        discard_reason: None,
        metadata: Default::default(),
    };
    let input = PipelineInput {
        document: tsa_document(),
        ir_document: IRDocument {
            document_id: doc_id,
            nodes: vec![bad_fact],
        edges: vec![],
    },
    };
    let out = run(
        input,
        AdjudicationId::new("adj-tsa-blocked"),
        deterministic_clock(),
    );

    match out.verdict {
        Verdict::Blocked { violation_count } => assert!(violation_count > 0),
        other => panic!("expected Blocked, got {other:?}"),
    }

    // Engine must not have run.
    assert!(out.audit_trail.engine_artifacts.is_none());
    assert!(matches!(
        out.audit_trail.outcome,
        AdjudicationOutcome::ClarificationExhausted { .. }
    ));
}
