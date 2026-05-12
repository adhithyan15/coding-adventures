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
//! ## Two pipeline modes
//!
//! Arm B has two sub-modes, selected by `DemoConfig::ir_mode`:
//!
//! - **`IrMode::HandBuilt`** (default): the demo constructs the TSA
//!   fixture IR programmatically (same shape as the ADJ10 integration
//!   test). Useful as a clean baseline that proves the pipeline
//!   machinery itself works, independent of how good the model is at
//!   extraction.
//!
//! - **`IrMode::LlmExtracted`**: the demo calls
//!   `llm_primitives::decompose_text` to ask the model to produce the
//!   IR, then converts the JSON output into a typed `IRDocument` via a
//!   tolerant parser. This is the *full* LLM-driven flow: extraction +
//!   coverage + propagation + round-trip + engine. ADJ02/ADJ03/ADJ04
//!   surface any mistakes the model made during extraction.
//!
//! Both modes use the same `Renderer` + `Nli` + (in LlmExtracted mode)
//! `Extractor` clients.
//!
//! ## What the demo does NOT cover (yet)
//!
//! - **ADJ05 adversary** stays Skipped — a future demo will install a
//!   second model family (e.g., `llama3.1:8b`) as `Role::Adversary`
//!   and let the adversarial check flip from Skipped to Passed/Failed.

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
use adjudication_clarification::{
    retry_decompose_on_coverage_failure, ClarificationError, CoverageClarificationRequest,
};
use llm_primitives::{
    decompose_text, DecomposeTextRequest, GatewayConfig, PrimitiveError, Role as PrimitiveRole,
};
use llm_provider_ollama::OllamaClient;
use logic_core::{atom, compound, Term};

// ---------------------------------------------------------------------------
// Demo configuration
// ---------------------------------------------------------------------------

/// How Arm B's IR is constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrMode {
    /// Build the canonical TSA fixture IR in code (no LLM involvement
    /// in the extraction step). This is the "clean baseline" — proves
    /// the pipeline machinery works.
    HandBuilt,
    /// Call `llm_primitives::decompose_text` to ask the model to
    /// produce the IR, then convert its JSON output into a typed
    /// `IRDocument`. This is the full LLM-driven flow.
    LlmExtracted,
}

/// Inputs to the demo. The defaults match what most local Ollama
/// installs look like (`http://localhost:11434`, with two pulled
/// models for Extractor/Renderer/Nli vs Adversary), so callers
/// usually only need to override one of them.
#[derive(Debug, Clone)]
pub struct DemoConfig {
    pub endpoint: String,
    /// Primary model — Arm A's raw call AND Arm B's `Extractor` /
    /// `Renderer` / `Nli` roles.
    pub model: String,
    /// Adversary model — used as Arm B's `Role::Adversary`. MUST be
    /// from a different `(vendor, model_family)` than `model` for
    /// ADJ05 independence; the framework's
    /// `GatewayConfig::check_independence` enforces this and the
    /// pipeline skips ADJ05 with a structured reason if it fails.
    ///
    /// When `None`, ADJ05 records as Skipped — which is fine for the
    /// quick demo. To turn it on, set `Some("llama3.1:8b")` (assumes
    /// `ollama pull llama3.1:8b`).
    pub adversary_model: Option<String>,
    /// Wall-clock cap per HTTP call. Ollama responses on commodity
    /// hardware can take 10–60s; 120s is the default in the provider.
    pub timeout: Duration,
    /// The TSA source text. Defaults to the ADJ10 fixture.
    pub source_text: String,
    /// How Arm B builds the IR. Defaults to [`IrMode::HandBuilt`] for
    /// the clean baseline; set to [`IrMode::LlmExtracted`] to drive
    /// the full LLM-from-source flow.
    pub ir_mode: IrMode,
    /// Maximum number of ADJ06 clarification rounds when the model's
    /// first IR fails ADJ02 coverage. `0` disables clarification
    /// (Blocked verdict on first failure). Default `2` — usually
    /// enough for a small model to self-correct.
    pub max_clarification_attempts: usize,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".into(),
            model: "gemma4:latest".into(),
            adversary_model: None,
            timeout: Duration::from_secs(120),
            source_text: "1 carry-on bag, matches.".into(),
            ir_mode: IrMode::HandBuilt,
            max_clarification_attempts: 2,
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
    /// Human-readable description of every ADJ04 round-trip
    /// violation — one entry per IR leaf where the model's
    /// rendering didn't match the source. Empty when ADJ04
    /// passed or was skipped.
    pub adj04_drift_findings: Vec<Adj04DriftFinding>,
    /// Human-readable description of every ADJ05 adversarial
    /// violation — one entry per IR leaf where a *different model*
    /// produced a plausible contradicting reading. Empty when ADJ05
    /// passed or was skipped.
    pub adj05_adversarial_findings: Vec<Adj05AdversarialFinding>,
    /// Records how Arm B's IR was built. For LlmExtracted mode this
    /// includes the model's raw JSON so the report shows exactly
    /// what the model produced.
    pub ir_source: IrSourceTelemetry,
}

/// One ADJ04 drift finding pulled out of the audit trail. Mirrors
/// the `RoundTripDrift` violation but as plain strings + scores so
/// the binary can print them cleanly without re-deriving JSON.
#[derive(Debug, Clone)]
pub struct Adj04DriftFinding {
    pub node_id: String,
    pub source_excerpt: String,
    pub model_rendering: String,
    pub source_to_rendering_score: f32,
    pub rendering_to_source_score: f32,
    pub threshold: f32,
}

