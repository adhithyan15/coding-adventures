//! Integration tests for `coding-adventures-rpc`.
//!
//! ## Test Strategy
//!
//! We test the RPC layer in isolation using a `MockCodec` and a `MockFramer`.
//! These mocks implement the `RpcCodec` and `RpcFramer` traits using in-memory
//! buffers (`Cursor<Vec<u8>>`). This lets us:
//!
//! 1. Verify server dispatch without any real I/O.
//! 2. Verify client request/response flow with full round-trips.
//! 3. Inject errors (decode failures, framing errors, panics) deterministically.
//!
//! ## Mock Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  RpcServer / RpcClient                                      │
//! │    ↕  RpcMessage<Value>                                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  MockCodec                                                  │
//! │  — encodes: serde_json::to_vec(msg)                         │
//! │  — decodes: serde_json::from_slice(bytes) then classifies   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  MockFramer                                                 │
//! │  — read_frame: read u32 length prefix, then payload         │
//! │  — write_frame: write u32 length prefix, then payload       │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Cursor<Vec<u8>>  (in-memory I/O — no OS calls)             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! The MockCodec uses serde_json for simplicity. The JSON object shape
//! determines message type:
//! - `{ "id": ..., "method": ... }` → Request
//! - `{ "method": ... }` (no id) → Notification
//! - `{ "id": ..., "result": ... }` → Response
//! - `{ "id": ..., "error": { "code": ..., "message": ... } }` → ErrorResponse

use coding_adventures_rpc::client::RpcClient;
use coding_adventures_rpc::codec::RpcCodec;
use coding_adventures_rpc::errors::{RpcError, INTERNAL_ERROR, METHOD_NOT_FOUND};
use coding_adventures_rpc::framer::RpcFramer;
use coding_adventures_rpc::message::{
    RpcErrorResponse, RpcMessage, RpcNotification, RpcRequest, RpcResponse,
};
use coding_adventures_rpc::server::RpcServer;
use serde_json::{json, Value};
use std::io::{Cursor, Read, Write};
use std::sync::{Arc, Mutex};

// =============================================================================
// MockFramer
// =============================================================================
//
// Frames using a 4-byte big-endian length prefix followed by the payload.
//
// ```text
// ┌──────────────────────┬───────────────────────────────┐
// │  length (4 bytes BE) │  payload (length bytes)       │
// └──────────────────────┴───────────────────────────────┘
// ```
//
// This is one of the simplest possible framing schemes — just enough to have
// unambiguous message boundaries.

/// An in-memory framer for testing. Uses a 4-byte big-endian length prefix.
struct MockFramer {
    reader: Cursor<Vec<u8>>,
    writer: Cursor<Vec<u8>>,
}

impl MockFramer {
    /// Create a framer that reads from `input` and writes to an internal buffer.
    fn new(input: Vec<u8>) -> Self {
        Self {
            reader: Cursor::new(input),
            writer: Cursor::new(Vec::new()),
        }
    }
}

impl RpcFramer for MockFramer {
    fn read_frame(&mut self) -> Option<Result<Vec<u8>, RpcError>> {
        // Read the 4-byte big-endian length prefix.
        let mut len_buf = [0u8; 4];
        match self.reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return None,
            Err(e) => return Some(Err(RpcError::new(e.to_string()))),
        }
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read exactly `len` payload bytes.
        let mut payload = vec![0u8; len];
        match self.reader.read_exact(&mut payload) {
            Ok(()) => Some(Ok(payload)),
            Err(e) => Some(Err(RpcError::new(format!("payload read error: {}", e)))),
        }
    }

    fn write_frame(&mut self, data: &[u8]) -> Result<(), RpcError> {
        // Write the 4-byte big-endian length prefix.
        let len = data.len() as u32;
        self.writer
            .write_all(&len.to_be_bytes())
            .map_err(|e| RpcError::new(e.to_string()))?;
        // Write the payload.
        self.writer
            .write_all(data)
            .map_err(|e| RpcError::new(e.to_string()))
    }
}

// =============================================================================
// frame() / unframe() helpers
// =============================================================================
//
// Helpers to build framed byte sequences for test inputs.

