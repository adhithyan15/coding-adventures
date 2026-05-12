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
//! ADJ04 (round-trip) runs **when** the caller provides a
//! [`GatewayConfig`] with `Renderer` + `Nli` clients registered. If
//! no gateway is supplied (or those roles aren't bound), ADJ04 is
//! recorded as [`PassOutcome::Skipped`] with `pass_version =
//! "not-yet-wired"`, preserving the v0.1/v0.2 behaviour.
//!
//! ADJ05 (adversarial) still records as `Skipped` — it needs a
//! second, family-disjoint `Adversary` client and lands in v0.4.
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
use adjudication_round_trip::{
    check_round_trip, CheckError as RoundTripCheckError, CheckOptions as RoundTripOptions,
    RoundTripResult, RoundTripViolation,
};
use llm_primitives::GatewayConfig;

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

/// One-call end-to-end. Runs coverage + polarity/modality + (optional)
/// ADJ04 round-trip, records each into the audit trail, then runs the
/// engine if (and only if) every gating check passed.
///
/// `adjudication_id` and `now()` are caller-supplied because the
/// pipeline is otherwise pure: the same input + the same id + the
/// same timestamps deterministically produces the same audit trail
/// (for the LLM-free passes — ADJ04 records whatever the gateway
/// returned, which is the model's job to make deterministic via
/// `temperature = 0.0`).
///
/// `gateway` controls ADJ04. Passing `None` preserves the v0.2
/// behaviour: ADJ04 is recorded as `Skipped`. Passing `Some(&g)`
/// with `Renderer` + `Nli` clients registered runs the real check
/// and surfaces violations as `Failed` in the audit trail. If a
/// required role is missing, ADJ04 records `Failed` with a single
/// telemetry-only violation describing the configuration gap, and
/// the engine still runs (round-trip is advisory, not gating, at
/// v0.3).
pub fn run<F: Fn() -> String>(
    input: PipelineInput,
    adjudication_id: AdjudicationId,
    now: F,
) -> PipelineOutput {
    run_with_gateway(input, adjudication_id, now, None)
}

/// Same as [`run`] but with an explicit `GatewayConfig`. v0.3's
/// preferred entry point; [`run`] is kept for binary-compat with v0.2
/// callers.
pub fn run_with_gateway<F: Fn() -> String>(
    input: PipelineInput,
    adjudication_id: AdjudicationId,
    now: F,
    gateway: Option<&GatewayConfig>,
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

    // ---------- ADJ04 round-trip (gated on a gateway being supplied) ----------
    // We only attempt ADJ04 when the prior gating checks passed —
    // running the LLM on an IR that doesn't even cover its source
    // burns tokens to discover what ADJ02/ADJ03 already told us.
    let prior_gating_ok =
        matches!(cov_result, CoverageResult::Pass) && pm_result.pass();
    let adj04_started = now();
    let adj04_result = if prior_gating_ok {
        run_adj04(gateway, &input.document.normalized_text, &input.ir_document)
    } else {
        Adj04Decision::Skipped
    };
    let adj04_completed = now();
    trail
        .checker_results
        .push(adj04_to_checker_result(adj04_started, adj04_completed, &adj04_result));

    // ---------- ADJ05 still parked — record Skipped ----------
    let skipped_at = now();
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

// ---------------------------------------------------------------------------
// ADJ04 wiring
// ---------------------------------------------------------------------------

/// What the pipeline learned from attempting ADJ04 on a given run.
/// Kept separate from `RoundTripResult` so the `Skipped` and
/// `CheckErrored` cases don't need to manufacture an empty
/// `RoundTripResult`.
enum Adj04Decision {
    /// No gateway supplied OR a prior gating check failed — the
    /// pipeline did not attempt the round-trip.
    Skipped,
    /// The checker ran. `result.violations.is_empty()` is the pass
    /// signal.
    Ran(RoundTripResult),
    /// The checker errored before producing a verdict (missing role,
    /// gateway error, primitive validation exhaustion, …). The
    /// pipeline records this as a Failed pass with the error in
    /// telemetry so the audit trail stays complete.
    CheckErrored(String),
}

fn run_adj04(
    gateway: Option<&GatewayConfig>,
    document_text: &str,
    ir_doc: &IRDocument,
) -> Adj04Decision {
    let Some(g) = gateway else {
        return Adj04Decision::Skipped;
    };
    match check_round_trip(document_text, ir_doc, g, &RoundTripOptions::default()) {
        Ok(result) => Adj04Decision::Ran(result),
        Err(e) => Adj04Decision::CheckErrored(round_trip_err_summary(&e)),
    }
}

fn round_trip_err_summary(e: &RoundTripCheckError) -> String {
    // The checker's Display impl already produces a human-friendly
    // message; we just relay it. The trail records this string in
    // telemetry, not in `violations` — a checker error is operator-
    // surface rather than reviewer-surface.
    format!("{e}")
}

fn adj04_to_checker_result(
    started_at: String,
    completed_at: String,
    decision: &Adj04Decision,
) -> CheckerResult {
    match decision {
        Adj04Decision::Skipped => CheckerResult {
            pass_name: PassName::Adj04RoundTrip,
            pass_version: "not-yet-wired".to_string(),
            started_at,
            completed_at,
            outcome: PassOutcome::Skipped,
            violations: Vec::new(),
            telemetry: Default::default(),
        },
        Adj04Decision::Ran(result) => {
            let outcome = if result.pass() {
                PassOutcome::Passed
            } else {
                PassOutcome::Failed
            };
            let violations: Vec<Violation> = result
                .violations
                .iter()
                .map(round_trip_violation_to_audit)
                .collect();
            let mut telemetry = std::collections::BTreeMap::new();
            telemetry.insert(
                "call_count".to_string(),
                serde_json::json!(result.call_records.len()),
            );
            telemetry.insert(
                "primitive_calls".to_string(),
                serde_json::Value::Array(
                    result
                        .call_records
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "primitive": c.primitive,
                                "role": c.role,
                                "prompt_version": c.prompt_version,
                                "prompt_hash": c.prompt_hash,
                                "latency_ms": c.latency_ms,
                                "input_tokens": c.usage.input_tokens,
                                "output_tokens": c.usage.output_tokens,
                            })
                        })
                        .collect(),
                ),
            );
            CheckerResult {
                pass_name: PassName::Adj04RoundTrip,
                pass_version: "v1.0".to_string(),
                started_at,
                completed_at,
                outcome,
                violations,
                telemetry,
            }
        }
        Adj04Decision::CheckErrored(detail) => {
            let mut telemetry = std::collections::BTreeMap::new();
            telemetry.insert(
                "check_error".to_string(),
                serde_json::Value::String(detail.clone()),
            );
            CheckerResult {
                pass_name: PassName::Adj04RoundTrip,
                pass_version: "v1.0".to_string(),
                started_at,
                completed_at,
                outcome: PassOutcome::Failed,
                violations: Vec::new(),
                telemetry,
            }
        }
    }
}