/// One ADJ05 adversarial finding. The adversary model produced a
/// plausible *alternative* reading of the same source span — meaning
/// the IR's interpretation isn't the only defensible one.
#[derive(Debug, Clone)]
pub struct Adj05AdversarialFinding {
    pub node_id: String,
    pub ir_rendered: String,
    pub adversary_reading: String,
    pub adversary_explanation: String,
    pub judge_reason: String,
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
    let primary = OllamaClient::new(cfg.model.clone())
        .with_endpoint(cfg.endpoint.clone())
        .with_timeout(cfg.timeout);
    // GatewayConfig needs Box<dyn LlmClient>; we register clones of
    // the primary client for Extractor/Renderer/Nli/Plausibility so
    // each call site uses its own connection.
    let extractor = Box::new(primary.clone()) as Box<dyn LlmClient>;
    let renderer = Box::new(primary.clone()) as Box<dyn LlmClient>;
    let nli = Box::new(primary.clone()) as Box<dyn LlmClient>;
    let plausibility = Box::new(primary.clone()) as Box<dyn LlmClient>;
    let mut gateway = GatewayConfig::new()
        .with_client(PrimitiveRole::Extractor, extractor)
        .with_client(PrimitiveRole::Renderer, renderer)
        .with_client(PrimitiveRole::Nli, nli)
        .with_client(PrimitiveRole::Plausibility, plausibility);

    // ADJ05 adversary: a SECOND model from a different family. The
    // framework's `check_independence` enforces this and skips the
    // check with a typed reason if it sees the same family.
    if let Some(adv_model) = &cfg.adversary_model {
        let adv_client = OllamaClient::new(adv_model.clone())
            .with_endpoint(cfg.endpoint.clone())
            .with_timeout(cfg.timeout);
        gateway = gateway.with_client(
            PrimitiveRole::Adversary,
            Box::new(adv_client) as Box<dyn LlmClient>,
        );
    }

    // Build the IR according to the configured mode. The
    // `IrSourceTelemetry` captures what the model produced (if any)
    // so the report can surface it.
    let (mut ir_document, mut ir_source_telemetry) =
        build_ir(cfg, &gateway);

    let make_now = || {
        let tick = std::cell::Cell::new(0u32);
        move || {
            let t = tick.get();
            tick.set(t + 1);
            format!("2026-05-11T00:00:{:02}Z", t.min(59))
        }
    };

    // Initial pipeline run.
    let input = PipelineInput {
        document: PipelineDocument {
            id: "tsa-demo-001".into(),
            name: "tsa_declaration".into(),
            received_at: "2026-05-11T00:00:00Z".into(),
            normalized_text: cfg.source_text.clone(),
            normalization_pipeline: "plain-text-v1".into(),
            normalization_version: "1.0.0".into(),
        },
        ir_document: ir_document.clone(),
    };
    let mut output =
        run_with_gateway(input, AdjudicationId::new("adj-tsa-demo"), make_now(), Some(&gateway));

    // ADJ06 clarification loop: only fires in LlmExtracted mode when
    // ADJ02 failed AND we have budget left. After each round we
    // re-run the entire pipeline so ADJ02 + ADJ03 + ADJ04 + ADJ05 +
    // engine all see the corrected IR.
    if matches!(cfg.ir_mode, IrMode::LlmExtracted) && cfg.max_clarification_attempts > 0 {
        let mut clarification_turns: Vec<adjudication_audit_trail::DialogueTurn> = Vec::new();
        for attempt in 1..=cfg.max_clarification_attempts {
            if matches!(
                output.audit_trail.checker_results[0].outcome,
                PassOutcome::Passed
            ) {
                break;
            }
            let violation_description = format_first_adj02_violation(
                &output.audit_trail.checker_results[0],
            );
            // Use the previous IR JSON we have on hand from
            // `ir_source_telemetry` if available; otherwise serialize
            // a best-effort placeholder.
            let previous_ir_json = previous_ir_for_clarification(&ir_source_telemetry);
            let clar_req = CoverageClarificationRequest {
                original: DecomposeTextRequest {
                    document_id: "tsa-demo-001".into(),
                    source_text: cfg.source_text.clone(),
                    domain_hint: "tsa-declaration".into(),
                    language_hint: Some("en".into()),
                },
                violation_description,
                previous_ir: previous_ir_json,
            };
            match retry_decompose_on_coverage_failure(&clar_req, &gateway, 1, make_now()) {
                Ok(out) => {
                    clarification_turns.extend(out.dialogue);
                    match json_to_ir_document(
                        &out.corrected_ir,
                        "tsa-demo-001",
                        &cfg.source_text,
                    ) {
                        Ok((new_ir, mut new_warnings)) => {
                            // Update the telemetry so the report
                            // surfaces the corrected IR + the
                            // accumulated warnings.
                            if let IrSourceTelemetry::LlmExtracted {
                                ref mut node_count,
                                ref mut raw_ir_json,
                                ref mut converter_warnings,
                                ..
                            } = ir_source_telemetry
                            {
                                *node_count = new_ir.nodes.len();
                                *raw_ir_json = serde_json::to_string_pretty(&out.corrected_ir)
                                    .unwrap_or_else(|_| out.corrected_ir.to_string());
                                converter_warnings.append(&mut new_warnings);
                            }
                            ir_document = new_ir.clone();
                            let retry_input = PipelineInput {
                                document: PipelineDocument {
                                    id: "tsa-demo-001".into(),
                                    name: "tsa_declaration".into(),
                                    received_at: "2026-05-11T00:00:00Z".into(),
                                    normalized_text: cfg.source_text.clone(),
                                    normalization_pipeline: "plain-text-v1".into(),
                                    normalization_version: "1.0.0".into(),
                                },
                                ir_document: ir_document.clone(),
                            };
                            output = run_with_gateway(
                                retry_input,
                                AdjudicationId::new(format!("adj-tsa-demo-r{attempt}")),
                                make_now(),
                                Some(&gateway),
                            );
                        }
                        Err(e) => {
                            // Corrected IR was unparseable; give up and
                            // keep the previous pipeline output.
                            clarification_turns
                                .last_mut()
                                .map(|t| t.outcome = adjudication_audit_trail::DialogueOutcome::Abandoned);
                            let _ = e;
                            break;
                        }
                    }
                }
                Err(ClarificationError::Exhausted { dialogue, .. }) => {
                    clarification_turns.extend(dialogue);
                    break;
                }
                Err(ClarificationError::Primitive(_)) => {
                    break;
                }
            }
        }
        // Stitch dialogue into the audit trail + the report telemetry.
        if !clarification_turns.is_empty() {
            let attempts = clarification_turns.len();
            let resolved = matches!(
                output.audit_trail.checker_results[0].outcome,
                PassOutcome::Passed
            );
            let summary = format!(
                "{attempts} clarification round(s) ({})",
                if resolved { "resolved" } else { "exhausted" }
            );
            if let IrSourceTelemetry::LlmExtracted {
                clarification_summary: ref mut cs,
                clarification_turns: ref mut tgt,
                ..
            } = ir_source_telemetry
            {
                *cs = Some(summary);
                *tgt = clarification_turns.clone();
            }
            output.audit_trail.dialogue.extend(clarification_turns);
        }
    }

