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
use llm_cache::{CacheStats, CacheStatsHandle, CachingClient};
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
    /// Optional rulebook injected into Arm A's system prompt
    /// (v0.3 parity with adjudication-tsa-demo v0.12).
    pub rulebook_text: Option<String>,
    /// Output-token cap for Arm A (v0.3 parity).
    pub max_answer_tokens: usize,
    /// Arm A dispatch mode (v0.3 parity).
    pub arm_a_mode: ArmAMode,
}

/// Arm A dispatch strategies. Duplicated from
/// `adjudication-tsa-demo` and `adjudication-clinical-demo`;
/// extracting to a shared crate becomes natural if a fourth demo
/// wants the same field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmAMode {
    SingleTurn,
    Priming,
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
            rulebook_text: None,
            max_answer_tokens: 2048,
            arm_a_mode: ArmAMode::SingleTurn,
        }
    }
}

/// The canonical fixture contract rulebook. Covers the demo's
/// existing "delivery within window + force majeure exception"
/// shape plus enough breadth for ADJ19's declaration set
/// (force-majeure cycle, plain breach, ordinary delay, war event,
/// non-enumerated act-of-god → DISPUTED, etc.).
///
/// Same shape as `fixture_tsa_rulebook` and
/// `fixture_clinical_rulebook` so the cross-domain bench harness
/// can treat the three domains uniformly.
pub fn fixture_contract_rulebook() -> String {
    "CONTRACT-CLAUSE RULEBOOK (v0.1, as of 2026-05-13):\n\
     1. Vendor must deliver within the contractually-specified \
        window; failure to deliver on time constitutes BREACH \
        absent an applicable exception.\n\
     2. Force majeure events (acts of God, war, civil unrest, \
        natural disasters such as hurricanes, earthquakes, and \
        floods) extend the deadline by the contract's force-majeure \
        extension (default 14 days) per Restatement (Second) of \
        Contracts §261.\n\
     3. Supplier delays, market conditions, currency fluctuations, \
        and pricing disputes are NOT force-majeure events absent \
        specific contract language to the contrary.\n\
     4. Hurricanes, earthquakes, and floods are explicit acts of \
        God in most U.S. jurisdictions per Restatement §261.\n\
     5. If a force-majeure event occurs during the delivery window \
        and the vendor delivers within the extended deadline \
        (original window + force-majeure extension), the vendor is \
        NOT in breach.\n\
     6. If a force-majeure event occurs but the vendor delivers \
        BEYOND the extended deadline, the vendor IS in breach \
        despite the force-majeure event.\n\
     7. If a stock-related exception is enumerated in the contract \
        (e.g., 'unless the goods are out of stock'), the vendor is \
        NOT in breach when the exception's predicate holds.\n\
     8. Disputed cases (where an event's classification as \
        force-majeure is uncertain — e.g., a non-enumerated event \
        with ambiguous jurisdictional support) should be flagged \
        as DISPUTED rather than resolved unilaterally.\n\
     9. On-time delivery is always NOT-IN-BREACH regardless of \
        circumstances.\n\
     10. The framework's job is to apply these rules to a given \
         contract scenario; if a scenario doesn't match any of the \
         above, default to flagging as DISPUTED for human review."
        .to_string()
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

/// Two-turn priming dispatch. Mirrors tsa-demo and clinical-demo:
/// turn 1 hands the model the rulebook with an ACK-only
/// instruction; turn 2 sends the contract clause and asks for a
/// verdict-first answer. Falls back to single-turn when no
/// rulebook is configured.
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
        "CONTRACT CLAUSE: {src}\n\nDoes the seller have to deliver the goods?",
        src = source_text,
    )
}

