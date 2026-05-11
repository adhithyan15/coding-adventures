# LM00 — LLM Gateway Architecture

## Overview

The framework's checker passes, extractor, clarification dialogue,
and adversarial verifier all talk to large language models. They do
so through **one abstract interface** — the LLM gateway — that
admits cloud providers (Anthropic, OpenAI, Google, Mistral) and
local backends (Ollama, llama.cpp, vLLM) as equally first-class
implementations.

This spec defines the gateway's contract: request and response
shapes, the `LlmClient` trait, polyfills for capabilities not all
providers support natively, audit-trail integration, and the cost
and rate-limiting hooks every deployment will need. Provider-specific
behaviour lives in [`LM00a`](LM00a-llm-provider-implementations.md);
the IR-generation primitives that consume the gateway live in
[`LM00b`](LM00b-llm-primitives.md).

The central design constraint:

> Every checker, extractor, or framework component that calls an
> LLM does so against an interface that hides which LLM is on the
> other end. The audit trail records the specific model, version,
> prompt, parameters, tokens, and cost — so any past adjudication
> is replayable under the same configuration — but the calling code
> is provider-agnostic.

## Layer Position

```
   framework consumers
     │
     ▼
   LM00b primitives (extract_ir, render_node, entail, ...)
     │
     ▼
   LM00  LlmClient trait              ← this document
     │
     ▼
   LM00a providers (Anthropic / OpenAI / Ollama / mock / ...)
     │
     ▼
   provider HTTP APIs / local processes
```

## Why a Gateway

Three concerns drive the gateway abstraction:

1. **Substitutability**. The framework should run unchanged when a
   deployment switches from cloud Claude to a local Llama, or
   between OpenAI and Anthropic for the adversary role. Calling
   code does not know or care which.
2. **Polyfills**. Some providers offer native JSON-mode structured
   output, tool / function calling, prompt caching, and streaming.
   Others do not. The gateway provides uniform polyfills (prompt-
   wrap + post-hoc parse + retry-on-failure) so callers see the
   same interface regardless.
3. **Auditability**. Every framework decision recorded in `ADJ07`
   carries the model identity, version, prompt hash, parameters,
   token counts, latency, and cost. The gateway is the only point
   where the LLM is invoked, so it is the only place this metadata
   needs to be collected.

## The Core Trait

```rust
pub trait LlmClient: Send + Sync {
    /// Provider identity for audit-trail recording.
    fn identity(&self) -> ProviderIdentity;

    /// Issue a chat-completion request and return the response.
    async fn complete(&self, req: CompletionRequest)
        -> Result<CompletionResponse, LlmError>;

    /// Issue a chat-completion request constrained to produce JSON
    /// matching the given schema. Polyfilled for providers that
    /// don't support JSON mode natively.
    async fn complete_json(
        &self,
        req: CompletionRequest,
        schema: &JsonSchema,
    ) -> Result<CompletionJsonResponse, LlmError>;

    /// Optional streaming completion. Default implementation falls
    /// back to non-streaming.
    async fn stream_complete(&self, req: CompletionRequest)
        -> Result<CompletionStream, LlmError> {
        // default: complete + wrap as single-element stream
    }

    /// Capability self-report — what does this provider support
    /// natively? Polyfills use this to decide whether to invoke
    /// directly or wrap.
    fn capabilities(&self) -> Capabilities;
}
```

The trait deliberately exposes a small surface. Providers report
their capabilities (`json_mode_native`, `tool_use_native`,
`streaming_native`, `prompt_caching_native`); the gateway's polyfills
sit between caller and provider when the provider's capability is
absent.

## Request and Response Shapes

```rust
pub struct CompletionRequest {
    pub model:           ModelId,
    pub messages:        Vec<Message>,
    pub temperature:     f32,
    pub max_tokens:      Option<usize>,
    pub stop_sequences:  Vec<String>,
    pub seed:            Option<u64>,        // reproducibility
    pub system:          Option<String>,
    pub metadata:        HashMap<String, String>,  // free-form for audit
}

pub struct Message {
    pub role:    Role,         // System | User | Assistant
    pub content: MessageContent,
}

pub enum MessageContent {
    Text(String),
    Multimodal(Vec<ContentBlock>),  // for image / audio / file inputs
}

pub struct CompletionResponse {
    pub text:           String,
    pub model:          ModelId,
    pub usage:          TokenUsage,
    pub finish_reason:  FinishReason,
    pub provider_id:    ProviderIdentity,
    pub latency_ms:     u64,
}

pub struct CompletionJsonResponse {
    pub raw_text:       String,         // what the LLM emitted
    pub parsed:         serde_json::Value,
    pub schema_valid:   bool,
    pub model:          ModelId,
    pub usage:          TokenUsage,
    pub provider_id:    ProviderIdentity,
    pub latency_ms:     u64,
    pub polyfill_used:  bool,           // true iff non-native JSON mode
}

pub struct TokenUsage {
    pub input_tokens:   usize,
    pub output_tokens:  usize,
    pub cached_tokens:  usize,          // 0 if no prompt cache hit
}
```

