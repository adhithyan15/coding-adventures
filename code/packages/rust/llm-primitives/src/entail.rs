//! # `entail` — bidirectional textual entailment primitive
//!
//! First concrete primitive from
//! [LM00b §"entail"](../../../specs/LM00b-llm-primitives.md). Given two
//! pieces of text, returns whether each entails the other and a
//! confidence score for each direction.
//!
//! ## Why bidirectional
//!
//! ADJ04 (the round-trip checker) needs to know whether the rendered IR
//! and the original source-span text are mutually entailing. A
//! one-directional entailment ("does the source entail the rendering?")
//! is not enough — symmetric drift hides in the other direction. The
//! primitive returns both directions so the round-trip checker can
//! flag drift in either.
//!
//! ## What this primitive does NOT do
//!
//! - **Pick a model.** The deployment's `GatewayConfig` maps
//!   `Role::Nli` to a concrete client; the primitive just uses
//!   whatever is registered. ADJ04 strongly recommends that the
//!   `Nli` role differ from the `Renderer` role (a self-consistency
//!   trap) but the primitive doesn't enforce that — it's a
//!   role-mapping configuration concern.
//! - **Cache.** Caching is a primitive-layer concern but lives in
//!   the (forthcoming) `cache.rs` module, not here. v0.1 calls the
//!   gateway once per `entail` invocation.

use llm_gateway::{
    CompletionRequest, JsonSchema, LlmClient, Message, MessageContent, Role as MsgRole,
};

use crate::{
    fingerprint_prompt, GatewayConfig, LlmCallRecord, PrimitiveError, Role, ENTAIL_PROMPT_VERSION,
};

/// Inputs to [`entail`]. Both `premise` and `hypothesis` are plain
/// text — the primitive does not interpret IR structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntailRequest {
    pub premise: String,
    pub hypothesis: String,
}

/// Result of one [`entail`] call. Carries the bidirectional booleans
/// plus their confidence scores (both in `[0, 1]`) so ADJ04 can
/// distinguish a confident no-entailment from a low-confidence one.
#[derive(Debug, Clone, PartialEq)]
pub struct EntailResponse {
    pub premise_entails_hypothesis: bool,
    pub p_to_h_score: f32,
    pub hypothesis_entails_premise: bool,
    pub h_to_p_score: f32,
    pub call_record: LlmCallRecord,
}

/// Stable system prompt for the NLI role. Kept verbatim here rather
/// than in an external file so the v0.1 surface has no filesystem
/// dependency. Bump [`ENTAIL_PROMPT_VERSION`] if this text changes.
const SYSTEM_PROMPT: &str = "\
You are a precise textual-entailment annotator. Given a PREMISE and a \
HYPOTHESIS, report:\n\
  - whether the premise entails the hypothesis,\n\
  - whether the hypothesis entails the premise,\n\
  - a calibrated confidence in [0, 1] for each direction.\n\
\n\
Entailment means a competent reader of the premise would commit to \
the hypothesis being true given only the premise. If the premise leaves \
the hypothesis indeterminate, do NOT entail. Evaluate both directions \
independently — the relation is not symmetric.";

/// JSON schema for the response. v0.1 keeps the schema as a string
/// literal so consumers see the exact wire shape; v0.2 may generate
/// it from the Rust types.
const RESPONSE_SCHEMA: &str = r#"{
    "type": "object",
    "required": [
        "premise_entails_hypothesis",
        "p_to_h_score",
        "hypothesis_entails_premise",
        "h_to_p_score"
    ],
    "properties": {
        "premise_entails_hypothesis": { "type": "boolean" },
        "p_to_h_score":               { "type": "number", "minimum": 0, "maximum": 1 },
        "hypothesis_entails_premise": { "type": "boolean" },
        "h_to_p_score":               { "type": "number", "minimum": 0, "maximum": 1 }
    },
    "additionalProperties": false
}"#;

/// Build the user message embedding the premise and hypothesis.
/// Tagged with `PREMISE:` / `HYPOTHESIS:` markers so the typical
/// case (well-formed text without literal `HYPOTHESIS:` lines) parses
/// unambiguously. The framework treats the LLM as a trust boundary;
/// adversarial input that *embeds* the markers themselves is an
/// accepted-risk prompt-injection vector. If a deployment cares,
/// wrap each field in a nonce-delimited block before calling here.
fn build_user_text(req: &EntailRequest) -> String {
    format!(
        "PREMISE:\n{premise}\n\nHYPOTHESIS:\n{hypothesis}\n\nReturn the JSON object now.",
        premise = req.premise,
        hypothesis = req.hypothesis,
    )
}

