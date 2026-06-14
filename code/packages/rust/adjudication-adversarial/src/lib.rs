// PrimitiveError carried inside CheckError::Primitive is large; same
// audit-trail discipline as the sibling checker crates.
#![allow(clippy::result_large_err)]

//! # adjudication-adversarial — ADJ05 checker
//!
//! For each sampled leaf IR node, run the three-step ADJ05 loop:
//!
//!   1. [`render_node`](llm_primitives::render_node) — translate the
//!      IR back into natural language.
//!   2. [`find_contradicting_reading`](llm_primitives::find_contradicting_reading)
//!      — ask the **Adversary** role for the strongest alternative
//!      reading of the source that contradicts the rendering. Returns
//!      either `Concurs` or a `Reading { text, explanation }`.
//!   3. [`judge_plausibility`](llm_primitives::judge_plausibility) —
//!      if the adversary found a reading, ask the **Plausibility**
//!      role whether a competent practitioner would actually adopt it.
//!
//! Outcomes:
//!
//! - Adversary returns `Concurs` → nothing recorded for this node.
//! - Adversary returns `Reading`, judge says `IMPLAUSIBLE` → reading
//!   logged into `call_records` for the audit trail; no gating
//!   violation (per ADJ05, the implausible-but-found case still
//!   appears in the trail).
//! - Adversary returns `Reading`, judge says `PLAUSIBLE` →
//!   [`AdversarialViolation`] recorded. ADJ06 picks it up to clarify.
//!
//! ## Independence requirement
//!
//! ADJ05's whole point is that the Adversary must be a *different
//! model family* from the Extractor. The framework enforces this at
//! startup via `GatewayConfig::check_independence`; this checker
//! does not double-check (a redundant check would just slow every
//! call). Deployments that skip the startup check accept the
//! consequence: an adversary that rubber-stamps the IR.
//!
//! ## What v0.1 does NOT do
//!
//! - **Sample.** v0.1 visits every leaf node. ADJ05's intended
//!   sample-rate knob (`adversary_sample_rate` in the configuration)
//!   is a follow-up.
//! - **Retry on primitive validation failure.** Surfaces as
//!   `CheckError::Primitive`. A future retry harness can wrap.

use adjudication_ir::{IRDocument, IRNode, NodeId, NodeKind};
use llm_primitives::{
    find_contradicting_reading, judge_plausibility, render_node, FindContradictingReadingRequest,
    FindContradictingReadingResponse, GatewayConfig, JudgePlausibilityRequest, LlmCallRecord,
    PrimitiveError, RenderNodeRequest, RenderStyle,
};

/// Per-call tuning.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckOptions {
    /// Style passed to `render_node`. Default `Plain`.
    pub style: RenderStyle,
    /// Free-text domain hint forwarded to the adversary and the
    /// judge (e.g., `"clinical-note"`, `"tsa-declaration"`). When
    /// the LM00b `DomainHints` enum lands, this becomes typed.
    pub domain_hint: String,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            style: RenderStyle::Plain,
            domain_hint: String::new(),
        }
    }
}

/// Outcome of one [`check_adversarial`] call.
#[derive(Debug, Clone, PartialEq)]
pub struct AdversarialResult {
    /// Gating violations: per-node `(reading, judge)` pairs where the
    /// adversary's reading was deemed plausible.
    pub violations: Vec<AdversarialViolation>,
    /// All LLM call records produced during the check, in invocation
    /// order. The pipeline writes these into the audit trail.
    pub call_records: Vec<LlmCallRecord>,
}

impl AdversarialResult {
    pub fn pass(&self) -> bool {
        self.violations.is_empty()
    }
}

/// One AdversarialReading finding.
#[derive(Debug, Clone, PartialEq)]
pub struct AdversarialViolation {
    pub node_id: NodeId,
    /// The IR's rendering (from `render_node`).
    pub ir_rendered: String,
    /// The adversary's contradicting reading.
    pub adversary_reading: String,
    /// The adversary's explanation of how it diverges from the IR.
    pub adversary_explanation: String,
    /// The judge's reason for ruling `PLAUSIBLE`.
    pub judge_reason: String,
}

/// Errors the checker can surface. The structural-failure variants
/// should never fire in a healthy pipeline (ADJ01/ADJ02 catch them
/// upstream) but we surface them explicitly rather than panicking.
#[derive(Debug)]
pub enum CheckError {
    Primitive(PrimitiveError),
    LeafMissingSpans { node_id: NodeId },
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
                node_id, start, end, text_len,
            } => write!(
                f,
                "node {} span [{}..{}] exceeds document length {}",
                node_id.0, start, end, text_len
            ),
        }
    }
}

