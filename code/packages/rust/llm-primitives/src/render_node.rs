//! # `render_node` — render an IR node back into natural language
//!
//! Second concrete primitive from
//! [LM00b §"render_node"](../../../specs/LM00b-llm-primitives.md). Given
//! a textual description of one IR node (plus optional document
//! context and a target style), returns a faithful natural-language
//! rendering. The rendering is intentionally **weak** — a trivial
//! paraphrase rather than a clever rewrite. Cleverness masks IR loss;
//! trivial paraphrasing exposes it (per ADJ04 §"Render IR → Natural
//! Language").
//!
//! ## Why a string `node_description` instead of typed `IRNode`
//!
//! The LM00b spec carries an `adjudication_ir::IRNode` directly in
//! the request. v0.1 of this primitive takes the **caller-formatted
//! textual description** of the node instead, because
//! `adjudication-ir` does not yet derive `Serialize` and the
//! primitive does not (yet) want to bind to a specific in-process
//! shape. The caller builds whatever text representation makes
//! sense for its consumer; the LLM sees a stable shape regardless.
//!
//! v0.2 will swap the request type to `IRNode` once
//! `adjudication-ir` ships its `serde` feature; the prompt and
//! audit-trail behaviour will be unchanged.

use llm_gateway::{
    CompletionRequest, LlmClient, Message, MessageContent, Role as MsgRole,
};

use crate::{
    fingerprint_prompt, GatewayConfig, LlmCallRecord, PrimitiveError, Role,
    RENDER_NODE_PROMPT_VERSION,
};

/// Render style. The LLM is instructed to produce a *faithful* trivial
/// paraphrase in the requested register — not a clever rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStyle {
    /// Everyday-English ("The patient has chest pain.").
    Plain,
    /// Clinical shorthand ("Chest pain (acute, severe), affirmed.").
    Clinical,
    /// Formal-register restatement appropriate for legal text.
    Legal,
}

impl RenderStyle {
    /// Stable string tag — flows into the audit trail.
    pub fn as_str(&self) -> &'static str {
        match self {
            RenderStyle::Plain => "plain",
            RenderStyle::Clinical => "clinical",
            RenderStyle::Legal => "legal",
        }
    }

    /// Prompt-side instruction the LLM follows.
    fn directive(&self) -> &'static str {
        match self {
            RenderStyle::Plain => "everyday English, indicative mood, no shorthand",
            RenderStyle::Clinical => "concise clinical shorthand; abbreviations are fine",
            RenderStyle::Legal => "formal legal register; precise modal verbs",
        }
    }
}

/// Inputs to [`render_node`]. The `node_description` is whatever
/// textual representation of the IR node the caller wants the LLM
/// to render; the `document_excerpt` is the relevant slice of the
/// source document so the rendering can stay close to the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderNodeRequest {
    pub node_description: String,
    pub document_excerpt: String,
    pub style: RenderStyle,
}

/// Result of one [`render_node`] call.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderNodeResponse {
    /// The LLM's natural-language rendering. ADJ04 then uses
    /// [`crate::entail`] in both directions to flag round-trip drift
    /// between this rendering and the original source.
    pub rendering: String,
    pub call_record: LlmCallRecord,
}

const SYSTEM_PROMPT: &str = "\
You are a faithful paraphraser. Given a structured description of one \
fact from a document, restate it as a single short natural-language \
sentence. Be faithful: do not embellish, do not add information that is \
not in the description, do not strip information that is. A trivial \
restatement is the goal — cleverness hides extraction errors. Respond \
with the rendered sentence only, no prefix and no commentary.";

fn build_user_text(req: &RenderNodeRequest) -> String {
    format!(
        "STYLE: {style} ({directive})\n\n\
         NODE:\n{node}\n\n\
         SOURCE EXCERPT:\n{src}\n\n\
         Render the NODE as one sentence in the requested style now.",
        style = req.style.as_str(),
        directive = req.style.directive(),
        node = req.node_description,
        src = req.document_excerpt,
    )
}

fn build_completion_request(client: &dyn LlmClient, req: &RenderNodeRequest) -> CompletionRequest {
    CompletionRequest {
        model: client.identity().model_family.clone(),
        system: Some(SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(build_user_text(req)),
        }],
        // Deterministic by default — renderings should be reproducible
        // in tests and replays.
        temperature: 0.0,
        max_tokens: Some(256),
        stop_sequences: Vec::new(),
        seed: None,
        metadata: Default::default(),
    }
}

