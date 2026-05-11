// LlmError variants carry provider identity + detail strings on every
// arm — the audit-trail discipline of the framework. Larger error
// type, no behaviour difference.
#![allow(clippy::result_large_err)]

//! # llm-provider-ollama — local Ollama provider for the LLM gateway
//!
//! Reference implementation of the Ollama half of
//! [LM00a](../../../specs/LM00a-llm-provider-implementations.md).
//! Ollama runs as a local HTTP server (default `http://localhost:11434`)
//! and serves any model the operator has pulled with `ollama pull`.
//!
//! ## Why a bespoke HTTP client (no `reqwest` / `ureq`)
//!
//! Ollama is **local-only** — plain HTTP/1.1 over a TCP socket, no
//! TLS, no auth, no retries beyond a single attempt. The whole
//! request/response cycle is ~50 lines of [`std::net`] + [`serde_json`].
//! Pulling in a third-party HTTP client just to talk to localhost
//! buys nothing and adds a transitive dependency tree. The cloud
//! providers (Anthropic, OpenAI) need TLS and will pull in an HTTP
//! crate; Ollama does not.
//!
//! ## Wire-format mapping (per LM00a §"Ollama Provider")
//!
//! | Neutral field          | Ollama field                    |
//! |------------------------|---------------------------------|
//! | `model`                | `model`                         |
//! | `system`               | first message with `role:"system"` |
//! | `messages`             | `messages[]`                    |
//! | `temperature`          | `options.temperature`           |
//! | `max_tokens`           | `options.num_predict`           |
//! | `stop_sequences`       | `options.stop[]`                |
//! | `seed`                 | `options.seed`                  |
//!
//! ## What this crate does not do (yet)
//!
//! - **Streaming.** Always sends `stream: false`. Streaming
//!   support belongs in a v0.2 alongside an async surface.
//! - **`GET /api/tags` reachability probe.** The spec calls for it
//!   on construction; we defer to the first `complete()` call so
//!   `OllamaClient::new` stays infallible and synchronous.
//! - **Token-by-token usage attribution.** Ollama's `eval_count` and
//!   `prompt_eval_count` are reported as `output_tokens` /
//!   `input_tokens`. Cached-token reporting is zero (Ollama has no
//!   cross-request cache).

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use llm_gateway::{
    Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse, ContentBlock,
    FinishReason, JsonSchema, LlmClient, LlmError, MessageContent, ProviderIdentity, Role,
    TokenUsage,
};

const DEFAULT_ENDPOINT: &str = "http://localhost:11434";
const CHAT_PATH: &str = "/api/chat";

// Cap on response body size. A misconfigured or malicious endpoint
// could otherwise stream gigabytes before the timeout elapsed and
// exhaust process memory. 64 MiB is generous for any reasonable
// chat completion (typical responses are <100 KB) while keeping the
// process safe.
const MAX_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;

/// Ollama HTTP client. Holds the endpoint and the model name; the
/// neutral `CompletionRequest::model` field can override the
/// constructor's `model_name` per-call.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    endpoint: String,
    model_name: String,
    timeout: Duration,
}

impl OllamaClient {
    /// Construct against the default `http://localhost:11434`.
    /// `model_name` should be the local model tag (e.g.,
    /// `"llama3.1:8b-instruct-q4_K_M"`).
    pub fn new(model_name: impl Into<String>) -> Self {
        Self {
            endpoint: DEFAULT_ENDPOINT.to_string(),
            model_name: model_name.into(),
            timeout: Duration::from_secs(120),
        }
    }

