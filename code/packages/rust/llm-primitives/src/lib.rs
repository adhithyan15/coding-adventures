// LlmError carried inside PrimitiveError::Gateway is itself a large
// variant; the same audit-trail discipline (every error carries full
// context) is worth the size cost as long as errors are not on a hot
// path. Box later if profiling shows it.
#![allow(clippy::result_large_err)]

//! # llm-primitives — typed LLM operations for the adjudication framework
//!
//! Reference skeleton for
//! [LM00b](../../../specs/LM00b-llm-primitives.md).
//!
//! This crate defines the **scaffolding** every primitive needs and
//! that every framework consumer (extractor, ADJ02–05, ADJ06, ADJ09)
//! programs against:
//!
//!   * [`Role`] — the role a primitive plays in the framework
//!     (extractor / renderer / nli / adversary / plausibility /
//!     rule_extractor). The deployment maps roles to concrete
//!     [`LlmClient`](llm_gateway::LlmClient) instances so that the
//!     same primitive can run against different models in different
//!     deployments.
//!   * [`GatewayConfig`] — the role-keyed registry of clients.
//!   * [`LlmCallRecord`] — the audit-trail record produced by every
//!     LLM call. Carries the provider identity, the prompt version
//!     and hash, token usage, and finish reason — enough to replay
//!     and to attribute cost.
//!   * [`PrimitiveCallRecord`] — wraps one or more `LlmCallRecord`s
//!     (one per retry attempt) plus the primitive-level context
//!     (primitive name, attempts, cache hit, total cost).
//!   * [`PrimitiveError`] — the failure shape every primitive returns.
//!   * Six [`PROMPT_VERSION_*`](DECOMPOSE_TEXT_PROMPT_VERSION) constants —
//!     one per primitive — so the framework can carry the version
//!     into the audit trail without each primitive crate hard-coding
//!     a magic string.
//!
//! ## Primitives shipped here
//!
//! Each of the six LM00b primitives ships in its own module so they
//! can land in parallel without conflicting on one giant file:
//!
//!   * [`entail`] — bidirectional textual entailment (v0.2.0)
//!   * [`render_node`] — faithful natural-language rendering of an
//!     IR node (v0.3.0)
//!   * [`judge_plausibility`] — ADJ05 binary plausibility judge (v0.4.0)
//!   * [`find_contradicting_reading`] — ADJ05 adversary (v0.5.0)
//!   * [`decompose_text`] — headline extractor (v0.6.0)
//!   * `extract_rules` — rule extractor; not yet here
//!
//! Until a primitive lands, its `PROMPT_VERSION_*` constant is the
//! only thing this crate exposes for it.
//!
//! ## Why a separate `LlmCallRecord` here
//!
//! The audit-trail record lives in `llm-primitives` rather than
//! `llm-gateway` because it's the *primitive layer* that always
//! produces one — the gateway trait is intentionally I/O-only and
//! does not concern itself with audit semantics. A future refactor
//! may pull `LlmCallRecord` down into `llm-gateway` if other
//! gateway-direct consumers appear; for now its home is here.

use std::collections::HashMap;

use llm_gateway::{
    CompletionRequest, FinishReason, LlmClient, LlmError, ProviderIdentity, TokenUsage,
};

pub mod decompose_text;
pub mod entail;
pub mod find_contradicting_reading;
pub mod judge_plausibility;
pub mod render_node;

// Re-exports at the crate root. Each primitive's function and module
// share a name; Rust allows that since they live in different
// namespaces, so callers can write `llm_primitives::entail(...)` /
// `llm_primitives::render_node(...)` / `llm_primitives::judge_plausibility(...)`
// directly.
pub use decompose_text::{decompose_text, DecomposeTextRequest, DecomposeTextResponse};
pub use entail::{entail, EntailRequest, EntailResponse};
pub use find_contradicting_reading::{
    find_contradicting_reading, FindContradictingReadingRequest, FindContradictingReadingResponse,
};
pub use judge_plausibility::{
    judge_plausibility, JudgePlausibilityRequest, JudgePlausibilityResponse,
};
pub use render_node::{render_node, RenderNodeRequest, RenderNodeResponse, RenderStyle};

