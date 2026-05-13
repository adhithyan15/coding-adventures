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
use llm_cache::{CacheStats, CacheStatsHandle, CachingClient};
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
    /// Optional rulebook text to inject into Arm A's system
    /// prompt. v0.3 parity with `adjudication-tsa-demo` v0.12.
    /// Pair with [`fixture_clinical_rulebook`] for a deterministic
    /// baseline.
    pub rulebook_text: Option<String>,
    /// Output-token cap for Arm A. v0.3 parity with
    /// `adjudication-tsa-demo`'s `ADJ_DEMO_MAX_ANSWER_TOKENS`.
    pub max_answer_tokens: usize,
    /// Arm A dispatch mode. Single-turn is the v0.1 behaviour;
    /// Priming engages the two-turn protocol from
    /// `adjudication-tsa-demo` v0.12.
    pub arm_a_mode: ArmAMode,
}

/// Arm A dispatch strategies. Duplicated from
/// `adjudication-tsa-demo` deliberately — keeping demo crates
/// independent. If a third demo gains the same field a future
/// `adjudication-demo-common` crate can pull this out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmAMode {
    SingleTurn,
    Priming,
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
            rulebook_text: None,
            max_answer_tokens: 2048,
            arm_a_mode: ArmAMode::SingleTurn,
        }
    }
}

/// The canonical fixture clinical rulebook. Short, numbered, with
/// references to authoritative reference material per ADJ19's
/// methodology. v0.3 ships a small set covering the canonical
/// fixture's symptoms (dyspnea, fever, denied-allergy reasoning);
/// later iterations expand coverage as the cross-domain bench
/// declaration set grows.
///
/// Same shape as `adjudication_tsa_demo::fixture_tsa_rulebook` so
/// the bench harness can treat both domains uniformly.
pub fn fixture_clinical_rulebook() -> String {
    "CLINICAL TRIAGE RULEBOOK (v0.1, as of 2026-05-13):\n\
     1. Chest pain radiating to arm + sweating + cardiac history → \
        urgent evaluation per AHA 2019 ACS guideline.\n\
     2. Stiff neck + photophobia + fever above 38.5°C → urgent \
        evaluation; suspect bacterial meningitis per IDSA criteria.\n\
     3. Severe asthma exacerbation indicators (peak flow <50% personal \
        best, accessory muscle use, tripod position) → urgent \
        evaluation per GINA 2024.\n\
     4. Mild URI without red-flag features (no stridor, normal \
        vitals, no chest pain) → outpatient supportive care per IDSA \
        pharyngitis guideline.\n\
     5. Mild-to-moderate dehydration in adults with normal vitals \
        and ability to tolerate fluids → oral rehydration per WHO; \
        outpatient is appropriate.\n\
     6. Severe pediatric dehydration (lethargic, sunken eyes, \
        capillary refill > 3s) → urgent IV fluids per WHO.\n\
     7. Denied allergies (e.g., \"no known drug allergy\") remove a \
        contraindication but do not change the indication. \
        Decision-making proceeds with standard monitoring per ACR \
        practice parameter.\n\
     8. Isolated dyspnea without other red flags (no chest pain, \
        normal vitals, no exertional component) is a clinical \
        gray-zone; default to keep-for-observation pending further \
        history per general triage guidelines.\n\
     9. Fever above 39.5°C with rigors or altered mental status → \
        urgent evaluation regardless of other findings.\n\
     10. The framework's job is to apply these rules to a given \
         patient assessment; if a finding doesn't trigger any rule \
         above, default to keep-for-observation."
        .to_string()
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
    match cfg.arm_a_mode {
        ArmAMode::SingleTurn => run_raw_arm_single_turn(cfg),
        ArmAMode::Priming => run_raw_arm_priming(cfg),
    }
}

fn run_raw_arm_single_turn(cfg: &DemoConfig) -> Result<RawArmReport, LlmError> {
    let client = OllamaClient::new(cfg.model.clone())
        .with_endpoint(cfg.endpoint.clone())
        .with_timeout(cfg.timeout);

    let prompt = build_raw_user_prompt(&cfg.source_text);
    let system = build_raw_system_prompt(cfg.rulebook_text.as_deref());
    let req = CompletionRequest {
        model: cfg.model.clone(),
        system: Some(system),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(prompt.clone()),
        }],
        temperature: 0.0,
        max_tokens: Some(cfg.max_answer_tokens),
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