/// Wrap `payload` in the 4-byte length-prefix frame format.
fn frame(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u32;
    let mut out = len.to_be_bytes().to_vec();
    out.extend_from_slice(payload);
    out
}

/// Read all framed payloads from `bytes` (multiple frames concatenated).
fn read_all_frames(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut cursor = Cursor::new(bytes);
    let mut frames = Vec::new();
    loop {
        let mut len_buf = [0u8; 4];
        match cursor.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(_) => break,
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        cursor.read_exact(&mut payload).unwrap();
        frames.push(payload);
    }
    frames
}

// =============================================================================
// MockCodec
// =============================================================================
//
// Encodes RpcMessage<Value> as a JSON object and decodes it back. The JSON
// shape distinguishes message types:
//
// | Fields present           | Type            |
// |--------------------------|-----------------|
// | "id" + "method"          | Request         |
// | "method" (no "id")       | Notification    |
// | "id" + "result"          | Response        |
// | "id" + "error"           | ErrorResponse   |

/// An in-memory codec for testing. Uses serde_json for serialisation.
struct MockCodec;

impl RpcCodec<Value> for MockCodec {
    fn encode(&self, msg: &RpcMessage<Value>) -> Result<Vec<u8>, RpcError> {
        let v = match msg {
            RpcMessage::Request(req) => {
                let mut m = serde_json::Map::new();
                m.insert("id".into(), req.id.clone());
                m.insert("method".into(), Value::String(req.method.clone()));
                if let Some(p) = &req.params {
                    m.insert("params".into(), p.clone());
                }
                Value::Object(m)
            }
            RpcMessage::Response(resp) => {
                let mut m = serde_json::Map::new();
                m.insert("id".into(), resp.id.clone());
                m.insert("result".into(), resp.result.clone());
                Value::Object(m)
            }
            RpcMessage::ErrorResponse(err) => {
                let mut m = serde_json::Map::new();
                m.insert(
                    "id".into(),
                    err.id.clone().unwrap_or(Value::Null),
                );
                let mut e = serde_json::Map::new();
                e.insert("code".into(), json!(err.code));
                e.insert("message".into(), Value::String(err.message.clone()));
                if let Some(d) = &err.data {
                    e.insert("data".into(), d.clone());
                }
                m.insert("error".into(), Value::Object(e));
                Value::Object(m)
            }
            RpcMessage::Notification(notif) => {
                let mut m = serde_json::Map::new();
                m.insert("method".into(), Value::String(notif.method.clone()));
                if let Some(p) = &notif.params {
                    m.insert("params".into(), p.clone());
                }
                Value::Object(m)
            }
        };
        serde_json::to_vec(&v).map_err(|e| RpcError::new(e.to_string()))
    }

