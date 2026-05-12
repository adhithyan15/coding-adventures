// LoweringError carries detail strings on every variant; the
// audit-trail discipline (every error logged with full context) is
// worth the cost as long as the error path isn't hot. Same trade-off
// as `llm-gateway` and the adjudication checker crates.
#![allow(clippy::result_large_err)]

//! # adjudication-pipeline — end-to-end orchestrator
//!
//! Composes the framework's checker passes, the engine connector, and
//! the audit-trail schema into a single function: given a normalized
//! document and an IR document, produce a typed [`Verdict`] and a
//! fully-populated [`adjudication_audit_trail::AuditTrail`].
//!
//! This is the **semantic source map** running end-to-end. Today's
//! v0.1.0 composes the slices that have already shipped:
//!
//!   * ADJ02 v2 coverage check
//!     ([`adjudication_coverage::check_coverage`]).
//!   * ADJ03 v2 polarity/modality propagation check
//!     ([`adjudication_polarity_modality::check_propagation`]).
//!   * The engine connector
//!     ([`adjudication_connector::run_adjudication`]).
//!   * ADJ07 audit-trail population
//!     ([`adjudication_audit_trail::AuditTrail`]).
//!
//! ADJ04 (round-trip) and ADJ05 (adversarial) are recorded in the
//! audit trail as [`PassOutcome::Skipped`] for now — when those
//! checker crates ship, they slot in alongside the existing two
//! without changing the pipeline's public surface.
//!
//! ## What this crate deliberately does NOT do (yet)
//!
//! - **Extraction.** Today's pipeline accepts a pre-built
//!   `IRDocument`. v0.2 will wire `llm_primitives::decompose_text`
//!   in front so the input is just `(String, DocumentId)`.
//! - **ADJ06 clarification dialogue.** A failing check produces a
//!   `Verdict::Blocked` with the violations attached — the caller
//!   handles the conversation loop.
//! - **Persistence.** The pipeline returns an in-memory `AuditTrail`;
//!   the deployment chooses how to write it (inline response,
//!   append-only log, content-addressed storage).

use adjudication_audit_trail::{
    AdjudicationId, AdjudicationOutcome, AuditTrail, CheckerResult, ClarificationKind, Document,
    DocumentId, EngineArtifacts, IrNode, KbSummary, NodeId, NormalizationRecord, PassName,
    PassOutcome, SearchMode as TrailSearchMode, SearchMode, Violation,
};
use adjudication_connector::AdjudicationResult;
use adjudication_coverage::{check_coverage, CoverageResult, CoverageViolation, Document as CovDocument};
use adjudication_ir::{IRDocument, IRNode, NodeId as IRNodeId};
use adjudication_polarity_modality::{
    check_propagation, PropagationResult, PropagationViolation, PropagationWarning,
};