/// Two-turn priming dispatch. Mirrors `adjudication-tsa-demo`'s
/// implementation: turn 1 hands the model the rulebook with an
/// ACK-only instruction; turn 2 sends the patient assessment and
/// asks for a verdict-first answer. Falls back to single-turn when
/// no rulebook is configured (priming with no rulebook to digest
/// would be a wasted round-trip).
fn run_raw_arm_priming(cfg: &DemoConfig) -> Result<RawArmReport, LlmError> {
    let rulebook = match cfg.rulebook_text.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => return run_raw_arm_single_turn(cfg),
    };

    let client = OllamaClient::new(cfg.model.clone())
        .with_endpoint(cfg.endpoint.clone())
        .with_timeout(cfg.timeout);

    let priming_system = build_priming_system_prompt();
    let priming_user = build_priming_turn1_user_prompt(rulebook);
    let priming_req = CompletionRequest {
        model: cfg.model.clone(),
        system: Some(priming_system.clone()),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(priming_user.clone()),
        }],
        temperature: 0.0,
        max_tokens: Some(64),
        stop_sequences: Vec::new(),
        seed: Some(42),
        metadata: Default::default(),
    };
    let turn1 = client.complete(priming_req)?;

    let turn2_user = build_priming_turn2_user_prompt(&cfg.source_text);
    let turn2_req = CompletionRequest {
        model: cfg.model.clone(),
        system: Some(priming_system),
        messages: vec![
            Message {
                role: MsgRole::User,
                content: MessageContent::Text(priming_user),
            },
            Message {
                role: MsgRole::Assistant,
                content: MessageContent::Text(turn1.text.clone()),
            },
            Message {
                role: MsgRole::User,
                content: MessageContent::Text(turn2_user.clone()),
            },
        ],
        temperature: 0.0,
        max_tokens: Some(cfg.max_answer_tokens),
        stop_sequences: Vec::new(),
        seed: Some(42),
        metadata: Default::default(),
    };
    let turn2 = client.complete(turn2_req)?;

    Ok(RawArmReport {
        prompt: turn2_user,
        answer: turn2.text,
        model: turn2.model,
        input_tokens: turn1.usage.input_tokens + turn2.usage.input_tokens,
        output_tokens: turn1.usage.output_tokens + turn2.usage.output_tokens,
        latency_ms: turn1.latency_ms + turn2.latency_ms,
        finish_reason: turn2.finish_reason,
    })
}

fn build_raw_user_prompt(source_text: &str) -> String {
    format!(
        "ASSESSMENT: {src}\n\nIs this patient safe to discharge?",
        src = source_text,
    )
}

/// Build Arm A's single-turn system prompt. v0.3 changes the
/// format to put the VERDICT line FIRST so the verdict survives
/// truncation. With a rulebook, also forbids invented rules.
pub fn build_raw_system_prompt(rulebook_text: Option<&str>) -> String {
    match rulebook_text {
        None => "You are a primary-care triage assistant. Given a short patient \
                 assessment, decide whether the patient is safe to discharge.\n\
                 \n\
                 Your response MUST begin with the verdict line as the very \
                 first line of output:\n\
                 \n\
                 VERDICT: SAFE_TO_DISCHARGE\n\
                 (or)\n\
                 VERDICT: KEEP_FOR_OBSERVATION\n\
                 \n\
                 After the verdict line, give 2-3 sentences of reasoning. The \
                 verdict-first format ensures the verdict is captured even if \
                 your reasoning is truncated."
            .to_string(),
        Some(text) => format!(
            "You are a primary-care triage assistant. The clinical triage \
             rules you MUST apply are listed below. Do not invent any \
             additional rules; if a finding is not justified by a specific \
             numbered rule below, do not include it.\n\
             \n\
             {text}\n\
             \n\
             Given a patient assessment, decide whether the patient is safe \
             to discharge.\n\
             \n\
             Your response MUST begin with the verdict line as the very \
             first line of output:\n\
             \n\
             VERDICT: SAFE_TO_DISCHARGE\n\
             (or)\n\
             VERDICT: KEEP_FOR_OBSERVATION\n\
             \n\
             After the verdict line, give 2-3 sentences of reasoning citing \
             specific rule numbers for each finding. The verdict-first \
             format ensures the verdict is captured even if your reasoning \
             is truncated."
        ),
    }
}