impl std::error::Error for CheckError {}

/// Run ADJ05 over an IR document.
pub fn check_adversarial(
    document_text: &str,
    ir_doc: &IRDocument,
    gateway: &GatewayConfig,
    opts: &CheckOptions,
) -> Result<AdversarialResult, CheckError> {
    let mut violations = Vec::new();
    let mut call_records = Vec::new();

    for node in &ir_doc.nodes {
        if !is_attackable(node.kind) {
            continue;
        }
        if matches!(node.kind, NodeKind::Query) && node.source_spans.is_empty() {
            // Synthesized Query nodes (framework-added, no
            // corresponding source span) have nothing in the source
            // to attack. Skip them. Same special case as
            // adjudication-round-trip; Facts with missing spans
            // still surface as LeafMissingSpans because Facts MUST
            // come from the source per ADJ01 v2.
            continue;
        }

        let source_excerpt = excerpt_for_node(document_text, node)?;
        let node_description = describe_node(node);

        // 1. Render the IR node.
        let render_resp = render_node(
            &RenderNodeRequest {
                node_description,
                document_excerpt: source_excerpt.clone(),
                style: opts.style,
            },
            gateway,
        )?;
        call_records.push(render_resp.call_record.clone());

        // 2. Find a contradicting reading (or Concurs).
        let fcr_resp = find_contradicting_reading(
            &FindContradictingReadingRequest {
                source_span_text: source_excerpt.clone(),
                ir_rendered: render_resp.rendering.clone(),
                domain_hint: opts.domain_hint.clone(),
            },
            gateway,
        )?;
        call_records.push(fcr_resp.call_record().clone());

        let (reading, explanation) = match fcr_resp {
            FindContradictingReadingResponse::Concurs { .. } => continue,
            FindContradictingReadingResponse::Reading {
                text,
                explanation,
                ..
            } => (text, explanation),
        };

        // 3. Judge whether the reading is plausible.
        let judge_resp = judge_plausibility(
            &JudgePlausibilityRequest {
                source_span_text: source_excerpt.clone(),
                ir_rendered: render_resp.rendering.clone(),
                adversary_reading: reading.clone(),
                domain_hint: opts.domain_hint.clone(),
            },
            gateway,
        )?;
        call_records.push(judge_resp.call_record.clone());

        if judge_resp.plausible {
            violations.push(AdversarialViolation {
                node_id: node.id.clone(),
                ir_rendered: render_resp.rendering.clone(),
                adversary_reading: reading,
                adversary_explanation: explanation,
                judge_reason: judge_resp.reason,
            });
        }
        // IMPLAUSIBLE: the reading is in `call_records` for the audit
        // trail but is not promoted to a gating violation.
    }

    Ok(AdversarialResult {
        violations,
        call_records,
    })
}

// ---------------------------------------------------------------------------
// Helpers (same shape as adjudication-round-trip)
// ---------------------------------------------------------------------------