// ---------------------------------------------------------------------------
// Roles
// ---------------------------------------------------------------------------

/// Why role-based dispatch instead of just passing an `LlmClient`?
///
/// Different primitives play different roles in the adversarial /
/// audit-trail pipeline, and the **deployment** — not the primitive —
/// decides which model serves which role. Two reasons:
///
/// 1. **Independence requirement (ADJ05).** The extractor and the
///    adversary must come from different model families; if both
///    were the same client, the adversary would just rubber-stamp
///    the extraction. The framework enforces independence by checking
///    that `Role::Extractor` and `Role::Adversary` map to different
///    `ProviderIdentity::model_family` values.
/// 2. **Cost / latency policy.** Renderers and plausibility judges
///    can be cheap small models; extractors and adversaries usually
///    want the frontier. Pinning the model in primitive code would
///    force every deployment to negotiate a hard-coded choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Used by `decompose_text` and `extract_rules`. Heavy lifting —
    /// the deployment usually maps this to the frontier model.
    Extractor,
    /// Used by `render_node`. Faithful trivial paraphrase; small
    /// cheap model is fine.
    Renderer,
    /// Used by `entail`. The framework recommends a purpose-trained
    /// NLI model here (different from the renderer to avoid self-
    /// confirmation per ADJ04).
    Nli,
    /// Used by `find_contradicting_reading`. ADJ05 requires this to
    /// be a different model family from `Extractor`.
    Adversary,
    /// Used by `judge_plausibility`. Decision is binary; small model
    /// is fine.
    Plausibility,
    /// Used by `extract_rules`. Defaults to the same client as
    /// `Extractor`; deployments can override.
    RuleExtractor,
}

impl Role {
    /// Stable string name for audit-trail records.
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Extractor => "extractor",
            Role::Renderer => "renderer",
            Role::Nli => "nli",
            Role::Adversary => "adversary",
            Role::Plausibility => "plausibility",
            Role::RuleExtractor => "rule_extractor",
        }
    }
}

// ---------------------------------------------------------------------------
// GatewayConfig — role → client registry
// ---------------------------------------------------------------------------

/// Maps roles to concrete `LlmClient` instances.
///
/// Deployments construct this once at startup and pass it to every
/// primitive call. The framework treats it as immutable for the
/// duration of a request — swapping a client mid-request would
/// muddle the audit trail.
///
/// The same `LlmClient` may be registered against multiple roles
/// (e.g., a single small model serves both `Renderer` and
/// `Plausibility`); the framework's only hard rule is the
/// extractor/adversary independence check, performed by
/// [`GatewayConfig::check_independence`].
pub struct GatewayConfig {
    clients: HashMap<Role, Box<dyn LlmClient>>,
}

impl GatewayConfig {
    pub fn new() -> Self {
        Self { clients: HashMap::new() }
    }

    /// Register a client against a role. Replaces any prior client.
    pub fn with_client(mut self, role: Role, client: Box<dyn LlmClient>) -> Self {
        self.clients.insert(role, client);
        self
    }

    /// Look up the client for a role. Returns `None` if the role has
    /// no registered client; primitives translate this to
    /// [`PrimitiveError::NoClientForRole`].
    pub fn client(&self, role: Role) -> Option<&dyn LlmClient> {
        self.clients.get(&role).map(|c| c.as_ref())
    }

    /// ADJ05 independence check: `Extractor` and `Adversary` must
    /// come from different model families. Returns a description of
    /// the violation if both roles are registered and share a family;
    /// returns `Ok(())` otherwise (including when one or both roles
    /// are unregistered — that's a separate error surfaced at call time).
    pub fn check_independence(&self) -> Result<(), IndependenceViolation> {
        let extractor = self.clients.get(&Role::Extractor).map(|c| c.identity());
        let adversary = self.clients.get(&Role::Adversary).map(|c| c.identity());
        if let (Some(e), Some(a)) = (extractor, adversary) {
            if e.vendor == a.vendor && e.model_family == a.model_family {
                return Err(IndependenceViolation {
                    extractor: e,
                    adversary: a,
                });
            }
        }
        Ok(())
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Surfaced by [`GatewayConfig::check_independence`] when the
/// extractor and adversary share a model family — a configuration
/// bug that ADJ05 explicitly forbids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndependenceViolation {
    pub extractor: ProviderIdentity,
    pub adversary: ProviderIdentity,
}

impl std::fmt::Display for IndependenceViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ADJ05 independence violated: extractor and adversary both \
             use {vendor}/{family}",
            vendor = self.extractor.vendor,
            family = self.extractor.model_family,
        )
    }
}