/// Build the system prompt for the priming dispatch path. Same
/// role (triage assistant) but with explicit ground rules about
/// the two-turn protocol: read silently on turn 1, answer on
/// turn 2.
pub fn build_priming_system_prompt() -> String {
    "You are a primary-care triage assistant. You will receive \
     information in two turns:\n\
     \n\
     Turn 1: I will give you a clinical triage rulebook. Read it \
     carefully and store the rules in your working memory. Respond \
     with exactly the single word `ACK` and nothing else. Do NOT \
     summarise the rules, comment on them, or analyse them until I \
     ask my question in turn 2.\n\
     \n\
     Turn 2: I will give you a patient assessment. Apply the \
     rulebook from turn 1 and respond. Your response MUST begin \
     with the verdict line as the very first line of output:\n\
     \n\
     VERDICT: SAFE_TO_DISCHARGE\n\
     (or)\n\
     VERDICT: KEEP_FOR_OBSERVATION\n\
     \n\
     After the verdict line, give 2-3 sentences of reasoning citing \
     specific rule numbers from the turn 1 rulebook. The \
     verdict-first format ensures the verdict is captured even if \
     your reasoning is truncated. Do not invent rules that were not \
     in the turn 1 rulebook."
        .to_string()
}

/// Build the turn 1 user message for the priming path: the
/// rulebook, framed as "intake this and acknowledge".
pub fn build_priming_turn1_user_prompt(rulebook_text: &str) -> String {
    format!(
        "TURN 1: RULEBOOK INTAKE.\n\
         \n\
         The following clinical triage rulebook applies to my next \
         question. Read it. Respond with `ACK` only.\n\
         \n\
         {rulebook_text}"
    )
}

/// Build the turn 2 user message for the priming path: the patient
/// assessment, framed as "now apply turn 1's rulebook and answer".
pub fn build_priming_turn2_user_prompt(source_text: &str) -> String {
    format!(
        "TURN 2: QUESTION.\n\
         \n\
         Apply the rulebook from turn 1. Assessment: {source_text}\n\
         \n\
         Is this patient safe to discharge? Remember: first line MUST \
         be `VERDICT: SAFE_TO_DISCHARGE` or `VERDICT: KEEP_FOR_OBSERVATION`."
    )
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
    pub cache_stats: CacheStats,
}

