//! # `elicit_rules` — bootstrap a rulebook from the LLM's own weights.
//!
//! Reference implementation of the **Stage 0** primitive from
//! [`ADJ14`](../../../specs/ADJ14-rule-elicitation.md). The
//! `acquire_rulebook` orchestrator in `adjudication-rulebook` calls
//! this primitive, then pipes the resulting raw text through
//! `decompose_text` and the standard checker passes.
//!
//! ## The contract
//!
//! Given a domain hint (`"tsa-declaration"`, `"clinical-triage"`,
//! `"contract-review"`, …) and an optional scope refinement, ask the
//! LLM to **volunteer** the rules it knows for that domain. Be
//! exhaustive. Number each rule. Be precise about exceptions and
//! conditional logic. State limits of knowledge explicitly.
//!
//! The framework does NOT trust the raw output. The caller pipes
//! the response into [`decompose_text`](crate::decompose_text) so
//! the same ADJ02–05 audit discipline applies to rules that already
//! applies to extracted facts.

use llm_gateway::{
    CompletionRequest, LlmClient, Message, MessageContent, Role as MsgRole,
};

use crate::{
    complete_with_truncation_retry, fingerprint_prompt, GatewayConfig, LlmCallRecord,
    PrimitiveError, Role, ELICIT_RULES_PROMPT_VERSION,
};

/// Inputs to [`elicit_rules`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElicitRulesRequest {
    /// Stable identifier the framework attaches to the rulebook
    /// document that the response will eventually populate. The
    /// audit trail joins the elicitation back to the rulebook.
    pub document_id: String,
    /// Free-text domain hint. Drives the prompt's framing.
    pub domain_hint: String,
    /// Optional scope refinement. When absent, the model produces a
    /// broad rulebook for the domain.
    pub scope_hint: Option<String>,
    /// Optional ISO language code. Defaults to English (the default
    /// for ADJ14 rulebook elicitation, whose output feeds
    /// `decompose_text`).
    pub language_hint: Option<String>,
}

/// Outcome of one [`elicit_rules`] call.
#[derive(Debug, Clone, PartialEq)]
pub struct ElicitRulesResponse {
    /// The raw natural-language rulebook text the model produced.
    /// Consumed by `decompose_text` downstream — **not yet IR** and
    /// **not yet audited**.
    pub rule_text: String,
    /// Audit-trail call record. Carries the prompt version constant
    /// (`ELICIT_RULES_PROMPT_VERSION`) and a fingerprint of the
    /// exact prompt the LLM saw, so the call is replayable.
    pub call_record: LlmCallRecord,
}

const SYSTEM_PROMPT: &str = "\
You are a meticulous regulatory archivist. Given a DOMAIN hint and \
optional SCOPE, list every rule, regulation, or guideline you know \
that governs that domain. Be EXHAUSTIVE — readers will use this \
list to audit a real decision and will verify each rule against \
the source authority.\n\
\n\
RULES for the response:\n\
\n\
1. **Numbered list.** Start each rule on its own line, prefixed \
with a sequence number (`1.`, `2.`, …). Numbering supports later \
segmentation.\n\
2. **One rule per item.** Compound rules (\"X if Y, except Z\") \
should be expressed clearly within a single numbered item; do NOT \
split a single rule across multiple items.\n\
3. **Be precise about exceptions.** When a rule has exceptions or \
conditional clauses, state them inline using language like \
\"except when …\" or \"provided that …\". Exceptions matter; \
omitting them produces a wrong rulebook.\n\
4. **Cite sources when you are confident.** Citations like \
\"per 49 CFR § 1540.111(a)\" or \"per the Emergency Severity Index \
Algorithm v4\" are valuable. If you cannot produce a citation with \
confidence, omit it rather than fabricate.\n\
5. **State limits of knowledge explicitly.** Begin your response \
with a one-line \"COVERAGE:\" note describing what your training \
data is and isn't likely to cover (e.g., \"COVERAGE: TSA carry-on \
rules as of ~2024; post-2024 amendments may be missing\").\n\
6. **Do not invent rules.** If you are uncertain whether something \
is a rule in this domain, mark the item as `UNCERTAIN:` and \
explain what you are unsure about. The framework will route \
uncertainty through a clarification dialogue.\n\
7. **Punctuation matters.** A single comma can flip a rule's \
meaning. Read the rule you are about to write carefully — would a \
reasonable reviewer understand it the same way you do?\n\
8. **No prose outside the list.** Respond with the COVERAGE line, \
then the numbered list. No preamble, no markdown headers, no \
backticks.";