The shape is informed by Anthropic's and OpenAI's modern chat APIs.
Local providers (Ollama, llama.cpp) are routed through the same
shape via an adapter layer.

## Provider Identity

```rust
pub struct ProviderIdentity {
    pub vendor:        String,    // "anthropic", "openai", "ollama", ...
    pub model_family:  String,    // "claude-opus", "gpt-4", "llama-3.1", ...
    pub model_version: String,    // "2026-05-10", "8b-instruct-q4", ...
    pub endpoint:      Option<String>,  // base URL for cloud / self-hosted
}
```

Every audit-trail entry that records an LLM call records this
identity verbatim. Replay (`ADJ08`) matches on `(vendor,
model_family, model_version)` exactly; any mismatch is logged as
configuration drift.

## Capabilities

```rust
pub struct Capabilities {
    pub json_mode_native:        bool,
    pub tool_use_native:         bool,
    pub streaming_native:        bool,
    pub prompt_caching_native:   bool,
    pub multimodal_image_input:  bool,
    pub max_context_window:      usize,
}
```

The capability self-report is **trust-but-verify**: a polyfill
fallback kicks in if a "native" capability fails (e.g., the
provider returns a non-JSON response despite advertising JSON mode).
Failures of advertised capabilities are logged for the deployment's
attention.

## Errors

```rust
pub enum LlmError {
    /// HTTP / transport failure. May be transient; retry policy applies.
    Transport { provider: ProviderIdentity, source: ErrorSource },

    /// Rate limit exceeded. Retry-after duration when known.
    RateLimit { provider: ProviderIdentity, retry_after: Option<Duration> },

    /// Context window exceeded by the request.
    ContextTooLarge {
        provider: ProviderIdentity,
        requested_tokens: usize,
        max_tokens: usize,
    },

    /// Provider returned a malformed response.
    ProtocolError { provider: ProviderIdentity, detail: String },

    /// Authentication failure (bad key, expired token).
    Auth { provider: ProviderIdentity, detail: String },

    /// Schema validation failure on complete_json (after retries).
    SchemaInvalid {
        provider: ProviderIdentity,
        schema: String,
        raw_text: String,
        validator_error: String,
    },

    /// The provider returned a refusal or safety stop.
    Refused { provider: ProviderIdentity, reason: Option<String> },

    /// Wrapped underlying error.
    Other { provider: ProviderIdentity, message: String },
}
```

Each variant carries the provider identity so logs and audit trails
record which backend failed.

## Polyfills

### JSON-mode polyfill

For providers without native JSON mode (`json_mode_native: false`):

1. Append a system suffix instructing the model to emit only JSON
   matching the given schema. Include the schema in the prompt.
2. Issue the completion.
3. Strip optional Markdown code fences (```...```).
4. Parse as JSON; validate against the schema.
5. On failure, retry up to `max_retries` times (default 2) with a
   correction message appended (the parsed-error text).
6. After exhaustion, return `LlmError::SchemaInvalid`.

Polyfill behaviour and retry count are configurable per request.
The audit trail records whether the polyfill was used and how many
retries occurred.

### Tool-use polyfill

For providers without native tool calling: emit a JSON object whose
schema is the union of tool signatures, parse on receipt, dispatch
to local tool implementations. The polyfill is documented in
[`LM00a`](LM00a-llm-provider-implementations.md).

### Streaming polyfill

For providers that only support unary completion: the streaming
trait method returns a single-element stream containing the full
response. Callers that genuinely need streaming check
`capabilities().streaming_native` to gate their UI.

## Retry and Rate-Limit Policy

The gateway exposes a `RetryConfig`:

```rust
pub struct RetryConfig {
    pub max_attempts:        usize,
    pub initial_backoff:     Duration,
    pub backoff_multiplier:  f64,
    pub jitter:              JitterStrategy,
    pub retryable_errors:    HashSet<LlmErrorKind>,
}
```

Defaults:
- 3 attempts on transport errors and rate-limit errors
- 500ms initial backoff, 2× multiplier
- Full jitter (uniform 0..backoff)
- Retryable: `Transport`, `RateLimit`

Per-request override is permitted (e.g., a clarification turn that
must not wait long can set `max_attempts: 1`).

## Cost and Telemetry

The gateway tracks **cumulative usage** per `LlmClient` instance and
**per-request usage** in each `CompletionResponse`. A small price
table maps `(vendor, model_family, model_version)` to `(price_in,
price_out)` per million tokens; deployments can override.

```rust
pub struct CumulativeUsage {
    pub total_calls:        usize,
    pub total_input_tokens: usize,
    pub total_output_tokens: usize,
    pub cached_tokens:      usize,
    pub estimated_cost_usd: f64,
    pub by_model:           HashMap<ModelId, ModelUsage>,
}
```

`CumulativeUsage` is reported in the audit trail (`ADJ07`) for every
adjudication; the deployment is responsible for aggregating across
adjudications.

## Mock Provider

A `MockLlmClient` is part of the standard set of providers. It does
not call any external service; instead it returns scripted
responses keyed by request fingerprint. Used by every test in the
framework's checker-pass and extractor crates so test runs are
deterministic and free of network dependencies.