pub fn run_pipeline_arm(cfg: &DemoConfig) -> PipelineArmReport {
    let primary = OllamaClient::new(cfg.model.clone())
        .with_endpoint(cfg.endpoint.clone())
        .with_timeout(cfg.timeout);
    let (extractor, h_extractor) = wrap_with_cache(Box::new(primary.clone()), cfg);
    let (renderer, h_renderer) = wrap_with_cache(Box::new(primary.clone()), cfg);
    let (nli, h_nli) = wrap_with_cache(Box::new(primary.clone()), cfg);
    let (plausibility, h_plausibility) = wrap_with_cache(Box::new(primary.clone()), cfg);
    let mut cache_handles = vec![h_extractor, h_renderer, h_nli, h_plausibility];
    let mut gateway = GatewayConfig::new()
        .with_client(PrimitiveRole::Extractor, extractor)
        .with_client(PrimitiveRole::Renderer, renderer)
        .with_client(PrimitiveRole::Nli, nli)
        .with_client(PrimitiveRole::Plausibility, plausibility);
    if let Some(adv) = &cfg.adversary_model {
        let adv_client = OllamaClient::new(adv.clone())
            .with_endpoint(cfg.endpoint.clone())
            .with_timeout(cfg.timeout);
        let (adv_boxed, h_adv) =
            wrap_with_cache(Box::new(adv_client) as Box<dyn LlmClient>, cfg);
        cache_handles.push(h_adv);
        gateway = gateway.with_client(PrimitiveRole::Adversary, adv_boxed);
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

    let aggregate_cache_stats = aggregate_stats(&cache_handles);

    PipelineArmReport {
        verdict_summary: summary,
        adj02_outcome: trail.checker_results[0].outcome,
        adj03_outcome: trail.checker_results[1].outcome,
        adj04_outcome: trail.checker_results[2].outcome,
        adj05_outcome: trail.checker_results[3].outcome,
        engine_ran: trail.engine_artifacts.is_some(),
        cache_stats: aggregate_cache_stats,
        pipeline_output: output,
    }
}

/// Cache-wrap an LlmClient and return its stats handle alongside.
fn wrap_with_cache(
    inner: Box<dyn LlmClient>,
    cfg: &DemoConfig,
) -> (Box<dyn LlmClient>, CacheStatsHandle) {
    let cached = match &cfg.cache_dir {
        Some(dir) => CachingClient::with_disk_persistence(inner, dir),
        None => CachingClient::new(inner),
    };
    let handle = cached.stats_handle();
    (Box::new(cached) as Box<dyn LlmClient>, handle)
}

fn aggregate_stats(handles: &[CacheStatsHandle]) -> CacheStats {
    let mut total = CacheStats::default();
    for h in handles {
        let s = h.stats();
        total.hits = total.hits.saturating_add(s.hits);
        total.misses = total.misses.saturating_add(s.misses);
        total.entries = total.entries.saturating_add(s.entries);
    }
    total
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
        discard_reason: None,
        metadata: Default::default(),
    });

    IRDocument {
        document_id: doc_id,
        nodes,
        edges: Vec::new(),
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
    let cs = &pipe.cache_stats;
    if cs.hits + cs.misses > 0 {
        out.push_str(&format!(
            "cache:                    {hits} hits / {misses} misses ({rate:.0}% hit rate), {entries} entries\n",
            hits = cs.hits,
            misses = cs.misses,
            rate = cs.hit_rate() * 100.0,
            entries = cs.entries,
        ));
    }
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

    // -----------------------------------------------------------------
    // v0.3 — rulebook injection + max_answer_tokens + priming tests
    // -----------------------------------------------------------------

    #[test]
    fn default_config_uses_single_turn_and_2048_token_cap() {
        let cfg = DemoConfig::default();
        assert_eq!(cfg.arm_a_mode, ArmAMode::SingleTurn);
        assert_eq!(cfg.max_answer_tokens, 2048);
        assert!(cfg.rulebook_text.is_none());
    }

    #[test]
    fn fixture_rulebook_covers_canonical_symptoms() {
        let rb = fixture_clinical_rulebook();
        assert!(rb.contains("Severe asthma"));
        assert!(rb.contains("URI"));
        assert!(rb.contains("dehydration"));
        // Denied-allergy reasoning is the load-bearing clinical edge
        // case from ADJ19 §clinical-domain.
        assert!(rb.contains("Denied allergies"));
        assert!(rb.contains("monitoring"));
    }

    #[test]
    fn raw_system_prompt_demands_verdict_first() {
        let no_rb = build_raw_system_prompt(None);
        assert!(no_rb.contains("first line"));
        assert!(no_rb.contains("truncated"));
        // Verdict set is clinical-specific.
        assert!(no_rb.contains("SAFE_TO_DISCHARGE"));
        assert!(no_rb.contains("KEEP_FOR_OBSERVATION"));

        let with_rb = build_raw_system_prompt(Some("rule x"));
        assert!(with_rb.contains("first line"));
        assert!(with_rb.contains("Do not invent"));
        assert!(with_rb.contains("citing specific rule numbers"));
    }

    #[test]
    fn priming_system_prompt_describes_two_turn_protocol() {
        let s = build_priming_system_prompt();
        assert!(s.contains("Turn 1"));
        assert!(s.contains("Turn 2"));
        assert!(s.contains("ACK"));
        assert!(s.contains("SAFE_TO_DISCHARGE"));
        assert!(s.contains("KEEP_FOR_OBSERVATION"));
        assert!(s.contains("Do NOT"));
    }

    #[test]
    fn priming_turn1_user_prompt_embeds_rulebook_and_demands_ack() {
        let rb = "1. test rule about dyspnea.\n2. test rule about fever.";
        let s = build_priming_turn1_user_prompt(rb);
        assert!(s.contains("RULEBOOK INTAKE"));
        assert!(s.contains("ACK"));
        assert!(s.contains("test rule about dyspnea"));
    }

    #[test]
    fn priming_turn2_user_prompt_embeds_assessment_and_restates_verdict_format() {
        let s = build_priming_turn2_user_prompt(
            "Patient: shortness of breath, mild fever.",
        );
        assert!(s.contains("TURN 2"));
        assert!(s.contains("shortness of breath"));
        assert!(s.contains("rulebook from turn 1"));
        assert!(s.contains("SAFE_TO_DISCHARGE"));
        assert!(s.contains("KEEP_FOR_OBSERVATION"));
    }

    #[test]
    fn config_with_priming_mode_is_addressable_via_struct_field() {
        let mut cfg = DemoConfig::default();
        cfg.arm_a_mode = ArmAMode::Priming;
        cfg.rulebook_text = Some(fixture_clinical_rulebook());
        assert_eq!(cfg.arm_a_mode, ArmAMode::Priming);
        assert!(cfg.rulebook_text.is_some());
    }

    #[test]
    fn arm_a_mode_round_trips_through_debug_clone_eq() {
        let a = ArmAMode::SingleTurn;
        let b = a;
        assert_eq!(a, b);
        let c = ArmAMode::Priming;
        assert_ne!(a, c);
        let s = format!("{a:?}");
        assert!(s.contains("SingleTurn"));
    }
}