// `SearchMode` and `TrailSearchMode` collapse to the same trail-side
// enum; alias for clarity.
const _: () = {
    let _ = TrailSearchMode::AutoDetect;
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// What you feed the pipeline.
#[derive(Debug, Clone)]
pub struct PipelineInput {
    /// The normalized source document. Spans in `ir_document` are
    /// byte offsets into `document.normalized_text`.
    pub document: PipelineDocument,
    /// The extracted hierarchical IR. v0.1 takes this pre-built; v0.2
    /// will replace it with a `source_text: String` once the
    /// `decompose_text` primitive lands.
    pub ir_document: IRDocument,
}

/// Stand-in for the `adjudication_coverage::Document` plus the
/// audit-trail metadata. Keeping the pipeline's input type minimal so
/// the deployment doesn't have to import the coverage crate just to
/// build a pipeline input.
#[derive(Debug, Clone)]
pub struct PipelineDocument {
    pub id: String,
    pub name: String,
    pub received_at: String,
    pub normalized_text: String,
    pub normalization_pipeline: String,
    pub normalization_version: String,
}

/// What the pipeline produces.
#[derive(Debug)]
pub struct PipelineOutput {
    pub verdict: Verdict,
    pub audit_trail: AuditTrail,
}

/// The pipeline's verdict — distinct from the audit trail's
/// `AdjudicationOutcome` so callers can pattern-match without
/// reaching into the trail.
#[derive(Debug)]
pub enum Verdict {
    /// Every gating check passed and the engine returned answers.
    Resolved { answers: Vec<AdjudicationResult> },
    /// At least one gating check (ADJ02 coverage, ADJ03
    /// polarity-modality) failed. The audit trail records the full
    /// violation list; this variant carries a summary count so
    /// callers can branch without parsing the trail.
    Blocked { violation_count: usize },
    /// Lowering or engine execution failed.
    EngineError(String),
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// One-call end-to-end. Runs coverage + polarity/modality, records
/// both into the audit trail, then runs the engine if (and only if)
/// every gating check passed. Returns the verdict and the populated
/// trail.
///
/// `adjudication_id` and `now()` are caller-supplied because the
/// pipeline is otherwise pure: the same input + the same id + the
/// same timestamps deterministically produces the same audit trail.
/// Deployments that want a UUID id and `chrono::Utc::now()` should
/// generate those themselves and hand them in.
pub fn run<F: Fn() -> String>(
    input: PipelineInput,
    adjudication_id: AdjudicationId,
    now: F,
) -> PipelineOutput {
    let started_at = now();
    let mut trail = AuditTrail::new(adjudication_id, started_at.clone());

    // ---------- record the input document ----------
    trail.documents.push(Document {
        id: DocumentId::new(input.document.id.clone()),
        name: input.document.name.clone(),
        received_at: input.document.received_at.clone(),
        normalized_text: input.document.normalized_text.clone(),
        normalization: NormalizationRecord {
            pipeline: input.document.normalization_pipeline.clone(),
            version: input.document.normalization_version.clone(),
            options: Default::default(),
        },
        raw_base64: None,
        appended_turns: Vec::new(),
    });

    // ---------- record the IR nodes ----------
    for node in &input.ir_document.nodes {
        trail.ir_nodes.push(ir_node_to_audit(&input.document.id, node));
    }

    // ---------- ADJ02 coverage ----------
    let cov_doc = CovDocument {
        id: input.ir_document.document_id.clone(),
        normalized_text: input.document.normalized_text.clone(),
    };
    let cov_started = now();
    let cov_result = check_coverage(&cov_doc, &input.ir_document);
    let cov_completed = now();
    trail.checker_results.push(coverage_to_checker_result(
        cov_started,
        cov_completed,
        &cov_result,
    ));

    // ---------- ADJ03 polarity/modality ----------
    let pm_started = now();
    let pm_result = check_propagation(&input.ir_document);
    let pm_completed = now();
    trail.checker_results.push(propagation_to_checker_result(
        pm_started,
        pm_completed,
        &pm_result,
    ));

    // ---------- ADJ04 + ADJ05 are not yet wired — record Skipped ----------
    let skipped_at = now();
    trail.checker_results.push(skipped_checker_result(
        PassName::Adj04RoundTrip,
        skipped_at.clone(),
    ));
    trail.checker_results.push(skipped_checker_result(
        PassName::Adj05Adversarial,
        skipped_at,
    ));

    // ---------- gate the engine on coverage + propagation ----------
    let coverage_ok = matches!(cov_result, CoverageResult::Pass);
    let propagation_ok = pm_result.pass();

    if !(coverage_ok && propagation_ok) {
        let violation_count = trail
            .checker_results
            .iter()
            .map(|cr| cr.violations.len())
            .sum();
        trail.outcome = AdjudicationOutcome::ClarificationExhausted {
            unresolved: collect_violations(&trail.checker_results),
        };
        trail.completed_at = Some(now());
        return PipelineOutput {
            verdict: Verdict::Blocked { violation_count },
            audit_trail: trail,
        };
    }

    // ---------- engine ----------
    let answers = match adjudication_connector::run_adjudication(&input.ir_document) {
        Ok(rs) => rs,
        Err(e) => {
            let detail = format!("{e:?}");
            trail.outcome = AdjudicationOutcome::Aborted {
                reason: detail.clone(),
            };
            trail.completed_at = Some(now());
            return PipelineOutput {
                verdict: Verdict::EngineError(detail),
                audit_trail: trail,
            };
        }
    };

    trail.engine_artifacts = Some(EngineArtifacts {
        engine_version: "logic-engine 0.x".to_string(),
        search_mode: SearchMode::AutoDetect,
        kb_summary: KbSummary {
            // v0.1 leaves the KB-summary counts empty — the connector
            // doesn't currently expose them on the result type.
            // A follow-up can plumb fact_count / rule_count through.
            fact_count: 0,
            rule_count: 0,
            fact_ids: Vec::new(),
            rule_ids: Vec::new(),
            all_certain: answers
                .iter()
                .all(|a| !matches!(a.result, logic_engine::SearchResult::EnumerateAllResult { .. })),
        },
        proof_dag: serde_json::Value::Null,
        formula: None,
        wmc_result: None,
        answer: answers_to_audit_json(&answers),
    });
    trail.outcome = AdjudicationOutcome::Resolved {
        answer: answers_to_audit_json(&answers),
    };
    trail.completed_at = Some(now());

    PipelineOutput {
        verdict: Verdict::Resolved { answers },
        audit_trail: trail,
    }
}

// ---------------------------------------------------------------------------
// Translations from checker types to audit-trail types
// ---------------------------------------------------------------------------

fn ir_node_to_audit(doc_id: &str, node: &IRNode) -> IrNode {
    IrNode {
        id: NodeId::new(node.id.0.clone()),
        document_id: DocumentId::new(doc_id.to_string()),
        payload: serde_json::json!({
            "id": node.id.0,
            "kind": format!("{:?}", node.kind),
            "polarity": format!("{:?}", node.polarity),
            "modality": format!("{:?}", node.modality),
        }),
    }
}

fn coverage_to_checker_result(
    started_at: String,
    completed_at: String,
    result: &CoverageResult,
) -> CheckerResult {
    let (outcome, violations) = match result {
        CoverageResult::Pass => (PassOutcome::Passed, Vec::new()),
        CoverageResult::Fail { violations } => (
            PassOutcome::Failed,
            violations.iter().map(coverage_violation_to_audit).collect(),
        ),
    };
    CheckerResult {
        pass_name: PassName::Adj02Coverage,
        pass_version: "v2.0".to_string(),
        started_at,
        completed_at,
        outcome,
        violations,
        telemetry: Default::default(),
    }
}

fn coverage_violation_to_audit(v: &CoverageViolation) -> Violation {
    let (node_id, detail) = match v {
        CoverageViolation::SpanWrongDocument {
            node_id, expected, found,
        } => (
            ir_node_id_to_audit(node_id),
            serde_json::json!({
                "kind": "SpanWrongDocument",
                "expected": &expected.0,
                "found": &found.0,
            }),
        ),
        CoverageViolation::InvalidSpan { node_id, .. } => (
            ir_node_id_to_audit(node_id),
            serde_json::json!({ "kind": "InvalidSpan" }),
        ),
        // Catch-all: every other variant gets its Debug rendering. The
        // pipeline keeps the audit-trail JSON open-ended; a follow-up
        // can pattern-match each variant explicitly if a downstream
        // consumer needs structured detail.
        other => (
            // A best-effort id. Coverage violations always carry a
            // node_id; this fallback is for variants the pipeline has
            // not yet specialised.
            NodeId::new(String::new()),
            serde_json::json!({
                "kind": "Other",
                "debug": format!("{other:?}"),
            }),
        ),
    };
    Violation {
        node_id,
        pass_name: PassName::Adj02Coverage,
        kind: ClarificationKind::UncoveredSpan,
        detail,
        triggered_dialogue_turn: None,
        resolved: false,
    }
}

fn propagation_to_checker_result(
    started_at: String,
    completed_at: String,
    result: &PropagationResult,
) -> CheckerResult {
    let outcome = if result.pass() {
        PassOutcome::Passed
    } else {
        PassOutcome::Failed
    };
    let violations: Vec<Violation> = result
        .violations
        .iter()
        .map(propagation_violation_to_audit)
        .collect();
    let mut telemetry = std::collections::BTreeMap::new();
    if !result.warnings.is_empty() {
        telemetry.insert(
            "warning_count".to_string(),
            serde_json::json!(result.warnings.len()),
        );
        telemetry.insert(
            "warnings".to_string(),
            serde_json::Value::Array(
                result
                    .warnings
                    .iter()
                    .map(propagation_warning_to_json)
                    .collect(),
            ),
        );
    }
    CheckerResult {
        pass_name: PassName::Adj03PolarityModality,
        pass_version: "v2.0".to_string(),
        started_at,
        completed_at,
        outcome,
        violations,
        telemetry,
    }
}

fn propagation_violation_to_audit(v: &PropagationViolation) -> Violation {
    let (node_id, kind, detail) = match v {
        PropagationViolation::InheritChainUnresolved { node_id } => (
            ir_node_id_to_audit(node_id),
            ClarificationKind::InheritChainUnresolved,
            serde_json::json!({ "kind": "InheritChainUnresolved" }),
        ),
        PropagationViolation::RuledOutMustBeAffirmed {
            node_id,
            actual_polarity,
        } => (
            ir_node_id_to_audit(node_id),
            ClarificationKind::AmbiguousPolarity,
            serde_json::json!({
                "kind": "RuledOutMustBeAffirmed",
                "actual_polarity": format!("{actual_polarity:?}"),
            }),
        ),
    };
    Violation {
        node_id,
        pass_name: PassName::Adj03PolarityModality,
        kind,
        detail,
        triggered_dialogue_turn: None,
        resolved: false,
    }
}

fn propagation_warning_to_json(w: &PropagationWarning) -> serde_json::Value {
    serde_json::json!({ "debug": format!("{w:?}") })
}

fn skipped_checker_result(pass_name: PassName, at: String) -> CheckerResult {
    CheckerResult {
        pass_name,
        pass_version: "not-yet-wired".to_string(),
        started_at: at.clone(),
        completed_at: at,
        outcome: PassOutcome::Skipped,
        violations: Vec::new(),
        telemetry: Default::default(),
    }
}

fn collect_violations(checker_results: &[CheckerResult]) -> Vec<Violation> {
    checker_results
        .iter()
        .flat_map(|cr| cr.violations.iter().cloned())
        .collect()
}

fn ir_node_id_to_audit(id: &IRNodeId) -> NodeId {
    NodeId::new(id.0.clone())
}

fn answers_to_audit_json(answers: &[AdjudicationResult]) -> serde_json::Value {
    let arr: Vec<serde_json::Value> = answers
        .iter()
        .map(|a| {
            serde_json::json!({
                "query": format!("{:?}", a.query),
                "result": format!("{:?}", a.result),
            })
        })
        .collect();
    serde_json::Value::Array(arr)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{
        DocumentId as IRDocumentId, IRNode, Modality, NodeId as IRNodeId, NodeKind, Polarity, Span,
    };
    use logic_core::Term;

    fn pipeline_doc() -> PipelineDocument {
        PipelineDocument {
            id: "doc1".into(),
            name: "tsa_declaration".into(),
            received_at: "2026-05-11T08:00:00Z".into(),
            normalized_text: "1 carry-on bag, 1 personal item.".into(),
            normalization_pipeline: "plain-text-v1".into(),
            normalization_version: "1.0.0".into(),
        }
    }

    fn make_ir(nodes: Vec<IRNode>) -> IRDocument {
        IRDocument {
            document_id: IRDocumentId::new("doc1"),
            nodes,
        }
    }

    fn fact_node(id: &str, term: Term, start: usize, end: usize) -> IRNode {
        IRNode {
            id: IRNodeId::new(id.to_string()),
            kind: NodeKind::Fact,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![Span::new(IRDocumentId::new("doc1"), start, end)],
            confidence: 1.0,
            part_of: None,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        }
    }

    fn make_clock() -> impl Fn() -> String {
        let tick = std::cell::Cell::new(0u32);
        move || {
            let t = tick.get();
            tick.set(t + 1);
            format!("2026-05-11T08:00:0{}Z", t.min(9))
        }
    }

    #[test]
    fn empty_ir_with_empty_text_passes_through_and_resolves() {
        // Smallest possible pipeline run: zero IR nodes, zero text.
        // Coverage is vacuously OK (no spans to validate), propagation
        // is vacuously OK (no nodes to propagate over), engine has no
        // queries, so the verdict is `Resolved` with zero answers.
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: String::new(),
                ..pipeline_doc()
            },
            ir_document: make_ir(Vec::new()),
        };
        let out = run(input, AdjudicationId::new("adj-empty"), make_clock());
        match out.verdict {
            Verdict::Resolved { answers } => assert!(answers.is_empty()),
            other => panic!("expected Resolved, got {other:?}"),
        }
        assert_eq!(out.audit_trail.adjudication_id.0, "adj-empty");
        assert_eq!(out.audit_trail.checker_results.len(), 4);
        // ADJ02 + ADJ03 passed; ADJ04 + ADJ05 are recorded as Skipped.
        assert!(matches!(
            out.audit_trail.checker_results[0].outcome,
            PassOutcome::Passed
        ));
        assert!(matches!(
            out.audit_trail.checker_results[1].outcome,
            PassOutcome::Passed
        ));
        assert!(matches!(
            out.audit_trail.checker_results[2].outcome,
            PassOutcome::Skipped
        ));
        assert!(matches!(
            out.audit_trail.checker_results[3].outcome,
            PassOutcome::Skipped
        ));
        assert!(out.audit_trail.completed_at.is_some());
        assert!(matches!(
            out.audit_trail.outcome,
            AdjudicationOutcome::Resolved { .. }
        ));
    }

    #[test]
    fn coverage_violation_blocks_engine_and_records_full_audit_trail() {
        // The IR cites a span outside the document text — coverage
        // fails, the pipeline reports Blocked, engine never runs.
        let node = fact_node(
            "n1",
            logic_core::atom("anomaly"),
            100,
            150, // way past the 5-char document text
        );
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![node]),
        };
        let out = run(input, AdjudicationId::new("adj-blocked"), make_clock());
        match out.verdict {
            Verdict::Blocked { violation_count } => assert!(violation_count > 0),
            other => panic!("expected Blocked, got {other:?}"),
        }
        // Audit trail should still be fully populated.
        assert_eq!(out.audit_trail.documents.len(), 1);
        assert_eq!(out.audit_trail.ir_nodes.len(), 1);
        // Coverage checker failed; ADJ03 still ran (we record both
        // even on early-exit, so the trail captures the full state).
        let cov = &out.audit_trail.checker_results[0];
        assert_eq!(cov.pass_name, PassName::Adj02Coverage);
        assert!(matches!(cov.outcome, PassOutcome::Failed));
        assert!(!cov.violations.is_empty());
        // Outcome is ClarificationExhausted, not Resolved.
        assert!(matches!(
            out.audit_trail.outcome,
            AdjudicationOutcome::ClarificationExhausted { .. }
        ));
        // Engine artifacts must NOT be populated.
        assert!(out.audit_trail.engine_artifacts.is_none());
    }

    #[test]
    fn audit_trail_records_input_document_with_normalization_metadata() {
        let input = PipelineInput {
            document: pipeline_doc(),
            ir_document: make_ir(Vec::new()),
        };
        let out = run(input, AdjudicationId::new("adj-doc-meta"), make_clock());
        let d = &out.audit_trail.documents[0];
        assert_eq!(d.id.0, "doc1");
        assert_eq!(d.name, "tsa_declaration");
        assert_eq!(d.normalization.pipeline, "plain-text-v1");
        assert_eq!(d.normalization.version, "1.0.0");
    }

    #[test]
    fn schema_version_is_recorded_on_audit_trail() {
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: String::new(),
                ..pipeline_doc()
            },
            ir_document: make_ir(Vec::new()),
        };
        let out = run(input, AdjudicationId::new("adj-schema"), make_clock());
        assert_eq!(out.audit_trail.schema_version, "ADJ07-v1");
    }

    #[test]
    fn ir_nodes_are_mirrored_into_audit_trail() {
        let n1 = fact_node("n1", logic_core::atom("a"), 0, 1);
        let n2 = fact_node("n2", logic_core::atom("b"), 2, 3);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "abc".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n1, n2]),
        };
        let out = run(input, AdjudicationId::new("adj-mirror"), make_clock());
        assert_eq!(out.audit_trail.ir_nodes.len(), 2);
        assert_eq!(out.audit_trail.ir_nodes[0].id.0, "n1");
        assert_eq!(out.audit_trail.ir_nodes[1].id.0, "n2");
        // The payload carries kind/polarity/modality stringified — v0.2
        // will store the typed adjudication_ir::IRNode once that crate
        // ships serde derives.
        assert_eq!(out.audit_trail.ir_nodes[0].payload["kind"], "Fact");
    }

    #[test]
    fn checker_pass_versions_are_recorded() {
        // Smoke-check: every checker_result must carry a non-empty
        // pass_version. Replay needs this to know which checker
        // version was used.
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: String::new(),
                ..pipeline_doc()
            },
            ir_document: make_ir(Vec::new()),
        };
        let out = run(input, AdjudicationId::new("adj-versions"), make_clock());
        for cr in &out.audit_trail.checker_results {
            assert!(
                !cr.pass_version.is_empty(),
                "{:?} has empty version",
                cr.pass_name
            );
        }
    }

    #[test]
    fn audit_trail_round_trips_through_serde_json() {
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let out = run(input, AdjudicationId::new("adj-json"), make_clock());
        let json = serde_json::to_string(&out.audit_trail).expect("AuditTrail serializes");
        let back: AuditTrail =
            serde_json::from_str(&json).expect("AuditTrail deserializes");
        assert_eq!(back, out.audit_trail);
    }
}