    fn decode(&self, data: &[u8]) -> Result<RpcMessage<Value>, RpcErrorResponse<Value>> {
        // Step 1: Parse JSON.
        let v: Value = serde_json::from_slice(data).map_err(|e| RpcErrorResponse {
            id: None,
            code: coding_adventures_rpc::errors::PARSE_ERROR,
            message: format!("parse error: {}", e),
            data: None,
        })?;

        // Step 2: Must be an object.
        let obj = match &v {
            Value::Object(m) => m.clone(),
            _ => {
                return Err(RpcErrorResponse {
                    id: None,
                    code: coding_adventures_rpc::errors::INVALID_REQUEST,
                    message: "expected JSON object".into(),
                    data: None,
                })
            }
        };

        let has_method = obj.contains_key("method");
        let has_id = obj.contains_key("id");
        let has_result = obj.contains_key("result");
        let has_error = obj.contains_key("error");

        if has_id && has_method {
            // Request
            let id = obj["id"].clone();
            let method = obj["method"].as_str().unwrap_or("").to_string();
            let params = obj.get("params").cloned();
            Ok(RpcMessage::Request(RpcRequest { id, method, params }))
        } else if has_method {
            // Notification
            let method = obj["method"].as_str().unwrap_or("").to_string();
            let params = obj.get("params").cloned();
            Ok(RpcMessage::Notification(RpcNotification { method, params }))
        } else if has_id && has_result {
            // Response
            let id = obj["id"].clone();
            let result = obj["result"].clone();
            Ok(RpcMessage::Response(RpcResponse { id, result }))
        } else if has_id && has_error {
            // ErrorResponse
            let id = obj["id"].clone();
            let id_opt = if id == Value::Null { None } else { Some(id) };
            let err_obj = &obj["error"];
            let code = err_obj["code"].as_i64().unwrap_or(-32_603);
            let message = err_obj["message"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let data = err_obj.get("data").cloned();
            Ok(RpcMessage::ErrorResponse(RpcErrorResponse {
                id: id_opt,
                code,
                message,
                data,
            }))
        } else {
            Err(RpcErrorResponse {
                id: None,
                code: coding_adventures_rpc::errors::INVALID_REQUEST,
                message: "missing method, result, or error field".into(),
                data: None,
            })
        }
    }
}

// =============================================================================
// Helper: build a framed request JSON byte sequence
// =============================================================================

/// Build a framed MockCodec-encoded request for `method` with optional params.
fn request_frame(id: i64, method: &str, params: Option<Value>) -> Vec<u8> {
    let codec = MockCodec;
    let msg = RpcMessage::Request(RpcRequest {
        id: json!(id),
        method: method.to_string(),
        params,
    });
    let payload = codec.encode(&msg).unwrap();
    frame(&payload)
}

/// Build a framed MockCodec-encoded notification for `method` with optional params.
fn notification_frame(method: &str, params: Option<Value>) -> Vec<u8> {
    let codec = MockCodec;
    let msg = RpcMessage::Notification(RpcNotification {
        method: method.to_string(),
        params,
    });
    let payload = codec.encode(&msg).unwrap();
    frame(&payload)
}

/// Build a framed MockCodec-encoded Response frame (for client tests).
fn response_frame(id: i64, result: Value) -> Vec<u8> {
    let codec = MockCodec;
    let msg = RpcMessage::Response(RpcResponse {
        id: json!(id),
        result,
    });
    let payload = codec.encode(&msg).unwrap();
    frame(&payload)
}

/// Build a framed MockCodec-encoded ErrorResponse frame (for client tests).
fn error_response_frame(id: Option<i64>, code: i64, message: &str) -> Vec<u8> {
    let codec = MockCodec;
    let msg = RpcMessage::ErrorResponse(RpcErrorResponse {
        id: id.map(|i| json!(i)),
        code,
        message: message.to_string(),
        data: None,
    });
    let payload = codec.encode(&msg).unwrap();
    frame(&payload)
}

// =============================================================================
// Helper: decode all response frames from a MockFramer's written bytes
// =============================================================================

fn decode_all_responses(bytes: &[u8]) -> Vec<RpcMessage<Value>> {
    let codec = MockCodec;
    read_all_frames(bytes)
        .into_iter()
        .map(|payload| codec.decode(&payload).expect("decode written response"))
        .collect()
}

// =============================================================================
// SharedMockFramer — allows accessing written bytes after serve()
// =============================================================================
//
// The problem: `RpcServer` owns the framer as a `Box<dyn RpcFramer>`. After
// `serve()` returns, we cannot get the framer back out (no `into_inner`).
//
// Solution: use `Arc<Mutex<Vec<u8>>>` so the test can read the written bytes
// without needing to own the framer.

struct SharedMockFramer {
    reader: Cursor<Vec<u8>>,
    written: Arc<Mutex<Vec<u8>>>,
}

impl SharedMockFramer {
    fn new(input: Vec<u8>, written: Arc<Mutex<Vec<u8>>>) -> Self {
        Self {
            reader: Cursor::new(input),
            written,
        }
    }
}

impl RpcFramer for SharedMockFramer {
    fn read_frame(&mut self) -> Option<Result<Vec<u8>, RpcError>> {
        let mut len_buf = [0u8; 4];
        match self.reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return None,
            Err(e) => return Some(Err(RpcError::new(e.to_string()))),
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        match self.reader.read_exact(&mut payload) {
            Ok(()) => Some(Ok(payload)),
            Err(e) => Some(Err(RpcError::new(format!("payload read error: {}", e)))),
        }
    }

