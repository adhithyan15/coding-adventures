// PrimitiveError carried inside ClarificationError::Primitive is a
// large variant; the same audit-trail discipline applies.
#![allow(clippy::result_large_err)]

//! # adjudication-clarification — ADJ06 clarification dialogue
//!
//! When a checker pass surfaces a violation, the framework's natural
//! response is *not* to give up — it's to **re-prompt the model with
//! the structured diagnostic and try again**. That's ADJ06.
//!
//! The crate's role is small but load-bearing: given a violation
//! (e.g., "your IR has a 1-byte coverage gap at byte 2"), it asks
//! the LLM to produce a corrected IR. The result is recorded as a
//! [`adjudication_audit_trail::DialogueTurn`] so the audit trail
//! captures every back-and-forth.
//!
//! ## Why this matters for small models
//!
//! A frontier model usually gets the IR right the first time. A
//! 7B-class local model often doesn't — it makes a small structural
//! error and then we'd be stuck with a Blocked verdict. ADJ06 turns
//! that situation around: the deterministic checkers tell the model
//! exactly what's wrong, the model tries again, the checkers re-run,
//! and (usually) the model gets it right the second or third time.
//! The model didn't get smarter; the system gave it feedback.
//!
//! This is the central mechanism the framework offers for "small
//! models doing extraordinary work" (per the project's design
//! principle).
//!
//! ## What v0.1 ships
//!
//! - [`retry_decompose_on_coverage_failure`] — the headline entry
//!   point. Takes the original `DecomposeTextRequest`, a list of
//!   coverage violations, the gateway, and a `max_attempts` budget;
//!   returns either a corrected IR + dialogue turns, or
//!   [`ClarificationError::Exhausted`] if the model still fails
//!   after `max_attempts`.
//! - Stable system-prompt template + version constant
//!   ([`CLARIFICATION_PROMPT_VERSION`]) so the audit trail records
//!   which version of the dialogue prompt produced each turn.
//! - One `DialogueTurn` emitted per retry, with the violation it
//!   was triggered by, the rung (always `Rung1ReprompT` at v0.1),
//!   the question text, and the model's response.
//!
//! ## What v0.1 deliberately does NOT do
//!
//! - **Other violation types.** ADJ03 polarity/modality, ADJ04
//!   round-trip drift, ADJ05 adversarial readings all have their
//!   own correction shapes. They'll get their own functions in
//!   follow-ups; v0.1 focuses on coverage because that's the most
//!   common small-model failure mode.
//! - **Rung 2 (different model) / Rung 3 (human).** v0.1 stays at
//!   Rung 1 (same model). ADJ06 spec's escalation policy is
//!   future work.

use adjudication_audit_trail::{
    DialogueOutcome, DialogueResponse, DialogueResponseSource, DialogueRung, DialogueTurn, TurnId,
};
use llm_primitives::{
    decompose_text, render_node, DecomposeTextRequest, DecomposeTextResponse, GatewayConfig,
    PrimitiveError, RenderNodeRequest, RenderNodeResponse,
};

/// Stable version of the coverage-clarification prompt template.
/// Bumping this is an audit-trail-affecting change.
pub const CLARIFICATION_PROMPT_VERSION: &str = "clarification-v1";

/// Stable version of the polarity/modality clarification-prompt
/// template (ADJ06-for-ADJ03). Distinct from
/// [`CLARIFICATION_PROMPT_VERSION`] so audit-trail replay can tell
/// the two correction flavours apart.
pub const POLARITY_CLARIFICATION_PROMPT_VERSION: &str = "polarity-clarification-v1";

/// Stable version of the round-trip drift clarification-prompt
/// template (ADJ06-for-ADJ04). The drift retry asks the renderer
/// to produce a CORRECTED rendering for ONE specific IR node, not
/// to re-extract the whole IR — distinct from coverage / polarity
/// retries, hence its own version.
pub const RENDER_CLARIFICATION_PROMPT_VERSION: &str = "render-clarification-v1";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// What the caller hands ADJ06 when ADJ02 found coverage problems.
#[derive(Debug, Clone)]
pub struct CoverageClarificationRequest {
    /// The original `decompose_text` request that produced the bad IR.
    pub original: DecomposeTextRequest,
    /// Description of the violation surfaced by ADJ02. The framework
    /// renders this verbatim into the correction prompt, so it
    /// should be human-actionable (e.g.,
    /// `"missing_ranges: [(2, 3)]"`).
    pub violation_description: String,
    /// The previous IR the model produced, as raw JSON. Included in
    /// the correction prompt so the model sees its own prior output
    /// and can edit rather than restart.
    pub previous_ir: serde_json::Value,
}

/// What ADJ06 returns on success.
#[derive(Debug, Clone)]
pub struct CoverageClarificationOutcome {
    /// The model's corrected IR (raw JSON; the caller's converter
    /// turns it into a typed `IRDocument`).
    pub corrected_ir: serde_json::Value,
    /// One `DialogueTurn` per retry attempt. Empty list means the
    /// first attempt succeeded (very rare — by definition we got
    /// here because the first attempt failed).
    pub dialogue: Vec<DialogueTurn>,
    /// Whether `corrected_ir` came from a successful retry, or from
    /// the original (if we never had to retry).
    pub used_attempts: usize,
}

