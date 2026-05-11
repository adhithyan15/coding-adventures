# llm-provider-ollama (Rust)

Local-only `LlmClient` implementation for [Ollama](https://ollama.com).
Talks to a running `ollama serve` over plain HTTP/1.1, no TLS, no
auth, no third-party HTTP crate.

## What This Is

Reference implementation of the Ollama half of [LM00a — LLM Provider
Implementations](../../../specs/LM00a-llm-provider-implementations.md).
Wraps `POST /api/chat` to give the framework access to any model the
operator has pulled locally (`ollama pull llama3.1:8b-instruct-q4_K_M`,
etc.).

## Where It Fits

```text
   llm-primitives / framework consumers
        │
        ▼
   llm-gateway (LlmClient trait + types)
        │
        ▼
   llm-provider-ollama (this crate)
        │     POST /api/chat over plain HTTP
        ▼
   ollama serve (local)
```

## Why no third-party HTTP crate

Ollama is local-by-design — `http://localhost:11434`, no TLS, no
auth. The whole request/response cycle is ~50 lines of
[`std::net::TcpStream`] + [`serde_json`]. Pulling in `reqwest` or
`ureq` just to talk to localhost adds a transitive dependency tree
that doesn't earn its keep. The cloud providers (Anthropic, OpenAI)
will need TLS and will bring in an HTTP crate; Ollama does not.

## Usage

```rust
use llm_provider_ollama::OllamaClient;
use llm_gateway::{LlmClient, CompletionRequest, Message};

let client = OllamaClient::new("llama3.1:8b-instruct-q4_K_M");

let resp = client.complete(CompletionRequest {
    model: String::new(),                  // empty → use client default
    system: Some("be terse".into()),
    messages: vec![Message::user("what is 2+2?")],
    temperature: 0.0,
    max_tokens: Some(64),
    stop_sequences: vec![],
    seed: Some(42),
    metadata: Default::default(),
}).expect("ollama running");

println!("{}", resp.text);
```

Override the endpoint for a non-default Ollama install:

```rust
let client = OllamaClient::new("mistral:7b")
    .with_endpoint("http://my-ollama-host:11434");
```

## Capability Profile

| Capability | Native | Notes |
|---|---|---|
| `json_mode_native` | ✅ | via `format: "json"` on `/api/chat` |
| `tool_use_native` | ❌ | polyfilled at primitive layer |
| `streaming_native` | ✅ (impl deferred) | non-streaming used at v0.1.0 |
| `prompt_caching_native` | ❌ | Ollama has no cross-request cache |
| `multimodal_image_input` | ❌ (conservative) | model-dependent; flip in your deployment if your model supports it |
| `max_context_window` | 8 192 (conservative default) | model-dependent |

## Limitations (v0.1.0)

- **Streaming not yet exposed.** The `stream: false` path is the
  whole story for now. A v0.2 will pair with an async surface.
- **No pre-flight reachability check.** The spec calls for
  `GET /api/tags` at construction; we defer to the first
  `complete()` so `new()` is infallible. A `ping()` helper is
  available for callers that want to check manually.
- **No chunked-encoding support.** Ollama returns Content-Length on
  non-streaming calls, which is all we use today.
- **Multimodal images dropped.** `MessageContent::Multimodal` with
  `ImageBase64` blocks falls back to text-only; vision support is a
  follow-up that maps to Ollama's `images` field.