```rust
pub struct MockLlmClient {
    pub script: HashMap<RequestFingerprint, MockResponse>,
    pub default: MockResponse,  // for unscripted requests
}
```

Mock responses can return success, errors of any LlmError variant,
delayed responses, or chained responses (for streaming tests). The
test discipline is **mock by default**: every test imports
`MockLlmClient`; production wiring chooses a real client.

## Audit Trail Integration

Every gateway invocation emits a record that the framework's audit
trail (`ADJ07`) consumes:

```text
LlmCallRecord := {
    request_id:        uuid,
    timestamp:         ISO-8601,
    provider:          ProviderIdentity,
    request_summary:   {
        message_count:    usize,
        total_prompt_len: usize,
        temperature:      f32,
        seed:             Option<u64>,
        max_tokens:       Option<usize>,
    },
    response_summary:  {
        finish_reason:    FinishReason,
        latency_ms:       u64,
        polyfill_used:    bool,
        usage:            TokenUsage,
    },
    estimated_cost_usd: f64,
    prompt_hash:        Sha256Hex,    -- for reproducibility / cache
    response_hash:      Sha256Hex,
}
```

`prompt_hash` is content-addressed against the full prompt; replay
matches on this hash to detect prompt drift between adjudication
and replay time. `response_hash` lets the audit trail prove which
specific response shaped the adjudication.

Full prompt text and full response text are stored only if the
deployment's privacy policy permits; otherwise the hashes serve as
opaque references. The framework does not impose a privacy policy;
the audit trail accommodates either choice.

## Concurrency

`LlmClient` implementations are `Send + Sync`. The framework may
call `complete` and `complete_json` concurrently across multiple
adjudication contexts. Provider implementations are responsible for
their own connection pooling and rate-limit coordination.

The default `RetryConfig`'s jitter prevents thundering-herd retries
when multiple concurrent calls hit a rate limit simultaneously.

## Configuration

A `GatewayConfig` collects everything a deployment specifies:

```rust
pub struct GatewayConfig {
    pub default_client:     ClientName,
    pub clients:            HashMap<ClientName, Box<dyn LlmClient>>,
    pub retry:              RetryConfig,
    pub privacy:            PrivacyMode,        // log_prompts | hash_only
    pub price_table:        PriceTable,
    pub telemetry_sink:     Option<Box<dyn TelemetrySink>>,
}
```

The framework's checker passes / extractor receive a
`&dyn LlmClient` reference and do not see the broader
configuration; the audit trail does.

## Routing by Role

Different roles within the framework prefer different models:

```text
Extractor       : best structured-output model available
Renderer (ADJ04): smallest competent model (cheap)
NLI judge       : purpose-trained NLI model or small LLM
Adversary       : different model family from extractor (for ADJ05's
                   independence requirement)
Plausibility    : small competent model
```

A deployment registers roles → client names in `GatewayConfig`:

```rust
pub struct RoleAssignments {
    pub extractor:        ClientName,
    pub renderer:         ClientName,
    pub nli_judge:        ClientName,
    pub adversary:        ClientName,
    pub plausibility:     ClientName,
}
```

The framework retrieves the right client by role; the deployment
controls the mapping.

## Open Questions

1. **Function-calling vs. JSON-mode for structured output.** Some
   providers express structured output as tool calls; others as JSON
   responses. The trait currently exposes both surfaces (`complete`,
   `complete_json`, plus tool-use polyfill). Whether to prefer one
   over the other in primitives (`LM00b`) is an open choice.
2. **Streaming for clarification dialogue.** A live clarification UI
   benefits from streaming responses. The streaming surface exists
   but isn't required. Whether to make it the default for user-facing
   prompts (vs. background extraction) is a deployment choice.
3. **Multimodal extraction.** Clinical notes embedded in scanned PDFs
   or images require multimodal input. The trait accommodates it via
   `MessageContent::Multimodal`; primitives in `LM00b` will exercise
   this once multi-modal becomes a concrete deployment requirement.
4. **Local-LLM tuning differences.** A Llama-3 model at `temperature
   0` may still produce non-deterministic outputs due to batch effects.
   Reproducibility for local models is best-effort; the audit trail
   records the seed even when the provider cannot honour it.

## Limitations

1. **Provider divergence is real.** Even with polyfills, the same
   prompt at the "same temperature" produces different responses
   across providers. Deployments need to test their prompts against
   the specific provider they'll run against.
2. **The trait is not a least-common-denominator.** It exposes
   capabilities every modern provider supports natively (chat,
   structured output via JSON, streaming) and polyfills the rest;
   exotic features (provider-specific safety controls, custom
   sampling) are not in the contract and require provider-specific
   wiring.
3. **No built-in caching beyond what providers offer.** Anthropic's
   prompt caching is exposed via `cached_tokens`; OpenAI's is similar.
   Persistent response caching (cache the *answer*, not the prompt)
   is a deployment concern, not gateway responsibility.

## Status

Draft. Sufficient for the Rust `llm-gateway` crate to begin. Provider
implementations and the IR-generation primitives that consume this
gateway are specified separately in `LM00a` and `LM00b`.