/// Errors ADJ06 can return.
#[derive(Debug)]
pub enum ClarificationError {
    /// The model still failed to produce a valid response after
    /// `max_attempts`. The dialogue trail is returned so the caller
    /// can escalate (Rung 2 / Rung 3) with full context.
    Exhausted {
        attempts: usize,
        dialogue: Vec<DialogueTurn>,
    },
    /// The primitive itself errored mid-retry. Surfaced separately so
    /// the caller can distinguish "model produced bad output" from
    /// "the gateway is down".
    Primitive(PrimitiveError),
}

impl From<PrimitiveError> for ClarificationError {
    fn from(e: PrimitiveError) -> Self {
        ClarificationError::Primitive(e)
    }
}

impl std::fmt::Display for ClarificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClarificationError::Exhausted { attempts, .. } => {
                write!(
                    f,
                    "clarification dialogue exhausted after {attempts} attempt(s)"
                )
            }
            ClarificationError::Primitive(e) => write!(f, "primitive error: {e}"),
        }
    }
}

impl std::error::Error for ClarificationError {}

// ---------------------------------------------------------------------------
// Entry point: coverage retry
// ---------------------------------------------------------------------------

/// Ask the model to fix a coverage violation. Re-runs
/// `decompose_text` up to `max_attempts` times, each time prepending
/// a correction prompt that includes:
///
/// - The violation description (e.g.,
///   `RootsDoNotTileDocument { missing_ranges: [(2, 3)] }`).
/// - The model's previous IR JSON.
/// - The instruction "produce a new IR where every byte is covered".
///
/// Every attempt is recorded as a [`DialogueTurn`]. On the first
/// attempt that returns *some* IR (regardless of whether it's
/// correct), the function returns — the caller's pipeline will
/// re-run ADJ02 on the new IR and either accept it or call back into
/// this function. **This crate does NOT re-validate coverage** —
/// that's the pipeline's job. v0.1 keeps the loop simple: ask,
/// receive, hand back to the caller.
pub fn retry_decompose_on_coverage_failure(
    req: &CoverageClarificationRequest,
    gateway: &GatewayConfig,
    max_attempts: usize,
    now: impl Fn() -> String,
) -> Result<CoverageClarificationOutcome, ClarificationError> {
    retry_with_correction_prompt(
        &req.original,
        gateway,
        max_attempts,
        now,
        || build_correction_prompt(&req.violation_description, &req.previous_ir),
        CLARIFICATION_PROMPT_VERSION,
    )
}

// ---------------------------------------------------------------------------
// Entry point: polarity/modality retry (ADJ06-for-ADJ03)
// ---------------------------------------------------------------------------

/// What the caller hands ADJ06 when ADJ03 found polarity/modality
/// problems. Same shape as [`CoverageClarificationRequest`] plus a
/// hint about which node(s) the polarity is wrong on.
#[derive(Debug, Clone)]
pub struct PolarityClarificationRequest {
    /// The original `decompose_text` request that produced the bad IR.
    pub original: DecomposeTextRequest,
    /// Description of the violation surfaced by ADJ03 (e.g.,
    /// `"RuledOutMustBeAffirmed { node_id: F3, actual_polarity: Denied }"`).
    pub violation_description: String,
    /// The previous IR JSON for the model to edit.
    pub previous_ir: serde_json::Value,
    /// Optional human-readable hint about which node has the wrong
    /// polarity and why. The framework synthesizes this when it can
    /// — e.g., when the source contains an obvious negation like
    /// `"no known drug allergy"`, the hint can say `"node F3
    /// covers a negation; its polarity should be Denied, not Affirmed"`.
    pub polarity_hint: Option<String>,
}

/// Ask the model to fix a polarity/modality violation. Same retry
/// loop as the coverage version, different correction prompt.
///
/// Notable failure modes this catches:
/// - The model recorded `"no known drug allergy"` as Affirmed
///   (wrong) instead of Denied (right). Small models often miss
///   the negation.
/// - The model recorded a hypothetical clause as Present instead of
///   Hypothetical. Modality polarity, distinct from logical polarity.
/// - The model recorded a query node with Inherit polarity but no
///   ancestor to inherit from — ADJ03 catches this as
///   `InheritChainUnresolved`.
pub fn retry_decompose_on_polarity_failure(
    req: &PolarityClarificationRequest,
    gateway: &GatewayConfig,
    max_attempts: usize,
    now: impl Fn() -> String,
) -> Result<CoverageClarificationOutcome, ClarificationError> {
    retry_with_correction_prompt(
        &req.original,
        gateway,
        max_attempts,
        now,
        || {
            build_polarity_correction_prompt(
                &req.violation_description,
                &req.previous_ir,
                req.polarity_hint.as_deref(),
            )
        },
        POLARITY_CLARIFICATION_PROMPT_VERSION,
    )
}

// ---------------------------------------------------------------------------
// Entry point: round-trip drift retry (ADJ06-for-ADJ04)
// ---------------------------------------------------------------------------

/// Which direction(s) of the bidirectional entail check failed.
/// ADJ04 emits one `RoundTripDrift` violation per drifting node;
/// the violation detail records both directional scores against
/// the same threshold. This enum lets the correction prompt focus
/// on the actual failure mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftDirection {
    /// `source → rendering` score below threshold. The rendering
    /// claims something the source does NOT support — the model
    /// added or fabricated content.
    SourceToRendering,
    /// `rendering → source` score below threshold. The rendering
    /// OMITS something the source says — the model dropped detail.
    RenderingToSource,
    /// Both directions failed.
    Both,
}