    let trail = &output.audit_trail;
    let adj04_drift_findings = collect_adj04_drift(&trail.checker_results);
    let adj05_adversarial_findings = collect_adj05_findings(&trail.checker_results);
    let adj04_passed = matches!(trail.checker_results[2].outcome, PassOutcome::Passed);

    let summary = match &output.verdict {
        Verdict::Resolved { answers } if adj04_passed => format!(
            "Resolved with {n} engine answer(s); all gating checks passed; ADJ04 round-trip agreed with the source",
            n = answers.len()
        ),
        Verdict::Resolved { answers } if !adj04_drift_findings.is_empty() => format!(
            "Resolved with {n} engine answer(s), BUT ADJ04 caught {d} round-trip drift(s) — model's IR rendering doesn't faithfully reflect the source. See per-finding detail below.",
            n = answers.len(),
            d = adj04_drift_findings.len(),
        ),
        Verdict::Resolved { answers } => format!(
            "Resolved with {n} engine answer(s); ADJ04 errored before producing a verdict (see audit trail telemetry)",
            n = answers.len()
        ),
        Verdict::Blocked { violation_count } => {
            format!("Blocked with {violation_count} violation(s); engine did not run")
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
        adj04_drift_findings,
        adj05_adversarial_findings,
        ir_source: ir_source_telemetry,
        pipeline_output: output,
    }
}

/// How Arm B's IR was built — recorded so the report can show the
/// LLM's raw JSON output when the extractor was involved.
#[derive(Debug, Clone)]
pub enum IrSourceTelemetry {
    HandBuilt,
    LlmExtracted {
        /// Number of nodes the converter pulled out of the model's
        /// JSON. May be 0 if the LLM didn't produce a `nodes` array.
        node_count: usize,
        /// The model's raw IR JSON (pretty-printed), included so the
        /// report shows exactly what the model said.
        raw_ir_json: String,
        /// Any warnings the converter raised while normalizing the
        /// LLM output (e.g., "node F1 had non-string `kind`; defaulted
        /// to Fact"). Empty when the model produced clean output.
        converter_warnings: Vec<String>,
        /// One-line summary of any ADJ06 clarification dialogue that
        /// happened during IR construction. Empty when the model got
        /// it right on the first try; "1 retry (resolved)" when a
        /// single ADJ06 round-trip corrected an ADJ02 failure;
        /// "2 retries (exhausted)" when the loop gave up.
        clarification_summary: Option<String>,
        /// Detailed clarification turns (audit-trail rows) — surfaced
        /// in the report so reviewers can see what the model was
        /// told and how it responded.
        clarification_turns: Vec<adjudication_audit_trail::DialogueTurn>,
    },
    /// The model's extractor call failed (transport, validation, etc.).
    /// We fall back to the hand-built IR so the pipeline still has
    /// *something* to run on, and the report explains the fallback.
    LlmExtractionFailed {
        error: String,
        fell_back_to_hand_built: bool,
    },
}

fn build_ir(cfg: &DemoConfig, gateway: &GatewayConfig) -> (IRDocument, IrSourceTelemetry) {
    match cfg.ir_mode {
        IrMode::HandBuilt => (tsa_ir_document(&cfg.source_text), IrSourceTelemetry::HandBuilt),
        IrMode::LlmExtracted => match call_decompose(cfg, gateway) {
            Ok((ir, raw_json, warnings)) => {
                let node_count = ir.nodes.len();
                (
                    ir,
                    IrSourceTelemetry::LlmExtracted {
                        node_count,
                        raw_ir_json: raw_json,
                        converter_warnings: warnings,
                        clarification_summary: None,
                        clarification_turns: Vec::new(),
                    },
                )
            }
            Err(e) => (
                tsa_ir_document(&cfg.source_text),
                IrSourceTelemetry::LlmExtractionFailed {
                    error: e,
                    fell_back_to_hand_built: true,
                },
            ),
        },
    }
}

/// Drive `decompose_text` and convert the JSON output into a typed
/// `IRDocument`. Returns `(ir, pretty_raw_json, warnings)` on
/// success.
fn call_decompose(
    cfg: &DemoConfig,
    gateway: &GatewayConfig,
) -> Result<(IRDocument, String, Vec<String>), String> {
    let req = DecomposeTextRequest {
        document_id: "tsa-demo-001".into(),
        source_text: cfg.source_text.clone(),
        domain_hint: "tsa-declaration".into(),
        language_hint: Some("en".into()),
    };
    let resp = decompose_text(&req, gateway).map_err(|e: PrimitiveError| format!("{e}"))?;
    let raw_json = serde_json::to_string_pretty(&resp.ir_document)
        .unwrap_or_else(|_| resp.ir_document.to_string());
    let (ir, warnings) =
        json_to_ir_document(&resp.ir_document, &req.document_id, &cfg.source_text)
            .map_err(|e| format!("JSON-to-IR conversion failed: {e}"))?;
    Ok((ir, raw_json, warnings))
}

// ---------------------------------------------------------------------------
// Tolerant JSON-to-IR converter
// ---------------------------------------------------------------------------

/// Convert the LLM's IR JSON into a typed `IRDocument`. Models
/// produce a range of shapes; the converter is deliberately
/// forgiving:
///
/// - Missing `kind` → defaults to `Fact`.
/// - Missing `polarity` / `modality` → defaults to `Affirmed` /
///   `Present` (the safest neutrals; downstream ADJ03 will flag if
///   the choice was actually meaningful).
/// - Missing `source_spans` → falls back to a single span covering
///   `[0, source.len())` for Fact/Rule/Uncertainty/Exception/Discarded
///   nodes, and to an empty span list for Query (which ADJ02 v2 allows).
/// - Unknown `kind` strings → defaults to `Fact` (so ADJ02 still
///   runs against the text).
/// - Spans with `end > source.len()` → clamped to `source.len()`.
///
/// Every fallback is recorded in `warnings` so the report can surface
/// them. The converter does NOT enforce ADJ01 well-formedness —
/// that's `adjudication-ir::validate`'s job (and ADJ02/ADJ03 enforce
/// the same constraints structurally).
fn json_to_ir_document(
    v: &serde_json::Value,
    expected_doc_id: &str,
    source_text: &str,
) -> Result<(IRDocument, Vec<String>), String> {
    let mut warnings = Vec::new();
    let obj = v
        .as_object()
        .ok_or_else(|| "IR root is not a JSON object".to_string())?;

    let doc_id = obj
        .get("document_id")
        .and_then(|x| x.as_str())
        .unwrap_or_else(|| {
            warnings.push("document_id missing; using fallback".to_string());
            expected_doc_id
        })
        .to_string();
    if doc_id != expected_doc_id {
        warnings.push(format!(
            "document_id mismatch (LLM said {doc_id:?}; expected {expected_doc_id:?})"
        ));
    }
    let document_id = DocumentId::new(doc_id);

    let nodes_arr = obj
        .get("nodes")
        .and_then(|x| x.as_array())
        .ok_or_else(|| "IR has no nodes array".to_string())?;

    let source_len = source_text.len();
    let mut nodes = Vec::new();
    let mut idx_counter = 0usize;
    for node_v in nodes_arr.iter() {
        // Some models (Gemma 4 included) produce a nested tree
        // (`{ "children": [...] }`) rather than the flat list the
        // prompt asks for. Walk the tree and flatten — every leaf
        // becomes an IRNode, parents are dropped because they don't
        // carry atomic claims at this layer.
        flatten_nodes(
            node_v,
            None,
            &document_id,
            source_len,
            &mut idx_counter,
            &mut nodes,
            &mut warnings,
        );
    }

    // Ensure the IR has at least one Query node so the engine has
    // something to do. If the model didn't produce one, synthesize
    // `compliant(passenger_a)?` — same as the hand-built fixture.
    if !nodes.iter().any(|n| matches!(n.kind, NodeKind::Query)) {
        warnings.push(
            "no Query node in LLM IR; synthesizing compliant(passenger_a) so the engine can run"
                .to_string(),
        );
        nodes.push(synth_query_node(&document_id));
    }

    Ok((
        IRDocument {
            document_id,
            nodes,
        },
        warnings,
    ))
}

/// Walk a possibly-nested JSON node tree and emit each leaf (or each
/// node that contains atomic content) as an `IRNode`. Parent
/// `TextRun` nodes are dropped — they carry structural grouping in
/// the model's output but don't represent claims this layer cares
/// about. If a node has no `children` field, it's treated as a leaf
/// itself.
fn flatten_nodes(
    v: &serde_json::Value,
    parent: Option<&NodeId>,
    document_id: &DocumentId,
    source_len: usize,
    idx_counter: &mut usize,
    out: &mut Vec<IRNode>,
    warnings: &mut Vec<String>,
) {
    let obj = match v.as_object() {
        Some(o) => o,
        None => {
            warnings.push("encountered non-object node; skipping".into());
            return;
        }
    };

    let has_children = obj
        .get("children")
        .and_then(|x| x.as_array())
        .is_some_and(|a| !a.is_empty());

    if has_children {
        // We DON'T emit the grouping TextRun parent because the demo's
        // engine only consumes Fact/Query/etc. — and crucially we
        // also DON'T propagate a `part_of` pointer to children that
        // would reference a non-existent node. The flattened children
        // become document roots, and ADJ02 sees the leaves' spans
        // directly. Walk every child as a fresh root.
        for child in obj.get("children").and_then(|x| x.as_array()).unwrap() {
            flatten_nodes(
                child,
                None,
                document_id,
                source_len,
                idx_counter,
                out,
                warnings,
            );
        }
        return;
    }

    // Leaf node — convert it.
    let mut node = match json_to_ir_node(
        v,
        *idx_counter,
        document_id,
        source_len,
        warnings,
    ) {
        Ok(n) => n,
        Err(e) => {
            warnings.push(format!("skipped malformed node: {e}"));
            return;
        }
    };
    *idx_counter += 1;
    if node.part_of.is_none() {
        node.part_of = parent.cloned();
    }
    // Clear any stale part_of pointer; the LLM may have produced one
    // referencing a parent we dropped during flattening. ADJ01 v2
    // requires part_of to point to an existing node — better to
    // drop a stale pointer than leave a dangling reference.
    if let Some(p) = &node.part_of {
        if !out.iter().any(|n| n.id == *p) {
            node.part_of = None;
        }
    }
    out.push(node);
}

fn synth_query_node(document_id: &DocumentId) -> IRNode {
    IRNode {
        id: NodeId::new("Q-synth"),
        kind: NodeKind::Query,
        term: compound("compliant", vec![atom("passenger_a")]),
        polarity: Polarity::Affirmed,
        modality: Modality::Present,
        source_spans: Vec::new(),
        confidence: 1.0,
        part_of: None,
        lowered_from: None,
        discard_reason: None,
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert("synthesized".to_string(), "true".to_string());
            m.insert("document_id".to_string(), document_id.0.clone());
            m
        },
    }
}