    /// Override the endpoint URL. Accepts forms like
    /// `http://localhost:11434` or `http://my-ollama:11434`. HTTPS
    /// is not supported — Ollama is local-by-design.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Override the per-request timeout (default 120s).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn provider_identity(&self, model_used: &str) -> ProviderIdentity {
        ProviderIdentity {
            vendor: "ollama".to_string(),
            model_family: model_used.to_string(),
            model_version: model_used.to_string(),
            endpoint: Some(self.endpoint.clone()),
        }
    }

    /// Translate any IO / parse error into an `LlmError::Transport`.
    fn transport_err(&self, model: &str, detail: impl Into<String>) -> LlmError {
        LlmError::Transport {
            provider: self.provider_identity(model),
            detail: detail.into(),
        }
    }

    /// Internal: serialize `CompletionRequest` to Ollama's chat-API
    /// JSON body. `format_json` flips on Ollama's native JSON mode.
    fn build_body(&self, req: &CompletionRequest, format_json: bool) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // Ollama wants `system` as the first message with role=system
        // rather than a top-level field. Inline if present.
        if let Some(sys) = &req.system {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys,
            }));
        }

        for m in &req.messages {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
            };
            let content = flatten_content(&m.content);
            messages.push(serde_json::json!({
                "role": role,
                "content": content,
            }));
        }

        let mut options = serde_json::Map::new();
        // `temperature` is f32 in the neutral request; Ollama
        // accepts any float >= 0.
        options.insert(
            "temperature".to_string(),
            serde_json::Value::from(req.temperature),
        );
        if let Some(max) = req.max_tokens {
            options.insert(
                "num_predict".to_string(),
                serde_json::Value::from(max as u64),
            );
        }
        if !req.stop_sequences.is_empty() {
            options.insert(
                "stop".to_string(),
                serde_json::Value::Array(
                    req.stop_sequences
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        if let Some(seed) = req.seed {
            options.insert("seed".to_string(), serde_json::Value::from(seed));
        }

        let model = if req.model.is_empty() {
            &self.model_name
        } else {
            &req.model
        };

        let mut body = serde_json::Map::new();
        body.insert(
            "model".to_string(),
            serde_json::Value::String(model.clone()),
        );
        body.insert("messages".to_string(), serde_json::Value::Array(messages));
        body.insert("stream".to_string(), serde_json::Value::Bool(false));
        if format_json {
            body.insert(
                "format".to_string(),
                serde_json::Value::String("json".to_string()),
            );
        }
        body.insert(
            "options".to_string(),
            serde_json::Value::Object(options),
        );

        serde_json::Value::Object(body)
    }

    /// Internal: POST the body to `<endpoint><path>`, return the raw
    /// response body. The function is shared between `complete` and
    /// `complete_json` so the wire-protocol logic lives in one place.
    fn post(&self, path: &str, body: &serde_json::Value, model: &str) -> Result<String, LlmError> {
        let (host, port) = parse_endpoint(&self.endpoint)
            .ok_or_else(|| self.transport_err(model, format!("invalid endpoint: {}", self.endpoint)))?;

        let addr = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(|e| self.transport_err(model, format!("resolve {host}:{port}: {e}")))?
            .next()
            .ok_or_else(|| self.transport_err(model, format!("no address for {host}:{port}")))?;

        let mut stream = TcpStream::connect_timeout(&addr, self.timeout)
            .map_err(|e| self.transport_err(model, format!("connect: {e}")))?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|e| self.transport_err(model, format!("set_read_timeout: {e}")))?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|e| self.transport_err(model, format!("set_write_timeout: {e}")))?;

        let body_bytes = serde_json::to_vec(body)
            .map_err(|e| self.transport_err(model, format!("serialize body: {e}")))?;

        let request = format!(
            "POST {path} HTTP/1.1\r\n\
             Host: {host}:{port}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {len}\r\n\
             Connection: close\r\n\
             \r\n",
            host = host,
            port = port,
            path = path,
            len = body_bytes.len(),
        );

        stream
            .write_all(request.as_bytes())
            .map_err(|e| self.transport_err(model, format!("write header: {e}")))?;
        stream
            .write_all(&body_bytes)
            .map_err(|e| self.transport_err(model, format!("write body: {e}")))?;

        let mut raw = Vec::new();
        (&mut stream)
            .take(MAX_RESPONSE_BYTES)
            .read_to_end(&mut raw)
            .map_err(|e| self.transport_err(model, format!("read response: {e}")))?;
        if raw.len() as u64 == MAX_RESPONSE_BYTES {
            return Err(self.transport_err(
                model,
                format!("response exceeded {MAX_RESPONSE_BYTES} byte cap"),
            ));
        }

        parse_http_response(&raw)
            .map_err(|detail| self.transport_err(model, detail))
    }
}