impl std::error::Error for IndependenceViolation {}

// ---------------------------------------------------------------------------
// LlmCallRecord — per-call audit-trail row
// ---------------------------------------------------------------------------

/// One row of the LLM audit trail. Produced by every LLM call a
/// primitive makes (including retries) and emitted into the
/// document's audit trail (ADJ07).
///
/// The record is intentionally **content-addressed**: `prompt_hash`
/// is a deterministic hash of the prompt text, so a replay can match
/// recorded calls exactly without storing the prompt verbatim
/// alongside every record. The full prompt lives once in the prompts
/// directory under the same version.
///
/// `Eq` is intentionally not derived because `cost_usd: f64`
/// (`f64` is not `Eq` since NaN ≠ NaN). Replay comparison should go
/// through `prompt_hash` rather than struct-level equality.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmCallRecord {
    /// Which primitive made this call (e.g., `"decompose_text"`).
    pub primitive: String,
    /// Which role's client was used (e.g., `"extractor"`).
    pub role: String,
    /// The prompt-version string baked into the call.
    pub prompt_version: String,
    /// Content-addressed hash of the rendered prompt (system + messages).
    pub prompt_hash: String,
    /// The provider identity at call time. Recorded verbatim so
    /// replay can detect "model drift" — same prompt, different
    /// model version producing different output.
    pub provider: ProviderIdentity,
    /// Token usage as reported by the provider.
    pub usage: TokenUsage,
    /// Why the provider stopped generating.
    pub finish_reason: FinishReason,
    /// Wall-clock latency of this call in milliseconds.
    pub latency_ms: u64,
    /// Estimated cost in USD. Zero for local providers by default.
    pub cost_usd: f64,
}

// ---------------------------------------------------------------------------
// PrimitiveCallRecord — wraps the per-primitive context
// ---------------------------------------------------------------------------

/// What the audit trail records for one primitive invocation. Wraps
/// the per-attempt `LlmCallRecord` list with primitive-level context
/// (was it a cache hit, how many attempts, total cost).
#[derive(Debug, Clone, PartialEq)]
pub struct PrimitiveCallRecord {
    pub primitive: String,
    pub prompt_version: String,
    pub role: String,
    /// Hash of the typed input (e.g., the source text and domain hint).
    pub inputs_hash: String,
    /// Hash of the typed output (e.g., the IR document).
    pub outputs_hash: String,
    /// Whether the primitive cache served this call without an LLM round-trip.
    pub cache_hit: bool,
    /// Number of LLM attempts including retries. `1` on success-first-try.
    pub attempts: usize,
    /// One record per attempt. Empty on a cache hit; the cache may
    /// preserve the original record separately.
    pub llm_calls: Vec<LlmCallRecord>,
    pub total_cost_usd: f64,
}

// ---------------------------------------------------------------------------
// PrimitiveError
// ---------------------------------------------------------------------------

/// Every primitive returns `Result<_, PrimitiveError>`. The four
/// variants cover the failure axes ADJ06 (clarification) needs to
/// distinguish to render a useful question.
#[derive(Debug)]
pub enum PrimitiveError {
    /// Gateway-level error (transport, auth, rate limit, refused, …).
    /// ADJ06 cannot meaningfully clarify these; they're operator-
    /// surface errors.
    Gateway(LlmError),

    /// The LLM's output failed JSON-schema validation after N retries.
    /// ADJ06 surfaces the last response so a reviewer can see what
    /// the model actually produced.
    ValidationExhausted {
        last_response: String,
        last_error: String,
        attempts: usize,
    },

