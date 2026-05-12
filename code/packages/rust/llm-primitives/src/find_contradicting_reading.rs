//! # `find_contradicting_reading` — ADJ05 adversary primitive
//!
//! Fourth concrete primitive from
//! [LM00b §"find_contradicting_reading"](../../../specs/LM00b-llm-primitives.md).
//! Given a source span and the IR's natural-language rendering of that
//! span, find the strongest reading of the source that **contradicts**
//! the IR — or report `CONCURS` if no plausible contradiction exists.
//!
//! ## Role in ADJ05
//!
//! ADJ05 (adversarial verifier) samples IR nodes and, for each, runs:
//!
//!   1. [`render_node`](crate::render_node) — render the IR back into
//!      text.
//!   2. **This primitive** — adversarially read SOURCE against the
//!      rendered IR.
//!   3. [`judge_plausibility`](crate::judge_plausibility) — decide
//!      whether the adversary's reading is one a competent practitioner
//!      would adopt.
//!
//! The prompt is intentionally **asymmetric** — "assume the extraction
//! is wrong, find a reading that contradicts it". A symmetric "review
//! this" prompt produces boring agreement. Asymmetry is the whole point.
//!
//! ## ADJ05 independence requirement
//!
//! ADJ05 requires the `Adversary` and `Extractor` roles to come from
//! different model families. The primitive doesn't enforce this — it's
//! a deployment-time concern surfaced by
//! [`crate::GatewayConfig::check_independence`]. The framework's only
//! hard rule lives there.

use llm_gateway::{
    CompletionRequest, JsonSchema, LlmClient, Message, MessageContent, Role as MsgRole,
};

use crate::{
    fingerprint_prompt, GatewayConfig, LlmCallRecord, PrimitiveError, Role, ADVERSARY_PROMPT_VERSION,
};

/// Inputs to [`find_contradicting_reading`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindContradictingReadingRequest {
    /// The relevant slice of the source document, verbatim.
    pub source_span_text: String,
    /// The IR's natural-language rendering of that span — produced by
    /// `render_node` upstream.
    pub ir_rendered: String,
    /// Free-text domain hint (e.g., `"clinical-note"`, `"tsa-declaration"`).
    /// The LM00b spec carries a typed `DomainHints` enum; v0.4 of this
    /// crate keeps it a string to avoid binding to a not-yet-existent
    /// type. A future minor version will swap.
    pub domain_hint: String,
}

/// Outcome of one [`find_contradicting_reading`] call.
#[derive(Debug, Clone, PartialEq)]
pub enum FindContradictingReadingResponse {
    /// No plausible alternative reading was found — the adversary
    /// agrees with the IR. ADJ05 records this in the audit trail and
    /// moves on; no clarification is triggered.
    Concurs { call_record: LlmCallRecord },
    /// The adversary's strongest alternative reading, plus a short
    /// explanation of how it diverges from the IR. ADJ05 hands this
    /// to [`judge_plausibility`](crate::judge_plausibility) next.
    Reading {
        text: String,
        explanation: String,
        call_record: LlmCallRecord,
    },
}

impl FindContradictingReadingResponse {
    /// Convenience: returns the wrapped `LlmCallRecord` so callers
    /// don't have to pattern-match just to log.
    pub fn call_record(&self) -> &LlmCallRecord {
        match self {
            FindContradictingReadingResponse::Concurs { call_record } => call_record,
            FindContradictingReadingResponse::Reading { call_record, .. } => call_record,
        }
    }
}

const SYSTEM_PROMPT: &str = "\
You are an adversarial reader. You will see a SOURCE excerpt and an \
IR-RENDERED interpretation of it. Your task: assume the IR is wrong. \
Find the strongest reading of SOURCE that contradicts IR-RENDERED.\n\
\n\
The reading must be a reasonable interpretation a careful reader of \
SOURCE could legitimately reach in the named DOMAIN — not a stretch, \
not a hypothetical. If no such contradicting reading exists, answer \
with `concurs: true` and leave `text` and `explanation` empty.\n\
\n\
When you find one, set `concurs: false` and fill `text` with the \
alternative reading (one sentence, same style as IR-RENDERED) plus a \
short `explanation` of how it diverges from IR-RENDERED.";

