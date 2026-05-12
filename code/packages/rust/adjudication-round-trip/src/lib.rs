// PrimitiveError carried inside CheckError::Primitive is large; see
// the parent crates for the audit-trail discipline argument.
#![allow(clippy::result_large_err)]

//! # adjudication-round-trip — ADJ04 checker
//!
//! For each leaf IR node, render the node back into natural language
//! and then run bidirectional textual entailment between the
//! rendering and the original source span. Drift in either direction
//! surfaces as a [`RoundTripViolation`] that the pipeline writes into
//! the audit trail and clarification dialogue can pick up.
//!
//! ## Why bidirectional
//!
//! A one-way "does the source entail the rendering?" check misses
//! drift in the *other* direction (the IR claiming more than the
//! source supports). ADJ04 catches both:
//!
//!   * `p_to_h_score < threshold` ⇒ source doesn't support the IR.
//!   * `h_to_p_score < threshold` ⇒ IR claims more than the source.
//!
//! Both surface as the same violation kind — `RoundTripDrift` — with
//! the failing direction(s) recorded in `detail`.
//!
//! ## Threshold policy
//!
//! The threshold is configurable per call ([`CheckOptions::threshold`],
//! default `0.6`). Strict deployments tighten it; exploratory ones
//! relax it. The threshold flows into the audit trail so a reviewer
//! can see which config produced a verdict.
//!
//! ## What this checker DOES NOT do
//!
//! - **Pick a model.** The deployment's `GatewayConfig` maps the
//!   `Renderer` and `Nli` roles to concrete clients. Per ADJ04 the
//!   two roles should be different model families.
//! - **Retry on validation failure.** Each primitive surfaces its own
//!   `ValidationExhausted` — this checker propagates as a
//!   `CheckError`. A future retry harness can wrap the loop.
//! - **Sample.** v0.1 runs every leaf node. Sampling for large IR
//!   documents is a follow-up.

use adjudication_ir::{IRDocument, IRNode, NodeId, NodeKind};
use llm_primitives::{
    entail, render_node, EntailRequest, GatewayConfig, LlmCallRecord, PrimitiveError,
    RenderNodeRequest, RenderStyle,
};

/// Per-call tuning. Defaults are calibrated for the canonical TSA-
/// shape example in ADJ10.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckOptions {
    /// Each direction's `entail` score must be at least this to count
    /// as "no drift". Default `0.6`.
    pub threshold: f32,
    /// Style passed to `render_node`. Defaults to `Plain`.
    pub style: RenderStyle,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            threshold: 0.6,
            style: RenderStyle::Plain,
        }
    }
}

/// Outcome of one [`check_round_trip`] call.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundTripResult {
    /// Gating violations. Non-empty ⇒ the check failed.
    pub violations: Vec<RoundTripViolation>,
    /// One audit-trail record per LLM call made during the check.
    /// The pipeline copies these into `CheckerResult.telemetry` (or
    /// a follow-up "calls" field) so the trail records exactly what
    /// the checker did.
    pub call_records: Vec<LlmCallRecord>,
}

impl RoundTripResult {
    pub fn pass(&self) -> bool {
        self.violations.is_empty()
    }
}

/// One round-trip drift finding.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundTripViolation {
    pub node_id: NodeId,
    /// The LLM's rendering for the node. Useful in the clarification
    /// turn — "you said X, the source says Y; which is right?".
    pub rendering: String,
    /// Source-span text the checker compared against. Concatenated
    /// across the node's `source_spans` (separated by ` … `).
    pub source_excerpt: String,
    /// `p_to_h` (source → rendering). `None` if the entail call
    /// itself failed — in that case the structural-failure path
    /// runs instead, see [`CheckError`].
    pub source_to_rendering: f32,
    /// `h_to_p` (rendering → source).
    pub rendering_to_source: f32,
    pub threshold: f32,
}

/// Errors the checker can surface. Two flavours: a primitive
/// (`render_node` or `entail`) returned an error, OR the IR has a
/// structural issue the checker can't paper over (e.g., a leaf with
/// no source spans).
#[derive(Debug)]
pub enum CheckError {
    /// One of the primitives failed (transport, validation, etc.).
    /// The pipeline surfaces this as an operator-facing alert.
    Primitive(PrimitiveError),
    /// A leaf node had no source spans. ADJ01 v2 validation should
    /// have caught this upstream; we surface it explicitly rather
    /// than panicking.
    LeafMissingSpans { node_id: NodeId },
    /// A node's source span exceeded the document text. The pipeline
    /// usually catches this in ADJ02 coverage; we re-check rather
    /// than panicking on a bad index.
    SpanOutOfBounds {
        node_id: NodeId,
        start: usize,
        end: usize,
        text_len: usize,
    },
}

