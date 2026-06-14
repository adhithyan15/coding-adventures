# llm-gateway (Rust)

Provider-agnostic LLM gateway. The `LlmClient` trait every framework
component uses to talk to an LLM. Neutral request / response shapes.
A mock provider for deterministic tests.

## What This Is

This crate is the Rust implementation of [LM00 — LLM Gateway Architecture](../../../specs/LM00-llm-gateway-architecture.md).
Cloud providers (Anthropic, OpenAI, Google, Mistral) and local
backends (Ollama, llama.cpp, vLLM) implement the same `LlmClient`
trait. Calling code is provider-agnostic; the audit trail records
full provider identity for replay.

## Where It Fits

```text
   framework consumers
        │
        ▼
   llm-primitives (extract_ir, render_node, entail, ...)
        │
        ▼
   llm-gateway (LlmClient trait + types)   ← this crate
        │
        ▼
   llm-provider-anthropic / -openai / -ollama / -mock
        │
        ▼
   provider HTTP / local processes
```

## API at a Glance

```rust
use llm_gateway::{
    LlmClient, CompletionRequest, Message, Role, MessageContent,
    MockLlmClient, MockResponse, RequestFingerprint,
};

// In tests:
let mock = MockLlmClient::new()
    .with_response(
        RequestFingerprint::new("test", None, &[Message::user("Hello")]),
        MockResponse::text("Hi!"),
    )
    .with_strict_default();

let req = CompletionRequest {
    model: "test".into(),
    system: None,
    messages: vec![Message::user("Hello")],
    temperature: 0.0,
    max_tokens: Some(64),
    stop_sequences: vec![],
    seed: None,
    metadata: Default::default(),
};

let resp = mock.complete(req).await.unwrap();
assert_eq!(resp.text, "Hi!");
```

## Provider Implementations

Real providers (`llm-provider-anthropic`, `llm-provider-openai`,
`llm-provider-ollama`) live in their own crates so deployments
import only what they use. The framework's checker / extractor
crates depend on `llm-gateway` (the trait) and on
`llm-provider-mock` (for tests), never on a specific real provider.

## Status

v0.1.0 — trait, neutral types, mock provider, error taxonomy. Real
providers, `complete_json` polyfill, and streaming come in v0.2.0
once the trait is exercised by a concrete provider.