/// Build Arm A's single-turn system prompt. v0.3 changes the
/// format to put the VERDICT line FIRST so it survives truncation.
/// With a rulebook, also forbids invented rules.
pub fn build_raw_system_prompt(rulebook_text: Option<&str>) -> String {
    match rulebook_text {
        None => "You are a contract-review assistant. Given a contract clause, \
                 decide whether the obligation it describes is currently in force.\n\
                 \n\
                 Your response MUST begin with the verdict line as the very \
                 first line of output:\n\
                 \n\
                 VERDICT: OBLIGATION_HOLDS\n\
                 (or)\n\
                 VERDICT: OBLIGATION_EXCUSED\n\
                 \n\
                 After the verdict line, give 2-3 sentences of reasoning. The \
                 verdict-first format ensures the verdict is captured even if \
                 your reasoning is truncated."
            .to_string(),
        Some(text) => format!(
            "You are a contract-review assistant. Use only the rules \
             listed below. Do not invent or infer additional rules.\n\
             \n\
             {text}\n\
             \n\
             Given a contract clause, decide whether the obligation it \
             describes is currently in force.\n\
             \n\
             Your response MUST begin with the verdict line as the very \
             first line of output:\n\
             \n\
             VERDICT: OBLIGATION_HOLDS — only when a rule above \
             explicitly affirms the obligation.\n\
             VERDICT: OBLIGATION_EXCUSED — only when a rule above \
             explicitly excuses the obligation.\n\
             VERDICT: ESCALATE — <one sentence describing what a \
             supervisor needs to clarify> — when the rules above don't \
             cover the case, when you cannot evaluate a rule's \
             condition from the clause, or when the clause is \
             ambiguous.\n\
             \n\
             Important: silence is not permission. If no rule above \
             either explicitly affirms the obligation or explicitly \
             excuses it, the correct verdict is ESCALATE — not one of \
             the binary verdicts. Use ESCALATE whenever you would \
             otherwise need to reason beyond the rules above, \
             fabricate a rule that isn't listed, or default to one of \
             the binary verdicts because the rulebook doesn't resolve \
             the case.\n\
             \n\
             After the verdict line, give 2-3 sentences of reasoning \
             citing the specific rule number(s) that produced your \
             verdict (e.g., \"per rule N, ...\"). The verdict-first \
             format ensures the verdict is captured even if your \
             reasoning is truncated."
        ),
    }
}

/// Build the system prompt for the priming dispatch path. Same
/// role (contract-review assistant) but with explicit ground
/// rules about the two-turn protocol.
pub fn build_priming_system_prompt() -> String {
    "You are a contract-review assistant. You will receive \
     information in two turns:\n\
     \n\
     Turn 1: I will give you a contract-clause rulebook. Read it \
     carefully and store the rules in your working memory. Respond \
     with exactly the single word `ACK` and nothing else. Do NOT \
     summarise the rules, comment on them, or analyse them until I \
     ask my question in turn 2.\n\
     \n\
     Turn 2: I will give you a contract clause. Apply only the \
     rulebook from turn 1 — do not invent or infer additional \
     rules. Your response MUST begin with the verdict line as the \
     very first line of output:\n\
     \n\
     VERDICT: OBLIGATION_HOLDS — only when a rule from turn 1 \
     explicitly affirms the obligation.\n\
     VERDICT: OBLIGATION_EXCUSED — only when a rule from turn 1 \
     explicitly excuses the obligation.\n\
     VERDICT: ESCALATE — <one sentence describing what a supervisor \
     needs to clarify> — when no rule from turn 1 covers the case, \
     when you cannot evaluate a rule's condition from the clause, \
     or when the clause is ambiguous.\n\
     \n\
     Important: silence is not permission. If no rule from turn 1 \
     either explicitly affirms the obligation or explicitly excuses \
     it, the correct verdict is ESCALATE — not one of the binary \
     verdicts. Use ESCALATE whenever you would otherwise need to \
     reason beyond the rules from turn 1, fabricate a rule, or \
     default to one of the binary verdicts because the rulebook \
     doesn't resolve the case.\n\
     \n\
     After the verdict line, give 2-3 sentences of reasoning \
     citing the specific rule number(s) from the turn 1 rulebook. \
     The verdict-first format ensures the verdict is captured even \
     if your reasoning is truncated."
        .to_string()
}

/// Build the turn 1 user message for the priming path.
pub fn build_priming_turn1_user_prompt(rulebook_text: &str) -> String {
    format!(
        "TURN 1: RULEBOOK INTAKE.\n\
         \n\
         The following contract-clause rulebook applies to my next \
         question. Read it. Respond with `ACK` only.\n\
         \n\
         {rulebook_text}"
    )
}