fn json_to_ir_node(
    v: &serde_json::Value,
    idx: usize,
    document_id: &DocumentId,
    source_len: usize,
    warnings: &mut Vec<String>,
) -> Result<IRNode, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| format!("node {idx} is not a JSON object"))?;

    let id = obj
        .get("id")
        .and_then(|x| x.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| {
            warnings.push(format!("node[{idx}] missing id; assigning N{idx}"));
            format!("N{idx}")
        });

    // Accept `kind` (per the prompt) OR `node_type` (which Gemma 4
    // tends to emit). Either field with an unknown value defaults
    // to Fact.
    let kind_str = obj
        .get("kind")
        .or_else(|| obj.get("node_type"))
        .and_then(|x| x.as_str());
    let kind = match kind_str {
        Some(s) => match parse_node_kind(s) {
            Some(k) => k,
            None => {
                warnings.push(format!("node {id} has unknown kind {s:?}; defaulting to Fact"));
                NodeKind::Fact
            }
        },
        None => {
            warnings.push(format!("node {id} missing kind; defaulting to Fact"));
            NodeKind::Fact
        }
    };

    let polarity = obj
        .get("polarity")
        .and_then(|x| x.as_str())
        .map(|s| match parse_polarity(s) {
            Some(p) => p,
            None => {
                warnings.push(format!("node {id} unknown polarity {s:?}; defaulting to Affirmed"));
                Polarity::Affirmed
            }
        })
        .unwrap_or(Polarity::Affirmed);

    let modality = obj
        .get("modality")
        .and_then(|x| x.as_str())
        .map(|s| match parse_modality(s) {
            Some(m) => m,
            None => {
                warnings.push(format!("node {id} unknown modality {s:?}; defaulting to Present"));
                Modality::Present
            }
        })
        .unwrap_or(Modality::Present);

    // Accept `term` (per the prompt) OR `text` (Gemma 4's preferred
    // field). For `text` we wrap the string in `claim/1`-style atom
    // so the engine has a deterministic term to work with.
    let term = if let Some(t) = obj.get("term") {
        json_to_term(t, &id, warnings)
    } else if let Some(t) = obj.get("text").and_then(|x| x.as_str()) {
        // The model gave us the raw text of the leaf rather than a
        // structured term. Wrap it so the engine doesn't have to
        // parse it as logic — the audit trail still records the
        // model's literal claim.
        compound("text_claim", vec![atom(t)])
    } else {
        warnings.push(format!("node {id} missing term; using atom \"unknown\""));
        atom("unknown")
    };

    let source_spans = parse_source_spans(
        obj.get("source_spans"),
        document_id,
        source_len,
        &id,
        kind,
        warnings,
    );

    Ok(IRNode {
        id: NodeId::new(id),
        kind,
        term,
        polarity,
        modality,
        source_spans,
        confidence: obj
            .get("confidence")
            .and_then(|x| x.as_f64())
            .unwrap_or(1.0),
        part_of: obj
            .get("part_of")
            .and_then(|x| x.as_str())
            .map(|s| NodeId::new(s.to_string())),
        lowered_from: obj
            .get("lowered_from")
            .and_then(|x| x.as_str())
            .map(|s| NodeId::new(s.to_string())),
        discard_reason: None,
        metadata: Default::default(),
    })
}