impl DriftDirection {
    /// Construct from the two NLI scores + threshold. Returns
    /// `None` if neither direction is actually below threshold (the
    /// caller shouldn't be calling drift correction in that case).
    pub fn classify(source_to_rendering: f32, rendering_to_source: f32, threshold: f32) -> Option<Self> {
        let p_low = source_to_rendering < threshold;
        let h_low = rendering_to_source < threshold;
        match (p_low, h_low) {
            (true, true) => Some(DriftDirection::Both),
            (true, false) => Some(DriftDirection::SourceToRendering),
            (false, true) => Some(DriftDirection::RenderingToSource),
            (false, false) => None,
        }
    }

    fn description(&self) -> &'static str {
        match self {
            DriftDirection::SourceToRendering => {
                "Your rendering claimed something the source does NOT support \
                 (added or fabricated content)."
            }
            DriftDirection::RenderingToSource => {
                "Your rendering OMITTED something the source DOES say \
                 (dropped detail)."
            }
            DriftDirection::Both => {
                "Your rendering both added unsupported claims AND omitted \
                 source content (drift in both directions)."
            }
        }
    }
}

/// What the caller hands ADJ06 when ADJ04 found round-trip drift
/// on a specific IR node. Unlike the coverage / polarity retries,
/// the corrected output is a *single rendering string* for one
/// node — not a whole IR document.
#[derive(Debug, Clone)]
pub struct RenderClarificationRequest {
    /// The original `render_node` request (carries the IR node
    /// description, the document excerpt, and the render style).
    pub original: RenderNodeRequest,
    /// The previous rendering the model produced. Embedded in the
    /// correction prompt so the model can edit rather than restart.
    pub previous_rendering: String,
    /// Optional structured drift direction. When `Some`, the
    /// correction prompt focuses on the actual failure mode (e.g.,
    /// "you added content; trim it" vs "you dropped content; add
    /// it back"). When `None`, the prompt is generic.
    pub failing_direction: Option<DriftDirection>,
    /// Free-form description of the drift (e.g.,
    /// `"NLI scores: p→h=0.10, h→p=0.10 vs threshold 0.60"`).
    pub drift_description: String,
}

/// Outcome of a successful drift retry. Mirrors the coverage /
/// polarity outcome shape but carries a String (the corrected
/// rendering) instead of a JSON IR.
#[derive(Debug, Clone)]
pub struct RenderClarificationOutcome {
    /// The model's corrected rendering.
    pub corrected_rendering: String,
    /// One `DialogueTurn` per retry attempt.
    pub dialogue: Vec<DialogueTurn>,
    pub used_attempts: usize,
}

/// Ask the renderer to fix a round-trip drift on a single IR node.
/// Re-runs `render_node` up to `max_attempts` times. Same retry
/// loop as the coverage and polarity variants, but using
/// `render_node` instead of `decompose_text`.
///
/// **Caveat**: this crate does NOT re-validate the corrected
/// rendering against the source — that's the pipeline's job. We
/// hand the new rendering back; the caller's pipeline will re-run
/// ADJ04 on it and either accept it or call back into this
/// function.
pub fn retry_render_on_drift_failure(
    req: &RenderClarificationRequest,
    gateway: &GatewayConfig,
    max_attempts: usize,
    now: impl Fn() -> String,
) -> Result<RenderClarificationOutcome, ClarificationError> {
    let mut dialogue: Vec<DialogueTurn> = Vec::new();

    for attempt in 1..=max_attempts.max(1) {
        let question_text = build_render_correction_prompt(
            &req.drift_description,
            &req.previous_rendering,
            req.failing_direction,
        );

        // The render_node primitive uses `document_excerpt` as the
        // user-facing content. Prepend the correction notice so the
        // model sees it before the original instruction.
        let revised = RenderNodeRequest {
            node_description: req.original.node_description.clone(),
            document_excerpt: format!(
                "[CORRECTION FROM CHECKER PASS]:\n{q}\n\n[ORIGINAL SOURCE EXCERPT]:\n{src}",
                q = question_text,
                src = req.original.document_excerpt,
            ),
            style: req.original.style,
        };

        let at = now();
        let resp_result: Result<RenderNodeResponse, PrimitiveError> =
            render_node(&revised, gateway);

        match resp_result {
            Ok(resp) => {
                dialogue.push(DialogueTurn {
                    turn_id: TurnId(attempt as u64),
                    at,
                    triggering_violation: None,
                    rung: DialogueRung::Rung1ReprompT,
                    question_text,
                    response: DialogueResponse {
                        source: DialogueResponseSource::Llm,
                        text: resp.rendering.clone(),
                        actor_id: Some(format!(
                            "{vendor}/{family}",
                            vendor = resp.call_record.provider.vendor,
                            family = resp.call_record.provider.model_family,
                        )),
                        model_version: Some(resp.call_record.provider.model_version.clone()),
                        prompt_version: Some(RENDER_CLARIFICATION_PROMPT_VERSION.to_string()),
                        prompt_hash: Some(resp.call_record.prompt_hash.clone()),
                    },
                    outcome: DialogueOutcome::Resolved,
                });
                return Ok(RenderClarificationOutcome {
                    corrected_rendering: resp.rendering,
                    dialogue,
                    used_attempts: attempt,
                });
            }
            Err(e) => {
                dialogue.push(DialogueTurn {
                    turn_id: TurnId(attempt as u64),
                    at,
                    triggering_violation: None,
                    rung: DialogueRung::Rung1ReprompT,
                    question_text,
                    response: DialogueResponse {
                        source: DialogueResponseSource::Llm,
                        text: format!("(error) {e}"),
                        actor_id: None,
                        model_version: None,
                        prompt_version: Some(RENDER_CLARIFICATION_PROMPT_VERSION.to_string()),
                        prompt_hash: None,
                    },
                    outcome: DialogueOutcome::Abandoned,
                });
                if attempt >= max_attempts.max(1) {
                    return Err(ClarificationError::Exhausted {
                        attempts: attempt,
                        dialogue,
                    });
                }
            }
        }
    }
    Err(ClarificationError::Exhausted {
        attempts: 0,
        dialogue,
    })
}