const RESPONSE_SCHEMA: &str = r#"{
    "type": "object",
    "required": ["concurs", "text", "explanation"],
    "properties": {
        "concurs":     { "type": "boolean" },
        "text":        { "type": "string", "maxLength": 1024 },
        "explanation": { "type": "string", "maxLength": 1024 }
    },
    "additionalProperties": false
}"#;

fn build_user_text(req: &FindContradictingReadingRequest) -> String {
    format!(
        "DOMAIN: {domain}\n\n\
         SOURCE:\n{src}\n\n\
         IR-RENDERED:\n{ir}\n\n\
         Return the JSON object now.",
        domain = req.domain_hint,
        src = req.source_span_text,
        ir = req.ir_rendered,
    )
}

fn build_completion_request(
    client: &dyn LlmClient,
    req: &FindContradictingReadingRequest,
) -> CompletionRequest {
    CompletionRequest {
        model: client.identity().model_family.clone(),
        system: Some(SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(build_user_text(req)),
        }],
        // Default deterministic; deployments override at the gateway
        // layer if they want sampling.
        temperature: 0.0,
        max_tokens: Some(512),
        stop_sequences: Vec::new(),
        seed: None,
        metadata: Default::default(),
    }
}

/// Ask the `Adversary` role for the strongest contradicting reading
/// of the source span against the IR's rendering.
///
/// Returns [`PrimitiveError::NoClientForRole`] when no `Adversary`
/// client is registered, [`PrimitiveError::Gateway`] on transport
/// failures, and [`PrimitiveError::ValidationExhausted`] when the
/// LLM's JSON output is malformed or self-contradictory (e.g.,
/// `concurs: false` with an empty `text`).
pub fn find_contradicting_reading(
    req: &FindContradictingReadingRequest,
    gateway: &GatewayConfig,
) -> Result<FindContradictingReadingResponse, PrimitiveError> {
    let client = gateway
        .client(Role::Adversary)
        .ok_or(PrimitiveError::NoClientForRole {
            role: Role::Adversary,
        })?;

    let completion_req = build_completion_request(client, req);
    let prompt_hash = fingerprint_prompt(&completion_req);

    let schema = JsonSchema {
        name: "FindContradictingReadingResponse".to_string(),
        schema_json: RESPONSE_SCHEMA.to_string(),
    };

    let json_resp = client
        .complete_json(completion_req, &schema)
        .map_err(PrimitiveError::Gateway)?;

    let v = &json_resp.parsed;
    let concurs = v.get("concurs").and_then(|x| x.as_bool());
    let text = v
        .get("text")
        .and_then(|x| x.as_str())
        .map(str::to_string);
    let explanation = v
        .get("explanation")
        .and_then(|x| x.as_str())
        .map(str::to_string);

    let (Some(concurs), Some(text), Some(explanation)) = (concurs, text, explanation) else {
        return Err(PrimitiveError::ValidationExhausted {
            last_response: json_resp.raw_text,
            last_error: "missing or wrong-typed field in adversary response".to_string(),
            attempts: 1,
        });
    };

    let call_record = LlmCallRecord {
        primitive: "find_contradicting_reading".to_string(),
        role: Role::Adversary.as_str().to_string(),
        prompt_version: ADVERSARY_PROMPT_VERSION.to_string(),
        prompt_hash,
        provider: json_resp.provider_id,
        usage: json_resp.usage,
        finish_reason: llm_gateway::FinishReason::Stop,
        latency_ms: json_resp.latency_ms,
        cost_usd: 0.0,
    };

    let text_trimmed = text.trim().to_string();
    let explanation_trimmed = explanation.trim().to_string();

    if concurs {
        // CONCURS: text and explanation may be empty by spec, so we
        // don't fail on missing content here.
        Ok(FindContradictingReadingResponse::Concurs { call_record })
    } else {
        // The adversary claims a contradicting reading but didn't
        // populate either field — that's structurally inconsistent.
        // ADJ06 should surface this for clarification rather than
        // silently treating it as Concurs.
        if text_trimmed.is_empty() || explanation_trimmed.is_empty() {
            return Err(PrimitiveError::ValidationExhausted {
                last_response: json_resp.raw_text,
                last_error: "adversary set concurs=false but left text or explanation empty"
                    .to_string(),
                attempts: 1,
            });
        }
        Ok(FindContradictingReadingResponse::Reading {
            text: text_trimmed,
            explanation: explanation_trimmed,
            call_record,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, JsonSchema, LlmClient, LlmError,
        MockLlmClient, ProviderIdentity, TokenUsage,
    };
    use std::sync::Mutex;

    fn adv_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "adv-frontier".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    struct ScriptedJson {
        identity: ProviderIdentity,
        response: Mutex<Option<Result<CompletionJsonResponse, LlmError>>>,
    }

    impl ScriptedJson {
        fn new(value: serde_json::Value) -> Self {
            let raw_text = value.to_string();
            Self {
                identity: adv_identity(),
                response: Mutex::new(Some(Ok(CompletionJsonResponse {
                    raw_text,
                    parsed: value,
                    schema_valid: true,
                    model: "adv-frontier".into(),
                    usage: TokenUsage {
                        input_tokens: 110,
                        output_tokens: 18,
                        cached_tokens: 0,
                    },
                    provider_id: adv_identity(),
                    latency_ms: 64,
                    polyfill_used: false,
                }))),
            }
        }

        fn with_error(err: LlmError) -> Self {
            Self {
                identity: adv_identity(),
                response: Mutex::new(Some(Err(err))),
            }
        }
    }

    impl LlmClient for ScriptedJson {
        fn identity(&self) -> ProviderIdentity {
            self.identity.clone()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(
            &self,
            _req: CompletionRequest,
        ) -> Result<llm_gateway::CompletionResponse, LlmError> {
            unreachable!("find_contradicting_reading uses complete_json")
        }
        fn complete_json(
            &self,
            _req: CompletionRequest,
            _schema: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            self.response
                .lock()
                .unwrap()
                .take()
                .expect("ScriptedJson::complete_json called more than once")
        }
    }

    fn req() -> FindContradictingReadingRequest {
        FindContradictingReadingRequest {
            source_span_text: "Patient denies chest pain.".into(),
            ir_rendered: "The patient has no chest pain.".into(),
            domain_hint: "clinical-note".into(),
        }
    }

    #[test]
    fn missing_adversary_client_returns_no_client_for_role() {
        let g = GatewayConfig::new();
        let err = find_contradicting_reading(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::NoClientForRole { role } => assert_eq!(role, Role::Adversary),
            other => panic!("expected NoClientForRole, got {other:?}"),
        }
    }

    #[test]
    fn concurs_response_is_recognized() {
        let mock = ScriptedJson::new(serde_json::json!({
            "concurs": true,
            "text": "",
            "explanation": "",
        }));
        let g = GatewayConfig::new().with_client(Role::Adversary, Box::new(mock));
        match find_contradicting_reading(&req(), &g).unwrap() {
            FindContradictingReadingResponse::Concurs { call_record } => {
                assert_eq!(call_record.primitive, "find_contradicting_reading");
                assert_eq!(call_record.role, "adversary");
                assert_eq!(call_record.prompt_version, "adversary-v1");
                assert!(!call_record.prompt_hash.is_empty());
            }
            other => panic!("expected Concurs, got {other:?}"),
        }
    }

    #[test]
    fn reading_response_carries_text_and_explanation() {
        let mock = ScriptedJson::new(serde_json::json!({
            "concurs": false,
            "text": "The patient denied chest pain at this visit but had it last week.",
            "explanation": "SOURCE is silent on timing; a careful reader might infer a recent episode the IR ignores.",
        }));
        let g = GatewayConfig::new().with_client(Role::Adversary, Box::new(mock));
        match find_contradicting_reading(&req(), &g).unwrap() {
            FindContradictingReadingResponse::Reading {
                text,
                explanation,
                call_record,
            } => {
                assert!(text.contains("last week"));
                assert!(explanation.contains("careful reader"));
                assert_eq!(call_record.usage.output_tokens, 18);
                assert_eq!(call_record.latency_ms, 64);
            }
            other => panic!("expected Reading, got {other:?}"),
        }
    }

    #[test]
    fn user_message_tags_source_ir_and_domain_separately() {
        let r = req();
        let text = build_user_text(&r);
        assert!(text.contains("DOMAIN: clinical-note"));
        assert!(text.contains("SOURCE:\nPatient denies chest pain."));
        assert!(text.contains("IR-RENDERED:\nThe patient has no chest pain."));
    }

    #[test]
    fn gateway_refused_propagates_as_gateway_variant() {
        let mock = ScriptedJson::with_error(LlmError::Refused {
            provider: adv_identity(),
            reason: Some("safety filter".into()),
        });
        let g = GatewayConfig::new().with_client(Role::Adversary, Box::new(mock));
        let err = find_contradicting_reading(&req(), &g).unwrap_err();
        assert!(matches!(
            err,
            PrimitiveError::Gateway(LlmError::Refused { .. })
        ));
    }

    #[test]
    fn missing_concurs_field_returns_validation_exhausted() {
        let mock = ScriptedJson::new(serde_json::json!({
            "text": "x",
            "explanation": "y",
        }));
        let g = GatewayConfig::new().with_client(Role::Adversary, Box::new(mock));
        let err = find_contradicting_reading(&req(), &g).unwrap_err();
        assert!(matches!(err, PrimitiveError::ValidationExhausted { .. }));
    }

    #[test]
    fn wrong_typed_concurs_returns_validation_exhausted() {
        let mock = ScriptedJson::new(serde_json::json!({
            "concurs": "maybe",
            "text": "x",
            "explanation": "y",
        }));
        let g = GatewayConfig::new().with_client(Role::Adversary, Box::new(mock));
        let err = find_contradicting_reading(&req(), &g).unwrap_err();
        assert!(matches!(err, PrimitiveError::ValidationExhausted { .. }));
    }

    #[test]
    fn concurs_false_with_empty_text_returns_validation_exhausted() {
        // Structural inconsistency: the model claimed a contradicting
        // reading exists but didn't supply one. ADJ06 surfaces this.
        let mock = ScriptedJson::new(serde_json::json!({
            "concurs": false,
            "text": "  \t\n",
            "explanation": "explanation present but text empty",
        }));
        let g = GatewayConfig::new().with_client(Role::Adversary, Box::new(mock));
        let err = find_contradicting_reading(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::ValidationExhausted { last_error, .. } => {
                assert!(last_error.contains("concurs=false"));
            }
            other => panic!("expected ValidationExhausted, got {other:?}"),
        }
    }

    #[test]
    fn concurs_false_with_empty_explanation_returns_validation_exhausted() {
        let mock = ScriptedJson::new(serde_json::json!({
            "concurs": false,
            "text": "alt reading",
            "explanation": "",
        }));
        let g = GatewayConfig::new().with_client(Role::Adversary, Box::new(mock));
        let err = find_contradicting_reading(&req(), &g).unwrap_err();
        assert!(matches!(err, PrimitiveError::ValidationExhausted { .. }));
    }

    #[test]
    fn reading_response_trims_text_and_explanation() {
        let mock = ScriptedJson::new(serde_json::json!({
            "concurs": false,
            "text": "  trimmed text  \n",
            "explanation": "\nleading newline",
        }));
        let g = GatewayConfig::new().with_client(Role::Adversary, Box::new(mock));
        match find_contradicting_reading(&req(), &g).unwrap() {
            FindContradictingReadingResponse::Reading { text, explanation, .. } => {
                assert_eq!(text, "trimmed text");
                assert_eq!(explanation, "leading newline");
            }
            other => panic!("expected Reading, got {other:?}"),
        }
    }

    #[test]
    fn call_record_prompt_hash_matches_built_request() {
        let stub: Box<dyn LlmClient> =
            Box::new(MockLlmClient::new().with_identity(adv_identity()));
        let cr = build_completion_request(stub.as_ref(), &req());
        let expected_hash = crate::fingerprint_prompt(&cr);

        let mock = ScriptedJson::new(serde_json::json!({
            "concurs": true,
            "text": "",
            "explanation": "",
        }));
        let g = GatewayConfig::new().with_client(Role::Adversary, Box::new(mock));
        let resp = find_contradicting_reading(&req(), &g).unwrap();
        assert_eq!(resp.call_record().prompt_hash, expected_hash);
    }
}