impl From<PrimitiveError> for CheckError {
    fn from(e: PrimitiveError) -> Self {
        CheckError::Primitive(e)
    }
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::Primitive(e) => write!(f, "primitive error: {e}"),
            CheckError::LeafMissingSpans { node_id } => {
                write!(f, "leaf node {} has no source spans", node_id.0)
            }
            CheckError::SpanOutOfBounds {
                node_id,
                start,
                end,
                text_len,
            } => write!(
                f,
                "node {} span [{}..{}] exceeds document length {}",
                node_id.0, start, end, text_len
            ),
        }
    }
}

impl std::error::Error for CheckError {}

/// Run ADJ04 on a document + IR. The pipeline calls this after ADJ02
/// (coverage) and ADJ03 (polarity-modality) have passed; it assumes
/// the IR is well-formed and the spans are in-bounds, but rechecks
/// defensively so a bug elsewhere can't crash the checker.
pub fn check_round_trip(
    document_text: &str,
    ir_doc: &IRDocument,
    gateway: &GatewayConfig,
    opts: &CheckOptions,
) -> Result<RoundTripResult, CheckError> {
    let mut violations = Vec::new();
    let mut call_records = Vec::new();

    for node in &ir_doc.nodes {
        if !is_round_trippable(node.kind) {
            // TextRun grouping nodes carry no atomic content to render;
            // Discarded nodes are accounted for in ADJ02 already.
            continue;
        }

        let excerpt = excerpt_for_node(document_text, node)?;
        let node_description = describe_node(node);

        // 1. Render the node back into text via the `Renderer` role.
        let render_resp = render_node(
            &RenderNodeRequest {
                node_description,
                document_excerpt: excerpt.clone(),
                style: opts.style,
            },
            gateway,
        )?;
        call_records.push(render_resp.call_record.clone());

        // 2. Run bidirectional entailment via the `Nli` role.
        let entail_resp = entail(
            &EntailRequest {
                premise: excerpt.clone(),
                hypothesis: render_resp.rendering.clone(),
            },
            gateway,
        )?;
        call_records.push(entail_resp.call_record.clone());

        let p_to_h = entail_resp.p_to_h_score;
        let h_to_p = entail_resp.h_to_p_score;

        if p_to_h < opts.threshold || h_to_p < opts.threshold {
            violations.push(RoundTripViolation {
                node_id: node.id.clone(),
                rendering: render_resp.rendering.clone(),
                source_excerpt: excerpt,
                source_to_rendering: p_to_h,
                rendering_to_source: h_to_p,
                threshold: opts.threshold,
            });
        }
    }

    Ok(RoundTripResult {
        violations,
        call_records,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_round_trippable(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Fact
            | NodeKind::Query
            | NodeKind::Uncertainty
            | NodeKind::Rule
            | NodeKind::Exception
    )
}

/// Concatenate the text covered by a node's source spans, separated
/// by ` … ` when there are multiple non-contiguous spans. Returns
/// `CheckError::SpanOutOfBounds` if any span exceeds the document.
fn excerpt_for_node(document_text: &str, node: &IRNode) -> Result<String, CheckError> {
    if node.source_spans.is_empty() {
        return Err(CheckError::LeafMissingSpans {
            node_id: node.id.clone(),
        });
    }
    let bytes = document_text.as_bytes();
    let mut pieces = Vec::with_capacity(node.source_spans.len());
    for span in &node.source_spans {
        if span.end > bytes.len() || span.start >= span.end {
            return Err(CheckError::SpanOutOfBounds {
                node_id: node.id.clone(),
                start: span.start,
                end: span.end,
                text_len: bytes.len(),
            });
        }
        // Spans are guaranteed by ADJ01 v2 to be byte-aligned on
        // character boundaries; fall back to lossy if a misuse slips
        // through. We choose lossy over panic so the audit trail
        // still records the violation.
        pieces.push(String::from_utf8_lossy(&bytes[span.start..span.end]).into_owned());
    }
    Ok(pieces.join(" … "))
}

/// Build a short textual description of an IR node for `render_node`.
/// v0.1 uses Debug rendering for `term` (we can't depend on
/// `Display`-impl coverage of every `Term` variant from logic-core).
/// A future version with serde-derived IR will use a structured
/// shape.
fn describe_node(node: &IRNode) -> String {
    format!(
        "id={id} kind={kind:?} polarity={pol:?} modality={mod_:?} term={term:?}",
        id = node.id.0,
        kind = node.kind,
        pol = node.polarity,
        mod_ = node.modality,
        term = node.term,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{DocumentId, IRNode, Modality, NodeId as IRNodeId, NodeKind, Polarity, Span};
    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse, FinishReason,
        JsonSchema, LlmClient, LlmError, ProviderIdentity, TokenUsage,
    };
    use llm_primitives::Role;
    use logic_core::Term;
    use std::sync::Mutex;

    // -----------------------------------------------------------------------
    // Test clients — keep two separate scripted clients so the checker
    // can hit Renderer and Nli independently.
    // -----------------------------------------------------------------------

    fn renderer_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "haiku-renderer".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    fn nli_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "nli-debertav3".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    /// Returns the same rendering for every call. Tests that need
    /// per-node renderings can build a list and index by call count
    /// (not used in v0.1 since tests use one node at a time mostly).
    struct ScriptedRenderer {
        renderings: Mutex<Vec<String>>,
    }

    impl ScriptedRenderer {
        fn new(renderings: Vec<&str>) -> Self {
            Self {
                renderings: Mutex::new(
                    renderings.into_iter().rev().map(String::from).collect(),
                ),
            }
        }
    }

    impl LlmClient for ScriptedRenderer {
        fn identity(&self) -> ProviderIdentity {
            renderer_identity()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            let text = self
                .renderings
                .lock()
                .unwrap()
                .pop()
                .expect("ScriptedRenderer ran out of scripted renderings");
            Ok(CompletionResponse {
                text,
                model: "haiku-renderer".into(),
                usage: TokenUsage::default(),
                finish_reason: FinishReason::Stop,
                provider_id: renderer_identity(),
                latency_ms: 10,
            })
        }
        fn complete_json(
            &self,
            _req: CompletionRequest,
            _schema: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            unreachable!("render_node uses complete, not complete_json")
        }
    }

    /// Returns the same entail JSON for every call.
    struct ScriptedNli {
        scripts: Mutex<Vec<(bool, f32, bool, f32)>>,
    }

    impl ScriptedNli {
        fn new(scripts: Vec<(bool, f32, bool, f32)>) -> Self {
            Self {
                scripts: Mutex::new(scripts.into_iter().rev().collect()),
            }
        }
    }

    impl LlmClient for ScriptedNli {
        fn identity(&self) -> ProviderIdentity {
            nli_identity()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            unreachable!("entail uses complete_json")
        }
        fn complete_json(
            &self,
            _req: CompletionRequest,
            _schema: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            let (p_h, p_h_score, h_p, h_p_score) = self
                .scripts
                .lock()
                .unwrap()
                .pop()
                .expect("ScriptedNli ran out of scripted entailments");
            let parsed = serde_json::json!({
                "premise_entails_hypothesis": p_h,
                "p_to_h_score": p_h_score,
                "hypothesis_entails_premise": h_p,
                "h_to_p_score": h_p_score,
            });
            let raw_text = parsed.to_string();
            Ok(CompletionJsonResponse {
                raw_text,
                parsed,
                schema_valid: true,
                model: "nli-debertav3".into(),
                usage: TokenUsage::default(),
                provider_id: nli_identity(),
                latency_ms: 8,
                polyfill_used: false,
            })
        }
    }

    fn make_doc() -> &'static str {
        "1 carry-on bag, 1 personal item."
    }

    fn fact(id: &str, term: Term, start: usize, end: usize) -> IRNode {
        IRNode {
            id: IRNodeId::new(id.to_string()),
            kind: NodeKind::Fact,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![Span::new(DocumentId::new("doc1"), start, end)],
            confidence: 1.0,
            part_of: None,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        }
    }

    fn ir(nodes: Vec<IRNode>) -> IRDocument {
        IRDocument {
            document_id: DocumentId::new("doc1"),
            nodes,
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn no_round_trippable_nodes_means_no_violations_no_calls() {
        // TextRun grouping nodes have no atomic content; round-trip
        // ignores them. A document with only TextRuns yields zero
        // violations and zero LLM calls.
        let mut n = fact("g", Term::Atom("group".into()), 0, 5);
        n.kind = NodeKind::TextRun;
        let g = GatewayConfig::new();
        let r = check_round_trip(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert!(r.violations.is_empty());
        assert!(r.call_records.is_empty());
    }

    #[test]
    fn high_scoring_round_trip_passes() {
        // Rendering matches the source closely. Both entail scores
        // above the threshold → pass.
        let n = fact("n1", Term::Atom("carry_on(1)".into()), 0, 14);
        let g = GatewayConfig::new()
            .with_client(
                Role::Renderer,
                Box::new(ScriptedRenderer::new(vec![
                    "The passenger declared one carry-on bag.",
                ])),
            )
            .with_client(
                Role::Nli,
                Box::new(ScriptedNli::new(vec![(true, 0.95, true, 0.90)])),
            );
        let r = check_round_trip(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert!(r.pass());
        // One rendering + one entail call.
        assert_eq!(r.call_records.len(), 2);
        assert_eq!(r.call_records[0].primitive, "render_node");
        assert_eq!(r.call_records[1].primitive, "entail");
    }

    #[test]
    fn source_to_rendering_drift_is_flagged() {
        // The rendering claims more than the source supports. p_to_h
        // score is low.
        let n = fact("n1", Term::Atom("carry_on(1)".into()), 0, 14);
        let g = GatewayConfig::new()
            .with_client(
                Role::Renderer,
                Box::new(ScriptedRenderer::new(vec![
                    "The passenger declared one carry-on bag AND a service animal.",
                ])),
            )
            .with_client(
                Role::Nli,
                Box::new(ScriptedNli::new(vec![(false, 0.30, true, 0.85)])),
            );
        let r = check_round_trip(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert_eq!(r.violations.len(), 1);
        let v = &r.violations[0];
        assert_eq!(v.node_id.0, "n1");
        assert!(v.rendering.contains("service animal"));
        assert!((v.source_to_rendering - 0.30).abs() < 1e-6);
        assert!((v.threshold - 0.6).abs() < 1e-6);
    }

    #[test]
    fn rendering_to_source_drift_is_flagged() {
        // The source says more than the rendering captures. h_to_p
        // score is low.
        let n = fact("n1", Term::Atom("carry_on(1)".into()), 0, 14);
        let g = GatewayConfig::new()
            .with_client(
                Role::Renderer,
                Box::new(ScriptedRenderer::new(vec!["Bag was declared."])),
            )
            .with_client(
                Role::Nli,
                Box::new(ScriptedNli::new(vec![(true, 0.92, false, 0.35)])),
            );
        let r = check_round_trip(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert_eq!(r.violations.len(), 1);
        let v = &r.violations[0];
        assert!((v.rendering_to_source - 0.35).abs() < 1e-6);
    }

    #[test]
    fn custom_threshold_is_respected() {
        // The same scores that fail at the default 0.6 pass at 0.3.
        let n = fact("n1", Term::Atom("a".into()), 0, 14);
        let g = GatewayConfig::new()
            .with_client(
                Role::Renderer,
                Box::new(ScriptedRenderer::new(vec!["whatever"])),
            )
            .with_client(
                Role::Nli,
                Box::new(ScriptedNli::new(vec![(true, 0.40, true, 0.40)])),
            );
        let opts = CheckOptions {
            threshold: 0.3,
            style: RenderStyle::Plain,
        };
        let r = check_round_trip(make_doc(), &ir(vec![n]), &g, &opts).unwrap();
        assert!(r.pass());
    }

    #[test]
    fn missing_renderer_client_returns_primitive_error() {
        let n = fact("n1", Term::Atom("a".into()), 0, 14);
        let g = GatewayConfig::new();
        let err = check_round_trip(make_doc(), &ir(vec![n]), &g, &CheckOptions::default())
            .unwrap_err();
        assert!(matches!(
            err,
            CheckError::Primitive(PrimitiveError::NoClientForRole { .. })
        ));
    }

    #[test]
    fn span_out_of_bounds_is_a_typed_error() {
        // ADJ02 should have caught this, but check defensively.
        let n = fact("n1", Term::Atom("a".into()), 100, 200);
        let g = GatewayConfig::new();
        let err = check_round_trip(make_doc(), &ir(vec![n]), &g, &CheckOptions::default())
            .unwrap_err();
        match err {
            CheckError::SpanOutOfBounds {
                node_id, start, end, text_len,
            } => {
                assert_eq!(node_id.0, "n1");
                assert_eq!(start, 100);
                assert_eq!(end, 200);
                assert_eq!(text_len, make_doc().len());
            }
            other => panic!("expected SpanOutOfBounds, got {other:?}"),
        }
    }

    #[test]
    fn leaf_missing_spans_is_a_typed_error() {
        let mut n = fact("n1", Term::Atom("a".into()), 0, 14);
        n.source_spans.clear();
        let g = GatewayConfig::new();
        let err = check_round_trip(make_doc(), &ir(vec![n]), &g, &CheckOptions::default())
            .unwrap_err();
        match err {
            CheckError::LeafMissingSpans { node_id } => assert_eq!(node_id.0, "n1"),
            other => panic!("expected LeafMissingSpans, got {other:?}"),
        }
    }

    #[test]
    fn multiple_spans_are_concatenated_with_ellipsis_separator() {
        let mut n = fact("n1", Term::Atom("a".into()), 0, 5);
        n.source_spans
            .push(Span::new(DocumentId::new("doc1"), 16, 32));
        // "1 carry-on" + " … " + "1 personal item."
        let g = GatewayConfig::new()
            .with_client(
                Role::Renderer,
                Box::new(ScriptedRenderer::new(vec!["combined claim"])),
            )
            .with_client(
                Role::Nli,
                Box::new(ScriptedNli::new(vec![(true, 0.9, true, 0.9)])),
            );
        let r = check_round_trip(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        // No violations — we just want the call to succeed. To
        // inspect the excerpt, force a violation via low scores.
        let _ = r;

        // Now do the same with a violation so we can read back the excerpt.
        let mut m = fact("n2", Term::Atom("a".into()), 0, 5);
        m.source_spans
            .push(Span::new(DocumentId::new("doc1"), 16, 32));
        let g2 = GatewayConfig::new()
            .with_client(
                Role::Renderer,
                Box::new(ScriptedRenderer::new(vec!["combined claim"])),
            )
            .with_client(
                Role::Nli,
                Box::new(ScriptedNli::new(vec![(false, 0.1, false, 0.1)])),
            );
        let r2 =
            check_round_trip(make_doc(), &ir(vec![m]), &g2, &CheckOptions::default()).unwrap();
        assert_eq!(r2.violations.len(), 1);
        assert!(r2.violations[0].source_excerpt.contains("1 car"));
        assert!(r2.violations[0].source_excerpt.contains("personal item"));
        assert!(r2.violations[0].source_excerpt.contains(" … "));
    }

    #[test]
    fn call_records_in_order_one_per_primitive_per_node() {
        // Two nodes → 2 render calls + 2 entail calls.
        let n1 = fact("n1", Term::Atom("a".into()), 0, 5);
        let n2 = fact("n2", Term::Atom("b".into()), 16, 32);
        let g = GatewayConfig::new()
            .with_client(
                Role::Renderer,
                Box::new(ScriptedRenderer::new(vec!["r1", "r2"])),
            )
            .with_client(
                Role::Nli,
                Box::new(ScriptedNli::new(vec![
                    (true, 0.95, true, 0.95),
                    (true, 0.95, true, 0.95),
                ])),
            );
        let r =
            check_round_trip(make_doc(), &ir(vec![n1, n2]), &g, &CheckOptions::default()).unwrap();
        assert_eq!(r.call_records.len(), 4);
        // Interleaved: render(n1), entail(n1), render(n2), entail(n2).
        assert_eq!(r.call_records[0].primitive, "render_node");
        assert_eq!(r.call_records[1].primitive, "entail");
        assert_eq!(r.call_records[2].primitive, "render_node");
        assert_eq!(r.call_records[3].primitive, "entail");
    }

    #[test]
    fn discarded_nodes_are_skipped() {
        let mut n = fact("d1", Term::Atom("x".into()), 0, 5);
        n.kind = NodeKind::Discarded;
        let g = GatewayConfig::new();
        let r = check_round_trip(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert!(r.call_records.is_empty());
        assert!(r.pass());
    }
}
