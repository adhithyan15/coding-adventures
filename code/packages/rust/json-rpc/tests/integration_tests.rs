//! Integration tests for the `coding-adventures-json-rpc` crate.
//!
//! These tests exercise the full pipeline — message parsing, framing,
//! writing, and server dispatch — using in-memory `Cursor` I/O.
//!
//! ## Test Organisation
//!
//! 1. `errors` — ResponseError constructors and serialisation
//! 2. `message_parsing` — parse_message / message_to_value
//! 3. `writer` — Content-Length framing
//! 4. `reader` — header parsing, EOF, error cases
//! 5. `server` — dispatch, error responses
//! 6. `round_trip` — write then read

use coding_adventures_json_rpc::{
    errors::{self, ResponseError},
    message::{message_to_value, parse_message, Message, Notification, Request, Response},
    MessageReader, MessageWriter, Server,
};
use serde_json::{json, Value};
use std::io::{BufReader, Cursor};
use std::sync::{Arc, Mutex};

// ===========================================================================
// Helpers
// ===========================================================================

/// Build a Content-Length-framed byte string for feeding to a reader.
fn frame(json: &str) -> Vec<u8> {
    let n = json.len();
    format!("Content-Length: {}\r\n\r\n{}", n, json).into_bytes()
}

/// Collect all Content-Length-framed messages from a byte slice and parse them.
fn parse_all_responses(bytes: &[u8]) -> Vec<Message> {
    let mut messages = Vec::new();
    let mut pos = 0;

    while pos < bytes.len() {
        // Find the \r\n\r\n separator.
        let sep = bytes[pos..]
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("expected header separator");

        let header = std::str::from_utf8(&bytes[pos..pos + sep]).unwrap();

        // Extract Content-Length.
        let cl_line = header
            .lines()
            .find(|l| l.starts_with("Content-Length:"))
            .expect("missing Content-Length");
        let n: usize = cl_line
            .trim_start_matches("Content-Length:")
            .trim()
            .parse()
            .unwrap();

        let payload_start = pos + sep + 4;
        let payload = &bytes[payload_start..payload_start + n];
        messages.push(parse_message(payload).expect("failed to parse response"));

        pos = payload_start + n;
    }

    messages
}

// We collect server output via Arc<Mutex<Vec<u8>>> because Server<R,W> does
// not expose its writer after creation — this approach avoids needing to
// unwrap the server's internal writer.
struct VecWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl std::io::Write for VecWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run_server_collect(
    input: Vec<u8>,
    setup: impl FnOnce(&mut Server<BufReader<Cursor<Vec<u8>>>, VecWriter>),
) -> Vec<u8> {
    let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
    let writer = VecWriter {
        buf: Arc::clone(&shared),
    };
    let reader = BufReader::new(Cursor::new(input));
    let mut server = Server::new(reader, writer);
    setup(&mut server);
    server.serve();
    let result = shared.lock().unwrap().clone();
    result
}

// ===========================================================================
// 1. errors — ResponseError
// ===========================================================================

#[test]
fn test_error_constants() {
    assert_eq!(errors::PARSE_ERROR, -32_700);
    assert_eq!(errors::INVALID_REQUEST, -32_600);
    assert_eq!(errors::METHOD_NOT_FOUND, -32_601);
    assert_eq!(errors::INVALID_PARAMS, -32_602);
    assert_eq!(errors::INTERNAL_ERROR, -32_603);
}

#[test]
fn test_response_error_parse_error_no_data() {
    let err = ResponseError::parse_error(None);
    assert_eq!(err.code, errors::PARSE_ERROR);
    assert_eq!(err.message, "Parse error");
    assert!(err.data.is_none());
}

#[test]
fn test_response_error_parse_error_with_data() {
    let err = ResponseError::parse_error(Some(json!("unexpected token")));
    assert_eq!(err.data, Some(json!("unexpected token")));
}

#[test]
fn test_response_error_method_not_found() {
    let err = ResponseError::method_not_found("textDocument/hover");
    assert_eq!(err.code, errors::METHOD_NOT_FOUND);
    assert_eq!(err.data, Some(json!("textDocument/hover")));
}

