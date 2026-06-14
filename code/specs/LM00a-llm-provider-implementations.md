# LM00a — LLM Provider Implementations

## Overview

This spec defines concrete `LlmClient` implementations for the
provider set the framework's reference runtime ships with. Each
provider implementation is a thin adapter: it maps the framework's
neutral `CompletionRequest` to the provider's API shape, executes
the call, maps the response back, and reports capabilities so
polyfills know what to wrap.

The provider set is intentionally minimal at v1:

- **Anthropic** (`AnthropicClient`) — cloud, Claude family
- **OpenAI** (`OpenAiClient`) — cloud, GPT family
- **Ollama** (`OllamaClient`) — local, any model served by Ollama
- **Mock** (`MockLlmClient`) — for tests, never makes a network call

Future additions (Google Gemini, Mistral, Cohere, llama.cpp direct,
vLLM, Replicate, Anyscale, etc.) follow the same pattern. Each new
provider lives in its own crate or module; nothing in this list is
load-bearing for the framework.

The four providers listed above cover the deployment patterns the
framework needs in 2026:

- **Frontier cloud quality** for the extractor and adversary roles
  (Anthropic, OpenAI).
- **Local execution** for low-latency interactive use, private data,
  and air-gapped deployments (Ollama).
- **Deterministic testing** that doesn't depend on the network or
  a credit card (Mock).

## Layer Position

```
   LM00b primitives (extract_ir, render_node, entail, ...)
        │
        ▼
   LM00  LlmClient trait
        │
        ▼
   LM00a provider implementations         ← this document
        │
        ├── AnthropicClient
        ├── OpenAiClient
        ├── OllamaClient
        └── MockLlmClient
```

## Common Adapter Pattern

Every provider follows the same internal shape:

1. **Authenticate**. Read API key from environment (cloud) or
   confirm local endpoint reachable (Ollama).
2. **Map request**. Convert `CompletionRequest` to the provider's
   wire format (HTTP body JSON).
3. **POST**. Issue the HTTP call (or local socket call for Ollama),
   with per-provider auth headers.
4. **Handle errors**. Translate provider-specific error codes into
   `LlmError` variants.
5. **Map response**. Convert provider response to
   `CompletionResponse` or `CompletionJsonResponse`, including
   token usage and finish reason.
6. **Record telemetry**. Latency, tokens, cost. Emit
   `LlmCallRecord` for the audit trail.

Common cross-cutting concerns (retry, jitter, cumulative-usage
tracking) live in a shared `provider-common` module so individual
providers don't reimplement them.

## Anthropic Provider

### Identity

```text
vendor: "anthropic"
model_family: e.g. "claude-opus-4-7", "claude-sonnet-4-6", "claude-haiku-4-5"
model_version: the date stamp returned by the API
endpoint: defaults to https://api.anthropic.com
```

### Capabilities

```text
json_mode_native:       true     (via response_format on supported models)
tool_use_native:        true     (Messages API tools)
streaming_native:       true     (server-sent events)
prompt_caching_native:  true     (cache_control on message blocks)
multimodal_image_input: true     (base64 in content blocks)
max_context_window:     200_000  (model-dependent)
```

### Request Mapping

| Neutral field | Anthropic field |
|---|---|
| `model` | top-level `model` |
| `system` | top-level `system` |
| `messages` | `messages[]` with `role` ∈ {user, assistant} |
| `temperature` | `temperature` |
| `max_tokens` | `max_tokens` (required by API) |
| `stop_sequences` | `stop_sequences[]` |
| `seed` | not natively supported; logged but unused |

When `max_tokens` is `None` in the neutral request, the adapter
sets a default (8192) since the API requires it. This default is
recorded in the audit trail.

### JSON Mode

Anthropic supports structured output via tool use (a single
"emit_result" tool whose schema matches the requested JSON Schema)
or via response_format on newer models. The adapter prefers
response_format when available and falls back to the tool-use path
on older snapshots.

### Errors

| Anthropic status | LlmError variant |
|---|---|
| 401 / 403 | `Auth` |
| 429 | `RateLimit { retry_after }` |
| 400 with "context_window_exceeded" | `ContextTooLarge` |
| 400 other | `ProtocolError` |
| 5xx | `Transport` |
| `stop_reason: "refusal"` | `Refused { reason }` |

