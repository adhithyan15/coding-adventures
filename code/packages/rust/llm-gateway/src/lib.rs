// LlmError is intentionally large — it carries provider identity and
// rich diagnostic fields on every variant. The framework's audit-trail
// discipline (every error logged with full context) is worth the
// memory cost. If the trait surface ever becomes hot, the variants
// can be boxed without breaking callers.
#![allow(clippy::result_large_err)]

//! # llm-gateway — provider-agnostic LLM access for the framework.
//!
//! Reference implementation of
//! [LM00](../../../specs/LM00-llm-gateway-architecture.md). Defines
//! the `LlmClient` trait every framework component uses, the neutral
//! request / response shapes, the error taxonomy, and a `MockLlmClient`
//! for deterministic tests.
//!
//! Real providers (Anthropic, OpenAI, Ollama) live in separate crates
//! that depend on this one.
//!
//! ## Sync vs. async
//!
//! v0.1.0 exposes a **synchronous** trait. Real production providers
//! will typically wrap async HTTP clients; deployments that need
//! async-throughout can run `LlmClient` calls on a thread pool. This
//! keeps the v0.1.0 surface small and avoids picking an async runtime
//! prematurely.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Provider identity and capabilities
// ---------------------------------------------------------------------------

/// Stable identifier for the LLM behind a client. Recorded verbatim
/// in every `LlmCallRecord` so replay can match exactly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderIdentity {
    pub vendor: String,
    pub model_family: String,
    pub model_version: String,
    pub endpoint: Option<String>,
}

/// What a provider supports natively. Polyfills consult this to
/// decide whether to wrap or invoke directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub json_mode_native: bool,
    pub tool_use_native: bool,
    pub streaming_native: bool,
    pub prompt_caching_native: bool,
    pub multimodal_image_input: bool,
    pub max_context_window: usize,
}

impl Capabilities {
    /// A reasonable default for a frontier cloud provider in 2026.
    pub fn modern_frontier() -> Self {
        Self {
            json_mode_native: true,
            tool_use_native: true,
            streaming_native: true,
            prompt_caching_native: true,
            multimodal_image_input: true,
            max_context_window: 200_000,
        }
    }
}

// ---------------------------------------------------------------------------
// Request / response shapes
// ---------------------------------------------------------------------------

/// Conversation role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Message content. `Multimodal` accommodates image / audio blocks
/// for providers that support it; polyfills fall back to text.
#[derive(Debug, Clone, PartialEq)]
pub enum MessageContent {
    Text(String),
    Multimodal(Vec<ContentBlock>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    Text(String),
    /// Base64-encoded image data with MIME type.
    ImageBase64 { mime_type: String, data: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(text.into()),
        }
    }
}

/// Neutral request — what the framework hands to any `LlmClient`.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub max_tokens: Option<usize>,
    pub stop_sequences: Vec<String>,
    pub seed: Option<u64>,
    pub metadata: HashMap<String, String>,
}

/// Token usage reported by the provider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cached_tokens: usize,
}

/// Why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,           // natural end / stop sequence hit
    MaxTokens,
    Refusal,
    Other,
}

/// Standard completion response.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: FinishReason,
    pub provider_id: ProviderIdentity,
    pub latency_ms: u64,
}

/// Structured-output (JSON) response. `polyfill_used` records
/// whether the JSON came via a native JSON mode or via a
/// prompt-wrap + parse + validate fallback.
#[derive(Debug, Clone)]
pub struct CompletionJsonResponse {
    pub raw_text: String,
    pub parsed: serde_json::Value,
    pub schema_valid: bool,
    pub model: String,
    pub usage: TokenUsage,
    pub provider_id: ProviderIdentity,
    pub latency_ms: u64,
    pub polyfill_used: bool,
}