// ---------------------------------------------------------------------------
// Shared retry-loop machinery
// ---------------------------------------------------------------------------

/// Inner retry loop, parameterised on (a) the correction-prompt
/// builder and (b) the prompt-version tag for the audit trail.
/// Both the coverage and polarity entry points delegate here.
fn retry_with_correction_prompt(
    original: &DecomposeTextRequest,
    gateway: &GatewayConfig,
    max_attempts: usize,
    now: impl Fn() -> String,
    build_prompt: impl Fn() -> String,
    prompt_version: &'static str,
) -> Result<CoverageClarificationOutcome, ClarificationError> {
    let mut dialogue: Vec<DialogueTurn> = Vec::new();

    for attempt in 1..=max_attempts.max(1) {
        let question_text = build_prompt();
        let revised = DecomposeTextRequest {
            document_id: original.document_id.clone(),
            source_text: original.source_text.clone(),
            domain_hint: format!(
                "{original_hint}\n\n[CORRECTION FROM CHECKER PASS]:\n{q}",
                original_hint = original.domain_hint,
                q = question_text,
            ),
            language_hint: original.language_hint.clone(),
        };

        let at = now();
        let resp_result: Result<DecomposeTextResponse, PrimitiveError> =
            decompose_text(&revised, gateway);

        match resp_result {
            Ok(resp) => {
                dialogue.push(DialogueTurn {
                    turn_id: TurnId(attempt as u64),
                    at,
                    triggering_violation: None,
                    rung: DialogueRung::Rung1ReprompT,
                    question_text,
                    response: DialogueResponse {
                        source: DialogueResponseSource::Llm,
                        text: resp.ir_document.to_string(),
                        actor_id: Some(format!(
                            "{vendor}/{family}",
                            vendor = resp.call_record.provider.vendor,
                            family = resp.call_record.provider.model_family,
                        )),
                        model_version: Some(resp.call_record.provider.model_version.clone()),
                        prompt_version: Some(prompt_version.to_string()),
                        prompt_hash: Some(resp.call_record.prompt_hash.clone()),
                    },
                    outcome: DialogueOutcome::Resolved,
                });
                return Ok(CoverageClarificationOutcome {
                    corrected_ir: resp.ir_document,
                    dialogue,
                    used_attempts: attempt,
                });
            }
            Err(e) => {
                dialogue.push(DialogueTurn {
                    turn_id: TurnId(attempt as u64),
                    at,
                    triggering_violation: None,
                    rung: DialogueRung::Rung1ReprompT,
                    question_text,
                    response: DialogueResponse {
                        source: DialogueResponseSource::Llm,
                        text: format!("(error) {e}"),
                        actor_id: None,
                        model_version: None,
                        prompt_version: Some(prompt_version.to_string()),
                        prompt_hash: None,
                    },
                    outcome: DialogueOutcome::Abandoned,
                });
                if attempt >= max_attempts.max(1) {
                    return Err(ClarificationError::Exhausted {
                        attempts: attempt,
                        dialogue,
                    });
                }
            }
        }
    }

    Err(ClarificationError::Exhausted {
        attempts: 0,
        dialogue,
    })
}

// ---------------------------------------------------------------------------
// Correction-prompt builders
// ---------------------------------------------------------------------------

fn build_correction_prompt(
    violation: &str,
    previous_ir: &serde_json::Value,
) -> String {
    let previous_pretty = serde_json::to_string_pretty(previous_ir)
        .unwrap_or_else(|_| previous_ir.to_string());
    format!(
        "Your previous IR was REJECTED by the ADJ02 coverage checker.\n\
         \n\
         Violation:\n  {violation}\n\
         \n\
         The coverage rule is non-negotiable: every byte of SOURCE \
         must be covered by exactly one non-Query node's source_spans. \
         Whitespace and punctuation count. If a byte is intentionally \
         outside the domain, assign it to a `Discarded` node with a \
         `discard_reason` like `Pleasantry` or `DocumentMetadata`.\n\
         \n\
         Your previous output was:\n\
         {previous_pretty}\n\
         \n\
         Produce a CORRECTED IR with the same `document_id`, fixing \
         the coverage gap. Same flat-array shape, same field names, \
         same rules as before.",
    )
}