### Prompt Caching

When the deployment opts in, the adapter sets `cache_control:
{type: "ephemeral"}` on the system message and on long-stable
prefixes of `messages`. `cached_tokens` in the response is reported
in the audit trail; cost calculation uses cached-token pricing for
those tokens.

## OpenAI Provider

### Identity

```text
vendor: "openai"
model_family: e.g. "gpt-5", "gpt-4.1", "gpt-4o-mini"
model_version: snapshot date or "latest"
endpoint: defaults to https://api.openai.com/v1
```

### Capabilities

```text
json_mode_native:       true     (response_format / structured outputs)
tool_use_native:        true     (Chat Completions tools)
streaming_native:       true     (server-sent events)
prompt_caching_native:  true     (server-side; transparent)
multimodal_image_input: true     (vision-capable models only)
max_context_window:     model-dependent (128_000 typical at this writing)
```

### Request Mapping

| Neutral field | OpenAI field |
|---|---|
| `model` | top-level `model` |
| `system` | first `messages[]` entry with role=system |
| `messages` | `messages[]` |
| `temperature` | `temperature` |
| `max_tokens` | `max_tokens` (optional) |
| `stop_sequences` | `stop[]` |
| `seed` | `seed` (best-effort reproducibility per OpenAI's docs) |

### JSON Mode

The adapter uses `response_format: { type: "json_schema", json_schema:
{ ... } }` with strict mode enabled. On models without strict
structured-output support, falls back to `response_format: { type:
"json_object" }` plus prompt instructions.

### Errors

| OpenAI status | LlmError variant |
|---|---|
| 401 | `Auth` |
| 429 | `RateLimit { retry_after }` |
| 400 with "context_length_exceeded" | `ContextTooLarge` |
| 400 with content filter | `Refused` |
| 5xx | `Transport` |

## Ollama Provider (Local)

### Identity

```text
vendor: "ollama"
model_family: model name (e.g. "llama3.1:8b-instruct-q4_K_M")
model_version: model tag (e.g. ":8b-instruct-q4_K_M")
endpoint: defaults to http://localhost:11434
```

### Capabilities

```text
json_mode_native:       true     (via format=json on /api/chat)
tool_use_native:        false    (polyfilled)
streaming_native:       true     (NDJSON)
prompt_caching_native:  false    (Ollama has no cross-request cache)
multimodal_image_input: depends on model (llava-family yes)
max_context_window:     model-dependent
```

### Request Mapping

| Neutral field | Ollama field |
|---|---|
| `model` | top-level `model` (must match installed local tag) |
| `system` | embedded as first message with role=system |
| `messages` | `messages[]` |
| `temperature` | `options.temperature` |
| `max_tokens` | `options.num_predict` |
| `stop_sequences` | `options.stop[]` |
| `seed` | `options.seed` (Ollama honours this) |

### Notes

- Ollama is a local server (typically on port 11434). The adapter
  performs a `GET /api/tags` on construction to verify the endpoint
  responds and the model is installed.
- `format=json` is the JSON-mode path; the adapter sends it
  alongside a system instruction containing the schema.
- Tool use is polyfilled via the `complete_json` path: emit a JSON
  object whose schema is the union of tool signatures.
- Token usage is approximate (Ollama reports `eval_count` /
  `prompt_eval_count`). Cost is **zero** by default; deployments
  can override the price table if they want to attribute internal
  compute cost.

## Mock Provider (Tests)

### Identity

```text
vendor: "mock"
model_family: configurable (defaults to "mock-test")
model_version: configurable
endpoint: None
```

### Capabilities

```text
all native flags: true     (the mock can be told to behave any way)
max_context_window: configurable (default very large)
```

### Behaviour

The mock takes a script keyed by `RequestFingerprint` (a hash of
`(model, system, messages)` ignoring `temperature` and `seed` so
that retries match). When a request arrives:

1. Hash the request.
2. Look up the scripted response.
3. If present, return it (with configurable delay).
4. If absent, return the `default` response or `LlmError::Other` if
   none is set.

### Scripting Patterns

```rust
let mock = MockLlmClient::new()
    .with_response(req_fingerprint("extract_ir", "patient denies chest pain"),
                   MockResponse::json(ir_document_json))
    .with_response(req_fingerprint("entail", "premise: ...", "hypothesis: ..."),
                   MockResponse::text("ENTAILS"))
    .with_default(MockResponse::error(LlmError::Refused { ... }));
```

Tests are encouraged to **fail-on-unscripted** (use a strict
default) so missing mock entries surface as test failures rather
than silent passes.

The mock also supports:

- **Counted responses**: return a different response on the second
  call to the same fingerprint (useful for testing retry).
- **Latency injection**: simulate slow networks.
- **Streaming**: emit a sequence of partial responses.

## Configuration

Each provider has a `new` constructor that takes its specific
configuration:

```rust
impl AnthropicClient {
    pub fn new(api_key: String, model: ModelId) -> Self;
    pub fn with_endpoint(self, endpoint: String) -> Self;
    pub fn with_cache_policy(self, policy: CachePolicy) -> Self;
}

impl OpenAiClient {
    pub fn new(api_key: String, model: ModelId) -> Self;
    pub fn with_endpoint(self, endpoint: String) -> Self;
    pub fn with_organization(self, org: String) -> Self;
}

impl OllamaClient {
    pub fn new(model_name: String) -> Self;          // default localhost:11434
    pub fn with_endpoint(self, endpoint: String) -> Self;
}

impl MockLlmClient {
    pub fn new() -> Self;
    pub fn with_response(self, fp: RequestFingerprint, resp: MockResponse) -> Self;
    pub fn with_default(self, resp: MockResponse) -> Self;
}
```

Environment-based defaults (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`)
are honoured by builder helpers that read the environment if no
explicit key is passed.

## Crate Layout

```
code/packages/rust/llm-gateway/             -- trait + common pieces
code/packages/rust/llm-provider-anthropic/  -- AnthropicClient
code/packages/rust/llm-provider-openai/     -- OpenAiClient
code/packages/rust/llm-provider-ollama/     -- OllamaClient
code/packages/rust/llm-provider-mock/       -- MockLlmClient
```

Each provider is its own crate so that a deployment can pull in only
the providers it actually uses (network deps for cloud, local HTTP
for Ollama, none for mock). The framework's checker / extractor
crates depend only on `llm-gateway` (the trait) and `llm-provider-mock`
(for tests).

## Open Questions

1. **Multi-tenant / per-request keys.** A deployment serving multiple
   tenants may need to attribute LLM calls to tenant accounts. The
   trait currently does not carry tenant identity; the
   `metadata` map on `CompletionRequest` could carry it. Whether
   that's sufficient or whether a richer multi-tenant API is needed
   is a deployment-experience question.
2. **Pricing for self-hosted models.** Local execution has real cost
   (compute, electricity, GPU amortisation) that doesn't appear in
   API bills. The price table accommodates per-model overrides; the
   framework does not opine on what those should be.
3. **OpenAI's structured-output strictness levels.** Modern OpenAI
   APIs distinguish `strict: true` (schema is enforced) from looser
   modes. The adapter defaults to strict; deployments may relax for
   models that struggle with it.
4. **Function-calling parity.** Tool-use APIs differ in subtle ways
   across providers (parallel tool calls, multi-turn dispatch). The
   first version of the adapter exposes single-tool-emit only; richer
   tool workflows are a follow-up.

## Limitations

1. **API drift.** Provider APIs evolve. Each adapter targets a
   specific API version, documented in the crate's CHANGELOG. When
   providers change, adapters need updates.
2. **Capability self-reporting can be wrong.** The mock can be told
   to lie about capabilities for test purposes; real providers may
   advertise features that fail in edge cases. Polyfills always
   stand by as a safety net.
3. **Rate-limit handling is provider-specific in detail.** OpenAI and
   Anthropic both return retry-after headers but use slightly
   different shapes; Ollama's local nature usually doesn't rate-limit
   at all but can saturate compute.

## Status

Draft. The four provider crates can be implemented independently
once `llm-gateway` (the trait + common module) is in place.
