//! # adjudication-clinical-demo — clinical-note variant of the A/B demo
//!
//! Same shape as `adjudication-tsa-demo`, different domain. Source
//! text is a short patient assessment; the structured pipeline
//! decomposes it into per-symptom IR facts, then runs the full
//! ADJ02 + ADJ03 + ADJ04 + ADJ05 checker chain on the result.
//!
//! ## Why a second demo
//!
//! The framework's design principle is "the IR grammar + checkers
//! generalize across domains; only the prompts change." This crate
//! exists to make that claim verifiable: same `decompose_text`, same
//! `check_coverage`, same `check_propagation`, same `check_round_trip`,
//! same engine — different source text + different domain hint.
//!
//! v0.1 ships a 96-byte canonical fixture:
//!
//! ```text
//! Patient: shortness of breath, mild fever, no known drug allergy.
//! ```
//!
//! The IR's expected shape:
//!
//! - F1: `symptom(shortness_of_breath)` over `"shortness of breath"`.
//! - F2: `symptom(fever, mild)` over `"mild fever"`.
//! - F3: `denied(drug_allergy)` (polarity = Denied) over
//!   `"no known drug allergy"`.
//! - Q1: `safe_to_discharge(patient)?` — the query.
//!
//! ADJ02 will catch any coverage gap. ADJ03 will catch a mis-polarity
//! (e.g., recording `allergy` as Affirmed instead of Denied — a real
//! failure mode for small models). ADJ04 catches drift between IR
//! and source. ADJ05 attacks the IR with a different model.

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
// Configuration
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

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".into(),
            model: "gemma4:latest".into(),
            adversary_model: None,
            timeout: Duration::from_secs(120),
            source_text:
                "Patient: shortness of breath, mild fever, no known drug allergy.".into(),
            cache_dir: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Arm A — raw model
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
        "ASSESSMENT: {src}\n\nIs this patient safe to discharge?",
        src = cfg.source_text
    );
    let req = CompletionRequest {
        model: cfg.model.clone(),
        system: Some(
            "You are a primary-care triage assistant. Given a short patient \
             assessment, decide whether the patient is safe to discharge. \
             Explain in 2-3 sentences, then end with a final line: \
             `VERDICT: SAFE_TO_DISCHARGE` or `VERDICT: KEEP_FOR_OBSERVATION`."
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
// Arm B — structured pipeline (hand-built IR for v0.1)
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
            id: "clinical-demo-001".into(),
            name: "patient_assessment".into(),
            received_at: "2026-05-12T00:00:00Z".into(),
            normalized_text: cfg.source_text.clone(),
            normalization_pipeline: "plain-text-v1".into(),
            normalization_version: "1.0.0".into(),
        },
        ir_document: clinical_ir_document(&cfg.source_text),
    };
    let tick = std::cell::Cell::new(0u32);
    let now = move || {
        let t = tick.get();
        tick.set(t + 1);
        format!("2026-05-12T00:00:{:02}Z", t.min(59))
    };
    let output = run_with_gateway(
        input,
        AdjudicationId::new("adj-clinical-demo"),
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

/// Cache-wrap an LlmClient if configured. Same helper as the TSA
/// demo, copied here so this crate is self-contained.
fn wrap_with_cache(inner: Box<dyn LlmClient>, cfg: &DemoConfig) -> Box<dyn LlmClient> {
    match &cfg.cache_dir {
        Some(dir) => Box::new(CachingClient::with_disk_persistence(inner, dir)),
        None => Box::new(CachingClient::new(inner)),
    }
}

// ---------------------------------------------------------------------------
// Hand-built clinical IR fixture
// ---------------------------------------------------------------------------

/// Build the canonical clinical IR over the default source text.
/// The fixture tiles the document with three Facts (matching the
/// three clinical claims) plus one Query:
///
/// - F1: `symptom(shortness_of_breath)`, Affirmed, Present.
/// - F2: `symptom(fever, mild)`, Affirmed, Present.
/// - F3: `drug_allergy(unknown)`, Denied, Present — captures the
///   "no known drug allergy" phrasing.
/// - Q1: `safe_to_discharge(patient)?`.
///
/// For non-default text, falls back to a single Fact spanning the
/// whole document plus the Query (same pattern as the TSA demo).
pub fn clinical_ir_document(source_text: &str) -> IRDocument {
    let doc_id = DocumentId::new("clinical-demo-001");
    let len = source_text.len();
    let mut nodes = Vec::new();

    let canonical =
        "Patient: shortness of breath, mild fever, no known drug allergy.";
    if source_text == canonical {
        // Spans:
        //   "Patient: shortness of breath, " → 0..30
        //   "mild fever, "                   → 30..42
        //   "no known drug allergy."         → 42..64
        nodes.push(IRNode {
            id: NodeId::new("F1"),
            kind: NodeKind::Fact,
            term: compound("symptom", vec![atom("shortness_of_breath")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![Span::new(doc_id.clone(), 0, 30)],
            confidence: 1.0,
            part_of: None,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        });
        nodes.push(IRNode {
            id: NodeId::new("F2"),
            kind: NodeKind::Fact,
            term: compound("symptom", vec![atom("fever"), atom("mild")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![Span::new(doc_id.clone(), 30, 42)],
            confidence: 1.0,
            part_of: None,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        });
        nodes.push(IRNode {
            id: NodeId::new("F3"),
            kind: NodeKind::Fact,
            term: compound("drug_allergy", vec![atom("unknown")]),
            polarity: Polarity::Denied,
            modality: Modality::Present,
            source_spans: vec![Span::new(doc_id.clone(), 42, 64)],
            confidence: 1.0,
            part_of: None,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        });
    } else if len > 0 {
        nodes.push(IRNode {
            id: NodeId::new("F1"),
            kind: NodeKind::Fact,
            term: compound("assessment", vec![atom("text")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
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
        term: compound("safe_to_discharge", vec![atom("patient")]),
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
// Side-by-side report
// ---------------------------------------------------------------------------

pub fn format_side_by_side(raw: &RawArmReport, pipe: &PipelineArmReport) -> String {
    let mut out = String::new();
    out.push_str("============================================================\n");
    out.push_str("  Clinical adjudication: raw model vs structured pipeline\n");
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_source_yields_three_facts_plus_query() {
        let canonical =
            "Patient: shortness of breath, mild fever, no known drug allergy.";
        let ir = clinical_ir_document(canonical);
        assert_eq!(ir.nodes.len(), 4);
        assert!(matches!(ir.nodes[0].kind, NodeKind::Fact));
        assert!(matches!(ir.nodes[1].kind, NodeKind::Fact));
        assert!(matches!(ir.nodes[2].kind, NodeKind::Fact));
        assert!(matches!(ir.nodes[3].kind, NodeKind::Query));
    }

    #[test]
    fn allergy_fact_polarity_is_denied() {
        let canonical =
            "Patient: shortness of breath, mild fever, no known drug allergy.";
        let ir = clinical_ir_document(canonical);
        let allergy = &ir.nodes[2];
        assert_eq!(allergy.id.0, "F3");
        assert_eq!(allergy.polarity, Polarity::Denied);
    }

    #[test]
    fn spans_tile_the_canonical_source() {
        let canonical =
            "Patient: shortness of breath, mild fever, no known drug allergy.";
        let ir = clinical_ir_document(canonical);
        // F1 + F2 + F3 should cover [0, len(canonical_bytes)).
        let total_bytes = canonical.len();
        let mut covered = vec![false; total_bytes];
        for n in &ir.nodes {
            for span in &n.source_spans {
                for i in span.start..span.end {
                    covered[i] = true;
                }
            }
        }
        for (i, hit) in covered.iter().enumerate() {
            assert!(*hit, "byte {i} not covered by any IR span");
        }
    }

    #[test]
    fn non_canonical_text_falls_back_to_single_fact_plus_query() {
        let ir = clinical_ir_document("some other clinical note");
        assert_eq!(ir.nodes.len(), 2);
        assert!(matches!(ir.nodes[0].kind, NodeKind::Fact));
        assert!(matches!(ir.nodes[1].kind, NodeKind::Query));
    }

    #[test]
    fn empty_source_yields_query_only() {
        let ir = clinical_ir_document("");
        assert_eq!(ir.nodes.len(), 1);
        assert_eq!(ir.nodes[0].kind, NodeKind::Query);
    }

    #[test]
    fn default_config_uses_canonical_source_text() {
        let cfg = DemoConfig::default();
        assert!(cfg.source_text.contains("shortness of breath"));
        assert!(cfg.source_text.contains("no known drug allergy"));
    }

    #[test]
    fn outcome_formatter_handles_all_variants() {
        assert_eq!(format_outcome(&PassOutcome::Passed), "Passed");
        assert_eq!(format_outcome(&PassOutcome::Failed), "Failed");
        assert_eq!(format_outcome(&PassOutcome::Skipped), "Skipped");
    }
}
