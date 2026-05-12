//! # adjudication-tsa-demo — the A/B comparison harness
//!
//! Two arms, one source-text input, one side-by-side report.
//!
//! - **Arm A (raw)**: ask the model directly. "Is the passenger
//!   TSA-compliant? Source: <text>. Answer." We get whatever the
//!   model decides to say.
//!
//! - **Arm B (pipeline)**: build the TSA fixture IR (same shape the
//!   ADJ10 integration test uses), feed it through
//!   [`adjudication_pipeline::run_with_gateway`] with a `Renderer` +
//!   `Nli` gateway, and surface the structured verdict + audit trail.
//!
//! The harness is split into a library (for tests + reuse) and a
//! binary (`cargo run -p adjudication-tsa-demo`) that prints a
//! human-friendly side-by-side. The binary is the entry point the
//! user actually invokes; the library exists so the integration
//! tests can exercise the same wire-up without re-implementing it.
//!
//! ## Why this lives in its own crate
//!
//! - It depends on `llm-provider-ollama` to talk to a real local
//!   model, which `adjudication-pipeline` itself must not (the
//!   pipeline is provider-agnostic).
//! - The binary needs `main.rs`; the library code is a natural fit
//!   for sharing with future demos (e.g., a clinical-note variant).
//!
//! ## What the demo does NOT cover (yet)
//!
//! - **ADJ05 adversary** stays Skipped — a future demo will install a
//!   second model family (e.g., `llama3.1:8b`) as `Role::Adversary`
//!   and let the adversarial check flip from Skipped to Passed/Failed.
//! - **`decompose_text`** isn't called — the demo uses a hand-built
//!   IR because `adjudication-ir` does not yet derive
//!   `serde::Deserialize`. Once it does, the demo can replace the
//!   hand-built IR with a real `decompose_text` call and prove the
//!   end-to-end LLM-IR-engine loop runs against the local model.

use std::time::Duration;

use adjudication_audit_trail::{AdjudicationId, PassOutcome};
use adjudication_ir::{
    DocumentId, IRDocument, IRNode, Modality, NodeId, NodeKind, Polarity, Span,
};
use adjudication_pipeline::{
    run_with_gateway, PipelineDocument, PipelineInput, PipelineOutput, Verdict,
};
use llm_gateway::{
    CompletionRequest, FinishReason, LlmClient, LlmError, Message, MessageContent, Role as MsgRole,
};
use llm_primitives::{GatewayConfig, Role as PrimitiveRole};
use llm_provider_ollama::OllamaClient;
use logic_core::{atom, compound};

// ---------------------------------------------------------------------------
// Demo configuration
// ---------------------------------------------------------------------------