/// Render one IR-node-equivalent description into natural language.
/// Looks up `Role::Renderer` on the gateway; returns
/// [`PrimitiveError::NoClientForRole`] if absent,
/// [`PrimitiveError::Gateway`] on transport / auth failures, or
/// [`PrimitiveError::ValidationExhausted`] if the LLM returns
/// whitespace-only output (the only structural failure that's worth
/// catching at this layer — substantive faithfulness is ADJ04's job
/// via `entail`).
pub fn render_node(
    req: &RenderNodeRequest,
    gateway: &GatewayConfig,
) -> Result<RenderNodeResponse, PrimitiveError> {
    let client = gateway
        .client(Role::Renderer)
        .ok_or(PrimitiveError::NoClientForRole {
            role: Role::Renderer,
        })?;

    let completion_req = build_completion_request(client, req);
    let prompt_hash = fingerprint_prompt(&completion_req);

    let resp = client
        .complete(completion_req)
        .map_err(PrimitiveError::Gateway)?;

    let rendering = resp.text.trim().to_string();
    if rendering.is_empty() {
        return Err(PrimitiveError::ValidationExhausted {
            last_response: resp.text,
            last_error: "rendering was empty after trimming whitespace".to_string(),
            attempts: 1,
        });
    }

    let call_record = LlmCallRecord {
        primitive: "render_node".to_string(),
        role: Role::Renderer.as_str().to_string(),
        prompt_version: RENDER_NODE_PROMPT_VERSION.to_string(),
        prompt_hash,
        provider: resp.provider_id,
        usage: resp.usage,
        finish_reason: resp.finish_reason,
        latency_ms: resp.latency_ms,
        cost_usd: 0.0, // cost-table integration is a follow-up
    };

    Ok(RenderNodeResponse {
        rendering,
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
        Capabilities, CompletionRequest, CompletionResponse, FinishReason, JsonSchema, LlmClient,
        LlmError, MockLlmClient, ProviderIdentity, TokenUsage,
    };
    use std::sync::Mutex;

    fn renderer_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "haiku-renderer".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    struct ScriptedText {
        identity: ProviderIdentity,
        response: Mutex<Option<Result<CompletionResponse, LlmError>>>,
    }

    impl ScriptedText {
        fn new(text: &str) -> Self {
            Self {
                identity: renderer_identity(),
                response: Mutex::new(Some(Ok(CompletionResponse {
                    text: text.to_string(),
                    model: "haiku-renderer".into(),
                    usage: TokenUsage {
                        input_tokens: 30,
                        output_tokens: 9,
                        cached_tokens: 0,
                    },
                    finish_reason: FinishReason::Stop,
                    provider_id: renderer_identity(),
                    latency_ms: 23,
                }))),
            }
        }

        fn with_error(err: LlmError) -> Self {
            Self {
                identity: renderer_identity(),
                response: Mutex::new(Some(Err(err))),
            }
        }
    }

    impl LlmClient for ScriptedText {
        fn identity(&self) -> ProviderIdentity {
            self.identity.clone()
        }

        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }

        fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            self.response
                .lock()
                .unwrap()
                .take()
                .expect("ScriptedText::complete called more than once")
        }

        fn complete_json(
            &self,
            _req: CompletionRequest,
            _schema: &JsonSchema,
        ) -> Result<llm_gateway::CompletionJsonResponse, LlmError> {
            unreachable!("render_node uses complete, not complete_json")
        }
    }

    fn req(style: RenderStyle) -> RenderNodeRequest {
        RenderNodeRequest {
            node_description:
                "Fact: carry_on(passenger_a, 1). polarity=Affirmed modality=Stated".into(),
            document_excerpt: "1 carry-on bag, 1 personal item.".into(),
            style,
        }
    }

    #[test]
    fn render_style_as_str_is_stable() {
        assert_eq!(RenderStyle::Plain.as_str(), "plain");
        assert_eq!(RenderStyle::Clinical.as_str(), "clinical");
        assert_eq!(RenderStyle::Legal.as_str(), "legal");
    }

    #[test]
    fn missing_renderer_client_returns_no_client_for_role() {
        let g = GatewayConfig::new();
        let err = render_node(&req(RenderStyle::Plain), &g).unwrap_err();
        match err {
            PrimitiveError::NoClientForRole { role } => assert_eq!(role, Role::Renderer),
            other => panic!("expected NoClientForRole, got {other:?}"),
        }
    }

    #[test]
    fn happy_path_returns_trimmed_rendering() {
        let mock = ScriptedText::new("  The passenger declared one carry-on bag.\n");
        let g = GatewayConfig::new().with_client(Role::Renderer, Box::new(mock));
        let resp = render_node(&req(RenderStyle::Plain), &g).unwrap();
        assert_eq!(resp.rendering, "The passenger declared one carry-on bag.");

        assert_eq!(resp.call_record.primitive, "render_node");
        assert_eq!(resp.call_record.role, "renderer");
        assert_eq!(resp.call_record.prompt_version, "render-node-v1");
        assert_eq!(resp.call_record.usage.output_tokens, 9);
        assert_eq!(resp.call_record.latency_ms, 23);
    }

    #[test]
    fn user_message_includes_style_directive_and_node_text() {
        // Build the user text directly and assert. Going through the
        // gateway's Box<dyn LlmClient> doesn't give us a hook to peek
        // at the captured request.
        let r = req(RenderStyle::Clinical);
        let text = build_user_text(&r);
        assert!(text.contains("STYLE: clinical"));
        assert!(text.contains("clinical shorthand"));
        assert!(text.contains("NODE:\nFact: carry_on(passenger_a, 1)"));
        assert!(text.contains("SOURCE EXCERPT:\n1 carry-on bag"));
    }

    #[test]
    fn legal_style_directive_appears_in_user_message() {
        let text = build_user_text(&req(RenderStyle::Legal));
        assert!(text.contains("STYLE: legal"));
        assert!(text.contains("formal legal register"));
    }

    #[test]
    fn empty_rendering_returns_validation_exhausted() {
        let mock = ScriptedText::new("   \n\t  ");
        let g = GatewayConfig::new().with_client(Role::Renderer, Box::new(mock));
        let err = render_node(&req(RenderStyle::Plain), &g).unwrap_err();
        match err {
            PrimitiveError::ValidationExhausted { last_error, attempts, .. } => {
                assert_eq!(attempts, 1);
                assert!(last_error.contains("empty"));
            }
            other => panic!("expected ValidationExhausted, got {other:?}"),
        }
    }

    #[test]
    fn rate_limit_propagates_as_gateway_variant() {
        let mock = ScriptedText::with_error(LlmError::RateLimit {
            provider: renderer_identity(),
            retry_after_ms: Some(5000),
        });
        let g = GatewayConfig::new().with_client(Role::Renderer, Box::new(mock));
        let err = render_node(&req(RenderStyle::Plain), &g).unwrap_err();
        assert!(matches!(
            err,
            PrimitiveError::Gateway(LlmError::RateLimit { .. })
        ));
    }

    #[test]
    fn call_record_prompt_hash_matches_built_request() {
        let identity = renderer_identity();
        let stub: Box<dyn LlmClient> = Box::new(MockLlmClient::new().with_identity(identity));
        let cr = build_completion_request(stub.as_ref(), &req(RenderStyle::Plain));
        let expected_hash = crate::fingerprint_prompt(&cr);

        let mock = ScriptedText::new("ok");
        let g = GatewayConfig::new().with_client(Role::Renderer, Box::new(mock));
        let resp = render_node(&req(RenderStyle::Plain), &g).unwrap();
        assert_eq!(resp.call_record.prompt_hash, expected_hash);
    }

    #[test]
    fn finish_reason_is_passed_through_to_call_record() {
        let mock = ScriptedText {
            identity: renderer_identity(),
            response: Mutex::new(Some(Ok(CompletionResponse {
                text: "truncated".into(),
                model: "haiku-renderer".into(),
                usage: TokenUsage::default(),
                finish_reason: FinishReason::MaxTokens,
                provider_id: renderer_identity(),
                latency_ms: 11,
            }))),
        };
        let g = GatewayConfig::new().with_client(Role::Renderer, Box::new(mock));
        let resp = render_node(&req(RenderStyle::Plain), &g).unwrap();
        assert_eq!(resp.call_record.finish_reason, FinishReason::MaxTokens);
    }
}
