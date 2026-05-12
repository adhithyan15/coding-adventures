# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-05-11

### Added

`LlmError::OutputTruncated { provider, output_tokens, max_tokens }` —
a new error variant for the case where the provider stopped at the
`max_tokens` budget *before* emitting usable content. Thinking-mode
models (Gemma, DeepSeek-R1, Claude with extended thinking) routinely
burn the entire budget on chain-of-thought and emit nothing visible
afterwards; previously this surfaced as a confusing `SchemaInvalid`
or `ProtocolError`. The new variant lets caller-level retry loops
key on truncation specifically and try again with a larger budget.

- `LlmError::provider()` updated to include the new variant.
- `Display` impl renders a helpful hint about raising the cap.

### Compatibility

This is a non-breaking addition to the public enum from a consumer
standpoint — no existing `match` was exhaustive over `LlmError` in
the workspace, only `matches!` patterns. Downstream crates that DO
exhaustively match `LlmError` will need an extra arm.

## [0.1.0] - 2026-05-11

### Added

- `LlmClient` trait — the provider-agnostic contract every framework
  component uses to talk to an LLM. Methods: `identity`, `complete`,
  `complete_json`, `capabilities`.
- Neutral request / response types:
  - `CompletionRequest { model, messages, temperature, max_tokens,
    stop_sequences, seed, system, metadata }`
  - `Message { role, content }` with `Role :: System | User | Assistant`
  - `CompletionResponse` with text, model, usage, finish_reason,
    provider identity, latency
  - `CompletionJsonResponse` with raw_text, parsed (JSON), schema_valid
    flag, and a `polyfill_used` indicator
- `ProviderIdentity { vendor, model_family, model_version, endpoint }`
  recorded in every response for audit-trail replay matching.
- `Capabilities` self-report (json_mode_native, tool_use_native,
  streaming_native, prompt_caching_native, multimodal_image_input,
  max_context_window). Polyfills consult this.
- `TokenUsage { input_tokens, output_tokens, cached_tokens }` per
  response.
- `LlmError` enum with 8 specific variants: Transport, RateLimit,
  ContextTooLarge, ProtocolError, Auth, SchemaInvalid, Refused, Other.
  Each carries `ProviderIdentity`.
- `MockLlmClient` — scripted responses keyed by `RequestFingerprint`
  (a SHA-style hash of the request's content excluding temperature
  and seed so retries match). Strict-default-fail-on-unscripted
  encouraged.
- `RequestFingerprint::new(model, system, messages)` for test
  scripting.
- 14 tests covering: identity/capabilities round-trip, mock scripted
  response, mock default response, mock fail-on-unscripted, error
  variants carrying provider identity, ProviderIdentity equality,
  CompletionRequest construction.

### Scope

This crate is the trait + neutral types + mock provider. Real
provider implementations (Anthropic, OpenAI, Ollama) live in
separate crates (`llm-provider-anthropic`, etc.) that depend on this
crate. See [`LM00a`](../../../specs/LM00a-llm-provider-implementations.md)
for the provider catalog.

The `complete_json` polyfill (prompt-wrap with schema + parse +
retry-on-failure) and the `streaming_complete` default implementation
are scoped for v0.2.0 once the trait surface is exercised by a real
provider.

### Notes

Mirrors [LM00](../../../specs/LM00-llm-gateway-architecture.md). The
mock-by-default test discipline keeps the framework's checker /
extractor crates network-free in CI.