fn is_attackable(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Fact
            | NodeKind::Query
            | NodeKind::Uncertainty
            | NodeKind::Rule
            | NodeKind::Exception
    )
}

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
        pieces.push(String::from_utf8_lossy(&bytes[span.start..span.end]).into_owned());
    }
    Ok(pieces.join(" … "))
}

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
    use adjudication_ir::{
        DocumentId, IRNode, Modality, NodeId as IRNodeId, NodeKind, Polarity, Span,
    };
    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse, FinishReason,
        JsonSchema, LlmClient, LlmError, ProviderIdentity, TokenUsage,
    };
    use llm_primitives::Role;
    use logic_core::Term;
    use std::sync::Mutex;

    // -----------------------------------------------------------------------
    // Test scaffolds: one client per role, scripted by call order.
    // -----------------------------------------------------------------------

    fn renderer_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "haiku-renderer".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    fn adversary_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "opus-adversary".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    fn plausibility_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "haiku-judge".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

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
                .expect("ScriptedRenderer out of scripted renderings");
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

    /// Each entry is the FCR response shape: `(concurs, text, explanation)`.
    struct ScriptedAdversary {
        scripts: Mutex<Vec<(bool, String, String)>>,
        identity: ProviderIdentity,
    }

    impl ScriptedAdversary {
        fn new(scripts: Vec<(bool, &str, &str)>) -> Self {
            Self {
                scripts: Mutex::new(
                    scripts
                        .into_iter()
                        .rev()
                        .map(|(c, t, e)| (c, t.to_string(), e.to_string()))
                        .collect(),
                ),
                identity: adversary_identity(),
            }
        }
    }

    impl LlmClient for ScriptedAdversary {
        fn identity(&self) -> ProviderIdentity {
            self.identity.clone()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            unreachable!("FCR uses complete_json")
        }
        fn complete_json(
            &self,
            _req: CompletionRequest,
            _schema: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            let (c, t, e) = self
                .scripts
                .lock()
                .unwrap()
                .pop()
                .expect("ScriptedAdversary out of scripts");
            let parsed = serde_json::json!({
                "concurs": c,
                "text": t,
                "explanation": e,
            });
            let raw_text = parsed.to_string();
            Ok(CompletionJsonResponse {
                raw_text,
                parsed,
                schema_valid: true,
                model: "opus-adversary".into(),
                usage: TokenUsage::default(),
                provider_id: self.identity.clone(),
                latency_ms: 50,
                polyfill_used: false,
            })
        }
    }

    /// Each entry is the judge's response: `(plausible, reason)`.
    struct ScriptedJudge {
        scripts: Mutex<Vec<(bool, String)>>,
    }

    impl ScriptedJudge {
        fn new(scripts: Vec<(bool, &str)>) -> Self {
            Self {
                scripts: Mutex::new(
                    scripts
                        .into_iter()
                        .rev()
                        .map(|(p, r)| (p, r.to_string()))
                        .collect(),
                ),
            }
        }
    }

    impl LlmClient for ScriptedJudge {
        fn identity(&self) -> ProviderIdentity {
            plausibility_identity()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            unreachable!("judge uses complete_json")
        }
        fn complete_json(
            &self,
            _req: CompletionRequest,
            _schema: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            let (p, r) = self
                .scripts
                .lock()
                .unwrap()
                .pop()
                .expect("ScriptedJudge out of scripts");
            let parsed = serde_json::json!({
                "plausible": p,
                "reason": r,
            });
            let raw_text = parsed.to_string();
            Ok(CompletionJsonResponse {
                raw_text,
                parsed,
                schema_valid: true,
                model: "haiku-judge".into(),
                usage: TokenUsage::default(),
                provider_id: plausibility_identity(),
                latency_ms: 15,
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
            discard_reason: None,
            metadata: Default::default(),
        }
    }

    fn ir(nodes: Vec<IRNode>) -> IRDocument {
        IRDocument {
            document_id: DocumentId::new("doc1"),
            nodes,
            edges: Vec::new(),
        }
    }

    fn three_role_gateway(
        renderings: Vec<&str>,
        fcr: Vec<(bool, &str, &str)>,
        judge: Vec<(bool, &str)>,
    ) -> GatewayConfig {
        GatewayConfig::new()
            .with_client(Role::Renderer, Box::new(ScriptedRenderer::new(renderings)))
            .with_client(Role::Adversary, Box::new(ScriptedAdversary::new(fcr)))
            .with_client(Role::Plausibility, Box::new(ScriptedJudge::new(judge)))
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn adversary_concurs_means_no_violation_and_only_two_calls() {
        // render + FCR; no judge call because Concurs short-circuits.
        let n = fact("n1", Term::Atom("a".into()), 0, 14);
        let g = three_role_gateway(
            vec!["A faithful rendering."],
            vec![(true, "", "")],
            vec![],
        );
        let r = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert!(r.pass());
        assert_eq!(r.call_records.len(), 2);
        assert_eq!(r.call_records[0].primitive, "render_node");
        assert_eq!(r.call_records[1].primitive, "find_contradicting_reading");
    }

    #[test]
    fn implausible_reading_records_in_audit_but_no_violation() {
        let n = fact("n1", Term::Atom("a".into()), 0, 14);
        let g = three_role_gateway(
            vec!["A rendering."],
            vec![(false, "Silly alternative.", "It's silly.")],
            vec![(false, "A competent reader would not adopt that.")],
        );
        let r = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert!(r.pass());
        // All three calls recorded for the trail.
        assert_eq!(r.call_records.len(), 3);
        assert_eq!(r.call_records[2].primitive, "judge_plausibility");
    }

    #[test]
    fn plausible_reading_becomes_an_adversarial_violation() {
        let n = fact("n1", Term::Atom("a".into()), 0, 14);
        let g = three_role_gateway(
            vec!["Passenger declared one carry-on bag."],
            vec![(
                false,
                "Passenger declared one carry-on bag plus a service animal.",
                "SOURCE doesn't rule out an accompanying service animal.",
            )],
            vec![(true, "Plausible in practice — service animals are common.")],
        );
        let r = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert_eq!(r.violations.len(), 1);
        let v = &r.violations[0];
        assert_eq!(v.node_id.0, "n1");
        assert!(v.ir_rendered.contains("carry-on"));
        assert!(v.adversary_reading.contains("service animal"));
        assert!(v.adversary_explanation.contains("doesn't rule out"));
        assert!(v.judge_reason.contains("Plausible"));
    }

    #[test]
    fn no_attackable_nodes_means_no_calls_no_violations() {
        let mut n = fact("g", Term::Atom("group".into()), 0, 5);
        n.kind = NodeKind::Section;
        let g = GatewayConfig::new(); // no clients required
        let r = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert!(r.pass());
        assert!(r.call_records.is_empty());
    }

    #[test]
    fn missing_renderer_returns_primitive_error() {
        let n = fact("n1", Term::Atom("a".into()), 0, 14);
        let g = GatewayConfig::new()
            .with_client(Role::Adversary, Box::new(ScriptedAdversary::new(vec![])))
            .with_client(Role::Plausibility, Box::new(ScriptedJudge::new(vec![])));
        let err = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default())
            .unwrap_err();
        assert!(matches!(
            err,
            CheckError::Primitive(PrimitiveError::NoClientForRole { .. })
        ));
    }

    #[test]
    fn missing_adversary_returns_primitive_error() {
        let n = fact("n1", Term::Atom("a".into()), 0, 14);
        let g = GatewayConfig::new()
            .with_client(
                Role::Renderer,
                Box::new(ScriptedRenderer::new(vec!["whatever"])),
            )
            .with_client(Role::Plausibility, Box::new(ScriptedJudge::new(vec![])));
        let err = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default())
            .unwrap_err();
        assert!(matches!(
            err,
            CheckError::Primitive(PrimitiveError::NoClientForRole { .. })
        ));
    }

    #[test]
    fn missing_judge_returns_primitive_error_only_after_fcr_returns_reading() {
        // Judge isn't consulted on Concurs, so a missing judge is
        // only fatal when the adversary actually finds a reading.
        let n = fact("n1", Term::Atom("a".into()), 0, 14);
        let g = GatewayConfig::new()
            .with_client(
                Role::Renderer,
                Box::new(ScriptedRenderer::new(vec!["render"])),
            )
            .with_client(
                Role::Adversary,
                Box::new(ScriptedAdversary::new(vec![(false, "alt", "exp")])),
            );
        let err = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default())
            .unwrap_err();
        assert!(matches!(
            err,
            CheckError::Primitive(PrimitiveError::NoClientForRole { .. })
        ));
    }

    #[test]
    fn span_out_of_bounds_returns_typed_error() {
        let n = fact("n1", Term::Atom("a".into()), 100, 200);
        let g = GatewayConfig::new();
        let err = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default())
            .unwrap_err();
        assert!(matches!(err, CheckError::SpanOutOfBounds { .. }));
    }

    #[test]
    fn leaf_missing_spans_returns_typed_error() {
        let mut n = fact("n1", Term::Atom("a".into()), 0, 14);
        n.source_spans.clear();
        let g = GatewayConfig::new();
        let err = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default())
            .unwrap_err();
        assert!(matches!(err, CheckError::LeafMissingSpans { .. }));
    }

    #[test]
    fn call_records_are_interleaved_in_invocation_order_per_node() {
        let n1 = fact("n1", Term::Atom("a".into()), 0, 5);
        let n2 = fact("n2", Term::Atom("b".into()), 16, 32);
        let g = three_role_gateway(
            vec!["r1", "r2"],
            vec![(false, "alt1", "exp1"), (true, "", "")],
            vec![(false, "node1 implausible")],
        );
        let r =
            check_adversarial(make_doc(), &ir(vec![n1, n2]), &g, &CheckOptions::default()).unwrap();
        // n1: render + FCR + judge (3 calls, 0 violations because judge said implausible).
        // n2: render + FCR (2 calls, Concurs).
        assert_eq!(r.call_records.len(), 5);
        assert_eq!(r.call_records[0].primitive, "render_node");
        assert_eq!(r.call_records[1].primitive, "find_contradicting_reading");
        assert_eq!(r.call_records[2].primitive, "judge_plausibility");
        assert_eq!(r.call_records[3].primitive, "render_node");
        assert_eq!(r.call_records[4].primitive, "find_contradicting_reading");
        assert!(r.pass());
    }

    #[test]
    fn discarded_nodes_are_skipped() {
        let mut n = fact("d1", Term::Atom("x".into()), 0, 5);
        n.kind = NodeKind::Discarded;
        let g = GatewayConfig::new();
        let r = check_adversarial(make_doc(), &ir(vec![n]), &g, &CheckOptions::default()).unwrap();
        assert!(r.pass());
        assert!(r.call_records.is_empty());
    }
}
