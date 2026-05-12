//! # adjudication-contract-demo — contract-clause A/B demo
//!
//! Third domain alongside `adjudication-tsa-demo` (compliance) and
//! `adjudication-clinical-demo` (triage). Same shape; the source is
//! a short obligation clause and the IR captures the conditional
//! structure ("if X, then Y, unless Z").
//!
//! Why contracts: conditional + exception structure is exactly what
//! ADJ03 modality tracking (`Conditional` modality on rules) and the
//! IR's Exception node kind were designed for. A small model often
//! produces an IR that drops the exception entirely; the structured
//! pipeline catches that omission.
//!
//! ## Canonical fixture (105 bytes)
//!
//! ```text
//! If the buyer pays within 30 days, the seller delivers the goods, unless the goods are out of stock.
//! ```
//!
//! Hand-built IR:
//!
//! - R1: `payment_within(30_days) -> delivery`, Modality `Conditional`.
//! - E1: `out_of_stock` exception attached to R1, polarity Affirmed.
//! - Q1: `delivers(seller, goods)?` — the query.

use std::time::Duration;

use adjudication_audit_trail::{AdjudicationId, PassOutcome};
use adjudication_ir::{
    DocumentId, IRDocument, IRNode, Modality, NodeId, NodeKind, Polarity, Span,
};
use adjudication_pipeline::{
    run_with_gateway, PipelineDocument, PipelineInput, PipelineOutput, Verdict,
};
use llm_cache::CachingClient;
use llm_gateway::{
    CompletionRequest, FinishReason, LlmClient, LlmError, Message, MessageContent, Role as MsgRole,
};
use llm_primitives::{GatewayConfig, Role as PrimitiveRole};
use llm_provider_ollama::OllamaClient;
use logic_core::{atom, compound};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DemoConfig {
    pub endpoint: String,
    pub model: String,
    pub adversary_model: Option<String>,
    pub timeout: Duration,
    pub source_text: String,
    pub cache_dir: Option<String>,
}

const CANONICAL_SOURCE: &str =
    "If the buyer pays within 30 days, the seller delivers the goods, unless the goods are out of stock.";

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".into(),
            model: "gemma4:latest".into(),
            adversary_model: None,
            timeout: Duration::from_secs(120),
            source_text: CANONICAL_SOURCE.into(),
            cache_dir: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Arm A
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RawArmReport {
    pub prompt: String,
    pub answer: String,
    pub model: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub latency_ms: u64,
    pub finish_reason: FinishReason,
}

pub fn run_raw_arm(cfg: &DemoConfig) -> Result<RawArmReport, LlmError> {
    let client = OllamaClient::new(cfg.model.clone())
        .with_endpoint(cfg.endpoint.clone())
        .with_timeout(cfg.timeout);
    let prompt = format!(
        "CONTRACT CLAUSE: {src}\n\nDoes the seller have to deliver the goods?",
        src = cfg.source_text,
    );
    let req = CompletionRequest {
        model: cfg.model.clone(),
        system: Some(
            "You are a contract-review assistant. Given a contract clause, decide \
             whether the obligation it describes is currently in force. Explain \
             in 2-3 sentences, then end with a final line: `VERDICT: OBLIGATION_HOLDS` \
             or `VERDICT: OBLIGATION_EXCUSED`."
                .into(),
        ),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(prompt.clone()),
        }],
        temperature: 0.0,
        max_tokens: Some(512),
        stop_sequences: Vec::new(),
        seed: Some(42),
        metadata: Default::default(),
    };
    let resp = client.complete(req)?;
    Ok(RawArmReport {
        prompt,
        answer: resp.text,
        model: resp.model,
        input_tokens: resp.usage.input_tokens,
        output_tokens: resp.usage.output_tokens,
        latency_ms: resp.latency_ms,
        finish_reason: resp.finish_reason,
    })
}