/// Flatten our neutral `MessageContent` into a plain string for
/// Ollama (which doesn't understand multimodal blocks in the same
/// shape we do). Multimodal images are silently dropped at v0.1.0 —
/// a follow-up can map them to Ollama's `images` field for vision
/// models, but most local Ollama deployments are text-only.
fn flatten_content(c: &MessageContent) -> String {
    match c {
        MessageContent::Text(t) => t.clone(),
        MessageContent::Multimodal(blocks) => blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text(t) => Some(t.clone()),
                ContentBlock::ImageBase64 { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Split an `http://host:port` endpoint into `(host, port)`. Returns
/// `None` for any other shape (https, missing port). Local-by-design.
///
/// Defense-in-depth: rejects hosts containing characters outside the
/// hostname character set so a misconfigured endpoint cannot smuggle
/// CRLF (or other control characters) into the outgoing `Host:`
/// header line. Allowed: ASCII alphanumerics, `.`, `-`, `_`.
fn parse_endpoint(endpoint: &str) -> Option<(String, u16)> {
    let rest = endpoint.strip_prefix("http://")?;
    let rest = rest.split('/').next()?;
    let mut parts = rest.split(':');
    let host = parts.next()?.to_string();
    let port: u16 = parts.next()?.parse().ok()?;
    if host.is_empty() {
        return None;
    }
    if !host
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return None;
    }
    Some((host, port))
}

/// Parse a minimal HTTP/1.1 response: skip status line + headers,
/// return the body. Honours Content-Length; chunked encoding is not
/// supported (Ollama returns Content-Length on non-streaming calls).
fn parse_http_response(raw: &[u8]) -> Result<String, String> {
    let sep = b"\r\n\r\n";
    let split = raw
        .windows(sep.len())
        .position(|w| w == sep)
        .ok_or_else(|| "no header/body separator in response".to_string())?;
    let header_block = std::str::from_utf8(&raw[..split])
        .map_err(|e| format!("non-UTF-8 header: {e}"))?;
    let body_bytes = &raw[split + sep.len()..];

    // Status line: "HTTP/1.1 200 OK"
    let status_line = header_block
        .lines()
        .next()
        .ok_or_else(|| "empty response".to_string())?;
    let mut sl = status_line.split_whitespace();
    let _ = sl.next(); // protocol
    let code: u16 = sl
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("could not parse status line: {status_line}"))?;
    if !(200..300).contains(&code) {
        let body = String::from_utf8_lossy(body_bytes).into_owned();
        return Err(format!("HTTP {code}: {body}"));
    }

    std::str::from_utf8(body_bytes)
        .map(|s| s.to_string())
        .map_err(|e| format!("non-UTF-8 body: {e}"))
}

/// Parse Ollama's chat response. We need: assistant text,
/// prompt_eval_count, eval_count, done_reason.
fn parse_ollama_response(
    body: &str,
) -> Result<(String, TokenUsage, FinishReason), String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("response is not JSON: {e}"))?;

    let text = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| "missing message.content".to_string())?
        .to_string();

    let input_tokens = v
        .get("prompt_eval_count")
        .and_then(|n| n.as_u64())
        .unwrap_or(0) as usize;
    let output_tokens = v
        .get("eval_count")
        .and_then(|n| n.as_u64())
        .unwrap_or(0) as usize;

    let finish_reason = match v.get("done_reason").and_then(|s| s.as_str()) {
        Some("stop") => FinishReason::Stop,
        Some("length") => FinishReason::MaxTokens,
        Some(_) => FinishReason::Other,
        None if v.get("done").and_then(|b| b.as_bool()).unwrap_or(false) => FinishReason::Stop,
        None => FinishReason::Other,
    };

    Ok((
        text,
        TokenUsage {
            input_tokens,
            output_tokens,
            cached_tokens: 0,
        },
        finish_reason,
    ))
}