#[test]
fn test_response_error_serialises_without_null_data() {
    let err = ResponseError::parse_error(None);
    let json = serde_json::to_string(&err).unwrap();
    // data field should NOT appear when it is None.
    assert!(!json.contains("data"));
    assert!(json.contains("-32700"));
}

#[test]
fn test_response_error_display() {
    let err = ResponseError::internal_error(None);
    let s = format!("{}", err);
    assert!(s.contains("-32603"));
    assert!(s.contains("Internal error"));
}

// ===========================================================================
// 2. message_parsing — parse_message
// ===========================================================================

#[test]
fn test_parse_request_minimal() {
    let json = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let msg = parse_message(json).unwrap();
    match msg {
        Message::Request(req) => {
            assert_eq!(req.id, json!(1));
            assert_eq!(req.method, "ping");
            assert!(req.params.is_none());
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_parse_request_with_params() {
    let json = br#"{"jsonrpc":"2.0","id":2,"method":"hover","params":{"line":5}}"#;
    let msg = parse_message(json).unwrap();
    match msg {
        Message::Request(req) => {
            assert_eq!(req.params, Some(json!({"line": 5})));
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_parse_request_string_id() {
    let json = br#"{"jsonrpc":"2.0","id":"abc","method":"ping"}"#;
    let msg = parse_message(json).unwrap();
    match msg {
        Message::Request(req) => assert_eq!(req.id, json!("abc")),
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_parse_notification() {
    let json = br#"{"jsonrpc":"2.0","method":"textDocument/didOpen"}"#;
    let msg = parse_message(json).unwrap();
    match msg {
        Message::Notification(n) => {
            assert_eq!(n.method, "textDocument/didOpen");
            assert!(n.params.is_none());
        }
        _ => panic!("expected Notification"),
    }
}

#[test]
fn test_parse_notification_with_params() {
    let json = br#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    let msg = parse_message(json).unwrap();
    match msg {
        Message::Notification(n) => {
            assert_eq!(n.params, Some(json!({})));
        }
        _ => panic!("expected Notification"),
    }
}

#[test]
fn test_parse_response_success() {
    let json = br#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
    let msg = parse_message(json).unwrap();
    match msg {
        Message::Response(r) => {
            assert_eq!(r.id, json!(1));
            assert!(r.result.is_some());
            assert!(r.error.is_none());
        }
        _ => panic!("expected Response"),
    }
}

#[test]
fn test_parse_response_error() {
    let json = br#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
    let msg = parse_message(json).unwrap();
    match msg {
        Message::Response(r) => {
            let err = r.error.as_ref().unwrap();
            assert_eq!(err.code, errors::METHOD_NOT_FOUND);
        }
        _ => panic!("expected Response"),
    }
}

#[test]
fn test_parse_response_null_id() {
    let json = br#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}"#;
    let msg = parse_message(json).unwrap();
    match msg {
        Message::Response(r) => assert_eq!(r.id, Value::Null),
        _ => panic!("expected Response"),
    }
}

#[test]
fn test_parse_invalid_json_returns_parse_error() {
    let err = parse_message(b"not json!!!").unwrap_err();
    assert_eq!(err.code, errors::PARSE_ERROR);
}

#[test]
fn test_parse_json_array_returns_invalid_request() {
    let err = parse_message(b"[1,2,3]").unwrap_err();
    assert_eq!(err.code, errors::INVALID_REQUEST);
}

#[test]
fn test_parse_json_number_returns_invalid_request() {
    let err = parse_message(b"42").unwrap_err();
    assert_eq!(err.code, errors::INVALID_REQUEST);
}

#[test]
fn test_parse_object_without_method_or_result() {
    let err = parse_message(br#"{"jsonrpc":"2.0","foo":"bar"}"#).unwrap_err();
    assert_eq!(err.code, errors::INVALID_REQUEST);
}

// ===========================================================================
// 3. message_to_value
// ===========================================================================

#[test]
fn test_request_to_value() {
    let req = Request {
        id: json!(5),
        method: "hover".to_string(),
        params: Some(json!({"line": 3})),
    };
    let val = message_to_value(&Message::Request(req));
    assert_eq!(val["jsonrpc"], "2.0");
    assert_eq!(val["id"], 5);
    assert_eq!(val["method"], "hover");
    assert_eq!(val["params"]["line"], 3);
}

#[test]
fn test_response_success_to_value() {
    let resp = Response {
        id: json!(1),
        result: Some(json!({"ok": true})),
        error: None,
    };
    let val = message_to_value(&Message::Response(resp));
    assert_eq!(val["result"]["ok"], true);
    assert!(val.get("error").is_none() || val["error"].is_null());
}

#[test]
fn test_notification_to_value_no_id() {
    let notif = Notification {
        method: "initialized".to_string(),
        params: None,
    };
    let val = message_to_value(&Message::Notification(notif));
    assert_eq!(val["method"], "initialized");
    // Notifications must NOT have an "id" field.
    assert!(val.get("id").is_none());
}

// ===========================================================================
// 4. Writer — Content-Length framing
// ===========================================================================

#[test]
fn test_writer_correct_content_length() {
    // Use into_inner() on the writer to extract the underlying Cursor.
    let mut writer = MessageWriter::new(Cursor::new(Vec::<u8>::new()));
    let msg = Message::Response(Response {
        id: json!(1),
        result: Some(json!(null)),
        error: None,
    });
    writer.write_message(&msg).unwrap();
    let bytes = writer.into_inner().into_inner();
    let text = std::str::from_utf8(&bytes).unwrap();

    // Extract declared Content-Length.
    let first_line = text.lines().next().unwrap();
    let len_str = first_line.trim_start_matches("Content-Length: ");
    let declared: usize = len_str.parse().unwrap();

    // Extract payload (everything after \r\n\r\n).
    let sep = text.find("\r\n\r\n").unwrap();
    let payload = &text[sep + 4..];
    assert_eq!(payload.len(), declared);
}

#[test]
fn test_writer_payload_is_valid_json() {
    let mut writer = MessageWriter::new(Cursor::new(Vec::<u8>::new()));
    let msg = Message::Notification(Notification {
        method: "ping".to_string(),
        params: None,
    });
    writer.write_message(&msg).unwrap();
    let bytes = writer.into_inner().into_inner();
    let text = std::str::from_utf8(&bytes).unwrap();
    let sep = text.find("\r\n\r\n").unwrap();
    let payload = &text[sep + 4..];
    // Should parse as valid JSON.
    let _: Value = serde_json::from_str(payload).expect("payload should be valid JSON");
}

#[test]
fn test_writer_crlf_separator() {
    let mut writer = MessageWriter::new(Cursor::new(Vec::<u8>::new()));
    let msg = Message::Notification(Notification {
        method: "test".to_string(),
        params: None,
    });
    writer.write_message(&msg).unwrap();
    let bytes = writer.into_inner().into_inner();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains("\r\n\r\n"), "header/payload separator must be \\r\\n\\r\\n");
}

#[test]
fn test_write_raw() {
    let mut writer = MessageWriter::new(Cursor::new(Vec::<u8>::new()));
    let json = br#"{"jsonrpc":"2.0","method":"test"}"#;
    writer.write_raw(json).unwrap();
    let bytes = writer.into_inner().into_inner();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.starts_with("Content-Length: "));
    let sep = text.find("\r\n\r\n").unwrap();
    let payload = &text[sep + 4..];
    assert_eq!(payload.as_bytes(), json);
}

// ===========================================================================
// 5. Reader — framed message reading
// ===========================================================================

#[test]
fn test_reader_single_request() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
    let reader = MessageReader::new(BufReader::new(Cursor::new(frame(json))));
    let msg = reader.read_message().unwrap().unwrap();
    assert!(matches!(msg, Message::Request(req) if req.method == "initialize"));
}

#[test]
fn test_reader_back_to_back_messages() {
    let json1 = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let json2 = r#"{"jsonrpc":"2.0","method":"notify"}"#;
    let mut input = frame(json1);
    input.extend(frame(json2));

    let reader = MessageReader::new(BufReader::new(Cursor::new(input)));
    let msg1 = reader.read_message().unwrap().unwrap();
    let msg2 = reader.read_message().unwrap().unwrap();

    assert!(matches!(msg1, Message::Request(_)));
    assert!(matches!(msg2, Message::Notification(_)));
}

#[test]
fn test_reader_returns_none_on_eof() {
    let reader = MessageReader::new(BufReader::new(Cursor::new(Vec::<u8>::new())));
    assert!(reader.read_message().is_none());
}

#[test]
fn test_reader_malformed_json_returns_parse_error() {
    // Valid Content-Length header, but the payload is not JSON.
    let framed = b"Content-Length: 4\r\n\r\nbrok".to_vec();
    let reader = MessageReader::new(BufReader::new(Cursor::new(framed)));
    let result = reader.read_message().unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, errors::PARSE_ERROR);
}

#[test]
fn test_reader_missing_content_length_header() {
    // A blank line but no Content-Length header.
    let framed = b"\r\n".to_vec();
    let reader = MessageReader::new(BufReader::new(Cursor::new(framed)));
    let result = reader.read_message().unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, errors::PARSE_ERROR);
}

#[test]
fn test_reader_valid_json_not_message_returns_invalid_request() {
    let json = "[1, 2, 3]";
    let reader = MessageReader::new(BufReader::new(Cursor::new(frame(json))));
    let result = reader.read_message().unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, errors::INVALID_REQUEST);
}

#[test]
fn test_reader_three_messages_then_eof() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"a"}"#;
    let reader = MessageReader::new(BufReader::new(Cursor::new(frame(json))));
    assert!(reader.read_message().is_some());
    assert!(reader.read_message().is_none());
}

// ===========================================================================
// 6. Server — dispatch
// ===========================================================================

#[test]
fn test_server_dispatches_request_to_handler() {
    let input = frame(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
    let output = run_server_collect(input, |server| {
        server.on_request("ping", |_id, _params| Ok(json!("pong")));
    });

    let messages = parse_all_responses(&output);
    assert_eq!(messages.len(), 1);
    match &messages[0] {
        Message::Response(r) => {
            assert_eq!(r.id, json!(1));
            assert_eq!(r.result, Some(json!("pong")));
            assert!(r.error.is_none());
        }
        _ => panic!("expected Response"),
    }
}

#[test]
fn test_server_unknown_method_returns_method_not_found() {
    let input = frame(r#"{"jsonrpc":"2.0","id":2,"method":"unknown"}"#);
    let output = run_server_collect(input, |_| {});

    let messages = parse_all_responses(&output);
    assert_eq!(messages.len(), 1);
    match &messages[0] {
        Message::Response(r) => {
            let err = r.error.as_ref().unwrap();
            assert_eq!(err.code, errors::METHOD_NOT_FOUND);
        }
        _ => panic!("expected error Response"),
    }
}

#[test]
fn test_server_handler_returns_error_propagated() {
    let input = frame(r#"{"jsonrpc":"2.0","id":3,"method":"fail"}"#);
    let output = run_server_collect(input, |server| {
        server.on_request("fail", |_id, _params| {
            Err(ResponseError::invalid_params(Some(json!("bad params"))))
        });
    });

    let messages = parse_all_responses(&output);
    match &messages[0] {
        Message::Response(r) => {
            let err = r.error.as_ref().unwrap();
            assert_eq!(err.code, errors::INVALID_PARAMS);
        }
        _ => panic!("expected error Response"),
    }
}

#[test]
fn test_server_notification_dispatched_no_response() {
    let notified = Arc::new(Mutex::new(false));
    let notified_clone = Arc::clone(&notified);

    let input = frame(r#"{"jsonrpc":"2.0","method":"notify"}"#);
    let output = run_server_collect(input, move |server| {
        let flag = Arc::clone(&notified_clone);
        server.on_notification("notify", move |_params| {
            *flag.lock().unwrap() = true;
        });
    });

    assert!(output.is_empty(), "notifications must not generate a response");
    assert!(*notified.lock().unwrap(), "notification handler must be called");
}

#[test]
fn test_server_unknown_notification_silent() {
    let input = frame(r#"{"jsonrpc":"2.0","method":"unknown"}"#);
    let output = run_server_collect(input, |_| {});
    assert!(output.is_empty());
}

#[test]
fn test_server_multiple_messages_in_sequence() {
    let parent_notified = Arc::new(Mutex::new(false));
    let clone = Arc::clone(&parent_notified);

    let mut input = frame(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
    input.extend(frame(r#"{"jsonrpc":"2.0","method":"notify"}"#));

    let output = run_server_collect(input, move |server| {
        server.on_request("ping", |_id, _params| Ok(json!("pong")));
        let flag = Arc::clone(&clone);
        server.on_notification("notify", move |_params| {
            *flag.lock().unwrap() = true;
        });
    });

    assert!(*parent_notified.lock().unwrap());
    let messages = parse_all_responses(&output);
    assert_eq!(messages.len(), 1);
    match &messages[0] {
        Message::Response(r) => assert_eq!(r.result, Some(json!("pong"))),
        _ => panic!("expected Response"),
    }
}

#[test]
fn test_server_terminates_on_empty_input() {
    let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
    let writer = VecWriter {
        buf: Arc::clone(&shared),
    };
    let reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
    let mut server = Server::new(reader, writer);
    server.serve(); // should return immediately without panic
}

// ===========================================================================
// 7. Round-trip — write then read
// ===========================================================================

#[test]
fn test_round_trip_request() {
    let mut out = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = MessageWriter::new(&mut out);
        let req = Request {
            id: json!(10),
            method: "hover".to_string(),
            params: Some(json!({"line": 5})),
        };
        writer.write_message(&Message::Request(req)).unwrap();
    }

    let bytes = out.into_inner();
    let reader = MessageReader::new(BufReader::new(Cursor::new(bytes)));
    let msg = reader.read_message().unwrap().unwrap();

    match msg {
        Message::Request(req) => {
            assert_eq!(req.id, json!(10));
            assert_eq!(req.method, "hover");
            assert_eq!(req.params, Some(json!({"line": 5})));
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_round_trip_notification() {
    let mut out = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = MessageWriter::new(&mut out);
        let notif = Notification {
            method: "textDocument/didSave".to_string(),
            params: Some(json!({"uri": "file:///a.bf"})),
        };
        writer.write_message(&Message::Notification(notif)).unwrap();
    }

    let bytes = out.into_inner();
    let reader = MessageReader::new(BufReader::new(Cursor::new(bytes)));
    let msg = reader.read_message().unwrap().unwrap();

    match msg {
        Message::Notification(n) => {
            assert_eq!(n.method, "textDocument/didSave");
        }
        _ => panic!("expected Notification"),
    }
}

#[test]
fn test_round_trip_response() {
    let mut out = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = MessageWriter::new(&mut out);
        let resp = Response {
            id: json!(99),
            result: Some(json!({"capabilities": {}})),
            error: None,
        };
        writer.write_message(&Message::Response(resp)).unwrap();
    }

    let bytes = out.into_inner();
    let reader = MessageReader::new(BufReader::new(Cursor::new(bytes)));
    let msg = reader.read_message().unwrap().unwrap();

    match msg {
        Message::Response(r) => {
            assert_eq!(r.id, json!(99));
            assert_eq!(r.result, Some(json!({"capabilities": {}})));
        }
        _ => panic!("expected Response"),
    }
}

#[test]
fn test_round_trip_back_to_back() {
    let mut out = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = MessageWriter::new(&mut out);
        let msgs = vec![
            Message::Request(Request {
                id: json!(1),
                method: "a".to_string(),
                params: None,
            }),
            Message::Notification(Notification {
                method: "b".to_string(),
                params: None,
            }),
            Message::Response(Response {
                id: json!(1),
                result: Some(json!(42)),
                error: None,
            }),
        ];
        for msg in &msgs {
            writer.write_message(msg).unwrap();
        }
    }

    let bytes = out.into_inner();
    let reader = MessageReader::new(BufReader::new(Cursor::new(bytes)));

    let m1 = reader.read_message().unwrap().unwrap();
    let m2 = reader.read_message().unwrap().unwrap();
    let m3 = reader.read_message().unwrap().unwrap();
    assert!(reader.read_message().is_none());

    assert!(matches!(m1, Message::Request(r) if r.method == "a"));
    assert!(matches!(m2, Message::Notification(n) if n.method == "b"));
    assert!(matches!(m3, Message::Response(r) if r.result == Some(json!(42))));
}