// ---------------------------------------------------------------------------
// Arm B
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct PipelineArmReport {
    pub verdict_summary: String,
    pub pipeline_output: PipelineOutput,
    pub adj02_outcome: PassOutcome,
    pub adj03_outcome: PassOutcome,
    pub adj04_outcome: PassOutcome,
    pub adj05_outcome: PassOutcome,
    pub engine_ran: bool,
}

pub fn run_pipeline_arm(cfg: &DemoConfig) -> PipelineArmReport {
    let primary = OllamaClient::new(cfg.model.clone())
        .with_endpoint(cfg.endpoint.clone())
        .with_timeout(cfg.timeout);
    let extractor = wrap_with_cache(Box::new(primary.clone()), cfg);
    let renderer = wrap_with_cache(Box::new(primary.clone()), cfg);
    let nli = wrap_with_cache(Box::new(primary.clone()), cfg);
    let plausibility = wrap_with_cache(Box::new(primary.clone()), cfg);
    let mut gateway = GatewayConfig::new()
        .with_client(PrimitiveRole::Extractor, extractor)
        .with_client(PrimitiveRole::Renderer, renderer)
        .with_client(PrimitiveRole::Nli, nli)
        .with_client(PrimitiveRole::Plausibility, plausibility);
    if let Some(adv) = &cfg.adversary_model {
        let adv_client = OllamaClient::new(adv.clone())
            .with_endpoint(cfg.endpoint.clone())
            .with_timeout(cfg.timeout);
        gateway = gateway.with_client(
            PrimitiveRole::Adversary,
            wrap_with_cache(Box::new(adv_client) as Box<dyn LlmClient>, cfg),
        );
    }
    let input = PipelineInput {
        document: PipelineDocument {
            id: "contract-demo-001".into(),
            name: "contract_clause".into(),
            received_at: "2026-05-12T00:00:00Z".into(),
            normalized_text: cfg.source_text.clone(),
            normalization_pipeline: "plain-text-v1".into(),
            normalization_version: "1.0.0".into(),
        },
        ir_document: contract_ir_document(&cfg.source_text),
    };
    let tick = std::cell::Cell::new(0u32);
    let now = move || {
        let t = tick.get();
        tick.set(t + 1);
        format!("2026-05-12T00:00:{:02}Z", t.min(59))
    };
    let output = run_with_gateway(
        input,
        AdjudicationId::new("adj-contract-demo"),
        now,
        Some(&gateway),
    );
    let trail = &output.audit_trail;
    let summary = match &output.verdict {
        Verdict::Resolved { answers } => {
            format!("Resolved with {n} engine answer(s)", n = answers.len())
        }
        Verdict::Blocked { violation_count } => {
            format!("Blocked with {violation_count} violation(s)")
        }
        Verdict::EngineError(detail) => format!("EngineError: {detail}"),
    };
    PipelineArmReport {
        verdict_summary: summary,
        adj02_outcome: trail.checker_results[0].outcome,
        adj03_outcome: trail.checker_results[1].outcome,
        adj04_outcome: trail.checker_results[2].outcome,
        adj05_outcome: trail.checker_results[3].outcome,
        engine_ran: trail.engine_artifacts.is_some(),
        pipeline_output: output,
    }
}

fn wrap_with_cache(inner: Box<dyn LlmClient>, cfg: &DemoConfig) -> Box<dyn LlmClient> {
    match &cfg.cache_dir {
        Some(dir) => Box::new(CachingClient::with_disk_persistence(inner, dir)),
        None => Box::new(CachingClient::new(inner)),
    }
}

// ---------------------------------------------------------------------------
// Contract IR fixture
// ---------------------------------------------------------------------------