fn build_user_text(req: &ElicitRulesRequest) -> String {
    let lang = req.language_hint.as_deref().unwrap_or("en");
    let scope = match &req.scope_hint {
        Some(s) => format!("SCOPE: {s}\n"),
        None => String::new(),
    };
    format!(
        "DOMAIN: {domain}\n{scope}LANGUAGE: {lang}\nDOCUMENT_ID: {doc_id}\n\n\
         List the rules for this domain now.",
        domain = req.domain_hint,
        scope = scope,
        lang = lang,
        doc_id = req.document_id,
    )
}

fn build_completion_request(
    client: &dyn LlmClient,
    req: &ElicitRulesRequest,
) -> CompletionRequest {
    CompletionRequest {
        model: client.identity().model_family.clone(),
        system: Some(SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(build_user_text(req)),
        }],
        // Determinism by default so the elicited rulebook is
        // replayable.
        temperature: 0.0,
        // Rulebooks can be long — many domains have dozens of
        // rules with exceptions. 8k tokens is a comfortable ceiling.
        max_tokens: Some(8192),
        stop_sequences: Vec::new(),
        seed: None,
        metadata: Default::default(),
    }
}

/// Elicit a rulebook from the LLM's own weights.
///
/// Returns the raw natural-language text the model produced plus the
/// audit-trail call record. The text is consumed by `decompose_text`
/// downstream; this primitive does not validate or audit it.
///
/// Looks up `Role::RuleExtractor` on the gateway (falls back to
/// `Role::Extractor` when no separate `RuleExtractor` is bound).
/// Returns `PrimitiveError::NoClientForRole` if neither is bound.
pub fn elicit_rules(
    req: &ElicitRulesRequest,
    gateway: &GatewayConfig,
) -> Result<ElicitRulesResponse, PrimitiveError> {
    let client = gateway
        .client(Role::RuleExtractor)
        .or_else(|| gateway.client(Role::Extractor))
        .ok_or(PrimitiveError::NoClientForRole {
            role: Role::RuleExtractor,
        })?;

    let completion_req = build_completion_request(client, req);
    let prompt_hash = fingerprint_prompt(&completion_req);

    let resp = complete_with_truncation_retry(client, completion_req)
        .map_err(PrimitiveError::Gateway)?;

    let call_record = LlmCallRecord {
        provider: client.identity().clone(),
        primitive: "elicit_rules".to_string(),
        role: Role::RuleExtractor.as_str().to_string(),
        prompt_version: ELICIT_RULES_PROMPT_VERSION.to_string(),
        prompt_hash,
        usage: resp.usage,
        latency_ms: resp.latency_ms,
        finish_reason: resp.finish_reason,
        cost_usd: 0.0,
    };

    Ok(ElicitRulesResponse {
        rule_text: resp.text,
        call_record,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway::{
        Capabilities, CompletionResponse, FinishReason, JsonSchema, LlmError, ProviderIdentity,
        TokenUsage,
    };
    use std::sync::Mutex;

    fn rule_extractor_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "test-rule-extractor".into(),
            model_version: "v1".into(),
            endpoint: None,
        }
    }

    struct ScriptedText {
        identity: ProviderIdentity,
        response: Mutex<Option<Result<CompletionResponse, LlmError>>>,
    }

    impl ScriptedText {
        fn new(text: String) -> Self {
            Self {
                identity: rule_extractor_identity(),
                response: Mutex::new(Some(Ok(CompletionResponse {
                    text,
                    model: "test-rule-extractor".into(),
                    usage: TokenUsage {
                        input_tokens: 200,
                        output_tokens: 350,
                        cached_tokens: 0,
                    },
                    finish_reason: FinishReason::Stop,
                    provider_id: rule_extractor_identity(),
                    latency_ms: 950,
                }))),
            }
        }

        fn with_error(err: LlmError) -> Self {
            Self {
                identity: rule_extractor_identity(),
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

        fn complete(
            &self,
            _req: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
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
            unreachable!("elicit_rules uses complete, not complete_json")
        }
    }

    fn req() -> ElicitRulesRequest {
        ElicitRulesRequest {
            document_id: "rulebook-tsa-2026-05-12".into(),
            domain_hint: "tsa-declaration".into(),
            scope_hint: Some("carry-on baggage".into()),
            language_hint: None,
        }
    }

    fn sample_rule_text() -> String {
        "COVERAGE: TSA carry-on rules as of ~2024.\n\
         1. Passengers may carry one (1) carry-on bag.\n\
         2. Liquids in containers larger than 3.4 oz are prohibited \
         except medicines.\n\
         3. Strike-anywhere matches are prohibited.\n"
            .to_string()
    }

    #[test]
    fn missing_rule_extractor_role_returns_no_client_for_role() {
        let g = GatewayConfig::new();
        let err = elicit_rules(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::NoClientForRole { role } => {
                assert_eq!(role, Role::RuleExtractor)
            }
            other => panic!("expected NoClientForRole, got {other:?}"),
        }
    }

    #[test]
    fn falls_back_to_extractor_role_when_rule_extractor_unbound() {
        let mock = ScriptedText::new(sample_rule_text());
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let resp = elicit_rules(&req(), &g).unwrap();
        assert!(resp.rule_text.contains("COVERAGE:"));
    }

    #[test]
    fn happy_path_returns_text_and_call_record() {
        let mock = ScriptedText::new(sample_rule_text());
        let g = GatewayConfig::new().with_client(Role::RuleExtractor, Box::new(mock));
        let resp = elicit_rules(&req(), &g).unwrap();

        assert!(resp.rule_text.starts_with("COVERAGE:"));
        assert!(resp.rule_text.contains("1."));
        assert_eq!(resp.call_record.primitive, "elicit_rules");
        assert_eq!(resp.call_record.role, "rule_extractor");
        assert_eq!(resp.call_record.prompt_version, "elicit-rules-v1");
        assert!(!resp.call_record.prompt_hash.is_empty());
    }

    #[test]
    fn gateway_error_propagates() {
        let mock = ScriptedText::with_error(LlmError::ContextTooLarge {
            provider: rule_extractor_identity(),
            requested_tokens: 100_000,
            max_tokens: 8_192,
        });
        let g = GatewayConfig::new().with_client(Role::RuleExtractor, Box::new(mock));
        let err = elicit_rules(&req(), &g).unwrap_err();
        assert!(matches!(
            err,
            PrimitiveError::Gateway(LlmError::ContextTooLarge { .. })
        ));
    }

    #[test]
    fn user_message_includes_domain_scope_and_doc_id() {
        let r = req();
        let text = build_user_text(&r);
        assert!(text.contains("DOMAIN: tsa-declaration"));
        assert!(text.contains("SCOPE: carry-on baggage"));
        assert!(text.contains("DOCUMENT_ID: rulebook-tsa-2026-05-12"));
        assert!(text.contains("LANGUAGE: en"));
    }

    #[test]
    fn user_message_omits_scope_when_absent() {
        let r = ElicitRulesRequest {
            scope_hint: None,
            ..req()
        };
        let text = build_user_text(&r);
        assert!(!text.contains("SCOPE:"));
    }
}