fn build_polarity_correction_prompt(
    violation: &str,
    previous_ir: &serde_json::Value,
    polarity_hint: Option<&str>,
) -> String {
    let previous_pretty = serde_json::to_string_pretty(previous_ir)
        .unwrap_or_else(|_| previous_ir.to_string());
    let hint_block = polarity_hint
        .map(|h| format!("\nFramework hint:\n  {h}\n"))
        .unwrap_or_default();
    format!(
        "Your previous IR was REJECTED by the ADJ03 polarity/modality \
         checker.\n\
         \n\
         Violation:\n  {violation}\n\
         {hint_block}\n\
         The polarity / modality rules:\n\
         - `polarity` is one of `Affirmed` / `Denied` / `Uncertain` / `Inherit`.\n\
         - **Negations like \"no\", \"not\", \"never\", \"denied\", \"absent\"** \
         change the polarity. \"No known drug allergy\" → `Denied`, not \
         `Affirmed`.\n\
         - `modality` is one of `Present` / `Past` / `Future` / `Hypothetical` \
         / `FamilyHistory` / `RuledOut` / `Conditional` / `Inherit`.\n\
         - `Hypothetical` is for clauses like \"if X happens\" or \
         \"would have done Y\". `RuledOut` is for explicitly excluded \
         possibilities.\n\
         - `Inherit` defers to the parent node's effective value. If you \
         use `Inherit`, the node MUST have a `part_of` ancestor whose \
         effective polarity/modality resolves.\n\
         \n\
         Your previous output was:\n\
         {previous_pretty}\n\
         \n\
         Produce a CORRECTED IR with the same `document_id`, fixing \
         the polarity/modality. Keep the same shape; only touch the \
         polarity/modality fields (and `part_of` if the violation is \
         `InheritChainUnresolved`).",
    )
}