    fn write_frame(&mut self, data: &[u8]) -> Result<(), RpcError> {
        let len = data.len() as u32;
        let mut guard = self.written.lock().unwrap();
        guard.extend_from_slice(&len.to_be_bytes());
        guard.extend_from_slice(data);
        Ok(())
    }
}

/// Build and run a server using SharedMockFramer, returning the written bytes.
fn run_server_shared(
    input: Vec<u8>,
    setup: impl FnOnce(&mut RpcServer<Cursor<Vec<u8>>, Cursor<Vec<u8>>, Value>),
) -> Vec<u8> {
    let written = Arc::new(Mutex::new(Vec::new()));
    let codec = Box::new(MockCodec);
    let framer = Box::new(SharedMockFramer::new(input, Arc::clone(&written)));
    let mut server: RpcServer<Cursor<Vec<u8>>, Cursor<Vec<u8>>, Value> =
        RpcServer::new(codec, framer);
    setup(&mut server);
    server.serve();
    // Drop the server so the framer (and its Arc clone) is dropped before we
    // call Arc::try_unwrap. After drop, `written` is the only owner.
    drop(server);
    Arc::try_unwrap(written)
        .expect("no other references")
        .into_inner()
        .unwrap()
}

// =============================================================================
// SERVER TESTS
// =============================================================================

// ---------------------------------------------------------------------------
// Test 1: known request is dispatched and result is returned
// ---------------------------------------------------------------------------
//
// The server should call the "ping" handler and write a Response with the
// handler's return value.

#[test]
fn test_server_dispatches_known_request() {
    let input = request_frame(1, "ping", None);
    let output = run_server_shared(input, |server| {
        server.on_request("ping", |_id, _params| Ok(json!("pong")));
    });

    let messages = decode_all_responses(&output);
    assert_eq!(messages.len(), 1, "expected exactly one response");

    match &messages[0] {
        RpcMessage::Response(resp) => {
            assert_eq!(resp.id, json!(1), "id must echo the request id");
            assert_eq!(resp.result, json!("pong"), "result must be 'pong'");
        }
        other => panic!("expected Response, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 2: unregistered method → METHOD_NOT_FOUND
// ---------------------------------------------------------------------------
//
// A request for a method with no handler should produce an ErrorResponse with
// code -32601 (METHOD_NOT_FOUND), not a panic or silent drop.

#[test]
fn test_server_unknown_method_returns_method_not_found() {
    let input = request_frame(2, "unknown_method", None);
    let output = run_server_shared(input, |_server| {
        // No handlers registered.
    });

    let messages = decode_all_responses(&output);
    assert_eq!(messages.len(), 1);

    match &messages[0] {
        RpcMessage::ErrorResponse(err) => {
            assert_eq!(err.id, Some(json!(2)), "id must echo request id");
            assert_eq!(err.code, METHOD_NOT_FOUND, "code must be METHOD_NOT_FOUND");
        }
        other => panic!("expected ErrorResponse, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 3: handler returns Err → error response with handler's error
// ---------------------------------------------------------------------------

#[test]
fn test_server_handler_error_is_returned() {
    let input = request_frame(3, "fail", None);
    let output = run_server_shared(input, |server| {
        server.on_request("fail", |_id, _params| {
            Err(RpcErrorResponse {
                id: None, // server will fill this in
                code: coding_adventures_rpc::errors::INVALID_PARAMS,
                message: "bad params".into(),
                data: Some(json!("reason")),
            })
        });
    });

    let messages = decode_all_responses(&output);
    assert_eq!(messages.len(), 1);

    match &messages[0] {
        RpcMessage::ErrorResponse(err) => {
            assert_eq!(err.code, coding_adventures_rpc::errors::INVALID_PARAMS);
            assert_eq!(err.message, "bad params");
        }
        other => panic!("expected ErrorResponse, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 4: panicking handler → INTERNAL_ERROR response (no crash)
// ---------------------------------------------------------------------------
//
// The server must survive a panicking handler. The loop continues and an
// INTERNAL_ERROR response is sent for the offending request.

#[test]
fn test_server_panicking_handler_sends_internal_error() {
    let input = request_frame(4, "panic_method", None);
    let output = run_server_shared(input, |server| {
        server.on_request("panic_method", |_id, _params| {
            panic!("deliberate test panic");
        });
    });

    let messages = decode_all_responses(&output);
    assert_eq!(messages.len(), 1, "must still produce exactly one response");

    match &messages[0] {
        RpcMessage::ErrorResponse(err) => {
            assert_eq!(
                err.code, INTERNAL_ERROR,
                "panic must produce INTERNAL_ERROR"
            );
        }
        other => panic!("expected ErrorResponse, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 5: server survives panicking handler and continues serving
// ---------------------------------------------------------------------------
//
// After a panicking handler, the server should continue processing subsequent
// requests.

#[test]
fn test_server_continues_after_panic() {
    let mut input = request_frame(5, "panic_method", None);
    input.extend(request_frame(6, "ok_method", None));

    let output = run_server_shared(input, |server| {
        server.on_request("panic_method", |_id, _params| {
            panic!("deliberate");
        });
        server.on_request("ok_method", |_id, _params| Ok(json!("ok")));
    });

    let messages = decode_all_responses(&output);
    assert_eq!(messages.len(), 2, "both requests must produce responses");

    // First: INTERNAL_ERROR from the panic.
    match &messages[0] {
        RpcMessage::ErrorResponse(err) => assert_eq!(err.code, INTERNAL_ERROR),
        other => panic!("expected ErrorResponse, got {:?}", other),
    }
    // Second: success from ok_method.
    match &messages[1] {
        RpcMessage::Response(resp) => {
            assert_eq!(resp.id, json!(6));
            assert_eq!(resp.result, json!("ok"));
        }
        other => panic!("expected Response, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 6: notification is dispatched (no response written)
// ---------------------------------------------------------------------------
//
// The server calls the handler but writes nothing to the framer.

#[test]
fn test_server_notification_dispatched_no_response() {
    let called = Arc::new(Mutex::new(false));
    let called_clone = Arc::clone(&called);

    let input = notification_frame("log", Some(json!("hello")));
    let output = run_server_shared(input, move |server| {
        let flag = Arc::clone(&called_clone);
        server.on_notification("log", move |_params| {
            *flag.lock().unwrap() = true;
        });
    });

    // No bytes written.
    assert!(output.is_empty(), "notifications must not produce a response");
    assert!(*called.lock().unwrap(), "handler must have been called");
}

// ---------------------------------------------------------------------------
// Test 7: unknown notification is silently dropped
// ---------------------------------------------------------------------------

#[test]
fn test_server_unknown_notification_silent() {
    let input = notification_frame("unknown_notification", None);
    let output = run_server_shared(input, |_server| {
        // No handlers.
    });

    assert!(output.is_empty(), "unknown notifications must be silently dropped");
}

// ---------------------------------------------------------------------------
// Test 8: notification handler panic does not kill server
// ---------------------------------------------------------------------------

#[test]
fn test_server_notification_handler_panic_is_recovered() {
    let mut input = notification_frame("panicky", None);
    input.extend(request_frame(7, "ping", None)); // must still be handled

    let output = run_server_shared(input, |server| {
        server.on_notification("panicky", |_params| {
            panic!("notification panic");
        });
        server.on_request("ping", |_id, _params| Ok(json!("pong")));
    });

    // The notification produced no output. The request that followed should
    // still produce a Response.
    let messages = decode_all_responses(&output);
    assert_eq!(messages.len(), 1, "request after panic notification must respond");
    match &messages[0] {
        RpcMessage::Response(resp) => assert_eq!(resp.result, json!("pong")),
        other => panic!("expected Response, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 9: malformed frame bytes → PARSE_ERROR response with null id
// ---------------------------------------------------------------------------
//
// When the codec cannot decode the frame, the server sends a PARSE_ERROR
// error response with id=null.

#[test]
fn test_server_decode_error_sends_parse_error() {
    // A valid frame length prefix but non-JSON payload.
    let bad_payload = b"this is not json at all!!!";
    let input = frame(bad_payload);

    let output = run_server_shared(input, |_server| {});

    let messages = decode_all_responses(&output);
    assert_eq!(messages.len(), 1);

    match &messages[0] {
        RpcMessage::ErrorResponse(err) => {
            assert_eq!(err.code, coding_adventures_rpc::errors::PARSE_ERROR);
            // id should be None (null) because we couldn't read the request id.
            assert_eq!(err.id, None, "null id expected for parse errors");
        }
        other => panic!("expected ErrorResponse, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 10: multiple sequential requests all get responses
// ---------------------------------------------------------------------------

#[test]
fn test_server_multiple_requests_in_sequence() {
    let mut input = Vec::new();
    for i in 1..=5i64 {
        input.extend(request_frame(i, "echo", Some(json!(i))));
    }

    let output = run_server_shared(input, |server| {
        server.on_request("echo", |_id, params| Ok(params.unwrap_or(Value::Null)));
    });

    let messages = decode_all_responses(&output);
    assert_eq!(messages.len(), 5, "each request must get a response");

    for (i, msg) in messages.iter().enumerate() {
        let expected_id = json!((i + 1) as i64);
        match msg {
            RpcMessage::Response(resp) => {
                assert_eq!(resp.id, expected_id);
                assert_eq!(resp.result, expected_id, "echo should return the param");
            }
            other => panic!("expected Response #{}, got {:?}", i + 1, other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test 11: method chaining on on_request / on_notification
// ---------------------------------------------------------------------------

#[test]
fn test_server_method_chaining() {
    let mut input = request_frame(10, "a", None);
    input.extend(request_frame(11, "b", None));

    let output = run_server_shared(input, |server| {
        server
            .on_request("a", |_id, _params| Ok(json!("from_a")))
            .on_request("b", |_id, _params| Ok(json!("from_b")));
    });

    let messages = decode_all_responses(&output);
    assert_eq!(messages.len(), 2);
    match &messages[0] {
        RpcMessage::Response(r) => assert_eq!(r.result, json!("from_a")),
        o => panic!("{:?}", o),
    }
    match &messages[1] {
        RpcMessage::Response(r) => assert_eq!(r.result, json!("from_b")),
        o => panic!("{:?}", o),
    }
}

// =============================================================================
// CLIENT TESTS
// =============================================================================
//
// For client tests we pre-load a `SharedMockFramer` with the server's expected
// response bytes and verify what the client wrote.

struct ClientTestSetup {
    /// Pre-canned response frames for the client to read.
    response_bytes: Vec<u8>,
    /// Shared buffer that records what the client wrote.
    written: Arc<Mutex<Vec<u8>>>,
}

impl ClientTestSetup {
    fn new(response_bytes: Vec<u8>) -> Self {
        Self {
            response_bytes,
            written: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn build_client(self) -> (RpcClient<Value>, Arc<Mutex<Vec<u8>>>) {
        let framer = Box::new(SharedMockFramer::new(
            self.response_bytes,
            Arc::clone(&self.written),
        ));
        let codec = Box::new(MockCodec);
        let client = RpcClient::new(codec, framer);
        (client, self.written)
    }
}

// ---------------------------------------------------------------------------
// Test 12: client request returns the server's result
// ---------------------------------------------------------------------------

#[test]
fn test_client_request_returns_result() {
    let setup = ClientTestSetup::new(response_frame(1, json!("pong")));
    let (mut client, _written) = setup.build_client();

    let result = client.request("ping", None).expect("should succeed");
    assert_eq!(result, json!("pong"));
}

// ---------------------------------------------------------------------------
// Test 13: client request propagates server error response
// ---------------------------------------------------------------------------

#[test]
fn test_client_request_returns_error_response() {
    let setup = ClientTestSetup::new(error_response_frame(Some(1), METHOD_NOT_FOUND, "Method not found"));
    let (mut client, _written) = setup.build_client();

    let err = client.request("unknown", None).expect_err("should fail");
    assert_eq!(err.code, METHOD_NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Test 14: client encodes and sends the request frame
// ---------------------------------------------------------------------------
//
// After calling `request()`, the written bytes must decode to a valid Request
// with the correct method.

#[test]
fn test_client_sends_request_frame() {
    let setup = ClientTestSetup::new(response_frame(1, json!(42)));
    let (mut client, written) = setup.build_client();

    client.request("add", Some(json!([1, 2]))).unwrap();

    let written_bytes = written.lock().unwrap().clone();
    let frames = read_all_frames(&written_bytes);
    assert_eq!(frames.len(), 1, "exactly one frame should be sent");

    let codec = MockCodec;
    let msg = codec.decode(&frames[0]).unwrap();
    match msg {
        RpcMessage::Request(req) => {
            assert_eq!(req.method, "add");
            assert_eq!(req.params, Some(json!([1, 2])));
        }
        other => panic!("expected Request, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 15: client ids are auto-generated and monotonically increasing
// ---------------------------------------------------------------------------

#[test]
fn test_client_ids_monotonically_increasing() {
    // Pre-load two responses: id=1 then id=2.
    let mut responses = response_frame(1, json!("r1"));
    responses.extend(response_frame(2, json!("r2")));

    let setup = ClientTestSetup::new(responses);
    let (mut client, written) = setup.build_client();

    client.request("m", None).unwrap();
    client.request("m", None).unwrap();

    let written_bytes = written.lock().unwrap().clone();
    let frames = read_all_frames(&written_bytes);
    assert_eq!(frames.len(), 2);

    let codec = MockCodec;
    let req1 = codec.decode(&frames[0]).unwrap();
    let req2 = codec.decode(&frames[1]).unwrap();

    let id1 = match req1 { RpcMessage::Request(r) => r.id, _ => panic!() };
    let id2 = match req2 { RpcMessage::Request(r) => r.id, _ => panic!() };

    assert_eq!(id1, json!(1), "first request id must be 1");
    assert_eq!(id2, json!(2), "second request id must be 2");
}

// ---------------------------------------------------------------------------
// Test 16: client notify sends frame without waiting
// ---------------------------------------------------------------------------

#[test]
fn test_client_notify_sends_frame() {
    let setup = ClientTestSetup::new(Vec::new()); // no response expected
    let (mut client, written) = setup.build_client();

    client.notify("log", Some(json!("hello"))).unwrap();

    let written_bytes = written.lock().unwrap().clone();
    let frames = read_all_frames(&written_bytes);
    assert_eq!(frames.len(), 1);

    let codec = MockCodec;
    let msg = codec.decode(&frames[0]).unwrap();
    match msg {
        RpcMessage::Notification(notif) => {
            assert_eq!(notif.method, "log");
            assert_eq!(notif.params, Some(json!("hello")));
        }
        other => panic!("expected Notification, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 17: client returns error when connection closed before response
// ---------------------------------------------------------------------------

#[test]
fn test_client_connection_closed_before_response() {
    let setup = ClientTestSetup::new(Vec::new()); // empty — EOF immediately
    let (mut client, _written) = setup.build_client();

    let err = client.request("ping", None).expect_err("should fail on EOF");
    assert_eq!(
        err.code, INTERNAL_ERROR,
        "EOF before response should be INTERNAL_ERROR"
    );
}

// ---------------------------------------------------------------------------
// Test 18: client dispatches server-push notification while waiting
// ---------------------------------------------------------------------------

#[test]
fn test_client_dispatches_push_notification_while_waiting() {
    let push_called = Arc::new(Mutex::new(false));
    let push_called_clone = Arc::clone(&push_called);

    // Pre-load: first a server-push notification, then the actual response.
    let push_notif = notification_frame("push", Some(json!("data")));
    let resp = response_frame(1, json!("ok"));
    let mut responses = push_notif;
    responses.extend(resp);

    let setup = ClientTestSetup::new(responses);
    let (mut client, _written) = setup.build_client();

    client.on_notification("push", move |_params| {
        *push_called_clone.lock().unwrap() = true;
    });

    let result = client.request("ping", None).expect("should succeed");
    assert_eq!(result, json!("ok"), "result must come from the response");
    assert!(
        *push_called.lock().unwrap(),
        "push notification handler must have been called"
    );
}

// =============================================================================
// CODEC TESTS
// =============================================================================
//
// These test the MockCodec in isolation — encode/decode round-trips.

#[test]
fn test_codec_round_trips_request() {
    let codec = MockCodec;
    let msg = RpcMessage::Request(RpcRequest {
        id: json!(99),
        method: "doThing".into(),
        params: Some(json!({"x": 1})),
    });
    let bytes = codec.encode(&msg).unwrap();
    let decoded = codec.decode(&bytes).unwrap();
    assert_eq!(msg, decoded);
}

#[test]
fn test_codec_round_trips_response() {
    let codec = MockCodec;
    let msg = RpcMessage::Response(RpcResponse {
        id: json!("abc"),
        result: json!({"ok": true}),
    });
    let bytes = codec.encode(&msg).unwrap();
    let decoded = codec.decode(&bytes).unwrap();
    assert_eq!(msg, decoded);
}

#[test]
fn test_codec_round_trips_error_response() {
    let codec = MockCodec;
    let msg = RpcMessage::ErrorResponse(RpcErrorResponse {
        id: Some(json!(5)),
        code: METHOD_NOT_FOUND,
        message: "Method not found".into(),
        data: Some(json!("foo")),
    });
    let bytes = codec.encode(&msg).unwrap();
    let decoded = codec.decode(&bytes).unwrap();
    assert_eq!(msg, decoded);
}

#[test]
fn test_codec_round_trips_notification() {
    let codec = MockCodec;
    let msg = RpcMessage::Notification(RpcNotification {
        method: "event".into(),
        params: Some(json!([1, 2, 3])),
    });
    let bytes = codec.encode(&msg).unwrap();
    let decoded = codec.decode(&bytes).unwrap();
    assert_eq!(msg, decoded);
}

#[test]
fn test_codec_decode_invalid_json_returns_parse_error() {
    let codec = MockCodec;
    let err = codec.decode(b"not json").unwrap_err();
    assert_eq!(err.code, coding_adventures_rpc::errors::PARSE_ERROR);
}

#[test]
fn test_codec_decode_valid_json_non_object_returns_invalid_request() {
    let codec = MockCodec;
    let err = codec.decode(b"[1,2,3]").unwrap_err();
    assert_eq!(err.code, coding_adventures_rpc::errors::INVALID_REQUEST);
}

// =============================================================================
// FRAMER TESTS
// =============================================================================

#[test]
fn test_framer_write_then_read_round_trips() {
    let written = Arc::new(Mutex::new(Vec::new()));
    let mut framer = SharedMockFramer::new(Vec::new(), Arc::clone(&written));

    framer.write_frame(b"hello world").unwrap();

    let bytes = written.lock().unwrap().clone();
    let mut reader = SharedMockFramer::new(bytes, Arc::new(Mutex::new(Vec::new())));
    let payload = reader.read_frame().unwrap().unwrap();
    assert_eq!(payload, b"hello world");
}

#[test]
fn test_framer_eof_returns_none() {
    let mut framer = MockFramer::new(Vec::new());
    assert!(
        framer.read_frame().is_none(),
        "empty input must return None (EOF)"
    );
}

#[test]
fn test_framer_multiple_frames_read_correctly() {
    let mut input = Vec::new();
    input.extend(frame(b"frame_one"));
    input.extend(frame(b"frame_two"));
    input.extend(frame(b"frame_three"));

    let mut framer = MockFramer::new(input);

    let f1 = framer.read_frame().unwrap().unwrap();
    let f2 = framer.read_frame().unwrap().unwrap();
    let f3 = framer.read_frame().unwrap().unwrap();
    let eof = framer.read_frame();

    assert_eq!(f1, b"frame_one");
    assert_eq!(f2, b"frame_two");
    assert_eq!(f3, b"frame_three");
    assert!(eof.is_none(), "after last frame, must return None");
}

// =============================================================================
// ERROR TYPE TESTS
// =============================================================================

#[test]
fn test_rpc_error_message() {
    let e = RpcError::new("something failed");
    assert_eq!(e.message(), "something failed");
    let display = format!("{}", e);
    assert!(display.contains("something failed"));
    assert!(display.contains("RPC error"));
}

#[test]
fn test_error_code_constants() {
    assert_eq!(coding_adventures_rpc::errors::PARSE_ERROR, -32_700);
    assert_eq!(coding_adventures_rpc::errors::INVALID_REQUEST, -32_600);
    assert_eq!(coding_adventures_rpc::errors::METHOD_NOT_FOUND, -32_601);
    assert_eq!(coding_adventures_rpc::errors::INVALID_PARAMS, -32_602);
    assert_eq!(coding_adventures_rpc::errors::INTERNAL_ERROR, -32_603);
}