/// JSON schema reference. Kept as a string-keyed schema document for
/// v0.1.0 so the crate doesn't depend on a specific schema-validator
/// library. Real providers / polyfills will deserialize as needed.
#[derive(Debug, Clone)]
pub struct JsonSchema {
    pub name: String,
    pub schema_json: String,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// What can go wrong when an LLM call is made. Every variant carries
/// the provider identity for telemetry / audit.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmError {
    Transport {
        provider: ProviderIdentity,
        detail: String,
    },
    RateLimit {
        provider: ProviderIdentity,
        retry_after_ms: Option<u64>,
    },
    ContextTooLarge {
        provider: ProviderIdentity,
        requested_tokens: usize,
        max_tokens: usize,
    },
    ProtocolError {
        provider: ProviderIdentity,
        detail: String,
    },
    Auth {
        provider: ProviderIdentity,
        detail: String,
    },
    SchemaInvalid {
        provider: ProviderIdentity,
        schema_name: String,
        raw_text: String,
        validator_error: String,
    },
    Refused {
        provider: ProviderIdentity,
        reason: Option<String>,
    },
    /// The provider stopped generating because the `max_tokens` budget
    /// was exhausted before the model produced a usable output. For
    /// thinking-mode models (Gemma, Claude with extended thinking,
    /// DeepSeek-R1, …) this commonly happens when the budget is small
    /// enough to fit the chain-of-thought but not the final answer,
    /// leaving `content` empty. Primitives translate this to a
    /// retry-with-bigger-cap loop rather than failing on an
    /// uninterpretable empty response.
    OutputTruncated {
        provider: ProviderIdentity,
        output_tokens: usize,
        max_tokens: Option<usize>,
    },
    Other {
        provider: ProviderIdentity,
        message: String,
    },
}

impl LlmError {
    pub fn provider(&self) -> &ProviderIdentity {
        match self {
            LlmError::Transport { provider, .. }
            | LlmError::RateLimit { provider, .. }
            | LlmError::ContextTooLarge { provider, .. }
            | LlmError::ProtocolError { provider, .. }
            | LlmError::Auth { provider, .. }
            | LlmError::SchemaInvalid { provider, .. }
            | LlmError::Refused { provider, .. }
            | LlmError::OutputTruncated { provider, .. }
            | LlmError::Other { provider, .. } => provider,
        }
    }
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::Transport { detail, .. } => write!(f, "transport error: {detail}"),
            LlmError::RateLimit { .. } => write!(f, "rate limited"),
            LlmError::ContextTooLarge { requested_tokens, max_tokens, .. } => {
                write!(f, "context too large: {requested_tokens} > {max_tokens}")
            }
            LlmError::ProtocolError { detail, .. } => write!(f, "protocol error: {detail}"),
            LlmError::Auth { detail, .. } => write!(f, "auth error: {detail}"),
            LlmError::SchemaInvalid {
                schema_name, validator_error, ..
            } => write!(f, "schema {schema_name} invalid: {validator_error}"),
            LlmError::Refused { reason, .. } => match reason {
                Some(r) => write!(f, "refused: {r}"),
                None => write!(f, "refused"),
            },
            LlmError::OutputTruncated {
                output_tokens, max_tokens, ..
            } => match max_tokens {
                Some(max) => write!(
                    f,
                    "output truncated: model emitted {output_tokens} tokens \
                     and stopped at the max_tokens cap ({max}); raise the cap and retry",
                ),
                None => write!(
                    f,
                    "output truncated at {output_tokens} tokens (no cap reported)",
                ),
            },
            LlmError::Other { message, .. } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for LlmError {}

// ---------------------------------------------------------------------------
// The trait
// ---------------------------------------------------------------------------

/// The contract every LLM provider implements. Synchronous in
/// v0.1.0; async wrappers can be added later without breaking
/// callers.
pub trait LlmClient: Send + Sync {
    fn identity(&self) -> ProviderIdentity;
    fn capabilities(&self) -> Capabilities;
    fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError>;
    fn complete_json(
        &self,
        req: CompletionRequest,
        schema: &JsonSchema,
    ) -> Result<CompletionJsonResponse, LlmError>;
}

// ---------------------------------------------------------------------------
// Request fingerprint (for the mock provider's keying)
// ---------------------------------------------------------------------------

/// A content-addressed hash of a request — used as the cache key in
/// `MockLlmClient`. Excludes `temperature` and `seed` so retries
/// match the same scripted response.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestFingerprint(pub String);

impl RequestFingerprint {
    pub fn new(model: &str, system: Option<&str>, messages: &[Message]) -> Self {
        // A simple deterministic hash without an external crate.
        // Format: model | system | role:text | role:text | ...
        let mut s = String::new();
        s.push_str(model);
        s.push('|');
        if let Some(sys) = system {
            s.push_str(sys);
        }
        s.push('|');
        for m in messages {
            s.push_str(role_str(m.role));
            s.push(':');
            match &m.content {
                MessageContent::Text(t) => s.push_str(t),
                MessageContent::Multimodal(blocks) => {
                    for b in blocks {
                        match b {
                            ContentBlock::Text(t) => s.push_str(t),
                            ContentBlock::ImageBase64 { mime_type, data } => {
                                s.push_str(mime_type);
                                s.push(':');
                                s.push_str(&data[..data.len().min(32)]);
                            }
                        }
                    }
                }
            }
            s.push('|');
        }
        Self(s)
    }
}

fn role_str(r: Role) -> &'static str {
    match r {
        Role::System => "S",
        Role::User => "U",
        Role::Assistant => "A",
    }
}