fn build_render_correction_prompt(
    drift_description: &str,
    previous_rendering: &str,
    failing_direction: Option<DriftDirection>,
) -> String {
    let direction_block = failing_direction
        .map(|d| format!("\nDiagnosis:\n  {d_desc}\n", d_desc = d.description()))
        .unwrap_or_default();
    format!(
        "Your previous rendering of this IR node was REJECTED by the \
         ADJ04 round-trip checker.\n\
         \n\
         Drift detail:\n  {drift_description}\n\
         {direction_block}\n\
         Your previous rendering was:\n\
         > {previous_rendering}\n\
         \n\
         The round-trip rule: your rendering must say EXACTLY what the \
         source span says — no more, no less. Do not add details the \
         source does not contain. Do not omit details the source does \
         contain. If the source is ambiguous, render the ambiguity \
         (e.g., \"the document mentions matches\" rather than \"matches \
         are prohibited\" or \"matches are allowed\").\n\
         \n\
         Produce a CORRECTED rendering that stays close to the source.",
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse,
        JsonSchema, LlmClient, LlmError, ProviderIdentity, TokenUsage,
    };
    use llm_primitives::Role;
    use std::sync::Mutex;

    fn extractor_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "opus-extractor".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    /// Scripted extractor: returns the next JSON value on each call.
    struct ScriptedExtractor {
        responses: Mutex<Vec<serde_json::Value>>,
    }

    impl ScriptedExtractor {
        fn new(values: Vec<serde_json::Value>) -> Self {
            Self {
                responses: Mutex::new(values.into_iter().rev().collect()),
            }
        }
    }

    impl LlmClient for ScriptedExtractor {
        fn identity(&self) -> ProviderIdentity {
            extractor_identity()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            unreachable!("decompose_text uses complete_json")
        }
        fn complete_json(
            &self,
            _r: CompletionRequest,
            _s: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            let parsed = self
                .responses
                .lock()
                .unwrap()
                .pop()
                .expect("ScriptedExtractor drained");
            let raw = parsed.to_string();
            Ok(CompletionJsonResponse {
                raw_text: raw,
                parsed,
                schema_valid: true,
                model: "opus-extractor".into(),
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                    cached_tokens: 0,
                },
                provider_id: extractor_identity(),
                latency_ms: 12,
                polyfill_used: false,
            })
        }
    }

    fn make_request() -> DecomposeTextRequest {
        DecomposeTextRequest {
            document_id: "doc1".into(),
            source_text: "1 carry-on bag, matches.".into(),
            domain_hint: "tsa-declaration".into(),
            language_hint: Some("en".into()),
        }
    }

    fn happy_ir() -> serde_json::Value {
        serde_json::json!({
            "document_id": "doc1",
            "nodes": [
                { "id": "N1", "kind": "Fact", "term": { "atom": "ok" },
                  "polarity": "Affirmed", "modality": "Present",
                  "source_spans": [{ "start": 0, "end": 24 }] }
            ]
        })
    }

    fn make_clock() -> impl Fn() -> String {
        let tick = std::cell::Cell::new(0u32);
        move || {
            let t = tick.get();
            tick.set(t + 1);
            format!("2026-05-12T00:00:{:02}Z", t.min(59))
        }
    }

    fn gateway_with(extractor: ScriptedExtractor) -> GatewayConfig {
        GatewayConfig::new().with_client(Role::Extractor, Box::new(extractor))
    }

    #[test]
    fn retry_returns_corrected_ir_on_first_success() {
        let gateway = gateway_with(ScriptedExtractor::new(vec![happy_ir()]));
        let req = CoverageClarificationRequest {
            original: make_request(),
            violation_description: "RootsDoNotTileDocument { missing_ranges: [(2, 3)] }".into(),
            previous_ir: serde_json::json!({ "document_id": "doc1", "nodes": [] }),
        };
        let out =
            retry_decompose_on_coverage_failure(&req, &gateway, 3, make_clock()).unwrap();
        assert_eq!(out.used_attempts, 1);
        assert_eq!(out.dialogue.len(), 1);
        assert_eq!(out.dialogue[0].rung, DialogueRung::Rung1ReprompT);
        assert!(matches!(out.dialogue[0].outcome, DialogueOutcome::Resolved));
        assert!(out.dialogue[0].question_text.contains("coverage"));
        assert!(out.dialogue[0]
            .question_text
            .contains("RootsDoNotTileDocument"));
        assert_eq!(out.corrected_ir["nodes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn correction_prompt_includes_violation_and_previous_ir() {
        let prev = serde_json::json!({ "nodes": [] });
        let p = build_correction_prompt("missing byte 2", &prev);
        assert!(p.contains("missing byte 2"));
        assert!(p.contains("Discarded"));
        assert!(p.contains("flat-array"));
    }

    #[test]
    fn dialogue_actor_id_records_provider_identity() {
        let gateway = gateway_with(ScriptedExtractor::new(vec![happy_ir()]));
        let req = CoverageClarificationRequest {
            original: make_request(),
            violation_description: "test".into(),
            previous_ir: serde_json::json!({}),
        };
        let out =
            retry_decompose_on_coverage_failure(&req, &gateway, 1, make_clock()).unwrap();
        let actor = out.dialogue[0].response.actor_id.as_deref().unwrap();
        assert_eq!(actor, "mock/opus-extractor");
    }

    #[test]
    fn dialogue_records_prompt_version_constant() {
        let gateway = gateway_with(ScriptedExtractor::new(vec![happy_ir()]));
        let req = CoverageClarificationRequest {
            original: make_request(),
            violation_description: "test".into(),
            previous_ir: serde_json::json!({}),
        };
        let out =
            retry_decompose_on_coverage_failure(&req, &gateway, 1, make_clock()).unwrap();
        assert_eq!(
            out.dialogue[0].response.prompt_version.as_deref(),
            Some(CLARIFICATION_PROMPT_VERSION)
        );
    }

    #[test]
    fn clarification_prompt_version_is_locked() {
        // Bumping is audit-trail-affecting; tracked here so any
        // change is a deliberate PR.
        assert_eq!(CLARIFICATION_PROMPT_VERSION, "clarification-v1");
    }

    #[test]
    fn exhaustion_returns_dialogue_with_abandoned_outcome() {
        // Scripted client returns an error on the first (only) attempt.
        struct AlwaysErr;
        impl LlmClient for AlwaysErr {
            fn identity(&self) -> ProviderIdentity {
                extractor_identity()
            }
            fn capabilities(&self) -> Capabilities {
                Capabilities::modern_frontier()
            }
            fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, LlmError> {
                unreachable!()
            }
            fn complete_json(
                &self,
                _r: CompletionRequest,
                _s: &JsonSchema,
            ) -> Result<CompletionJsonResponse, LlmError> {
                Err(LlmError::Transport {
                    provider: extractor_identity(),
                    detail: "simulated".into(),
                })
            }
        }
        let gateway = GatewayConfig::new().with_client(Role::Extractor, Box::new(AlwaysErr));
        let req = CoverageClarificationRequest {
            original: make_request(),
            violation_description: "test".into(),
            previous_ir: serde_json::json!({}),
        };
        let err =
            retry_decompose_on_coverage_failure(&req, &gateway, 2, make_clock()).unwrap_err();
        match err {
            ClarificationError::Exhausted { attempts, dialogue } => {
                assert_eq!(attempts, 2);
                assert_eq!(dialogue.len(), 2);
                assert!(matches!(
                    dialogue[0].outcome,
                    DialogueOutcome::Abandoned
                ));
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
    }

    #[test]
    fn second_attempt_succeeds_after_first_fails_with_bad_ir() {
        // The first call returns an IR that the caller would have
        // rejected (e.g., still has a gap). We don't validate
        // coverage in this crate — the caller does — so from the
        // perspective of this function, both responses are "Ok".
        // The point is: we return the first Ok and let the caller
        // decide. Two-attempt loops only fire when complete_json
        // itself returns Err.
        let gateway = gateway_with(ScriptedExtractor::new(vec![
            serde_json::json!({ "document_id": "doc1", "nodes": [] }),
            happy_ir(),
        ]));
        let req = CoverageClarificationRequest {
            original: make_request(),
            violation_description: "test".into(),
            previous_ir: serde_json::json!({}),
        };
        let out =
            retry_decompose_on_coverage_failure(&req, &gateway, 3, make_clock()).unwrap();
        // First Ok wins; only one dialogue turn recorded.
        assert_eq!(out.used_attempts, 1);
        assert_eq!(out.dialogue.len(), 1);
    }

    #[test]
    fn max_attempts_zero_is_treated_as_one() {
        let gateway = gateway_with(ScriptedExtractor::new(vec![happy_ir()]));
        let req = CoverageClarificationRequest {
            original: make_request(),
            violation_description: "test".into(),
            previous_ir: serde_json::json!({}),
        };
        let out =
            retry_decompose_on_coverage_failure(&req, &gateway, 0, make_clock()).unwrap();
        assert_eq!(out.used_attempts, 1);
    }

    // ---------------- ADJ06-for-ADJ03 polarity retry ----------------

    #[test]
    fn polarity_retry_returns_corrected_ir_on_first_success() {
        let gateway = gateway_with(ScriptedExtractor::new(vec![happy_ir()]));
        let req = PolarityClarificationRequest {
            original: make_request(),
            violation_description:
                "RuledOutMustBeAffirmed { node_id: F3, actual_polarity: Denied }".into(),
            previous_ir: serde_json::json!({ "document_id": "doc1", "nodes": [] }),
            polarity_hint: Some(
                "node F3 covers a negation; should be Denied, not Affirmed".into(),
            ),
        };
        let out =
            retry_decompose_on_polarity_failure(&req, &gateway, 3, make_clock()).unwrap();
        assert_eq!(out.used_attempts, 1);
        assert_eq!(out.dialogue.len(), 1);
        assert!(matches!(out.dialogue[0].outcome, DialogueOutcome::Resolved));
        assert!(out.dialogue[0].question_text.contains("polarity"));
        assert!(out.dialogue[0]
            .question_text
            .contains("RuledOutMustBeAffirmed"));
        // The hint should be embedded in the question.
        assert!(out.dialogue[0]
            .question_text
            .contains("covers a negation"));
        // And the prompt-version on the audit row should be the
        // polarity flavour, not the coverage one.
        assert_eq!(
            out.dialogue[0].response.prompt_version.as_deref(),
            Some(POLARITY_CLARIFICATION_PROMPT_VERSION)
        );
    }

    #[test]
    fn polarity_correction_prompt_works_without_hint() {
        let p = build_polarity_correction_prompt(
            "AmbiguousPolarity { node_id: N1 }",
            &serde_json::json!({}),
            None,
        );
        assert!(p.contains("polarity"));
        assert!(p.contains("Denied"));
        // No hint should be appended when caller passes None.
        assert!(!p.contains("Framework hint"));
    }

    #[test]
    fn polarity_correction_prompt_includes_hint_when_provided() {
        let p = build_polarity_correction_prompt(
            "AmbiguousPolarity { node_id: N1 }",
            &serde_json::json!({}),
            Some("node N1 looks negated"),
        );
        assert!(p.contains("Framework hint"));
        assert!(p.contains("looks negated"));
    }

    #[test]
    fn polarity_clarification_prompt_version_is_locked() {
        assert_eq!(
            POLARITY_CLARIFICATION_PROMPT_VERSION,
            "polarity-clarification-v1"
        );
    }

    #[test]
    fn polarity_retry_exhausts_gracefully_on_repeated_error() {
        struct AlwaysErr;
        impl LlmClient for AlwaysErr {
            fn identity(&self) -> ProviderIdentity {
                extractor_identity()
            }
            fn capabilities(&self) -> Capabilities {
                Capabilities::modern_frontier()
            }
            fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, LlmError> {
                unreachable!()
            }
            fn complete_json(
                &self,
                _r: CompletionRequest,
                _s: &JsonSchema,
            ) -> Result<CompletionJsonResponse, LlmError> {
                Err(LlmError::Transport {
                    provider: extractor_identity(),
                    detail: "simulated".into(),
                })
            }
        }
        let gateway = GatewayConfig::new().with_client(Role::Extractor, Box::new(AlwaysErr));
        let req = PolarityClarificationRequest {
            original: make_request(),
            violation_description: "test".into(),
            previous_ir: serde_json::json!({}),
            polarity_hint: None,
        };
        let err =
            retry_decompose_on_polarity_failure(&req, &gateway, 2, make_clock()).unwrap_err();
        match err {
            ClarificationError::Exhausted { attempts, dialogue } => {
                assert_eq!(attempts, 2);
                assert_eq!(dialogue.len(), 2);
                assert!(matches!(
                    dialogue[0].outcome,
                    DialogueOutcome::Abandoned
                ));
                // Audit version should still be the polarity flavour.
                assert_eq!(
                    dialogue[0].response.prompt_version.as_deref(),
                    Some(POLARITY_CLARIFICATION_PROMPT_VERSION)
                );
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
    }

    // ---------------- ADJ06-for-ADJ04 render-drift retry ----------------

    fn renderer_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "haiku-renderer".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    /// Scripted renderer that returns the next text response per call.
    struct ScriptedRenderer {
        responses: std::sync::Mutex<Vec<String>>,
    }

    impl ScriptedRenderer {
        fn new(responses: Vec<&str>) -> Self {
            Self {
                responses: std::sync::Mutex::new(
                    responses.into_iter().rev().map(String::from).collect(),
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
        fn complete(
            &self,
            _r: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            let text = self
                .responses
                .lock()
                .unwrap()
                .pop()
                .expect("ScriptedRenderer drained");
            Ok(CompletionResponse {
                text,
                model: "haiku-renderer".into(),
                usage: TokenUsage::default(),
                finish_reason: llm_gateway::FinishReason::Stop,
                provider_id: renderer_identity(),
                latency_ms: 1,
            })
        }
        fn complete_json(
            &self,
            _r: CompletionRequest,
            _s: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            unreachable!("render_node uses complete, not complete_json")
        }
    }

    fn make_render_request() -> RenderNodeRequest {
        RenderNodeRequest {
            node_description: "id=F2 kind=Fact polarity=Affirmed term=prohibited(matches)".into(),
            document_excerpt: "matches.".into(),
            style: llm_primitives::RenderStyle::Plain,
        }
    }

    fn gateway_with_renderer(renderer: ScriptedRenderer) -> GatewayConfig {
        GatewayConfig::new().with_client(Role::Renderer, Box::new(renderer))
    }

    #[test]
    fn render_retry_returns_corrected_text_on_first_success() {
        let gateway = gateway_with_renderer(ScriptedRenderer::new(vec![
            "The source mentions matches.",
        ]));
        let req = RenderClarificationRequest {
            original: make_render_request(),
            previous_rendering: "Matching is prohibited.".into(),
            failing_direction: Some(DriftDirection::SourceToRendering),
            drift_description: "p_to_h=0.10 vs threshold 0.60".into(),
        };
        let out = retry_render_on_drift_failure(&req, &gateway, 3, make_clock()).unwrap();
        assert_eq!(out.used_attempts, 1);
        assert_eq!(out.corrected_rendering, "The source mentions matches.");
        assert_eq!(out.dialogue.len(), 1);
        assert!(matches!(out.dialogue[0].outcome, DialogueOutcome::Resolved));
        // Audit row should carry the render flavour, not coverage/polarity.
        assert_eq!(
            out.dialogue[0].response.prompt_version.as_deref(),
            Some(RENDER_CLARIFICATION_PROMPT_VERSION)
        );
        // The correction prompt should mention round-trip drift + the
        // direction-specific diagnosis.
        assert!(out.dialogue[0].question_text.contains("round-trip"));
        assert!(out.dialogue[0]
            .question_text
            .contains("added or fabricated content"));
    }

    #[test]
    fn render_correction_prompt_focuses_on_direction_when_provided() {
        let p = build_render_correction_prompt(
            "test drift",
            "previous rendering",
            Some(DriftDirection::RenderingToSource),
        );
        assert!(p.contains("OMITTED"));
        assert!(p.contains("dropped detail"));
    }

    #[test]
    fn render_correction_prompt_handles_both_directions() {
        let p = build_render_correction_prompt(
            "test drift",
            "previous rendering",
            Some(DriftDirection::Both),
        );
        assert!(p.contains("both"));
    }

    #[test]
    fn render_correction_prompt_omits_diagnosis_block_without_direction() {
        let p = build_render_correction_prompt(
            "test drift",
            "previous rendering",
            None,
        );
        assert!(!p.contains("Diagnosis:"));
        assert!(p.contains("round-trip"));
    }

    #[test]
    fn drift_direction_classify_resolves_each_case() {
        // Both directions below threshold → Both.
        assert_eq!(
            DriftDirection::classify(0.10, 0.10, 0.60),
            Some(DriftDirection::Both)
        );
        // Only source→rendering below.
        assert_eq!(
            DriftDirection::classify(0.10, 0.90, 0.60),
            Some(DriftDirection::SourceToRendering)
        );
        // Only rendering→source below.
        assert_eq!(
            DriftDirection::classify(0.90, 0.10, 0.60),
            Some(DriftDirection::RenderingToSource)
        );
        // Both above → None (not actually drifting).
        assert_eq!(DriftDirection::classify(0.90, 0.90, 0.60), None);
    }

    #[test]
    fn render_clarification_prompt_version_is_locked() {
        assert_eq!(
            RENDER_CLARIFICATION_PROMPT_VERSION,
            "render-clarification-v1"
        );
    }

    #[test]
    fn render_retry_exhausts_gracefully_on_repeated_error() {
        struct AlwaysErr;
        impl LlmClient for AlwaysErr {
            fn identity(&self) -> ProviderIdentity {
                renderer_identity()
            }
            fn capabilities(&self) -> Capabilities {
                Capabilities::modern_frontier()
            }
            fn complete(
                &self,
                _r: CompletionRequest,
            ) -> Result<CompletionResponse, LlmError> {
                Err(LlmError::Transport {
                    provider: renderer_identity(),
                    detail: "simulated".into(),
                })
            }
            fn complete_json(
                &self,
                _r: CompletionRequest,
                _s: &JsonSchema,
            ) -> Result<CompletionJsonResponse, LlmError> {
                unreachable!()
            }
        }
        let gateway = GatewayConfig::new().with_client(Role::Renderer, Box::new(AlwaysErr));
        let req = RenderClarificationRequest {
            original: make_render_request(),
            previous_rendering: "x".into(),
            failing_direction: None,
            drift_description: "test".into(),
        };
        let err = retry_render_on_drift_failure(&req, &gateway, 2, make_clock()).unwrap_err();
        match err {
            ClarificationError::Exhausted { attempts, dialogue } => {
                assert_eq!(attempts, 2);
                assert_eq!(dialogue.len(), 2);
                assert!(matches!(
                    dialogue[0].outcome,
                    DialogueOutcome::Abandoned
                ));
                assert_eq!(
                    dialogue[0].response.prompt_version.as_deref(),
                    Some(RENDER_CLARIFICATION_PROMPT_VERSION)
                );
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
    }
}