/// Build the canonical contract IR. Conditional rule + exception
/// shape — the IR's `NodeKind::Rule` + `NodeKind::Exception`
/// machinery shines here.
///
/// Layout for the 105-byte canonical source:
///
/// - Bytes 0..51:  `"If the buyer pays within 30 days, the seller "`...
///   wait, byte counts vary. Use computed spans below.
///
/// Three IR fragments:
/// - R1: spans bytes [0..p1) — the conditional rule.
/// - E1: spans bytes [p1..end) — the exception clause.
/// - Q1: synthesized query (no source spans).
///
/// `p1` is the byte index where `unless` starts, so the spans tile.
pub fn contract_ir_document(source_text: &str) -> IRDocument {
    let doc_id = DocumentId::new("contract-demo-001");
    let len = source_text.len();
    let mut nodes = Vec::new();

    if source_text == CANONICAL_SOURCE {
        // ADJ02 v2 requires root nodes (`part_of = None`) to TILE the
        // document. The Exception is a non-root refinement (`part_of:
        // R1`) and so doesn't count toward tiling — only R1 does. R1
        // spans the entire 105-byte sentence; E1 carves out the
        // "unless..." portion as a child node.
        let p1 = source_text.find(" unless").map(|i| i + 1).unwrap_or(len / 2);
        nodes.push(IRNode {
            id: NodeId::new("R1"),
            kind: NodeKind::Rule,
            // The engine recognises rule subtypes `definitional/2`
            // and `probabilistic/3`. We use the definitional shape:
            //   definitional(head, [body...])
            // Lists use the Prolog cons form `.( head, tail )` ending
            // in `[]`. Here the head is `delivers(seller, goods)` and
            // the body is `[ payment_within(30_days) ]`.
            term: compound(
                "definitional",
                vec![
                    compound("delivers", vec![atom("seller"), atom("goods")]),
                    compound(
                        ".",
                        vec![
                            compound("payment_within", vec![atom("30_days")]),
                            atom("[]"),
                        ],
                    ),
                ],
            ),
            polarity: Polarity::Affirmed,
            modality: Modality::Conditional,
            // Cover the entire source so the document tiles cleanly
            // even though the Exception lives inside it as a child.
            source_spans: vec![Span::new(doc_id.clone(), 0, len)],
            confidence: 1.0,
            part_of: None,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        });
        nodes.push(IRNode {
            id: NodeId::new("E1"),
            kind: NodeKind::Exception,
            term: compound("out_of_stock", vec![atom("goods")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            // Exception carves the "unless..." portion out of R1.
            // Its span is contained in R1's span (which the IR
            // validator requires).
            source_spans: vec![Span::new(doc_id.clone(), p1, len)],
            confidence: 1.0,
            part_of: Some(NodeId::new("R1")),
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        });
    } else if len > 0 {
        nodes.push(IRNode {
            id: NodeId::new("R1"),
            kind: NodeKind::Rule,
            term: compound("clause", vec![atom("text")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Conditional,
            source_spans: vec![Span::new(doc_id.clone(), 0, len)],
            confidence: 1.0,
            part_of: None,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        });
    }

    nodes.push(IRNode {
        id: NodeId::new("Q1"),
        kind: NodeKind::Query,
        term: compound("delivers", vec![atom("seller"), atom("goods")]),
        polarity: Polarity::Affirmed,
        modality: Modality::Present,
        source_spans: vec![],
        confidence: 1.0,
        part_of: None,
        lowered_from: None,
        discard_reason: None,
        metadata: Default::default(),
    });

    IRDocument {
        document_id: doc_id,
        nodes,
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

pub fn format_side_by_side(raw: &RawArmReport, pipe: &PipelineArmReport) -> String {
    let mut out = String::new();
    out.push_str("============================================================\n");
    out.push_str("  Contract adjudication: raw model vs structured pipeline\n");
    out.push_str("============================================================\n\n");
    out.push_str("--- ARM A: raw model ---\n");
    out.push_str(&format!("model:           {}\n", raw.model));
    out.push_str(&format!(
        "tokens (in/out): {} / {}\n",
        raw.input_tokens, raw.output_tokens
    ));
    out.push_str(&format!("latency:         {} ms\n", raw.latency_ms));
    out.push_str("answer:\n");
    for line in raw.answer.lines() {
        out.push_str(&format!("  {line}\n"));
    }
    out.push_str("\n--- ARM B: structured pipeline ---\n");
    out.push_str(&format!("verdict:         {}\n", pipe.verdict_summary));
    out.push_str(&format!(
        "ADJ02 coverage:           {}\n",
        format_outcome(&pipe.adj02_outcome)
    ));
    out.push_str(&format!(
        "ADJ03 polarity/modality:  {}\n",
        format_outcome(&pipe.adj03_outcome)
    ));
    out.push_str(&format!(
        "ADJ04 round-trip:         {}\n",
        format_outcome(&pipe.adj04_outcome)
    ));
    out.push_str(&format!(
        "ADJ05 adversarial:        {}\n",
        format_outcome(&pipe.adj05_outcome)
    ));
    out.push_str(&format!("engine ran:               {}\n", pipe.engine_ran));
    out
}

fn format_outcome(o: &PassOutcome) -> &'static str {
    match o {
        PassOutcome::Passed => "Passed",
        PassOutcome::Failed => "Failed",
        PassOutcome::Skipped => "Skipped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_source_yields_rule_exception_query() {
        let ir = contract_ir_document(CANONICAL_SOURCE);
        assert_eq!(ir.nodes.len(), 3);
        assert_eq!(ir.nodes[0].kind, NodeKind::Rule);
        assert_eq!(ir.nodes[1].kind, NodeKind::Exception);
        assert_eq!(ir.nodes[2].kind, NodeKind::Query);
    }

    #[test]
    fn rule_is_conditional_modality() {
        let ir = contract_ir_document(CANONICAL_SOURCE);
        assert_eq!(ir.nodes[0].modality, Modality::Conditional);
    }

    #[test]
    fn exception_references_rule_via_part_of() {
        let ir = contract_ir_document(CANONICAL_SOURCE);
        let exception = &ir.nodes[1];
        assert_eq!(
            exception.part_of.as_ref().map(|n| n.0.as_str()),
            Some("R1")
        );
    }

    #[test]
    fn root_node_tiles_the_document() {
        // ADJ02 v2 requires root nodes (part_of=None) to tile the
        // document. R1 is the only root in this fixture; E1 is a
        // child of R1. R1 must cover [0, len) exactly.
        let ir = contract_ir_document(CANONICAL_SOURCE);
        let r1 = &ir.nodes[0];
        assert_eq!(r1.id.0, "R1");
        assert!(r1.part_of.is_none(), "R1 must be a root");
        assert_eq!(r1.source_spans.len(), 1);
        assert_eq!(r1.source_spans[0].start, 0);
        assert_eq!(r1.source_spans[0].end, CANONICAL_SOURCE.len());
    }

    #[test]
    fn exception_span_is_contained_within_rule_span() {
        let ir = contract_ir_document(CANONICAL_SOURCE);
        let r1 = &ir.nodes[0];
        let e1 = &ir.nodes[1];
        let r1s = &r1.source_spans[0];
        let e1s = &e1.source_spans[0];
        assert!(r1s.start <= e1s.start);
        assert!(e1s.end <= r1s.end);
    }

    #[test]
    fn non_canonical_text_falls_back_to_single_rule_plus_query() {
        let ir = contract_ir_document("some other clause text");
        assert_eq!(ir.nodes.len(), 2);
        assert_eq!(ir.nodes[0].kind, NodeKind::Rule);
        assert_eq!(ir.nodes[1].kind, NodeKind::Query);
    }

    #[test]
    fn empty_source_yields_query_only() {
        let ir = contract_ir_document("");
        assert_eq!(ir.nodes.len(), 1);
        assert_eq!(ir.nodes[0].kind, NodeKind::Query);
    }

    #[test]
    fn default_config_uses_canonical_source_text() {
        let cfg = DemoConfig::default();
        assert!(cfg.source_text.contains("buyer pays"));
        assert!(cfg.source_text.contains("unless"));
    }
}