/// Build the turn 2 user message for the priming path.
pub fn build_priming_turn2_user_prompt(source_text: &str) -> String {
    format!(
        "TURN 2: QUESTION.\n\
         \n\
         Apply the rulebook from turn 1. Contract clause: {source_text}\n\
         \n\
         Does the seller have to deliver the goods? Remember: first \
         line MUST be `VERDICT: OBLIGATION_HOLDS`, \
         `VERDICT: OBLIGATION_EXCUSED`, or \
         `VERDICT: ESCALATE — <reason>`. Silence in the rulebook is \
         not permission — ESCALATE if no rule covers the case."
    )
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

    // The v2 `exception_references_rule_via_part_of` and
    // `root_node_tiles_the_document` tests asserted on the obsolete
    // `part_of` field; ADJ01 v3 expresses Exception→Rule attachment
    // via `Excepts` edges instead. Equivalent tests will be added
    // when the contract demo emits the v3 graph IR directly (the
    // current path is HandBuilt and predates v3).

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
    fn fixture_rulebook_covers_canonical_clauses() {
        let rb = fixture_contract_rulebook();
        assert!(rb.contains("Force majeure"));
        assert!(rb.contains("Restatement"));
        assert!(rb.contains("DISPUTED"));
        // Stock-related exception rule for the canonical demo source.
        assert!(rb.contains("stock-related exception"));
    }

    #[test]
    fn raw_system_prompt_demands_verdict_first() {
        let no_rb = build_raw_system_prompt(None);
        assert!(no_rb.contains("first line"));
        assert!(no_rb.contains("truncated"));
        assert!(no_rb.contains("OBLIGATION_HOLDS"));
        assert!(no_rb.contains("OBLIGATION_EXCUSED"));

        let with_rb = build_raw_system_prompt(Some("rule x"));
        assert!(with_rb.contains("first line"));
        assert!(with_rb.contains("Do not invent"));
        assert!(with_rb.contains("citing the specific rule number"));
    }

    #[test]
    fn priming_system_prompt_describes_two_turn_protocol() {
        let s = build_priming_system_prompt();
        assert!(s.contains("Turn 1"));
        assert!(s.contains("Turn 2"));
        assert!(s.contains("ACK"));
        assert!(s.contains("OBLIGATION_HOLDS"));
        assert!(s.contains("OBLIGATION_EXCUSED"));
        assert!(s.contains("Do NOT"));
    }

    #[test]
    fn priming_turn1_user_prompt_embeds_rulebook_and_demands_ack() {
        let rb = "1. force-majeure clause.\n2. stock exception.";
        let s = build_priming_turn1_user_prompt(rb);
        assert!(s.contains("RULEBOOK INTAKE"));
        assert!(s.contains("ACK"));
        assert!(s.contains("force-majeure"));
    }

    #[test]
    fn priming_turn2_user_prompt_embeds_clause_and_restates_verdict_format() {
        let s = build_priming_turn2_user_prompt(
            "If the buyer pays within 30 days, the seller delivers the goods.",
        );
        assert!(s.contains("TURN 2"));
        assert!(s.contains("buyer pays"));
        assert!(s.contains("rulebook from turn 1"));
        assert!(s.contains("OBLIGATION_HOLDS"));
        assert!(s.contains("OBLIGATION_EXCUSED"));
    }

    #[test]
    fn config_with_priming_mode_is_addressable_via_struct_field() {
        let mut cfg = DemoConfig::default();
        cfg.arm_a_mode = ArmAMode::Priming;
        cfg.rulebook_text = Some(fixture_contract_rulebook());
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
    }

    // -----------------------------------------------------------------
    // v0.4 — ESCALATE verdict in with-rulebook prompts (mirrors tsa)
    // -----------------------------------------------------------------

    #[test]
    fn raw_system_prompt_no_rulebook_keeps_binary_verdict() {
        let s = build_raw_system_prompt(None);
        assert!(s.contains("VERDICT: OBLIGATION_HOLDS"));
        assert!(s.contains("VERDICT: OBLIGATION_EXCUSED"));
        assert!(!s.contains("VERDICT: ESCALATE"));
    }

    #[test]
    fn raw_system_prompt_with_rulebook_offers_escalate_verdict() {
        let s = build_raw_system_prompt(Some("RULES: 1. test."));
        assert!(s.contains("VERDICT: OBLIGATION_HOLDS"));
        assert!(s.contains("VERDICT: OBLIGATION_EXCUSED"));
        assert!(s.contains("VERDICT: ESCALATE"));
        assert!(s.contains("silence is not permission"));
    }

    #[test]
    fn priming_system_prompt_offers_escalate_verdict() {
        let s = build_priming_system_prompt();
        assert!(s.contains("VERDICT: OBLIGATION_HOLDS"));
        assert!(s.contains("VERDICT: OBLIGATION_EXCUSED"));
        assert!(s.contains("VERDICT: ESCALATE"));
        assert!(s.contains("silence is not permission"));
    }

    #[test]
    fn priming_turn2_user_prompt_lists_escalate_as_an_option() {
        let s = build_priming_turn2_user_prompt("test clause");
        assert!(s.contains("VERDICT: ESCALATE"));
        assert!(s.contains("ESCALATE if no rule covers the case"));
    }

    #[test]
    fn framework_instructions_do_not_leak_contract_specific_metaphors() {
        // Same discipline as tsa-demo and clinical-demo. The
        // ESCALATE / verdict-first instructions should be
        // domain-neutral.
        let with_rb = build_raw_system_prompt(Some("RULES: 1."));
        let priming = build_priming_system_prompt();

        for needle in &[
            "a real attorney",
            "a real lawyer",
            "a real contract-review officer",
            "consulting senior counsel",
            "escalate to legal",
        ] {
            assert!(
                !with_rb.contains(needle),
                "with-rulebook prompt should not invoke {needle:?}"
            );
            assert!(
                !priming.contains(needle),
                "priming prompt should not invoke {needle:?}"
            );
        }
    }
}