    /// The output parsed and validated against the JSON schema but
    /// failed a framework-specific check (e.g., ADJ02 coverage, ADJ01
    /// well-formedness). ADJ06 turns this into a structural
    /// clarification ("the IR doesn't cover X — please review").
    StructuralFailure {
        check_name: String,
        detail: String,
    },

    /// The deployment's `GatewayConfig` has no client registered for
    /// a role the primitive needs. This is a configuration bug, not
    /// a runtime failure; the framework wants to surface it loudly.
    NoClientForRole { role: Role },
}

impl std::fmt::Display for PrimitiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveError::Gateway(e) => write!(f, "gateway error: {e}"),
            PrimitiveError::ValidationExhausted { last_error, attempts, .. } => write!(
                f,
                "validation exhausted after {attempts} attempts: {last_error}",
            ),
            PrimitiveError::StructuralFailure { check_name, detail } => {
                write!(f, "structural failure in {check_name}: {detail}")
            }
            PrimitiveError::NoClientForRole { role } => {
                write!(f, "no client registered for role {}", role.as_str())
            }
        }
    }
}

impl std::error::Error for PrimitiveError {}

impl From<LlmError> for PrimitiveError {
    fn from(e: LlmError) -> Self {
        PrimitiveError::Gateway(e)
    }
}

// ---------------------------------------------------------------------------
// Prompt versions
// ---------------------------------------------------------------------------

// Each constant pairs with the prompt template that lives (or will
// live) under `code/packages/rust/llm-primitives/src/prompts/<name>.md`.
// Bumping the version (e.g., `"-v2"`) is the audited way to change a
// prompt: every `LlmCallRecord` carries the version, so replay
// matches `(prompt_version, prompt_hash)`.

pub const DECOMPOSE_TEXT_PROMPT_VERSION: &str = "decompose-text-v2";
pub const RENDER_NODE_PROMPT_VERSION: &str = "render-node-v1";
pub const ENTAIL_PROMPT_VERSION: &str = "entail-v1";
pub const ADVERSARY_PROMPT_VERSION: &str = "adversary-v1";
pub const PLAUSIBILITY_PROMPT_VERSION: &str = "plausibility-v1";
pub const EXTRACT_RULES_PROMPT_VERSION: &str = "extract-rules-v1";

// ---------------------------------------------------------------------------
// Retry helpers — thinking-mode-tolerant
// ---------------------------------------------------------------------------

/// Default ceiling for the retry-with-bigger-cap loop. Primitives
/// double their `max_tokens` budget on each [`LlmError::OutputTruncated`]
/// attempt and stop once the cap reaches this number. Frontier
/// thinking-mode models that need more than 32k tokens to answer a
/// single primitive question are misconfigured at a higher level —
/// the primitive does not paper over that with an unbounded loop.
pub const MAX_TOKENS_CEILING: usize = 32_768;

/// How many attempts a primitive makes before giving up on a
/// truncation loop. With `MAX_TOKENS_CEILING = 32_768` and a starting
/// cap of `1024`, doubling lands us at 32_768 on attempt 5 (1024 →
/// 2048 → 4096 → 8192 → 16384 → 32768).
pub const TRUNCATION_MAX_ATTEMPTS: usize = 6;