fn parse_node_kind(s: &str) -> Option<NodeKind> {
    match s.to_ascii_lowercase().as_str() {
        "textrun" | "text_run" => Some(NodeKind::TextRun),
        "fact" => Some(NodeKind::Fact),
        "query" => Some(NodeKind::Query),
        "uncertainty" => Some(NodeKind::Uncertainty),
        "rule" => Some(NodeKind::Rule),
        "exception" => Some(NodeKind::Exception),
        "discarded" => Some(NodeKind::Discarded),
        _ => None,
    }
}

fn parse_polarity(s: &str) -> Option<Polarity> {
    match s.to_ascii_lowercase().as_str() {
        "affirmed" => Some(Polarity::Affirmed),
        "denied" => Some(Polarity::Denied),
        "uncertain" => Some(Polarity::Uncertain),
        "inherit" => Some(Polarity::Inherit),
        _ => None,
    }
}

fn parse_modality(s: &str) -> Option<Modality> {
    match s.to_ascii_lowercase().as_str() {
        "present" => Some(Modality::Present),
        "past" => Some(Modality::Past),
        "future" => Some(Modality::Future),
        "hypothetical" => Some(Modality::Hypothetical),
        "familyhistory" | "family_history" => Some(Modality::FamilyHistory),
        "ruledout" | "ruled_out" => Some(Modality::RuledOut),
        "conditional" => Some(Modality::Conditional),
        "inherit" => Some(Modality::Inherit),
        _ => None,
    }
}

/// Convert a JSON "term" value into a logic-core `Term`. The
/// converter accepts a few common shapes models produce:
///
/// - `"atom"` (a bare string) → `Atom`.
/// - `{ "atom": "name" }` → `Atom("name")`.
/// - `{ "functor": "...", "args": [...] }` → `Compound { functor, args }`.
/// - `{ "compound": { "functor": "...", "args": [...] } }` → same.
/// - Anything else → `Atom("<debug-repr>")` with a warning.
fn json_to_term(v: &serde_json::Value, node_id: &str, warnings: &mut Vec<String>) -> Term {
    if let Some(s) = v.as_str() {
        return atom(s);
    }
    if let Some(obj) = v.as_object() {
        if let Some(s) = obj.get("atom").and_then(|x| x.as_str()) {
            return atom(s);
        }
        let compound_obj = obj
            .get("compound")
            .and_then(|x| x.as_object())
            .or(Some(obj));
        if let Some(c) = compound_obj {
            if let Some(functor) = c.get("functor").and_then(|x| x.as_str()) {
                let args: Vec<Term> = c
                    .get("args")
                    .and_then(|x| x.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|a| json_to_term(a, node_id, warnings))
                            .collect()
                    })
                    .unwrap_or_default();
                return compound(functor, args);
            }
        }
    }
    warnings.push(format!(
        "node {node_id} has unrecognized term shape; using atom(\"unknown\")"
    ));
    atom("unknown")
}