fn round_trip_violation_to_audit(v: &RoundTripViolation) -> Violation {
    Violation {
        node_id: NodeId::new(v.node_id.0.clone()),
        pass_name: PassName::Adj04RoundTrip,
        kind: ClarificationKind::RoundTripDrift,
        detail: serde_json::json!({
            "kind": "RoundTripDrift",
            "rendering": v.rendering,
            "source_excerpt": v.source_excerpt,
            "source_to_rendering": v.source_to_rendering,
            "rendering_to_source": v.rendering_to_source,
            "threshold": v.threshold,
        }),
        triggered_dialogue_turn: None,
        resolved: false,
    }
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

    // -----------------------------------------------------------------
    // ADJ04 gateway-wired tests
    // -----------------------------------------------------------------
    //
    // These tests use scripted LLM clients (one for `Renderer`, one
    // for `Nli`) so the pipeline can exercise the real `check_round_trip`
    // path without needing a live model. The pattern mirrors the
    // scripted clients used inside `adjudication-round-trip`; we keep
    // the two crates' fixtures separate so a future refactor (e.g.,
    // a shared `llm-test-utils` crate) is a local change.

    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse,
        FinishReason as LlmFinishReason, JsonSchema, LlmClient, LlmError, ProviderIdentity,
        TokenUsage,
    };
    use llm_primitives::{GatewayConfig, Role};
    use std::sync::Mutex;

    fn renderer_id() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "haiku-renderer".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    fn nli_id() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "nli-debertav3".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    struct ScriptedRenderer {
        texts: Mutex<Vec<String>>,
    }
    impl ScriptedRenderer {
        fn new(texts: Vec<&str>) -> Self {
            Self {
                texts: Mutex::new(texts.into_iter().rev().map(String::from).collect()),
            }
        }
    }
    impl LlmClient for ScriptedRenderer {
        fn identity(&self) -> ProviderIdentity {
            renderer_id()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            let text = self.texts.lock().unwrap().pop().expect("renderer drained");
            Ok(CompletionResponse {
                text,
                model: "haiku-renderer".into(),
                usage: TokenUsage::default(),
                finish_reason: LlmFinishReason::Stop,
                provider_id: renderer_id(),
                latency_ms: 1,
            })
        }
        fn complete_json(
            &self,
            _r: CompletionRequest,
            _s: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            unreachable!("render_node uses complete")
        }
    }

    struct ScriptedNli {
        scripts: Mutex<Vec<(bool, f32, bool, f32)>>,
    }
    impl ScriptedNli {
        fn new(s: Vec<(bool, f32, bool, f32)>) -> Self {
            Self {
                scripts: Mutex::new(s.into_iter().rev().collect()),
            }
        }
    }
    impl LlmClient for ScriptedNli {
        fn identity(&self) -> ProviderIdentity {
            nli_id()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            unreachable!("entail uses complete_json")
        }
        fn complete_json(
            &self,
            _r: CompletionRequest,
            _s: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            let (p_h, p_s, h_p, h_s) = self.scripts.lock().unwrap().pop().expect("nli drained");
            let parsed = serde_json::json!({
                "premise_entails_hypothesis": p_h,
                "p_to_h_score": p_s,
                "hypothesis_entails_premise": h_p,
                "h_to_p_score": h_s,
            });
            Ok(CompletionJsonResponse {
                raw_text: parsed.to_string(),
                parsed,
                schema_valid: true,
                model: "nli-debertav3".into(),
                usage: TokenUsage::default(),
                provider_id: nli_id(),
                latency_ms: 1,
                polyfill_used: false,
            })
        }
    }

    fn gateway_with_scripted(
        renderings: Vec<&str>,
        entailments: Vec<(bool, f32, bool, f32)>,
    ) -> GatewayConfig {
        GatewayConfig::new()
            .with_client(Role::Renderer, Box::new(ScriptedRenderer::new(renderings)))
            .with_client(Role::Nli, Box::new(ScriptedNli::new(entailments)))
    }

    #[test]
    fn adj04_runs_passed_with_high_scores_under_gateway() {
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let g = gateway_with_scripted(vec!["passenger said hello"], vec![(true, 0.95, true, 0.92)]);
        let out = run_with_gateway(input, AdjudicationId::new("adj-rt-pass"), make_clock(), Some(&g));
        let adj04 = &out.audit_trail.checker_results[2];
        assert_eq!(adj04.pass_name, PassName::Adj04RoundTrip);
        assert_eq!(adj04.pass_version, "v1.0");
        assert!(matches!(adj04.outcome, PassOutcome::Passed));
        assert!(adj04.violations.is_empty());
        // Telemetry should mention the calls (1 render + 1 entail = 2).
        assert_eq!(adj04.telemetry["call_count"], 2);
        // Verdict still Resolved (engine still runs; ADJ04 is advisory).
        assert!(matches!(out.verdict, Verdict::Resolved { .. }));
    }

    #[test]
    fn adj04_runs_failed_with_drift_under_gateway() {
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        // Source-to-rendering score way below the 0.6 default threshold.
        let g = gateway_with_scripted(
            vec!["passenger admitted to smuggling contraband"],
            vec![(false, 0.10, true, 0.90)],
        );
        let out = run_with_gateway(input, AdjudicationId::new("adj-rt-drift"), make_clock(), Some(&g));
        let adj04 = &out.audit_trail.checker_results[2];
        assert!(matches!(adj04.outcome, PassOutcome::Failed));
        assert_eq!(adj04.violations.len(), 1);
        assert_eq!(adj04.violations[0].pass_name, PassName::Adj04RoundTrip);
        assert_eq!(adj04.violations[0].kind, ClarificationKind::RoundTripDrift);
        // ADJ04 is *advisory* at v0.3 — engine still runs.
        assert!(matches!(out.verdict, Verdict::Resolved { .. }));
        assert!(out.audit_trail.engine_artifacts.is_some());
    }

    #[test]
    fn adj04_records_skipped_when_no_gateway_provided() {
        // The plain `run` entry point passes `None` for the gateway —
        // ADJ04 must record as Skipped exactly as in v0.2.
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let out = run(input, AdjudicationId::new("adj-no-gw"), make_clock());
        let adj04 = &out.audit_trail.checker_results[2];
        assert!(matches!(adj04.outcome, PassOutcome::Skipped));
        assert_eq!(adj04.pass_version, "not-yet-wired");
    }

    #[test]
    fn adj04_records_failed_when_required_role_missing_from_gateway() {
        // A gateway is supplied but the `Renderer` role isn't registered —
        // the round-trip checker surfaces `PrimitiveError::NoClientForRole`,
        // which the pipeline records as Failed with the error in telemetry.
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let g = GatewayConfig::new(); // empty
        let out = run_with_gateway(input, AdjudicationId::new("adj-rt-noclient"), make_clock(), Some(&g));
        let adj04 = &out.audit_trail.checker_results[2];
        assert!(matches!(adj04.outcome, PassOutcome::Failed));
        let detail = adj04.telemetry["check_error"].as_str().unwrap();
        assert!(detail.contains("renderer") || detail.contains("Renderer"));
    }

    #[test]
    fn adj04_is_skipped_when_prior_gating_failed_even_if_gateway_supplied() {
        // Coverage fails → don't waste LLM tokens on ADJ04.
        let n = fact_node("n1", logic_core::atom("anomaly"), 100, 150);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let g = gateway_with_scripted(vec!["unused"], vec![(true, 0.99, true, 0.99)]);
        let out = run_with_gateway(input, AdjudicationId::new("adj-rt-skip-on-fail"), make_clock(), Some(&g));
        let adj04 = &out.audit_trail.checker_results[2];
        assert!(matches!(adj04.outcome, PassOutcome::Skipped));
        // And the pipeline still Blocks due to the coverage failure.
        assert!(matches!(out.verdict, Verdict::Blocked { .. }));
    }
}