// ---------------------------------------------------------------------------
// MockLlmClient
// ---------------------------------------------------------------------------

/// What the mock should return for a particular request.
#[derive(Debug, Clone)]
pub enum MockResponse {
    /// Plain text completion.
    Text(String),
    /// Structured JSON response. Both raw_text and parsed.
    Json {
        raw_text: String,
        parsed: serde_json::Value,
    },
    /// An error that mimics a real provider failure.
    Error(LlmError),
}

/// What the mock does when a request has no matching script entry.
#[derive(Debug, Clone)]
pub enum MockDefault {
    /// Fail the test loudly. Recommended default for production-
    /// flavored tests so missing mock entries surface as test
    /// failures rather than silent passes.
    StrictFail,
    /// Return this response for any unscripted request.
    Permissive(MockResponse),
}

/// Deterministic in-process LLM stand-in for tests. Holds a
/// fingerprint → response script; returns the matching response or
/// applies the default policy.
pub struct MockLlmClient {
    script: HashMap<RequestFingerprint, MockResponse>,
    default: MockDefault,
    identity: ProviderIdentity,
    capabilities: Capabilities,
    /// Optional call counter for telemetry-style tests.
    call_count: AtomicU64,
}

impl MockLlmClient {
    pub fn new() -> Self {
        Self {
            script: HashMap::new(),
            default: MockDefault::StrictFail,
            identity: ProviderIdentity {
                vendor: "mock".to_string(),
                model_family: "mock-test".to_string(),
                model_version: "v1".to_string(),
                endpoint: None,
            },
            capabilities: Capabilities {
                json_mode_native: true,
                tool_use_native: true,
                streaming_native: true,
                prompt_caching_native: false,
                multimodal_image_input: true,
                max_context_window: 1_000_000,
            },
            call_count: AtomicU64::new(0),
        }
    }

    pub fn with_response(mut self, fp: RequestFingerprint, resp: MockResponse) -> Self {
        self.script.insert(fp, resp);
        self
    }

    pub fn with_default(mut self, resp: MockResponse) -> Self {
        self.default = MockDefault::Permissive(resp);
        self
    }

    pub fn with_strict_default(mut self) -> Self {
        self.default = MockDefault::StrictFail;
        self
    }

    pub fn with_identity(mut self, id: ProviderIdentity) -> Self {
        self.identity = id;
        self
    }

    pub fn with_capabilities(mut self, caps: Capabilities) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn call_count(&self) -> u64 {
        self.call_count.load(Ordering::SeqCst)
    }

    fn lookup(&self, fp: &RequestFingerprint) -> Result<&MockResponse, LlmError> {
        if let Some(r) = self.script.get(fp) {
            return Ok(r);
        }
        match &self.default {
            MockDefault::Permissive(r) => Ok(r),
            MockDefault::StrictFail => Err(LlmError::Other {
                provider: self.identity.clone(),
                message: format!(
                    "MockLlmClient: no scripted response for fingerprint '{}'",
                    fp.0
                ),
            }),
        }
    }
}

