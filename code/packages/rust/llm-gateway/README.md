# llm-gateway (Rust)

Provider-agnostic LLM gateway. The `LlmClient` trait every framework
component uses to talk to an LLM. Neutral request / response shapes.
A mock provider for deterministic tests.

Tool-aware turns use the same provider-neutral boundary. Callers supply a
bounded catalog of JSON-schema-shaped `ModelToolDefinition` records, an
automatic, required, or named selection policy, and any prior
`ModelToolResult` values. Each result retains its complete preceding call so a
native adapter can reconstruct the provider transcript without hidden session
state. The response is exactly one final-text value or one
`ModelToolCall`; authorization and execution remain the caller's responsibility.
Providers may implement this natively, while existing text-only providers use
the default deterministic JSON prompt polyfill.

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
    CompletionRequest, LlmClient, Message, MockLlmClient, MockResponse,
    RequestFingerprint,
};

// In tests:
let mock = MockLlmClient::new()
    .with_response(
        RequestFingerprint::new("test", None, &[Message::user("Hello")]),
        MockResponse::Text("Hi!".into()),
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

let resp = mock.complete(req).unwrap();
assert_eq!(resp.text, "Hi!");
```

Tool-aware callers wrap the unchanged text request and offer repository-owned
tool declarations:

```rust
use llm_gateway::{
    CompletionRequest, LlmClient, Message, ModelToolCall, ModelToolChoice,
    ModelToolDefinition, MockLlmClient, MockResponse, RequestFingerprint,
    ToolCompletionOutput, ToolCompletionRequest,
};

let tool_request = ToolCompletionRequest {
    completion: CompletionRequest {
        model: "test".into(),
        system: None,
        messages: vec![Message::user("Which devices are online?")],
        temperature: 0.0,
        max_tokens: Some(64),
        stop_sequences: vec![],
        seed: None,
        metadata: Default::default(),
    },
    tools: vec![ModelToolDefinition {
        name: "smart_home.list_devices".into(),
        description: "List authorized smart-home devices".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {}
        }),
    }],
    choice: ModelToolChoice::Auto,
    results: vec![],
};
let fingerprint = RequestFingerprint::for_tool_completion(&tool_request);
let mock = MockLlmClient::new().with_response(
    fingerprint,
    MockResponse::ToolCall(ModelToolCall {
        call_id: "call_1".into(),
        name: "smart_home.list_devices".into(),
        arguments: serde_json::json!({}),
    }),
);

match mock.complete_with_tools(tool_request).unwrap().output {
    ToolCompletionOutput::FinalText(text) => println!("{text}"),
    ToolCompletionOutput::ToolCall(call) => println!("call {}", call.name),
}
```

## Provider Implementations

Real providers (`llm-provider-anthropic`, `llm-provider-openai`,
`llm-provider-ollama`) live in their own crates so deployments
import only what they use. The framework's checker / extractor
crates depend on `llm-gateway` (the trait) and on
`llm-provider-mock` (for tests), never on a specific real provider.

## Status

The crate provides neutral text, structured-output, and tool-aware completion
contracts, deterministic test doubles, and a tool-use compatibility polyfill.
Provider-specific native tool encoders remain in their separate adapter crates.