impl LlmClient for OllamaClient {
    fn identity(&self) -> ProviderIdentity {
        self.provider_identity(&self.model_name)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            json_mode_native: true,         // via format=json
            tool_use_native: false,          // polyfilled
            streaming_native: true,          // we just don't expose it yet
            prompt_caching_native: false,    // no cross-request cache
            multimodal_image_input: false,   // model-dependent; conservative default
            max_context_window: 8_192,       // model-dependent; conservative default
        }
    }

    fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let model_used = if req.model.is_empty() {
            self.model_name.clone()
        } else {
            req.model.clone()
        };
        let start = Instant::now();
        let body = self.build_body(&req, /* format_json = */ false);
        let raw = self.post(CHAT_PATH, &body, &model_used)?;
        let (text, usage, finish_reason) =
            parse_ollama_response(&raw).map_err(|d| LlmError::ProtocolError {
                provider: self.provider_identity(&model_used),
                detail: d,
            })?;
        Ok(CompletionResponse {
            text,
            model: model_used.clone(),
            usage,
            finish_reason,
            provider_id: self.provider_identity(&model_used),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn complete_json(
        &self,
        req: CompletionRequest,
        schema: &JsonSchema,
    ) -> Result<CompletionJsonResponse, LlmError> {
        // Ollama supports native JSON mode via `format: "json"`, but
        // does NOT enforce the *shape* of the JSON against a schema.
        // We embed the schema in the system prompt so the model is
        // told what to produce, then validate the structural shape
        // by parsing the response. Schema-validity is a best-effort
        // check (`is_object()`) — primitives layer above will run
        // strict validation.

        let mut prefixed = req.clone();
        let schema_hint = format!(
            "Respond with a single JSON value matching this JSON Schema (name: {name}):\n{body}",
            name = schema.name,
            body = schema.schema_json,
        );
        prefixed.system = Some(match prefixed.system {
            Some(existing) => format!("{existing}\n\n{schema_hint}"),
            None => schema_hint,
        });

        let model_used = if prefixed.model.is_empty() {
            self.model_name.clone()
        } else {
            prefixed.model.clone()
        };
        let start = Instant::now();
        let body = self.build_body(&prefixed, /* format_json = */ true);
        let raw = self.post(CHAT_PATH, &body, &model_used)?;
        let (text, usage, _finish_reason) =
            parse_ollama_response(&raw).map_err(|d| LlmError::ProtocolError {
                provider: self.provider_identity(&model_used),
                detail: d,
            })?;

        let parsed: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| LlmError::SchemaInvalid {
                provider: self.provider_identity(&model_used),
                schema_name: schema.name.clone(),
                raw_text: text.clone(),
                validator_error: format!("response was not parseable JSON: {e}"),
            })?;

        // Minimal structural sanity: top-level should not be `null`.
        let schema_valid = !parsed.is_null();

        Ok(CompletionJsonResponse {
            raw_text: text,
            parsed,
            schema_valid,
            model: model_used.clone(),
            usage,
            provider_id: self.provider_identity(&model_used),
            latency_ms: start.elapsed().as_millis() as u64,
            polyfill_used: false, // native format=json
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers needed for tests but useful on their own.
// ---------------------------------------------------------------------------

/// Smoke-check whether an Ollama endpoint is reachable and responds.
/// Not called automatically; exposed for callers that want a fast
/// pre-flight before issuing real completions.
pub fn ping(endpoint: &str, timeout: Duration) -> Result<(), String> {
    let (host, port) = parse_endpoint(endpoint)
        .ok_or_else(|| format!("invalid endpoint: {endpoint}"))?;
    let addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("resolve: {e}"))?
        .next()
        .ok_or_else(|| "no address".to_string())?;
    let mut stream =
        TcpStream::connect_timeout(&addr, timeout).map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("set_read_timeout: {e}"))?;
    let req = format!(
        "GET /api/tags HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    let mut raw = Vec::new();
    (&mut stream)
        .take(MAX_RESPONSE_BYTES)
        .read_to_end(&mut raw)
        .map_err(|e| format!("read: {e}"))?;
    if raw.len() as u64 == MAX_RESPONSE_BYTES {
        return Err(format!("response exceeded {MAX_RESPONSE_BYTES} byte cap"));
    }
    parse_http_response(&raw).map(|_| ())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway::{Message, MessageContent, Role};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::thread;

    /// A zero-dep one-shot HTTP server. Binds to `127.0.0.1:0`, accepts
    /// exactly one connection, hands the parsed request body to the
    /// caller via a shared slot, and serves a scripted response.
    struct ScriptedServer {
        endpoint: String,
        captured_body: Arc<Mutex<Option<String>>>,
        captured_path: Arc<Mutex<Option<String>>>,
        join: Option<thread::JoinHandle<()>>,
    }

    impl ScriptedServer {
        fn spawn(response_status: u16, response_body: &str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let captured_body = Arc::new(Mutex::new(None));
            let captured_path = Arc::new(Mutex::new(None));
            let cb = Arc::clone(&captured_body);
            let cp = Arc::clone(&captured_path);
            let resp_body = response_body.to_string();

            let join = thread::spawn(move || {
                let (mut sock, _) = listener.accept().unwrap();
                let mut buf = [0u8; 8192];
                let mut read_so_far = Vec::new();
                let mut headers_done = false;
                let mut content_length: usize = 0;
                // Read until we have the full header + content-length body.
                loop {
                    let n = sock.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    read_so_far.extend_from_slice(&buf[..n]);
                    if !headers_done {
                        if let Some(idx) = read_so_far
                            .windows(4)
                            .position(|w| w == b"\r\n\r\n")
                        {
                            headers_done = true;
                            let header_str =
                                std::str::from_utf8(&read_so_far[..idx]).unwrap();
                            // Capture path from request line
                            if let Some(line) = header_str.lines().next() {
                                let mut parts = line.split_whitespace();
                                let _method = parts.next();
                                if let Some(p) = parts.next() {
                                    *cp.lock().unwrap() = Some(p.to_string());
                                }
                            }
                            for line in header_str.lines() {
                                if let Some(v) =
                                    line.strip_prefix("Content-Length:")
                                {
                                    content_length = v.trim().parse().unwrap_or(0);
                                }
                            }
                            let body_so_far = read_so_far.len() - (idx + 4);
                            if body_so_far >= content_length {
                                let body = &read_so_far[idx + 4..idx + 4 + content_length];
                                *cb.lock().unwrap() =
                                    Some(String::from_utf8_lossy(body).into_owned());
                                break;
                            }
                        }
                    } else {
                        // Header read; check whether body is complete.
                        // Find the body-start offset:
                        if let Some(idx) = read_so_far
                            .windows(4)
                            .position(|w| w == b"\r\n\r\n")
                        {
                            let body_so_far = read_so_far.len() - (idx + 4);
                            if body_so_far >= content_length {
                                let body = &read_so_far[idx + 4..idx + 4 + content_length];
                                *cb.lock().unwrap() =
                                    Some(String::from_utf8_lossy(body).into_owned());
                                break;
                            }
                        }
                    }
                }

                let reason = if (200..300).contains(&response_status) { "OK" } else { "ERR" };
                let resp = format!(
                    "HTTP/1.1 {status} {reason}\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {len}\r\n\
                     Connection: close\r\n\
                     \r\n{body}",
                    status = response_status,
                    reason = reason,
                    len = resp_body.len(),
                    body = resp_body,
                );
                let _ = sock.write_all(resp.as_bytes());
            });

            Self {
                endpoint: format!("http://127.0.0.1:{port}"),
                captured_body,
                captured_path,
                join: Some(join),
            }
        }

        fn finish(mut self) -> (String, String) {
            self.join.take().unwrap().join().unwrap();
            let body = self.captured_body.lock().unwrap().clone().unwrap_or_default();
            let path = self.captured_path.lock().unwrap().clone().unwrap_or_default();
            (path, body)
        }
    }

    fn req_user(text: &str) -> CompletionRequest {
        CompletionRequest {
            model: String::new(), // use client default
            system: None,
            messages: vec![Message {
                role: Role::User,
                content: MessageContent::Text(text.into()),
            }],
            temperature: 0.0,
            max_tokens: None,
            stop_sequences: Vec::new(),
            seed: None,
            metadata: Default::default(),
        }
    }

    fn ollama_chat_response(text: &str, prompt_tokens: u64, eval_tokens: u64) -> String {
        serde_json::json!({
            "model": "llama3.1:8b",
            "created_at": "2026-05-11T00:00:00Z",
            "message": { "role": "assistant", "content": text },
            "done": true,
            "done_reason": "stop",
            "prompt_eval_count": prompt_tokens,
            "eval_count": eval_tokens,
        })
        .to_string()
    }

    #[test]
    fn parse_endpoint_accepts_http_with_port() {
        assert_eq!(
            parse_endpoint("http://localhost:11434"),
            Some(("localhost".to_string(), 11434))
        );
    }

    #[test]
    fn parse_endpoint_rejects_https() {
        assert!(parse_endpoint("https://localhost:11434").is_none());
    }

    #[test]
    fn parse_endpoint_rejects_missing_port() {
        assert!(parse_endpoint("http://localhost").is_none());
    }

    #[test]
    fn parse_endpoint_rejects_empty_host() {
        assert!(parse_endpoint("http://:11434").is_none());
    }

    #[test]
    fn parse_endpoint_rejects_crlf_in_host() {
        // Defense-in-depth against header smuggling — even though
        // `to_socket_addrs` would almost certainly reject this host
        // before any bytes hit the wire, we refuse to construct the
        // request in the first place.
        assert!(parse_endpoint("http://evil.example.com\r\nX-Injected:11434").is_none());
        assert!(parse_endpoint("http://has space:11434").is_none());
        assert!(parse_endpoint("http://has\ttab:11434").is_none());
    }

    #[test]
    fn flatten_text_content_passes_through() {
        let c = MessageContent::Text("hi".into());
        assert_eq!(flatten_content(&c), "hi");
    }

    #[test]
    fn flatten_multimodal_drops_images() {
        let c = MessageContent::Multimodal(vec![
            ContentBlock::Text("hello".into()),
            ContentBlock::ImageBase64 {
                mime_type: "image/png".into(),
                data: "ignored".into(),
            },
            ContentBlock::Text("world".into()),
        ]);
        assert_eq!(flatten_content(&c), "hello\nworld");
    }

    #[test]
    fn identity_carries_endpoint() {
        let c = OllamaClient::new("llama3.1:8b");
        let id = c.identity();
        assert_eq!(id.vendor, "ollama");
        assert_eq!(id.model_family, "llama3.1:8b");
        assert_eq!(id.endpoint.as_deref(), Some("http://localhost:11434"));
    }

    #[test]
    fn capabilities_report_json_mode_native_and_no_tools() {
        let caps = OllamaClient::new("x").capabilities();
        assert!(caps.json_mode_native);
        assert!(!caps.tool_use_native);
        assert!(!caps.prompt_caching_native);
    }

    #[test]
    fn parse_http_response_extracts_body() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nhello";
        assert_eq!(parse_http_response(raw).unwrap(), "hello");
    }

    #[test]
    fn parse_http_response_surfaces_non_200() {
        let raw = b"HTTP/1.1 500 Internal Server Error\r\n\r\nboom";
        let err = parse_http_response(raw).unwrap_err();
        assert!(err.contains("HTTP 500"));
        assert!(err.contains("boom"));
    }

    #[test]
    fn parse_ollama_response_extracts_text_and_tokens() {
        let body = ollama_chat_response("hello there", 7, 4);
        let (text, usage, finish) = parse_ollama_response(&body).unwrap();
        assert_eq!(text, "hello there");
        assert_eq!(usage.input_tokens, 7);
        assert_eq!(usage.output_tokens, 4);
        assert_eq!(finish, FinishReason::Stop);
    }

    #[test]
    fn parse_ollama_response_maps_length_to_max_tokens() {
        let body = serde_json::json!({
            "message": { "role": "assistant", "content": "..." },
            "done": true,
            "done_reason": "length",
        })
        .to_string();
        let (_t, _u, finish) = parse_ollama_response(&body).unwrap();
        assert_eq!(finish, FinishReason::MaxTokens);
    }

    #[test]
    fn complete_round_trips_through_scripted_server() {
        let server = ScriptedServer::spawn(200, &ollama_chat_response("ack", 3, 1));
        let client = OllamaClient::new("llama3.1:8b").with_endpoint(server.endpoint.clone());
        let resp = client.complete(req_user("ping")).unwrap();
        assert_eq!(resp.text, "ack");
        assert_eq!(resp.model, "llama3.1:8b");
        assert_eq!(resp.usage.input_tokens, 3);
        assert_eq!(resp.usage.output_tokens, 1);
        assert_eq!(resp.finish_reason, FinishReason::Stop);

        let (path, body) = server.finish();
        assert_eq!(path, "/api/chat");
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["model"], "llama3.1:8b");
        assert_eq!(body["stream"], false);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "ping");
    }

    #[test]
    fn complete_uses_request_model_when_non_empty() {
        let server = ScriptedServer::spawn(200, &ollama_chat_response("ok", 1, 1));
        let client = OllamaClient::new("default-model").with_endpoint(server.endpoint.clone());
        let mut req = req_user("hi");
        req.model = "override-model:7b".into();
        let resp = client.complete(req).unwrap();
        assert_eq!(resp.model, "override-model:7b");

        let (_p, body) = server.finish();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["model"], "override-model:7b");
    }

    #[test]
    fn complete_passes_system_message() {
        let server = ScriptedServer::spawn(200, &ollama_chat_response("ok", 1, 1));
        let client = OllamaClient::new("m").with_endpoint(server.endpoint.clone());
        let mut req = req_user("user msg");
        req.system = Some("be terse".into());
        let _ = client.complete(req).unwrap();

        let (_p, body) = server.finish();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "be terse");
        assert_eq!(body["messages"][1]["role"], "user");
    }

    #[test]
    fn complete_serializes_options_when_set() {
        let server = ScriptedServer::spawn(200, &ollama_chat_response("ok", 1, 1));
        let client = OllamaClient::new("m").with_endpoint(server.endpoint.clone());
        let mut req = req_user("hi");
        req.temperature = 0.7;
        req.max_tokens = Some(128);
        req.seed = Some(42);
        req.stop_sequences = vec!["STOP".into()];
        let _ = client.complete(req).unwrap();

        let (_p, body) = server.finish();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!((body["options"]["temperature"].as_f64().unwrap() - 0.7).abs() < 1e-6);
        assert_eq!(body["options"]["num_predict"], 128);
        assert_eq!(body["options"]["seed"], 42);
        assert_eq!(body["options"]["stop"][0], "STOP");
    }

    #[test]
    fn complete_non_2xx_surfaces_as_transport_error() {
        let server = ScriptedServer::spawn(500, "internal boom");
        let client = OllamaClient::new("m").with_endpoint(server.endpoint.clone());
        let err = client.complete(req_user("x")).unwrap_err();
        match err {
            LlmError::Transport { detail, .. } => assert!(detail.contains("HTTP 500")),
            other => panic!("expected Transport, got {other:?}"),
        }
        let _ = server.finish();
    }

    #[test]
    fn complete_json_sets_format_and_appends_schema_hint() {
        let server = ScriptedServer::spawn(
            200,
            &ollama_chat_response("{\"answer\":42}", 5, 3),
        );
        let client = OllamaClient::new("m").with_endpoint(server.endpoint.clone());
        let schema = JsonSchema {
            name: "AnswerObj".into(),
            schema_json: r#"{"type":"object","properties":{"answer":{"type":"number"}}}"#.into(),
        };
        let resp = client.complete_json(req_user("answer?"), &schema).unwrap();
        assert!(resp.schema_valid);
        assert_eq!(resp.parsed["answer"], 42);
        assert!(!resp.polyfill_used);

        let (_p, body) = server.finish();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["format"], "json");
        let system_msg = body["messages"][0]["content"].as_str().unwrap();
        assert!(system_msg.contains("AnswerObj"));
        assert!(system_msg.contains("\"type\":\"object\""));
    }

    #[test]
    fn complete_json_returns_schema_invalid_when_response_is_not_json() {
        let server = ScriptedServer::spawn(
            200,
            &ollama_chat_response("not a json document", 2, 2),
        );
        let client = OllamaClient::new("m").with_endpoint(server.endpoint.clone());
        let schema = JsonSchema {
            name: "X".into(),
            schema_json: "{}".into(),
        };
        let err = client.complete_json(req_user("x"), &schema).unwrap_err();
        match err {
            LlmError::SchemaInvalid { schema_name, .. } => assert_eq!(schema_name, "X"),
            other => panic!("expected SchemaInvalid, got {other:?}"),
        }
        let _ = server.finish();
    }
}