impl Default for MockLlmClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmClient for MockLlmClient {
    fn identity(&self) -> ProviderIdentity {
        self.identity.clone()
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let fp = RequestFingerprint::new(&req.model, req.system.as_deref(), &req.messages);
        match self.lookup(&fp)? {
            MockResponse::Text(text) => Ok(CompletionResponse {
                text: text.clone(),
                model: req.model,
                usage: TokenUsage::default(),
                finish_reason: FinishReason::Stop,
                provider_id: self.identity.clone(),
                latency_ms: 0,
            }),
            MockResponse::Json { raw_text, .. } => Ok(CompletionResponse {
                text: raw_text.clone(),
                model: req.model,
                usage: TokenUsage::default(),
                finish_reason: FinishReason::Stop,
                provider_id: self.identity.clone(),
                latency_ms: 0,
            }),
            MockResponse::Error(e) => Err(e.clone()),
        }
    }

    fn complete_json(
        &self,
        req: CompletionRequest,
        _schema: &JsonSchema,
    ) -> Result<CompletionJsonResponse, LlmError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let fp = RequestFingerprint::new(&req.model, req.system.as_deref(), &req.messages);
        match self.lookup(&fp)? {
            MockResponse::Json { raw_text, parsed } => Ok(CompletionJsonResponse {
                raw_text: raw_text.clone(),
                parsed: parsed.clone(),
                schema_valid: true,
                model: req.model,
                usage: TokenUsage::default(),
                provider_id: self.identity.clone(),
                latency_ms: 0,
                polyfill_used: false,
            }),
            MockResponse::Text(t) => Ok(CompletionJsonResponse {
                raw_text: t.clone(),
                parsed: serde_json::Value::Null,
                schema_valid: false,
                model: req.model,
                usage: TokenUsage::default(),
                provider_id: self.identity.clone(),
                latency_ms: 0,
                polyfill_used: true,
            }),
            MockResponse::Error(e) => Err(e.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// Inline tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_identity_equality_and_hash() {
        let a = ProviderIdentity {
            vendor: "mock".into(),
            model_family: "x".into(),
            model_version: "1".into(),
            endpoint: None,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn capabilities_modern_frontier_has_everything_native() {
        let c = Capabilities::modern_frontier();
        assert!(c.json_mode_native);
        assert!(c.tool_use_native);
        assert!(c.streaming_native);
        assert!(c.prompt_caching_native);
        assert!(c.multimodal_image_input);
        assert!(c.max_context_window > 100_000);
    }

    #[test]
    fn message_helpers_produce_correct_role() {
        assert_eq!(Message::user("x").role, Role::User);
        assert_eq!(Message::system("y").role, Role::System);
        assert_eq!(Message::assistant("z").role, Role::Assistant);
    }

    #[test]
    fn request_fingerprint_is_deterministic() {
        let m = [Message::user("hi")];
        let fp1 = RequestFingerprint::new("model", Some("sys"), &m);
        let fp2 = RequestFingerprint::new("model", Some("sys"), &m);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn request_fingerprint_excludes_temperature_and_seed() {
        // Different temperature / seed but identical otherwise -> same fp.
        let m1 = [Message::user("hi")];
        let m2 = [Message::user("hi")];
        let fp1 = RequestFingerprint::new("model", None, &m1);
        let fp2 = RequestFingerprint::new("model", None, &m2);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn request_fingerprint_changes_with_message_content() {
        let m1 = [Message::user("hi")];
        let m2 = [Message::user("hello")];
        let fp1 = RequestFingerprint::new("model", None, &m1);
        let fp2 = RequestFingerprint::new("model", None, &m2);
        assert_ne!(fp1, fp2);
    }

    fn build_request(prompt: &str) -> CompletionRequest {
        CompletionRequest {
            model: "test".to_string(),
            system: None,
            messages: vec![Message::user(prompt)],
            temperature: 0.0,
            max_tokens: Some(64),
            stop_sequences: vec![],
            seed: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn mock_returns_scripted_response() {
        let req = build_request("ping");
        let fp = RequestFingerprint::new(&req.model, req.system.as_deref(), &req.messages);

        let mock = MockLlmClient::new()
            .with_response(fp, MockResponse::Text("pong".into()));
        let resp = mock.complete(req).unwrap();
        assert_eq!(resp.text, "pong");
        assert_eq!(resp.provider_id.vendor, "mock");
        assert_eq!(mock.call_count(), 1);
    }

    #[test]
    fn mock_strict_default_fails_loudly_on_unscripted() {
        let mock = MockLlmClient::new(); // strict by default
        let result = mock.complete(build_request("unknown"));
        match result {
            Err(LlmError::Other { provider, message }) => {
                assert_eq!(provider.vendor, "mock");
                assert!(
                    message.contains("no scripted response"),
                    "expected helpful message, got: {message}"
                );
            }
            other => panic!("expected strict-fail error, got {:?}", other),
        }
    }

    #[test]
    fn mock_permissive_default_returns_default_response() {
        let mock = MockLlmClient::new().with_default(MockResponse::Text("default".into()));
        let resp = mock.complete(build_request("anything")).unwrap();
        assert_eq!(resp.text, "default");
    }

    #[test]
    fn mock_returns_scripted_error() {
        let req = build_request("fail");
        let fp = RequestFingerprint::new(&req.model, req.system.as_deref(), &req.messages);
        let mock = MockLlmClient::new().with_response(
            fp,
            MockResponse::Error(LlmError::RateLimit {
                provider: ProviderIdentity {
                    vendor: "mock".into(),
                    model_family: "test".into(),
                    model_version: "v1".into(),
                    endpoint: None,
                },
                retry_after_ms: Some(1000),
            }),
        );
        match mock.complete(req) {
            Err(LlmError::RateLimit { retry_after_ms, .. }) => {
                assert_eq!(retry_after_ms, Some(1000));
            }
            other => panic!("expected RateLimit, got {:?}", other),
        }
    }

    #[test]
    fn mock_complete_json_parses_scripted_json() {
        let req = build_request("json");
        let fp = RequestFingerprint::new(&req.model, req.system.as_deref(), &req.messages);
        let parsed = serde_json::json!({"answer": 42});
        let mock = MockLlmClient::new().with_response(
            fp,
            MockResponse::Json {
                raw_text: r#"{"answer": 42}"#.into(),
                parsed: parsed.clone(),
            },
        );
        let schema = JsonSchema {
            name: "test".into(),
            schema_json: "{}".into(),
        };
        let resp = mock.complete_json(req, &schema).unwrap();
        assert_eq!(resp.parsed, parsed);
        assert!(resp.schema_valid);
        assert!(!resp.polyfill_used);
    }

    #[test]
    fn mock_call_count_increments() {
        let mock = MockLlmClient::new().with_default(MockResponse::Text("ok".into()));
        let _ = mock.complete(build_request("a"));
        let _ = mock.complete(build_request("b"));
        let _ = mock.complete(build_request("c"));
        assert_eq!(mock.call_count(), 3);
    }

    #[test]
    fn mock_with_identity_override() {
        let mock = MockLlmClient::new()
            .with_identity(ProviderIdentity {
                vendor: "test-vendor".into(),
                model_family: "test-family".into(),
                model_version: "9.9".into(),
                endpoint: Some("http://example.local".into()),
            })
            .with_default(MockResponse::Text("ok".into()));
        assert_eq!(mock.identity().vendor, "test-vendor");
        let resp = mock.complete(build_request("x")).unwrap();
        assert_eq!(resp.provider_id.vendor, "test-vendor");
    }

    #[test]
    fn llm_error_carries_provider_identity() {
        let id = ProviderIdentity {
            vendor: "v".into(),
            model_family: "f".into(),
            model_version: "0".into(),
            endpoint: None,
        };
        let e = LlmError::Auth {
            provider: id.clone(),
            detail: "bad key".into(),
        };
        assert_eq!(e.provider().vendor, "v");
        let display = format!("{}", e);
        assert!(display.contains("bad key"));
    }

    #[test]
    fn llm_error_display_is_useful() {
        let p = ProviderIdentity {
            vendor: "v".into(),
            model_family: "f".into(),
            model_version: "0".into(),
            endpoint: None,
        };
        assert!(
            format!("{}", LlmError::ContextTooLarge { provider: p.clone(), requested_tokens: 100_000, max_tokens: 50_000 })
                .contains("100000 > 50000")
        );
        assert!(format!("{}", LlmError::RateLimit { provider: p.clone(), retry_after_ms: None })
            .contains("rate limited"));
    }
}