fn build_completion_request(client: &dyn LlmClient, req: &EntailRequest) -> CompletionRequest {
    CompletionRequest {
        model: client.identity().model_family.clone(),
        system: Some(SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(build_user_text(req)),
        }],
        // Deterministic by default; the deployment can override at
        // the gateway layer if they want sampling.
        temperature: 0.0,
        max_tokens: Some(256),
        stop_sequences: Vec::new(),
        seed: None,
        metadata: Default::default(),
    }
}

/// Bidirectional textual-entailment primitive. Synchronous (matches
/// v0.1 `LlmClient`); a future async surface can wrap this without
/// breaking callers.
///
/// Returns [`PrimitiveError::NoClientForRole`] if the gateway has no
/// `Nli` client; [`PrimitiveError::Gateway`] on a transport / auth /
/// rate-limit failure; [`PrimitiveError::ValidationExhausted`] if the
/// LLM's JSON output cannot be parsed as the expected schema. The
/// validation case carries the raw text so ADJ06 can surface it.
pub fn entail(req: &EntailRequest, gateway: &GatewayConfig) -> Result<EntailResponse, PrimitiveError> {
    let client = gateway
        .client(Role::Nli)
        .ok_or(PrimitiveError::NoClientForRole { role: Role::Nli })?;

    let completion_req = build_completion_request(client, req);
    let prompt_hash = fingerprint_prompt(&completion_req);

    let schema = JsonSchema {
        name: "EntailResponse".to_string(),
        schema_json: RESPONSE_SCHEMA.to_string(),
    };

    let json_resp = client
        .complete_json(completion_req, &schema)
        .map_err(PrimitiveError::Gateway)?;

    let v = &json_resp.parsed;

    // Pull each field individually so we can produce a precise
    // ValidationExhausted message (rather than a generic "shape
    // wrong"). One missing field is enough to fail.
    let p_to_h = v
        .get("premise_entails_hypothesis")
        .and_then(|x| x.as_bool());
    let h_to_p = v
        .get("hypothesis_entails_premise")
        .and_then(|x| x.as_bool());
    let p_score = v.get("p_to_h_score").and_then(|x| x.as_f64());
    let h_score = v.get("h_to_p_score").and_then(|x| x.as_f64());

    let (Some(p_to_h), Some(h_to_p), Some(p_score), Some(h_score)) =
        (p_to_h, h_to_p, p_score, h_score)
    else {
        return Err(PrimitiveError::ValidationExhausted {
            last_response: json_resp.raw_text,
            last_error: "missing or wrong-typed field in entail response".to_string(),
            attempts: 1,
        });
    };

    // Range-check the scores. A misbehaving model that emits 1.7 or
    // -0.2 surfaces as a structural failure rather than being
    // silently clamped — ADJ06 can ask for re-emission.
    if !(0.0..=1.0).contains(&p_score) || !(0.0..=1.0).contains(&h_score) {
        return Err(PrimitiveError::ValidationExhausted {
            last_response: json_resp.raw_text,
            last_error: format!(
                "scores out of range [0,1]: p_to_h={p_score}, h_to_p={h_score}"
            ),
            attempts: 1,
        });
    }

    let call_record = LlmCallRecord {
        primitive: "entail".to_string(),
        role: Role::Nli.as_str().to_string(),
        prompt_version: ENTAIL_PROMPT_VERSION.to_string(),
        prompt_hash,
        provider: json_resp.provider_id,
        usage: json_resp.usage,
        // complete_json doesn't surface FinishReason on the JSON path;
        // record Stop as the default since a parseable JSON output
        // implies the model didn't truncate.
        finish_reason: llm_gateway::FinishReason::Stop,
        latency_ms: json_resp.latency_ms,
        cost_usd: 0.0, // cost-table integration is a follow-up
    };

    Ok(EntailResponse {
        premise_entails_hypothesis: p_to_h,
        p_to_h_score: p_score as f32,
        hypothesis_entails_premise: h_to_p,
        h_to_p_score: h_score as f32,
        call_record,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Role;
    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, JsonSchema, LlmClient, LlmError,
        MockLlmClient, MockResponse, ProviderIdentity, TokenUsage,
    };
    use std::sync::Mutex;

    fn nli_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "nli-debertav3".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    /// A purpose-built mock that captures the request it was called
    /// with and returns a scripted `CompletionJsonResponse`. The
    /// stock `MockLlmClient` uses fingerprint-based routing which is
    /// overkill here.
    struct ScriptedJson {
        identity: ProviderIdentity,
        response: Mutex<Option<Result<CompletionJsonResponse, LlmError>>>,
        captured: Mutex<Option<CompletionRequest>>,
    }

    impl ScriptedJson {
        fn new(resp: CompletionJsonResponse) -> Self {
            Self {
                identity: nli_identity(),
                response: Mutex::new(Some(Ok(resp))),
                captured: Mutex::new(None),
            }
        }

        fn with_error(err: LlmError) -> Self {
            Self {
                identity: nli_identity(),
                response: Mutex::new(Some(Err(err))),
                captured: Mutex::new(None),
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
            unreachable!("entail uses complete_json only")
        }

        fn complete_json(
            &self,
            req: CompletionRequest,
            _schema: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            *self.captured.lock().unwrap() = Some(req);
            self.response
                .lock()
                .unwrap()
                .take()
                .expect("ScriptedJson::complete_json called more than once")
        }
    }

    fn ok_json(value: serde_json::Value) -> CompletionJsonResponse {
        let raw_text = serde_json::to_string(&value).unwrap();
        CompletionJsonResponse {
            raw_text,
            parsed: value,
            schema_valid: true,
            model: "nli-debertav3".into(),
            usage: TokenUsage {
                input_tokens: 50,
                output_tokens: 8,
                cached_tokens: 0,
            },
            provider_id: nli_identity(),
            latency_ms: 42,
            polyfill_used: false,
        }
    }

    fn req() -> EntailRequest {
        EntailRequest {
            premise: "The patient denies chest pain.".into(),
            hypothesis: "The patient has chest pain.".into(),
        }
    }

    #[test]
    fn missing_nli_client_returns_no_client_for_role() {
        let g = GatewayConfig::new();
        let err = entail(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::NoClientForRole { role } => assert_eq!(role, Role::Nli),
            other => panic!("expected NoClientForRole, got {other:?}"),
        }
    }

    #[test]
    fn happy_path_returns_parsed_response() {
        let mock = ScriptedJson::new(ok_json(serde_json::json!({
            "premise_entails_hypothesis": false,
            "p_to_h_score": 0.05,
            "hypothesis_entails_premise": false,
            "h_to_p_score": 0.10,
        })));
        let g = GatewayConfig::new().with_client(Role::Nli, Box::new(mock));
        let resp = entail(&req(), &g).unwrap();

        assert!(!resp.premise_entails_hypothesis);
        assert!((resp.p_to_h_score - 0.05).abs() < 1e-6);
        assert!(!resp.hypothesis_entails_premise);
        assert!((resp.h_to_p_score - 0.10).abs() < 1e-6);

        assert_eq!(resp.call_record.primitive, "entail");
        assert_eq!(resp.call_record.role, "nli");
        assert_eq!(resp.call_record.prompt_version, "entail-v1");
        assert_eq!(resp.call_record.provider.vendor, "mock");
        assert!(!resp.call_record.prompt_hash.is_empty());
        assert_eq!(resp.call_record.usage.output_tokens, 8);
        assert_eq!(resp.call_record.latency_ms, 42);
    }

    #[test]
    fn premise_and_hypothesis_appear_separately_tagged_in_user_message() {
        let captured = std::sync::Arc::new(Mutex::new(None::<CompletionRequest>));
        let captured_clone = captured.clone();
        let mock = ScriptedJson {
            identity: nli_identity(),
            response: Mutex::new(Some(Ok(ok_json(serde_json::json!({
                "premise_entails_hypothesis": true,
                "p_to_h_score": 0.9,
                "hypothesis_entails_premise": true,
                "h_to_p_score": 0.85,
            }))))),
            captured: Mutex::new(None),
        };
        let g = GatewayConfig::new().with_client(Role::Nli, Box::new(mock));
        let r = EntailRequest {
            premise: "Cats are mammals.".into(),
            hypothesis: "Cats are animals.".into(),
        };
        let _ = entail(&r, &g).unwrap();

        // We can't easily peek into the Box<dyn LlmClient>'s captured
        // field through the GatewayConfig — instead, build the user
        // text directly and check the markers + content.
        let text = build_user_text(&r);
        assert!(text.contains("PREMISE:\nCats are mammals."));
        assert!(text.contains("HYPOTHESIS:\nCats are animals."));
        let _ = captured_clone; // silence unused
    }

    #[test]
    fn gateway_transport_error_propagates_as_gateway_variant() {
        let mock = ScriptedJson::with_error(LlmError::Transport {
            provider: nli_identity(),
            detail: "network unreachable".into(),
        });
        let g = GatewayConfig::new().with_client(Role::Nli, Box::new(mock));
        let err = entail(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::Gateway(LlmError::Transport { detail, .. }) => {
                assert!(detail.contains("network unreachable"));
            }
            other => panic!("expected Gateway(Transport), got {other:?}"),
        }
    }

    #[test]
    fn missing_field_in_response_returns_validation_exhausted() {
        let mock = ScriptedJson::new(ok_json(serde_json::json!({
            "premise_entails_hypothesis": false,
            "p_to_h_score": 0.1,
            // hypothesis_entails_premise missing
            "h_to_p_score": 0.2,
        })));
        let g = GatewayConfig::new().with_client(Role::Nli, Box::new(mock));
        let err = entail(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::ValidationExhausted { attempts, last_error, .. } => {
                assert_eq!(attempts, 1);
                assert!(last_error.contains("missing or wrong-typed"));
            }
            other => panic!("expected ValidationExhausted, got {other:?}"),
        }
    }

    #[test]
    fn wrong_type_in_response_returns_validation_exhausted() {
        let mock = ScriptedJson::new(ok_json(serde_json::json!({
            "premise_entails_hypothesis": "yes",  // wrong type: string not bool
            "p_to_h_score": 0.1,
            "hypothesis_entails_premise": false,
            "h_to_p_score": 0.2,
        })));
        let g = GatewayConfig::new().with_client(Role::Nli, Box::new(mock));
        let err = entail(&req(), &g).unwrap_err();
        assert!(matches!(err, PrimitiveError::ValidationExhausted { .. }));
    }

    #[test]
    fn score_above_one_returns_validation_exhausted() {
        let mock = ScriptedJson::new(ok_json(serde_json::json!({
            "premise_entails_hypothesis": true,
            "p_to_h_score": 1.5,
            "hypothesis_entails_premise": false,
            "h_to_p_score": 0.2,
        })));
        let g = GatewayConfig::new().with_client(Role::Nli, Box::new(mock));
        let err = entail(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::ValidationExhausted { last_error, .. } => {
                assert!(last_error.contains("out of range"));
                assert!(last_error.contains("1.5"));
            }
            other => panic!("expected ValidationExhausted, got {other:?}"),
        }
    }

    #[test]
    fn negative_score_returns_validation_exhausted() {
        let mock = ScriptedJson::new(ok_json(serde_json::json!({
            "premise_entails_hypothesis": true,
            "p_to_h_score": 0.9,
            "hypothesis_entails_premise": false,
            "h_to_p_score": -0.1,
        })));
        let g = GatewayConfig::new().with_client(Role::Nli, Box::new(mock));
        let err = entail(&req(), &g).unwrap_err();
        assert!(matches!(err, PrimitiveError::ValidationExhausted { .. }));
    }

    #[test]
    fn boundary_scores_zero_and_one_are_accepted() {
        let mock = ScriptedJson::new(ok_json(serde_json::json!({
            "premise_entails_hypothesis": true,
            "p_to_h_score": 1.0,
            "hypothesis_entails_premise": false,
            "h_to_p_score": 0.0,
        })));
        let g = GatewayConfig::new().with_client(Role::Nli, Box::new(mock));
        let resp = entail(&req(), &g).unwrap();
        assert!((resp.p_to_h_score - 1.0).abs() < 1e-6);
        assert!(resp.h_to_p_score.abs() < 1e-6);
    }

    #[test]
    fn call_record_prompt_hash_matches_built_request() {
        let mock = ScriptedJson::new(ok_json(serde_json::json!({
            "premise_entails_hypothesis": false,
            "p_to_h_score": 0.0,
            "hypothesis_entails_premise": false,
            "h_to_p_score": 0.0,
        })));
        // Build the same request the primitive would build, hash it,
        // and compare against the call record.
        let identity = nli_identity();
        let stub: Box<dyn LlmClient> = Box::new(MockLlmClient::new().with_identity(identity));
        let cr = build_completion_request(stub.as_ref(), &req());
        let expected_hash = crate::fingerprint_prompt(&cr);

        let g = GatewayConfig::new().with_client(Role::Nli, Box::new(mock));
        let resp = entail(&req(), &g).unwrap();
        assert_eq!(resp.call_record.prompt_hash, expected_hash);
    }

    // Silence unused-import warning when MockResponse isn't directly
    // exercised. `MockLlmClient` is used in `call_record_prompt_hash_matches_built_request`.
    #[allow(dead_code)]
    fn _force_use_mock_imports() {
        let _ = MockResponse::Json {
            raw_text: String::new(),
            parsed: serde_json::Value::Null,
        };
    }
}