/// Inputs to the demo. The defaults match what most local Ollama
/// installs look like (`http://localhost:11434`, a single Gemma
/// model), so callers usually only need to override the model name.
#[derive(Debug, Clone)]
pub struct DemoConfig {
    pub endpoint: String,
    /// Model to use for both arms — Arm A's raw call AND Arm B's
    /// `Renderer` + `Nli` roles. ADJ05 is intentionally skipped so
    /// the single-model setup doesn't trip the independence check.
    pub model: String,
    /// Wall-clock cap per HTTP call. Ollama responses on commodity
    /// hardware can take 10–60s; 120s is the default in the provider.
    pub timeout: Duration,
    /// The TSA source text. Defaults to the ADJ10 fixture.
    pub source_text: String,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".into(),
            model: "gemma4:latest".into(),
            timeout: Duration::from_secs(120),
            source_text: "1 carry-on bag, matches.".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Arm A — raw model
// ---------------------------------------------------------------------------

/// Outcome of Arm A. The model returns free-form text; we hand it
/// back verbatim plus a sidecar telemetry struct so the binary can
/// print it.
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

/// Ask the model directly. The prompt is intentionally minimal so
/// the model's unaided judgement is what shows up in the answer.
pub fn run_raw_arm(cfg: &DemoConfig) -> Result<RawArmReport, LlmError> {
    let client = OllamaClient::new(cfg.model.clone())
        .with_endpoint(cfg.endpoint.clone())
        .with_timeout(cfg.timeout);

    let prompt = build_raw_prompt(&cfg.source_text);
    let req = CompletionRequest {
        model: cfg.model.clone(),
        system: Some(
            "You are a TSA compliance officer. Given a passenger \
             declaration, decide whether the passenger is compliant \
             with TSA carry-on rules. Explain your reasoning in 2-3 \
             sentences, then end with a final line: `VERDICT: \
             COMPLIANT` or `VERDICT: NON-COMPLIANT`."
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

fn build_raw_prompt(source_text: &str) -> String {
    format!(
        "DECLARATION: {source_text}\n\nIs the passenger TSA-compliant?",
        source_text = source_text,
    )
}

// ---------------------------------------------------------------------------
// Arm B — structured pipeline
// ---------------------------------------------------------------------------

/// Outcome of Arm B. We surface both the pipeline `Verdict` and a
/// short text summary so the side-by-side print stays readable.
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

/// Run the pipeline arm. The same source text Arm A saw is mapped
/// onto a hand-built TSA IR (see [`tsa_ir_document`]) and fed through
/// [`adjudication_pipeline::run_with_gateway`] with the Ollama
/// instance registered for both `Renderer` and `Nli`.
///
/// **ADJ04 caveat**: a single-model gateway is fine for `Renderer`
/// (rendering is faithful paraphrase, no self-confirmation hazard)
/// but is theoretically loose for `Nli` (the entail check is asking
/// the model to grade its own renderer). The demo runs it anyway —
/// the framework's job is to make this explicit, and the audit
/// trail records both roles' provider identity so a reviewer sees
/// the configuration.
pub fn run_pipeline_arm(cfg: &DemoConfig) -> PipelineArmReport {
    let client = OllamaClient::new(cfg.model.clone())
        .with_endpoint(cfg.endpoint.clone())
        .with_timeout(cfg.timeout);
    // GatewayConfig needs Box<dyn LlmClient>; we register two clones
    // of the same client (one per role) so each call site uses its
    // own connection.
    let renderer = Box::new(client.clone()) as Box<dyn LlmClient>;
    let nli = Box::new(client.clone()) as Box<dyn LlmClient>;
    let gateway = GatewayConfig::new()
        .with_client(PrimitiveRole::Renderer, renderer)
        .with_client(PrimitiveRole::Nli, nli);

    let input = PipelineInput {
        document: PipelineDocument {
            id: "tsa-demo-001".into(),
            name: "tsa_declaration".into(),
            received_at: "2026-05-11T00:00:00Z".into(),
            normalized_text: cfg.source_text.clone(),
            normalization_pipeline: "plain-text-v1".into(),
            normalization_version: "1.0.0".into(),
        },
        ir_document: tsa_ir_document(&cfg.source_text),
    };

    let tick = std::cell::Cell::new(0u32);
    let now = move || {
        let t = tick.get();
        tick.set(t + 1);
        format!("2026-05-11T00:00:{:02}Z", t.min(59))
    };

    let output = run_with_gateway(input, AdjudicationId::new("adj-tsa-demo"), now, Some(&gateway));

    let trail = &output.audit_trail;
    let summary = match &output.verdict {
        Verdict::Resolved { answers } => format!(
            "Resolved with {n} engine answer(s); ADJ02 + ADJ03 passed; ADJ04 + ADJ05 reported in audit trail",
            n = answers.len()
        ),
        Verdict::Blocked { violation_count } => {
            format!("Blocked with {violation_count} violation(s); engine did not run")
        }
        Verdict::EngineError(detail) => format!("EngineError: {detail}"),
    };

    PipelineArmReport {
        verdict_summary: summary,
        adj02_outcome: trail.checker_results[0].outcome.clone(),
        adj03_outcome: trail.checker_results[1].outcome.clone(),
        adj04_outcome: trail.checker_results[2].outcome.clone(),
        adj05_outcome: trail.checker_results[3].outcome.clone(),
        engine_ran: trail.engine_artifacts.is_some(),
        pipeline_output: output,
    }
}

// ---------------------------------------------------------------------------
// TSA fixture — same shape as the ADJ10 integration test
// ---------------------------------------------------------------------------

/// Build a TSA-style IR over the demo source text.
///
/// The default text `"1 carry-on bag, matches."` (24 bytes) gets two
/// `Fact` nodes tiling the document:
///
/// - F1: `carry_on(1)` — spans 0..16 (`"1 carry-on bag, "`)
/// - F2: `prohibited(matches)` — spans 16..24 (`"matches."`)
/// - Q1: `compliant(passenger_a)` — no source spans (synthesized
///   query; coverage allows zero-span queries).
///
/// If the caller passes a different source text, the function falls
/// back to a single `Fact` node spanning the whole document plus the
/// query node. That keeps the demo "interesting" for non-default
/// inputs but won't match the cleanly-tiled ADJ10 fixture exactly.
pub fn tsa_ir_document(source_text: &str) -> IRDocument {
    let doc_id = DocumentId::new("tsa-demo-001");
    let len = source_text.len();

    let mut nodes = Vec::new();

    if source_text == "1 carry-on bag, matches." {
        nodes.push(IRNode {
            id: NodeId::new("F1"),
            kind: NodeKind::Fact,
            term: compound("carry_on", vec![atom("1")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![Span::new(doc_id.clone(), 0, 16)],
            confidence: 1.0,
            part_of: None,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        });
        nodes.push(IRNode {
            id: NodeId::new("F2"),
            kind: NodeKind::Fact,
            term: compound("prohibited", vec![atom("matches")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![Span::new(doc_id.clone(), 16, 24)],
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
            term: compound("declaration", vec![atom("text")]),
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
        term: compound("compliant", vec![atom("passenger_a")]),
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

/// Render the two arms side-by-side as a single multi-line string
/// the binary prints to stdout.
pub fn format_side_by_side(raw: &RawArmReport, pipeline: &PipelineArmReport) -> String {
    let mut out = String::new();
    out.push_str("============================================================\n");
    out.push_str("  TSA adjudication: raw model vs structured pipeline\n");
    out.push_str("============================================================\n");
    out.push_str("\n");
    out.push_str("--- ARM A: raw model ---\n");
    out.push_str(&format!("model:           {}\n", raw.model));
    out.push_str(&format!(
        "tokens (in/out): {} / {}\n",
        raw.input_tokens, raw.output_tokens
    ));
    out.push_str(&format!("latency:         {} ms\n", raw.latency_ms));
    out.push_str(&format!("finish reason:   {:?}\n", raw.finish_reason));
    out.push_str("answer:\n");
    for line in raw.answer.lines() {
        out.push_str(&format!("  {line}\n"));
    }
    out.push_str("\n");
    out.push_str("--- ARM B: structured pipeline ---\n");
    out.push_str(&format!("verdict:         {}\n", pipeline.verdict_summary));
    out.push_str(&format!(
        "ADJ02 coverage:           {}\n",
        format_outcome(&pipeline.adj02_outcome)
    ));
    out.push_str(&format!(
        "ADJ03 polarity/modality:  {}\n",
        format_outcome(&pipeline.adj03_outcome)
    ));
    out.push_str(&format!(
        "ADJ04 round-trip:         {}\n",
        format_outcome(&pipeline.adj04_outcome)
    ));
    out.push_str(&format!(
        "ADJ05 adversarial:        {}\n",
        format_outcome(&pipeline.adj05_outcome)
    ));
    out.push_str(&format!("engine ran:               {}\n", pipeline.engine_ran));
    if let Some(art) = &pipeline.pipeline_output.audit_trail.engine_artifacts {
        out.push_str(&format!("engine version:           {}\n", art.engine_version));
    }
    out.push_str("\n");
    out.push_str(
        "Note: ADJ05 is Skipped because the demo uses one model family \
         for both Renderer and Nli; installing a second model (e.g., \
         `ollama pull llama3.1:8b`) and registering it as Role::Adversary \
         flips ADJ05 from Skipped to Passed/Failed.\n",
    );
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
// Tests (offline-only — live Ollama test lives in tests/integration_ollama.rs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_source_text_yields_two_fact_nodes_plus_query() {
        let ir = tsa_ir_document("1 carry-on bag, matches.");
        assert_eq!(ir.nodes.len(), 3);
        assert_eq!(ir.nodes[0].id.0, "F1");
        assert_eq!(ir.nodes[1].id.0, "F2");
        assert_eq!(ir.nodes[2].id.0, "Q1");
    }

    #[test]
    fn non_default_source_text_yields_single_fact_plus_query() {
        let ir = tsa_ir_document("some other text");
        assert_eq!(ir.nodes.len(), 2);
        assert_eq!(ir.nodes[0].kind, NodeKind::Fact);
        assert_eq!(ir.nodes[1].kind, NodeKind::Query);
    }

    #[test]
    fn empty_source_yields_only_query_node() {
        let ir = tsa_ir_document("");
        assert_eq!(ir.nodes.len(), 1);
        assert_eq!(ir.nodes[0].kind, NodeKind::Query);
    }

    #[test]
    fn raw_prompt_contains_the_source_text() {
        let p = build_raw_prompt("1 carry-on bag, matches.");
        assert!(p.contains("1 carry-on bag, matches."));
        assert!(p.contains("TSA-compliant"));
    }

    #[test]
    fn default_config_targets_localhost_ollama() {
        let cfg = DemoConfig::default();
        assert!(cfg.endpoint.contains("11434"));
        assert!(cfg.source_text.contains("carry-on"));
    }

    #[test]
    fn outcome_formatter_renders_each_variant() {
        assert_eq!(format_outcome(&PassOutcome::Passed), "Passed");
        assert_eq!(format_outcome(&PassOutcome::Failed), "Failed");
        assert_eq!(format_outcome(&PassOutcome::Skipped), "Skipped");
    }
}