/// Convert a JSON `source_spans` array into typed `Span` values.
/// Tolerates missing/empty arrays and clamps out-of-bound `end`
/// values; Query nodes with empty spans pass through unchanged (ADJ02
/// v2 permits zero-span queries).
fn parse_source_spans(
    v: Option<&serde_json::Value>,
    document_id: &DocumentId,
    source_len: usize,
    node_id: &str,
    kind: NodeKind,
    warnings: &mut Vec<String>,
) -> Vec<Span> {
    let arr = match v.and_then(|x| x.as_array()) {
        Some(a) => a,
        None => {
            if matches!(kind, NodeKind::Query) {
                return Vec::new();
            }
            warnings.push(format!(
                "node {node_id} ({kind:?}) missing source_spans; covering full document as fallback"
            ));
            return vec![Span::new(document_id.clone(), 0, source_len.max(1))];
        }
    };
    let mut spans = Vec::with_capacity(arr.len());
    for (i, span_v) in arr.iter().enumerate() {
        let obj = match span_v.as_object() {
            Some(o) => o,
            None => {
                warnings.push(format!(
                    "node {node_id} span[{i}] is not an object; skipping"
                ));
                continue;
            }
        };
        let start = obj.get("start").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let end_raw = obj.get("end").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let end = end_raw.min(source_len);
        if end < end_raw {
            warnings.push(format!(
                "node {node_id} span[{i}] end {end_raw} > source length {source_len}; clamped to {end}"
            ));
        }
        if end <= start {
            warnings.push(format!(
                "node {node_id} span[{i}] is degenerate (start={start}, end={end}); skipping"
            ));
            continue;
        }
        spans.push(Span::new(document_id.clone(), start, end));
    }
    spans
}

/// Render the first ADJ02 violation as a short string suitable for
/// the ADJ06 correction prompt. ADJ02 violations are stringly typed
/// in the audit trail (the pipeline stores a `debug` field for
/// unspecialised variants), so the simplest reliable thing is to
/// pull that text out and prepend the violation `kind`.
fn format_first_adj02_violation(adj02: &adjudication_audit_trail::CheckerResult) -> String {
    match adj02.violations.first() {
        Some(v) => {
            let kind = v
                .detail
                .get("kind")
                .and_then(|x| x.as_str())
                .unwrap_or("UncoveredSpan");
            let debug = v
                .detail
                .get("debug")
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .unwrap_or_default();
            if debug.is_empty() {
                kind.to_string()
            } else {
                format!("{kind}: {debug}")
            }
        }
        None => "ADJ02 reported Failed with no violation entries".to_string(),
    }
}

/// Best-effort retrieval of the previous LLM IR JSON for the
/// correction prompt. When `IrSourceTelemetry::LlmExtracted` is in
/// hand we have the raw JSON verbatim; otherwise we fall back to an
/// empty placeholder so the correction prompt still flows.
fn previous_ir_for_clarification(t: &IrSourceTelemetry) -> serde_json::Value {
    match t {
        IrSourceTelemetry::LlmExtracted { raw_ir_json, .. } => serde_json::from_str(raw_ir_json)
            .unwrap_or_else(|_| serde_json::json!({ "note": "previous IR JSON unparseable" })),
        _ => serde_json::json!({ "note": "no previous IR JSON available" }),
    }
}

/// Pull `RoundTripDrift` violations out of the ADJ04 checker result
/// and convert them into plain-string findings for the demo report.
fn collect_adj04_drift(
    results: &[adjudication_audit_trail::CheckerResult],
) -> Vec<Adj04DriftFinding> {
    let Some(adj04) = results.iter().find(|cr| {
        matches!(cr.pass_name, adjudication_audit_trail::PassName::Adj04RoundTrip)
    }) else {
        return Vec::new();
    };
    adj04
        .violations
        .iter()
        .filter_map(|v| {
            let d = &v.detail;
            Some(Adj04DriftFinding {
                node_id: v.node_id.0.clone(),
                source_excerpt: d.get("source_excerpt").and_then(|x| x.as_str())?.to_string(),
                model_rendering: d.get("rendering").and_then(|x| x.as_str())?.to_string(),
                source_to_rendering_score: d
                    .get("source_to_rendering")
                    .and_then(|x| x.as_f64())? as f32,
                rendering_to_source_score: d
                    .get("rendering_to_source")
                    .and_then(|x| x.as_f64())? as f32,
                threshold: d.get("threshold").and_then(|x| x.as_f64())? as f32,
            })
        })
        .collect()
}