/// Run a `complete_json` call against `client`, doubling the
/// `max_tokens` budget on every [`LlmError::OutputTruncated`] up to
/// [`MAX_TOKENS_CEILING`] / [`TRUNCATION_MAX_ATTEMPTS`].
///
/// This is the helper every JSON-emitting primitive uses to survive
/// thinking-mode models. The model's chain-of-thought eats tokens
/// before it ever emits structured content; a fixed cap means
/// "sometimes works, sometimes returns empty content + `done_reason:
/// length`". The retry loop turns that into "always works as long as
/// the model can produce *some* JSON within the ceiling."
///
/// On any other [`LlmError`] (transport, schema-invalid with non-empty
/// content, refused, …) the helper returns immediately — those are
/// not retryable here. The primitive's own validation layer above is
/// responsible for retry-with-correction on schema mismatches.
pub fn complete_json_with_truncation_retry(
    client: &dyn llm_gateway::LlmClient,
    base: CompletionRequest,
    schema: &llm_gateway::JsonSchema,
) -> Result<llm_gateway::CompletionJsonResponse, LlmError> {
    let mut cap = base.max_tokens.unwrap_or(1024).max(1024);
    for attempt in 1..=TRUNCATION_MAX_ATTEMPTS {
        let mut req = base.clone();
        req.max_tokens = Some(cap);
        match client.complete_json(req, schema) {
            Ok(resp) => return Ok(resp),
            Err(LlmError::OutputTruncated { .. }) if attempt < TRUNCATION_MAX_ATTEMPTS => {
                let next = cap.saturating_mul(2);
                if next > MAX_TOKENS_CEILING {
                    cap = MAX_TOKENS_CEILING;
                } else {
                    cap = next;
                }
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    // Unreachable: the loop either returns Ok, returns Err on
    // attempt < MAX_ATTEMPTS, or returns Err on the final attempt
    // via the `Err(e) => return Err(e)` arm.
    unreachable!()
}

/// Same retry loop, but for `complete` (free-form text). Same
/// semantics: double the cap on `OutputTruncated`, return immediately
/// on any other error.
pub fn complete_with_truncation_retry(
    client: &dyn llm_gateway::LlmClient,
    base: CompletionRequest,
) -> Result<llm_gateway::CompletionResponse, LlmError> {
    let mut cap = base.max_tokens.unwrap_or(1024).max(1024);
    for attempt in 1..=TRUNCATION_MAX_ATTEMPTS {
        let mut req = base.clone();
        req.max_tokens = Some(cap);
        match client.complete(req) {
            Ok(resp) => return Ok(resp),
            Err(LlmError::OutputTruncated { .. }) if attempt < TRUNCATION_MAX_ATTEMPTS => {
                let next = cap.saturating_mul(2);
                if next > MAX_TOKENS_CEILING {
                    cap = MAX_TOKENS_CEILING;
                } else {
                    cap = next;
                }
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

// ---------------------------------------------------------------------------
// Prompt fingerprinting (deterministic, no external crate)
// ---------------------------------------------------------------------------

/// Content-addressed hash of a [`CompletionRequest`]'s prompt portion
/// (system + messages, ignoring temperature and seed so a retry of
/// the same prompt matches the same fingerprint).
///
/// This is the helper every primitive uses to populate
/// `LlmCallRecord::prompt_hash`. The implementation is a simple
/// FNV-1a 64-bit hash rendered as hex; cryptographic strength is
/// not required for replay-matching, only determinism.
pub fn fingerprint_prompt(req: &CompletionRequest) -> String {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a 64-bit offset basis
    fn step(h: &mut u64, bytes: &[u8]) {
        for b in bytes {
            *h ^= u64::from(*b);
            *h = h.wrapping_mul(0x100000001b3); // FNV-1a 64-bit prime
        }
    }
    step(&mut h, req.model.as_bytes());
    step(&mut h, b"|");
    if let Some(sys) = &req.system {
        step(&mut h, sys.as_bytes());
    }
    step(&mut h, b"|");
    for msg in &req.messages {
        match msg.role {
            llm_gateway::Role::System => step(&mut h, b"S:"),
            llm_gateway::Role::User => step(&mut h, b"U:"),
            llm_gateway::Role::Assistant => step(&mut h, b"A:"),
        }
        match &msg.content {
            llm_gateway::MessageContent::Text(t) => step(&mut h, t.as_bytes()),
            llm_gateway::MessageContent::Multimodal(blocks) => {
                for b in blocks {
                    if let llm_gateway::ContentBlock::Text(text) = b {
                        step(&mut h, text.as_bytes());
                    }
                }
            }
        }
        step(&mut h, b"|");
    }
    format!("{:016x}", h)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway::{
        CompletionRequest, Message, MessageContent, MockLlmClient, ProviderIdentity,
        Role as MsgRole,
    };

    fn req_with_text(text: &str) -> CompletionRequest {
        CompletionRequest {
            model: "test-model".into(),
            system: None,
            messages: vec![Message {
                role: MsgRole::User,
                content: MessageContent::Text(text.into()),
            }],
            temperature: 0.0,
            max_tokens: None,
            stop_sequences: Vec::new(),
            seed: None,
            metadata: Default::default(),
        }
    }

    #[test]
    fn role_as_str_is_stable() {
        assert_eq!(Role::Extractor.as_str(), "extractor");
        assert_eq!(Role::Adversary.as_str(), "adversary");
        assert_eq!(Role::Nli.as_str(), "nli");
        assert_eq!(Role::Plausibility.as_str(), "plausibility");
        assert_eq!(Role::Renderer.as_str(), "renderer");
        assert_eq!(Role::RuleExtractor.as_str(), "rule_extractor");
    }

    #[test]
    fn empty_gateway_returns_no_client() {
        let g = GatewayConfig::new();
        assert!(g.client(Role::Extractor).is_none());
    }

    #[test]
    fn gateway_serves_registered_client() {
        let g = GatewayConfig::new()
            .with_client(Role::Extractor, Box::new(MockLlmClient::new()));
        assert!(g.client(Role::Extractor).is_some());
        assert!(g.client(Role::Adversary).is_none());
    }

    fn mock_with_identity(vendor: &str, family: &str) -> MockLlmClient {
        MockLlmClient::new().with_identity(ProviderIdentity {
            vendor: vendor.into(),
            model_family: family.into(),
            model_version: "v1".into(),
            endpoint: None,
        })
    }

    #[test]
    fn independence_passes_when_only_one_role_set() {
        let g = GatewayConfig::new()
            .with_client(Role::Extractor, Box::new(mock_with_identity("a", "x")));
        assert!(g.check_independence().is_ok());
    }

    #[test]
    fn independence_passes_when_families_differ() {
        let g = GatewayConfig::new()
            .with_client(Role::Extractor, Box::new(mock_with_identity("anthropic", "claude-opus")))
            .with_client(Role::Adversary, Box::new(mock_with_identity("openai", "gpt-5")));
        assert!(g.check_independence().is_ok());
    }

    #[test]
    fn independence_fails_when_same_vendor_and_family() {
        let g = GatewayConfig::new()
            .with_client(Role::Extractor, Box::new(mock_with_identity("anthropic", "claude-opus")))
            .with_client(Role::Adversary, Box::new(mock_with_identity("anthropic", "claude-opus")));
        let err = g.check_independence().unwrap_err();
        assert_eq!(err.extractor.vendor, "anthropic");
        assert_eq!(err.adversary.vendor, "anthropic");
    }

    #[test]
    fn independence_passes_when_vendor_same_but_family_differs() {
        // Anthropic Opus vs Anthropic Haiku — same vendor but the
        // family field captures the model lineage, so this is two
        // independent models from the ADJ05 perspective.
        let g = GatewayConfig::new()
            .with_client(Role::Extractor, Box::new(mock_with_identity("anthropic", "claude-opus-4-7")))
            .with_client(Role::Adversary, Box::new(mock_with_identity("anthropic", "claude-haiku-4-5")));
        assert!(g.check_independence().is_ok());
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let r = req_with_text("hello");
        assert_eq!(fingerprint_prompt(&r), fingerprint_prompt(&r));
    }

    #[test]
    fn fingerprint_changes_with_text() {
        let r1 = req_with_text("hello");
        let r2 = req_with_text("world");
        assert_ne!(fingerprint_prompt(&r1), fingerprint_prompt(&r2));
    }

    #[test]
    fn fingerprint_ignores_temperature_and_seed() {
        let mut r1 = req_with_text("hello");
        let mut r2 = req_with_text("hello");
        r1.temperature = 0.0;
        r1.seed = Some(42);
        r2.temperature = 1.0;
        r2.seed = Some(7);
        assert_eq!(fingerprint_prompt(&r1), fingerprint_prompt(&r2));
    }

    #[test]
    fn fingerprint_changes_with_model() {
        let mut r1 = req_with_text("hello");
        let mut r2 = req_with_text("hello");
        r1.model = "a".into();
        r2.model = "b".into();
        assert_ne!(fingerprint_prompt(&r1), fingerprint_prompt(&r2));
    }

    #[test]
    fn no_client_for_role_error_displays_role_name() {
        let err = PrimitiveError::NoClientForRole { role: Role::Adversary };
        assert!(format!("{err}").contains("adversary"));
    }

    #[test]
    fn validation_exhausted_displays_attempts() {
        let err = PrimitiveError::ValidationExhausted {
            last_response: "{}".into(),
            last_error: "missing field 'nodes'".into(),
            attempts: 3,
        };
        let s = format!("{err}");
        assert!(s.contains("3 attempts"));
        assert!(s.contains("missing field"));
    }

    #[test]
    fn primitive_error_implements_from_llm_error() {
        let provider = ProviderIdentity {
            vendor: "mock".into(),
            model_family: "test".into(),
            model_version: "1".into(),
            endpoint: None,
        };
        let llm_err = LlmError::Transport { provider, detail: "boom".into() };
        let prim_err: PrimitiveError = llm_err.into();
        assert!(matches!(prim_err, PrimitiveError::Gateway(_)));
    }

    // ----- truncation retry helper -----

    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionResponse, FinishReason, JsonSchema,
        LlmError, TokenUsage,
    };
    use std::sync::Mutex;

    /// A client that returns `OutputTruncated` N times and then a
    /// successful response. Lets us assert the retry loop's cap-doubling
    /// behavior without round-tripping a real model.
    struct FlakyJsonClient {
        identity: ProviderIdentity,
        truncate_attempts_remaining: Mutex<usize>,
        observed_caps: Mutex<Vec<Option<usize>>>,
        success_value: serde_json::Value,
    }

    impl FlakyJsonClient {
        fn new(truncate_attempts: usize, value: serde_json::Value) -> Self {
            Self {
                identity: ProviderIdentity {
                    vendor: "mock".into(),
                    model_family: "flaky".into(),
                    model_version: "1".into(),
                    endpoint: None,
                },
                truncate_attempts_remaining: Mutex::new(truncate_attempts),
                observed_caps: Mutex::new(Vec::new()),
                success_value: value,
            }
        }
    }

    impl LlmClient for FlakyJsonClient {
        fn identity(&self) -> ProviderIdentity {
            self.identity.clone()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            self.observed_caps.lock().unwrap().push(req.max_tokens);
            let mut left = self.truncate_attempts_remaining.lock().unwrap();
            if *left > 0 {
                *left -= 1;
                return Err(LlmError::OutputTruncated {
                    provider: self.identity.clone(),
                    output_tokens: req.max_tokens.unwrap_or(0),
                    max_tokens: req.max_tokens,
                });
            }
            Ok(CompletionResponse {
                text: self.success_value.to_string(),
                model: "flaky".into(),
                usage: TokenUsage::default(),
                finish_reason: FinishReason::Stop,
                provider_id: self.identity.clone(),
                latency_ms: 1,
            })
        }
        fn complete_json(
            &self,
            req: CompletionRequest,
            _s: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            self.observed_caps.lock().unwrap().push(req.max_tokens);
            let mut left = self.truncate_attempts_remaining.lock().unwrap();
            if *left > 0 {
                *left -= 1;
                return Err(LlmError::OutputTruncated {
                    provider: self.identity.clone(),
                    output_tokens: req.max_tokens.unwrap_or(0),
                    max_tokens: req.max_tokens,
                });
            }
            Ok(CompletionJsonResponse {
                raw_text: self.success_value.to_string(),
                parsed: self.success_value.clone(),
                schema_valid: true,
                model: "flaky".into(),
                usage: TokenUsage::default(),
                provider_id: self.identity.clone(),
                latency_ms: 1,
                polyfill_used: false,
            })
        }
    }

    #[test]
    fn complete_json_retry_doubles_max_tokens_until_success() {
        // Truncates twice, then succeeds. Initial cap is 1024 →
        // attempts use 1024, 2048, 4096.
        let client = FlakyJsonClient::new(2, serde_json::json!({"ok": true}));
        let mut req = req_with_text("x");
        req.max_tokens = Some(1024);
        let schema = JsonSchema {
            name: "x".into(),
            schema_json: "{}".into(),
        };
        let resp = complete_json_with_truncation_retry(&client, req, &schema).unwrap();
        assert_eq!(resp.parsed, serde_json::json!({"ok": true}));
        let caps = client.observed_caps.lock().unwrap().clone();
        assert_eq!(caps, vec![Some(1024), Some(2048), Some(4096)]);
    }

    #[test]
    fn complete_json_retry_caps_at_ceiling() {
        // Five truncations would walk 1024 → 2048 → 4096 → 8192 → 16384 → 32768,
        // which is exactly MAX_TOKENS_CEILING. Verify the ceiling is honoured.
        let client = FlakyJsonClient::new(5, serde_json::json!({"ok": true}));
        let mut req = req_with_text("x");
        req.max_tokens = Some(1024);
        let schema = JsonSchema {
            name: "x".into(),
            schema_json: "{}".into(),
        };
        let _ = complete_json_with_truncation_retry(&client, req, &schema).unwrap();
        let caps = client.observed_caps.lock().unwrap().clone();
        assert_eq!(
            caps,
            vec![
                Some(1024),
                Some(2048),
                Some(4096),
                Some(8192),
                Some(16384),
                Some(MAX_TOKENS_CEILING)
            ]
        );
    }

    #[test]
    fn complete_json_retry_gives_up_after_max_attempts() {
        // More truncations than the loop tolerates → propagate the last
        // OutputTruncated.
        let client = FlakyJsonClient::new(100, serde_json::json!({}));
        let mut req = req_with_text("x");
        req.max_tokens = Some(1024);
        let schema = JsonSchema {
            name: "x".into(),
            schema_json: "{}".into(),
        };
        let err = complete_json_with_truncation_retry(&client, req, &schema).unwrap_err();
        assert!(matches!(err, LlmError::OutputTruncated { .. }));
    }

    #[test]
    fn complete_json_retry_does_not_retry_on_non_truncation_errors() {
        // A schema-invalid (or any non-truncation) error must return
        // immediately — only the OutputTruncated arm retries.
        struct AlwaysSchemaInvalid;
        impl LlmClient for AlwaysSchemaInvalid {
            fn identity(&self) -> ProviderIdentity {
                ProviderIdentity {
                    vendor: "mock".into(),
                    model_family: "always-invalid".into(),
                    model_version: "1".into(),
                    endpoint: None,
                }
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
                Err(LlmError::SchemaInvalid {
                    provider: self.identity(),
                    schema_name: "x".into(),
                    raw_text: "garbage".into(),
                    validator_error: "not parseable".into(),
                })
            }
        }
        let req = req_with_text("x");
        let schema = JsonSchema {
            name: "x".into(),
            schema_json: "{}".into(),
        };
        let err =
            complete_json_with_truncation_retry(&AlwaysSchemaInvalid, req, &schema).unwrap_err();
        assert!(matches!(err, LlmError::SchemaInvalid { .. }));
    }

    #[test]
    fn complete_retry_doubles_until_success() {
        // Same behavior as complete_json_with_truncation_retry, but for
        // the text-emitting path.
        let client = FlakyJsonClient::new(1, serde_json::json!("hello"));
        let mut req = req_with_text("x");
        req.max_tokens = Some(1024);
        let resp = complete_with_truncation_retry(&client, req).unwrap();
        assert!(resp.text.contains("hello"));
        let caps = client.observed_caps.lock().unwrap().clone();
        assert_eq!(caps, vec![Some(1024), Some(2048)]);
    }

    #[test]
    fn prompt_version_constants_are_stable() {
        // Locking the constants down — bumping any of these is an
        // audit-trail-affecting change and should be a separate PR.
        assert_eq!(DECOMPOSE_TEXT_PROMPT_VERSION, "decompose-text-v2");
        assert_eq!(RENDER_NODE_PROMPT_VERSION, "render-node-v1");
        assert_eq!(ENTAIL_PROMPT_VERSION, "entail-v1");
        assert_eq!(ADVERSARY_PROMPT_VERSION, "adversary-v1");
        assert_eq!(PLAUSIBILITY_PROMPT_VERSION, "plausibility-v1");
        assert_eq!(EXTRACT_RULES_PROMPT_VERSION, "extract-rules-v1");
    }
}
