# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-11

### Added

- `OllamaClient` — `LlmClient` implementation for a local Ollama
  server. Builder methods: `new(model_name)` (defaults to
  `http://localhost:11434`), `with_endpoint`, `with_timeout`.
- Bespoke HTTP/1.1 client over `std::net::TcpStream` (no `reqwest`,
  no `ureq`). Plain HTTP only — Ollama is local-by-design and HTTPS
  is rejected at endpoint parse time.
- `LlmClient::complete` — POSTs to `/api/chat` with `stream: false`.
  Maps the neutral request to Ollama's body shape:
  - `temperature` → `options.temperature`
  - `max_tokens` → `options.num_predict`
  - `seed` → `options.seed`
  - `stop_sequences` → `options.stop[]`
- `LlmClient::complete_json` — uses Ollama's native `format: "json"`
  and prepends the schema (as text) to the system prompt so the
  model is told what to produce. Response is parsed via
  `serde_json`; non-JSON responses surface as
  `LlmError::SchemaInvalid`.
- `parse_endpoint`, `parse_http_response`, `parse_ollama_response`
  — split out as testable pure helpers.
- `flatten_content` — collapses multimodal blocks to text-only.
  Image blocks are silently dropped (vision support is a follow-up).
- `ping(endpoint, timeout)` — optional helper for callers that want
  a pre-flight reachability check against `/api/tags`.
- Capability profile per spec: `json_mode_native = true`,
  `tool_use_native = false`, `prompt_caching_native = false`.

### Tests (19 passing)

In-process `ScriptedServer` (zero-dep `TcpListener`) runs end-to-end
HTTP exchanges in tests, no Ollama installation required. Coverage:

- Endpoint parsing — accept `http://host:port`; reject https,
  missing port, empty host.
- Identity carries the endpoint, capability profile matches the
  spec.
- HTTP response parsing — body extraction on 2xx, error message
  surface on non-2xx.
- Ollama response parsing — text + token counts + finish reason
  (Stop, MaxTokens via `done_reason: "length"`).
- Round-trip: `complete` posts to `/api/chat`, captures the
  serialized body, asserts model / messages / `stream: false`.
- Per-call model override (`req.model` non-empty replaces client
  default).
- System message inlined as first message with `role: "system"`.
- Options serialization for temperature, num_predict, seed, stop.
- HTTP 500 surfaces as `LlmError::Transport`.
- `complete_json` flips `format: "json"` and appends schema name +
  body to the system prompt.
- `complete_json` returns `LlmError::SchemaInvalid` when the model
  outputs non-JSON.

### Notes

This is the local provider. Anthropic and OpenAI providers (which
need TLS) live in sibling crates with their own dependencies.

Reference: [`LM00a`](../../../specs/LM00a-llm-provider-implementations.md).
