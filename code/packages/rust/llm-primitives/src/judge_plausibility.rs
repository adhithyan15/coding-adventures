//! # `judge_plausibility` — decide whether an adversary's reading is plausible
//!
//! Third concrete primitive from
//! [LM00b §"judge_plausibility"](../../../specs/LM00b-llm-primitives.md).
//! Given the source span, the IR's rendering, and an adversarial
//! reading produced by [`find_contradicting_reading`](crate::Role::Adversary),
//! decide whether a competent practitioner in the domain would
//! actually interpret the source the adversary's way.
//!
//! ## Role in ADJ05 (the adversarial verifier)
//!
//! ADJ05 runs a three-step check on every IR node it samples:
//!
//!   1. **Render** the node back into natural language (via
//!      [`render_node`](crate::render_node)).
//!   2. **Find a contradicting reading** of the source (via
//!      `find_contradicting_reading`, a separate PR).
//!   3. **Judge** whether the contradicting reading is *plausible*
//!      — this primitive.
//!
//! The judge prevents the adversary from winning by being silly.
//! An adversary that finds e.g. "the patient is a sentient cloud"
//! contradicts the IR, but a competent practitioner would not
//! adopt that reading; the judge must say IMPLAUSIBLE. The decision
//! is *binary* (plausible / not) plus a short rationale.
//!
//! An `IMPLAUSIBLE` verdict logs the adversary's reading in the
//! audit trail but does **not** fail the adjudication. A `PLAUSIBLE`
//! verdict surfaces as an `AdversarialReading` violation for ADJ06
//! to clarify with the user.

use llm_gateway::{
    CompletionRequest, JsonSchema, LlmClient, Message, MessageContent, Role as MsgRole,
};

use crate::{
    fingerprint_prompt, GatewayConfig, LlmCallRecord, PrimitiveError, Role,
    PLAUSIBILITY_PROMPT_VERSION,
};

/// Inputs to [`judge_plausibility`]. The judge sees the source, the
/// IR's rendering, and the adversary's proposed alternative reading —
/// enough to decide whether the alternative is one a competent reader
/// would actually adopt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgePlausibilityRequest {
    /// The relevant slice of the source document, verbatim.
    pub source_span_text: String,
    /// The IR's natural-language rendering of the same span.
    pub ir_rendered: String,
    /// The adversary's proposed alternative reading.
    pub adversary_reading: String,
    /// Free-text domain hint (e.g., "clinical-note", "tsa-declaration").
    /// The framework's LM00b spec defines a `DomainHints` enum; this
    /// primitive takes the string form at v0.2 to avoid binding to a
    /// not-yet-existent type. v0.3 will swap to the enum.
    pub domain_hint: String,
}

/// Result of one [`judge_plausibility`] call.
#[derive(Debug, Clone, PartialEq)]
pub struct JudgePlausibilityResponse {
    /// `true` iff a competent practitioner in the named domain would
    /// actually interpret the source the adversary's way.
    pub plausible: bool,
    /// Short rationale for the verdict. Always populated, even on a
    /// `plausible: false` answer — the audit trail records *why*
    /// the judge rejected the adversary.
    pub reason: String,
    pub call_record: LlmCallRecord,
}

const SYSTEM_PROMPT: &str = "\
You are a calibrated domain expert acting as a tie-breaker. You will \
see a SOURCE excerpt, an IR-RENDERED reading of it, and an ADVERSARY \
alternative reading that purports to contradict the IR. Decide one \
thing: would a competent practitioner in the named DOMAIN actually \
interpret SOURCE the way the adversary does?\n\
\n\
Answer `plausible: true` only when the adversary's reading is one a \
careful reader of SOURCE could legitimately reach. Answer \
`plausible: false` when the adversary's reading is fanciful, ignores \
domain conventions, or requires reading information into SOURCE that \
isn't there. Provide a short reason (one sentence) in either case.";

const RESPONSE_SCHEMA: &str = r#"{
    "type": "object",
    "required": ["plausible", "reason"],
    "properties": {
        "plausible": { "type": "boolean" },
        "reason":    { "type": "string", "minLength": 1, "maxLength": 1024 }
    },
    "additionalProperties": false
}"#;

fn build_user_text(req: &JudgePlausibilityRequest) -> String {
    format!(
        "DOMAIN: {domain}\n\n\
         SOURCE:\n{src}\n\n\
         IR-RENDERED:\n{ir}\n\n\
         ADVERSARY:\n{adv}\n\n\
         Return the JSON object now.",
        domain = req.domain_hint,
        src = req.source_span_text,
        ir = req.ir_rendered,
        adv = req.adversary_reading,
    )
}

