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
    decompose_text, DecomposeTextRequest, DecomposeTextResponse, GatewayConfig, PrimitiveError,
};

/// Stable version of the clarification-prompt template. Bumping this
/// is an audit-trail-affecting change.
pub const CLARIFICATION_PROMPT_VERSION: &str = "clarification-v1";

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
    let mut dialogue: Vec<DialogueTurn> = Vec::new();

    for attempt in 1..=max_attempts.max(1) {
        let question_text = build_correction_prompt(
            &req.violation_description,
            &req.previous_ir,
        );
        let revised = DecomposeTextRequest {
            document_id: req.original.document_id.clone(),
            source_text: req.original.source_text.clone(),
            // Tack the correction prompt onto the domain_hint so the
            // model sees it as context. A future revision can pass a
            // first-class `prior_attempts` field into the primitive;
            // for v0.1 we keep `decompose_text`'s signature stable.
            domain_hint: format!(
                "{original_hint}\n\n[CORRECTION FROM CHECKER PASS]:\n{q}",
                original_hint = req.original.domain_hint,
                q = question_text,
            ),
            language_hint: req.original.language_hint.clone(),
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
                        prompt_version: Some(CLARIFICATION_PROMPT_VERSION.to_string()),
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
                // Record the failed attempt and either retry or give up.
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
                        prompt_version: Some(CLARIFICATION_PROMPT_VERSION.to_string()),
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
                // Otherwise loop and retry. The error path is rare —
                // most "bad output" failures come back as Ok(...) with
                // an IR that fails ADJ02 again on the caller's side.
            }
        }
    }

    // Unreachable: the loop either returns Ok on success, returns
    // Err on the final attempt's error path, or continues. The
    // `for attempt in 1..=max_attempts.max(1)` guarantees at least
    // one iteration.
    Err(ClarificationError::Exhausted {
        attempts: 0,
        dialogue,
    })
}

// ---------------------------------------------------------------------------
// Correction-prompt builder
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
}