/// Pull `AdversarialReading` violations out of the ADJ05 checker
/// result and convert them into plain-string findings for the demo
/// report.
fn collect_adj05_findings(
    results: &[adjudication_audit_trail::CheckerResult],
) -> Vec<Adj05AdversarialFinding> {
    let Some(adj05) = results.iter().find(|cr| {
        matches!(cr.pass_name, adjudication_audit_trail::PassName::Adj05Adversarial)
    }) else {
        return Vec::new();
    };
    adj05
        .violations
        .iter()
        .filter_map(|v| {
            let d = &v.detail;
            Some(Adj05AdversarialFinding {
                node_id: v.node_id.0.clone(),
                ir_rendered: d.get("ir_rendered").and_then(|x| x.as_str())?.to_string(),
                adversary_reading: d
                    .get("adversary_reading")
                    .and_then(|x| x.as_str())?
                    .to_string(),
                adversary_explanation: d
                    .get("adversary_explanation")
                    .and_then(|x| x.as_str())?
                    .to_string(),
                judge_reason: d.get("judge_reason").and_then(|x| x.as_str())?.to_string(),
            })
        })
        .collect()
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
    match &pipeline.ir_source {
        IrSourceTelemetry::HandBuilt => {
            out.push_str("IR source:       hand-built TSA fixture (no LLM extraction)\n");
        }
        IrSourceTelemetry::LlmExtracted {
            node_count,
            converter_warnings,
            clarification_summary,
            ..
        } => {
            out.push_str(&format!(
                "IR source:       decompose_text via {} → typed IR ({n} nodes)\n",
                "Role::Extractor",
                n = node_count,
            ));
            if !converter_warnings.is_empty() {
                out.push_str(&format!(
                    "                 ({w} converter warning(s) — see audit dump)\n",
                    w = converter_warnings.len()
                ));
            }
            if let Some(s) = clarification_summary {
                out.push_str(&format!(
                    "                 ADJ06 clarification: {s}\n",
                ));
            }
        }
        IrSourceTelemetry::LlmExtractionFailed {
            error,
            fell_back_to_hand_built,
        } => {
            out.push_str(&format!(
                "IR source:       decompose_text FAILED: {error}\n  fallback:      {fb}\n",
                fb = if *fell_back_to_hand_built {
                    "hand-built TSA fixture"
                } else {
                    "(none — pipeline did not run)"
                },
            ));
        }
    }
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
    // ADJ02 coverage violations — surface so the user sees WHY the
    // pipeline blocked when it did.
    let adj02_vs = &pipeline.pipeline_output.audit_trail.checker_results[0].violations;
    if !adj02_vs.is_empty() {
        out.push_str("\n");
        out.push_str(&format!(
            "--- ADJ02 coverage violations ({n}) ---\n",
            n = adj02_vs.len()
        ));
        for v in adj02_vs {
            let kind = v
                .detail
                .get("kind")
                .and_then(|x| x.as_str())
                .unwrap_or("(no kind)");
            let debug = v
                .detail
                .get("debug")
                .and_then(|x| x.as_str())
                .unwrap_or("(no detail)");
            out.push_str(&format!("  {kind}: {debug}\n"));
        }
    }
    if !pipeline.adj04_drift_findings.is_empty() {
        out.push_str("\n");
        out.push_str(&format!(
            "--- ADJ04 round-trip drift findings ({n}) ---\n",
            n = pipeline.adj04_drift_findings.len()
        ));
        out.push_str(&format!(
            "(threshold = {:.2}; either direction below this counts as drift)\n",
            pipeline.adj04_drift_findings[0].threshold
        ));
        for f in &pipeline.adj04_drift_findings {
            out.push_str(&format!(
                "\n  node {id}:\n    source excerpt: {src:?}\n    model rendering: {ren:?}\n    NLI scores:     source\u{2192}rendering = {p:.2}, rendering\u{2192}source = {h:.2}\n",
                id = f.node_id,
                src = f.source_excerpt,
                ren = f.model_rendering,
                p = f.source_to_rendering_score,
                h = f.rendering_to_source_score,
            ));
        }
    }
    if !pipeline.adj05_adversarial_findings.is_empty() {
        out.push_str("\n");
        out.push_str(&format!(
            "--- ADJ05 adversarial findings ({n}) ---\n",
            n = pipeline.adj05_adversarial_findings.len()
        ));
        out.push_str(
            "(a *different model family* found a plausible alternative reading)\n",
        );
        for f in &pipeline.adj05_adversarial_findings {
            out.push_str(&format!(
                "\n  node {id}:\n    IR rendering:        {ir:?}\n    adversary reading:   {adv:?}\n    adversary's reason:  {exp}\n    judge ruled plausible because: {jr}\n",
                id = f.node_id,
                ir = f.ir_rendered,
                adv = f.adversary_reading,
                exp = f.adversary_explanation,
                jr = f.judge_reason,
            ));
        }
    }
    // ADJ05 skipped/check-error diagnostics from the audit trail.
    let adj05 = &pipeline.pipeline_output.audit_trail.checker_results[3];
    if matches!(adj05.outcome, PassOutcome::Skipped) {
        if let Some(reason) = adj05.telemetry.get("skipped_reason").and_then(|x| x.as_str()) {
            out.push_str(&format!("\nADJ05 skipped: {reason}\n"));
        }
    } else if matches!(adj05.outcome, PassOutcome::Failed)
        && pipeline.adj05_adversarial_findings.is_empty()
    {
        if let Some(err) = adj05.telemetry.get("check_error").and_then(|x| x.as_str()) {
            out.push_str(&format!("\nADJ05 errored: {err}\n"));
        }
    }
    out.push_str("\n");
    out.push_str(
        "Note: to enable ADJ05, install a second model from a different \
         family (e.g., `ollama pull llama3.1:8b`) and set \
         ADJ_DEMO_ADVERSARY_MODEL=llama3.1:8b. The framework enforces \
         (vendor, model_family) independence between Extractor and \
         Adversary so the adversary can't rubber-stamp the extractor.\n",
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

    // -------------------- json_to_ir_document --------------------

    #[test]
    fn converter_handles_well_formed_tsa_shape() {
        let v = serde_json::json!({
            "document_id": "tsa-demo-001",
            "nodes": [
                {
                    "id": "F1",
                    "kind": "Fact",
                    "term": { "functor": "carry_on", "args": [{"atom": "1"}] },
                    "polarity": "Affirmed",
                    "modality": "Present",
                    "source_spans": [{"start": 0, "end": 16}]
                },
                {
                    "id": "F2",
                    "kind": "Fact",
                    "term": { "functor": "prohibited", "args": [{"atom": "matches"}] },
                    "polarity": "Affirmed",
                    "modality": "Present",
                    "source_spans": [{"start": 16, "end": 24}]
                }
            ]
        });
        let (ir, w) =
            json_to_ir_document(&v, "tsa-demo-001", "1 carry-on bag, matches.").unwrap();
        // The two facts are clean; the converter adds a synthesized
        // Query so the engine has something to do. The synth is
        // recorded as a warning.
        assert_eq!(ir.nodes.len(), 3);
        assert_eq!(ir.nodes[0].id.0, "F1");
        assert_eq!(ir.nodes[0].kind, NodeKind::Fact);
        assert_eq!(ir.nodes[1].id.0, "F2");
        assert_eq!(ir.nodes[2].kind, NodeKind::Query);
        assert!(w.iter().any(|m| m.contains("synthesizing")));
    }

    #[test]
    fn converter_clamps_out_of_bounds_spans() {
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [{
                "id": "n1",
                "kind": "Fact",
                "term": { "atom": "a" },
                "source_spans": [{"start": 0, "end": 999}]
            }]
        });
        let (ir, warnings) = json_to_ir_document(&v, "doc1", "hello").unwrap();
        assert_eq!(ir.nodes[0].source_spans[0].end, 5);
        assert!(warnings.iter().any(|w| w.contains("clamped")));
    }

    #[test]
    fn converter_defaults_missing_kind_to_fact() {
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [{
                "id": "n1",
                "term": "a",
                "source_spans": [{"start": 0, "end": 1}]
            }]
        });
        let (ir, warnings) = json_to_ir_document(&v, "doc1", "x").unwrap();
        assert_eq!(ir.nodes[0].kind, NodeKind::Fact);
        assert!(warnings.iter().any(|w| w.contains("kind")));
    }

    #[test]
    fn converter_accepts_string_atom_term() {
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [{
                "id": "n1",
                "kind": "Fact",
                "term": "ready",
                "source_spans": [{"start": 0, "end": 1}]
            }]
        });
        let (ir, _) = json_to_ir_document(&v, "doc1", "x").unwrap();
        match &ir.nodes[0].term {
            Term::Atom(s) => assert_eq!(s, "ready"),
            other => panic!("expected Atom, got {other:?}"),
        }
    }

    #[test]
    fn converter_rejects_non_object_root() {
        let v = serde_json::json!(["not", "an", "object"]);
        assert!(json_to_ir_document(&v, "doc1", "x").is_err());
    }

    #[test]
    fn converter_rejects_missing_nodes_array() {
        let v = serde_json::json!({"document_id": "doc1"});
        assert!(json_to_ir_document(&v, "doc1", "x").is_err());
    }

    #[test]
    fn converter_skips_degenerate_spans() {
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [{
                "id": "n1",
                "kind": "Fact",
                "term": { "atom": "a" },
                "source_spans": [
                    {"start": 0, "end": 0},
                    {"start": 0, "end": 3}
                ]
            }]
        });
        let (ir, warnings) = json_to_ir_document(&v, "doc1", "hello").unwrap();
        assert_eq!(ir.nodes[0].source_spans.len(), 1);
        assert!(warnings.iter().any(|w| w.contains("degenerate")));
    }

    #[test]
    fn converter_synthesizes_full_span_when_missing_for_fact() {
        // If the LLM forgets `source_spans` on a Fact, we fall back
        // to a span covering the whole document so ADJ02 at least
        // has something to validate.
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [{ "id": "n1", "kind": "Fact", "term": "a" }]
        });
        let (ir, warnings) = json_to_ir_document(&v, "doc1", "hello").unwrap();
        assert_eq!(ir.nodes[0].source_spans.len(), 1);
        assert_eq!(ir.nodes[0].source_spans[0].start, 0);
        assert_eq!(ir.nodes[0].source_spans[0].end, 5);
        assert!(warnings.iter().any(|w| w.contains("missing source_spans")));
    }

    #[test]
    fn converter_flattens_nested_children_tree() {
        // Gemma 4 produces a tree like `{"node_type":"TextRun","children":[{...}]}`.
        // The converter must walk this and emit each leaf as an IR
        // node, dropping the grouping parents.
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [
                {
                    "node_type": "TextRun",
                    "text": "1 ",
                    "children": [
                        {
                            "node_type": "Fact",
                            "text": "1 ",
                            "source_spans": [{"start": 0, "end": 2}]
                        }
                    ]
                },
                {
                    "node_type": "TextRun",
                    "children": [
                        {
                            "node_type": "Fact",
                            "text": "carry-on bag, matches.",
                            "source_spans": [{"start": 2, "end": 24}]
                        }
                    ]
                }
            ]
        });
        let (ir, _warnings) =
            json_to_ir_document(&v, "doc1", "1 carry-on bag, matches.").unwrap();
        // Two leaves were flattened + one synthesized Query = 3 nodes.
        assert_eq!(ir.nodes.len(), 3);
        assert!(ir.nodes.iter().any(|n| matches!(n.kind, NodeKind::Query)));
        let fact_count = ir.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Fact)).count();
        assert_eq!(fact_count, 2);
    }

    #[test]
    fn converter_accepts_node_type_alias_for_kind() {
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [{
                "id": "n1",
                "node_type": "Query",
                "term": "compliant(p)",
                "source_spans": [{"start": 0, "end": 1}]
            }]
        });
        let (ir, _) = json_to_ir_document(&v, "doc1", "x").unwrap();
        assert_eq!(ir.nodes[0].kind, NodeKind::Query);
    }

    #[test]
    fn converter_uses_text_as_term_fallback() {
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [{
                "id": "n1",
                "node_type": "Fact",
                "text": "matches.",
                "source_spans": [{"start": 0, "end": 8}]
            }]
        });
        let (ir, _) = json_to_ir_document(&v, "doc1", "matches.").unwrap();
        match &ir.nodes[0].term {
            Term::Compound { functor, args } => {
                assert_eq!(functor, "text_claim");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Term::Atom(s) => assert_eq!(s, "matches."),
                    other => panic!("expected atom inside text_claim, got {other:?}"),
                }
            }
            other => panic!("expected text_claim compound, got {other:?}"),
        }
    }

    #[test]
    fn converter_synthesizes_query_when_llm_omits_it() {
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [{
                "id": "F1",
                "kind": "Fact",
                "term": "a",
                "source_spans": [{"start": 0, "end": 1}]
            }]
        });
        let (ir, warnings) = json_to_ir_document(&v, "doc1", "x").unwrap();
        assert!(ir.nodes.iter().any(|n| matches!(n.kind, NodeKind::Query)));
        assert!(warnings.iter().any(|w| w.contains("synthesizing")));
    }

    #[test]
    fn converter_allows_empty_source_spans_for_query() {
        let v = serde_json::json!({
            "document_id": "doc1",
            "nodes": [{
                "id": "Q1",
                "kind": "Query",
                "term": { "functor": "compliant", "args": [{"atom": "p"}] }
            }]
        });
        let (ir, warnings) = json_to_ir_document(&v, "doc1", "hello").unwrap();
        assert_eq!(ir.nodes[0].kind, NodeKind::Query);
        assert!(ir.nodes[0].source_spans.is_empty());
        // No warning for missing spans on Query.
        assert!(!warnings.iter().any(|w| w.contains("missing source_spans")));
    }
}