fn build_completion_request(
    client: &dyn LlmClient,
    req: &JudgePlausibilityRequest,
) -> CompletionRequest {
    CompletionRequest {
        model: client.identity().model_family.clone(),
        system: Some(SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(build_user_text(req)),
        }],
        temperature: 0.0,
        // 2048 leaves headroom for thinking-mode chains-of-thought.
        max_tokens: Some(2048),
        stop_sequences: Vec::new(),
        seed: None,
        metadata: Default::default(),
    }
}

/// Binary plausibility judge for ADJ05. Looks up `Role::Plausibility`
/// on the gateway; returns [`PrimitiveError::NoClientForRole`] when
/// absent, [`PrimitiveError::Gateway`] on transport / auth failures,
/// and [`PrimitiveError::ValidationExhausted`] when the LLM's JSON
/// output is missing required fields, wrong-typed, or has an empty
/// reason.
pub fn judge_plausibility(
    req: &JudgePlausibilityRequest,
    gateway: &GatewayConfig,
) -> Result<JudgePlausibilityResponse, PrimitiveError> {
    let client = gateway
        .client(Role::Plausibility)
        .ok_or(PrimitiveError::NoClientForRole {
            role: Role::Plausibility,
        })?;

    let completion_req = build_completion_request(client, req);
    let prompt_hash = fingerprint_prompt(&completion_req);

    let schema = JsonSchema {
        name: "JudgePlausibilityResponse".to_string(),
        schema_json: RESPONSE_SCHEMA.to_string(),
    };

    let json_resp =
        crate::complete_json_with_truncation_retry(client, completion_req, &schema)
            .map_err(PrimitiveError::Gateway)?;

    let v = &json_resp.parsed;
    let plausible = v.get("plausible").and_then(|x| x.as_bool());
    let reason = v.get("reason").and_then(|x| x.as_str()).map(str::to_string);

    let (Some(plausible), Some(reason)) = (plausible, reason) else {
        return Err(PrimitiveError::ValidationExhausted {
            last_response: json_resp.raw_text,
            last_error: "missing or wrong-typed field in plausibility response".to_string(),
            attempts: 1,
        });
    };

    let reason_trimmed = reason.trim().to_string();
    if reason_trimmed.is_empty() {
        return Err(PrimitiveError::ValidationExhausted {
            last_response: json_resp.raw_text,
            last_error: "plausibility judge returned empty reason".to_string(),
            attempts: 1,
        });
    }

    let call_record = LlmCallRecord {
        primitive: "judge_plausibility".to_string(),
        role: Role::Plausibility.as_str().to_string(),
        prompt_version: PLAUSIBILITY_PROMPT_VERSION.to_string(),
        prompt_hash,
        provider: json_resp.provider_id,
        usage: json_resp.usage,
        finish_reason: llm_gateway::FinishReason::Stop,
        latency_ms: json_resp.latency_ms,
        cost_usd: 0.0,
    };

    Ok(JudgePlausibilityResponse {
        plausible,
        reason: reason_trimmed,
        call_record,
    })
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

    fn judge_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "haiku-judge".into(),
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
                identity: judge_identity(),
                response: Mutex::new(Some(Ok(CompletionJsonResponse {
                    raw_text,
                    parsed: value,
                    schema_valid: true,
                    model: "haiku-judge".into(),
                    usage: TokenUsage {
                        input_tokens: 90,
                        output_tokens: 14,
                        cached_tokens: 0,
                    },
                    provider_id: judge_identity(),
                    latency_ms: 31,
                    polyfill_used: false,
                }))),
            }
        }

        fn with_error(err: LlmError) -> Self {
            Self {
                identity: judge_identity(),
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
            unreachable!("judge_plausibility uses complete_json")
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

    fn req() -> JudgePlausibilityRequest {
        JudgePlausibilityRequest {
            source_span_text: "1 carry-on bag, 1 personal item.".into(),
            ir_rendered: "The passenger declared one carry-on bag and one personal item.".into(),
            adversary_reading:
                "The passenger declared zero carry-on bags by saying \"1 carry-on bag\"."
                    .into(),
            domain_hint: "tsa-declaration".into(),
        }
    }

    #[test]
    fn missing_plausibility_client_returns_no_client_for_role() {
        let g = GatewayConfig::new();
        let err = judge_plausibility(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::NoClientForRole { role } => assert_eq!(role, Role::Plausibility),
            other => panic!("expected NoClientForRole, got {other:?}"),
        }
    }

    #[test]
    fn happy_path_implausible_returns_parsed_response() {
        let mock = ScriptedJson::new(serde_json::json!({
            "plausible": false,
            "reason": "Reading 1 as 0 contradicts the literal numeral; competent readers don't.",
        }));
        let g = GatewayConfig::new().with_client(Role::Plausibility, Box::new(mock));
        let resp = judge_plausibility(&req(), &g).unwrap();

        assert!(!resp.plausible);
        assert!(resp.reason.contains("contradicts the literal numeral"));
        assert_eq!(resp.call_record.primitive, "judge_plausibility");
        assert_eq!(resp.call_record.role, "plausibility");
        assert_eq!(resp.call_record.prompt_version, "plausibility-v1");
        assert!(!resp.call_record.prompt_hash.is_empty());
        assert_eq!(resp.call_record.usage.output_tokens, 14);
        assert_eq!(resp.call_record.latency_ms, 31);
    }

    #[test]
    fn happy_path_plausible_returns_parsed_response() {
        let mock = ScriptedJson::new(serde_json::json!({
            "plausible": true,
            "reason": "The source is ambiguous about quantity given trailing punctuation.",
        }));
        let g = GatewayConfig::new().with_client(Role::Plausibility, Box::new(mock));
        let resp = judge_plausibility(&req(), &g).unwrap();
        assert!(resp.plausible);
        assert!(resp.reason.contains("ambiguous"));
    }

    #[test]
    fn user_message_tags_source_ir_and_adversary_separately() {
        let r = req();
        let text = build_user_text(&r);
        assert!(text.contains("DOMAIN: tsa-declaration"));
        assert!(text.contains("SOURCE:\n1 carry-on"));
        assert!(text.contains("IR-RENDERED:\nThe passenger declared"));
        assert!(text.contains("ADVERSARY:\nThe passenger declared zero"));
    }

    #[test]
    fn gateway_auth_error_propagates_as_gateway_variant() {
        let mock = ScriptedJson::with_error(LlmError::Auth {
            provider: judge_identity(),
            detail: "expired key".into(),
        });
        let g = GatewayConfig::new().with_client(Role::Plausibility, Box::new(mock));
        let err = judge_plausibility(&req(), &g).unwrap_err();
        assert!(matches!(err, PrimitiveError::Gateway(LlmError::Auth { .. })));
    }

    #[test]
    fn missing_plausible_field_returns_validation_exhausted() {
        let mock = ScriptedJson::new(serde_json::json!({
            "reason": "ok",
        }));
        let g = GatewayConfig::new().with_client(Role::Plausibility, Box::new(mock));
        let err = judge_plausibility(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::ValidationExhausted { attempts, last_error, .. } => {
                assert_eq!(attempts, 1);
                assert!(last_error.contains("missing or wrong-typed"));
            }
            other => panic!("expected ValidationExhausted, got {other:?}"),
        }
    }

    #[test]
    fn wrong_typed_plausible_returns_validation_exhausted() {
        let mock = ScriptedJson::new(serde_json::json!({
            "plausible": "maybe",
            "reason": "hedging",
        }));
        let g = GatewayConfig::new().with_client(Role::Plausibility, Box::new(mock));
        let err = judge_plausibility(&req(), &g).unwrap_err();
        assert!(matches!(err, PrimitiveError::ValidationExhausted { .. }));
    }

    #[test]
    fn empty_reason_returns_validation_exhausted() {
        let mock = ScriptedJson::new(serde_json::json!({
            "plausible": true,
            "reason": "   \t\n",
        }));
        let g = GatewayConfig::new().with_client(Role::Plausibility, Box::new(mock));
        let err = judge_plausibility(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::ValidationExhausted { last_error, .. } => {
                assert!(last_error.contains("empty reason"));
            }
            other => panic!("expected ValidationExhausted, got {other:?}"),
        }
    }

    #[test]
    fn reason_is_trimmed_on_success() {
        let mock = ScriptedJson::new(serde_json::json!({
            "plausible": false,
            "reason": "  not plausible.  \n",
        }));
        let g = GatewayConfig::new().with_client(Role::Plausibility, Box::new(mock));
        let resp = judge_plausibility(&req(), &g).unwrap();
        assert_eq!(resp.reason, "not plausible.");
    }

    #[test]
    fn call_record_prompt_hash_matches_built_request() {
        let stub: Box<dyn LlmClient> =
            Box::new(MockLlmClient::new().with_identity(judge_identity()));
        let cr = build_completion_request(stub.as_ref(), &req());
        let expected_hash = crate::fingerprint_prompt(&cr);

        let mock = ScriptedJson::new(serde_json::json!({
            "plausible": false,
            "reason": "ok",
        }));
        let g = GatewayConfig::new().with_client(Role::Plausibility, Box::new(mock));
        let resp = judge_plausibility(&req(), &g).unwrap();
        assert_eq!(resp.call_record.prompt_hash, expected_hash);
    }
}
