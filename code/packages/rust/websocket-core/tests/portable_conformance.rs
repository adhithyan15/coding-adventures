use serde_json::Value;
use websocket_core::{
    accept_server_request, build_client_request, control_reply, derive_accept, encode_frame,
    validate_client_response, EndpointRole, Frame, FrameDecoder, MessageAssembler, MessageEvent,
    Opcode, WebSocketError,
};

const CASES: &str = include_str!("../../../../specs/fixtures/websocket-core-v1/cases.json");

fn document() -> Value {
    serde_json::from_str(CASES).expect("portable WebSocket fixture must be valid JSON")
}

fn operation_cases(operation: &str) -> Vec<Value> {
    document()["cases"]
        .as_array()
        .expect("fixture cases must be an array")
        .iter()
        .filter(|case| case["operation"] == operation)
        .cloned()
        .collect()
}

fn string<'a>(value: &'a Value, field: &str) -> &'a str {
    value[field]
        .as_str()
        .unwrap_or_else(|| panic!("fixture field {field} must be a string"))
}

fn hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "fixture hex must be byte aligned");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digits = core::str::from_utf8(pair).expect("fixture hex must be ASCII");
            u8::from_str_radix(digits, 16).expect("fixture hex must be lowercase hexadecimal")
        })
        .collect()
}

fn role(value: &Value) -> EndpointRole {
    match value.as_str().expect("role must be a string") {
        "client" => EndpointRole::Client,
        "server" => EndpointRole::Server,
        other => panic!("unknown fixture role {other}"),
    }
}

fn opcode(value: &Value) -> Opcode {
    match value.as_str().expect("opcode must be a string") {
        "continuation" => Opcode::Continuation,
        "text" => Opcode::Text,
        "binary" => Opcode::Binary,
        "close" => Opcode::Close,
        "ping" => Opcode::Ping,
        "pong" => Opcode::Pong,
        other => panic!("unknown fixture opcode {other}"),
    }
}

fn frame(value: &Value) -> Result<Frame, WebSocketError> {
    Frame::new(
        value["fin"].as_bool().expect("frame fin must be boolean"),
        opcode(&value["opcode"]),
        hex(string(value, "payload_hex")),
    )
}

fn error_code(error: WebSocketError) -> &'static str {
    match error {
        WebSocketError::IncompleteHandshake => "incomplete_handshake",
        WebSocketError::HandshakeTooLarge => "handshake_too_large",
        WebSocketError::InvalidHandshake => "invalid_handshake",
        WebSocketError::HeaderInjection => "header_injection",
        WebSocketError::InvalidBase64 => "invalid_base64",
        WebSocketError::ReservedBits => "reserved_bits",
        WebSocketError::InvalidOpcode => "invalid_opcode",
        WebSocketError::MaskDirection => "mask_direction",
        WebSocketError::NonCanonicalLength => "non_canonical_length",
        WebSocketError::FrameTooLarge => "frame_too_large",
        WebSocketError::InvalidControlFrame => "invalid_control_frame",
        WebSocketError::InvalidCloseFrame => "invalid_close_frame",
        WebSocketError::InvalidUtf8 => "invalid_utf8",
        WebSocketError::InvalidFragmentation => "invalid_fragmentation",
        WebSocketError::MessageTooLarge => "message_too_large",
        WebSocketError::ClosedSession => "closed_session",
    }
}

fn assert_error<T>(case: &Value, result: Result<T, WebSocketError>) {
    let id = string(case, "id");
    let expected = &case["expected"];
    let error = match result {
        Ok(_) => panic!("fixture {id} expected an error"),
        Err(error) => error,
    };
    assert_eq!(error_code(error), string(expected, "error"), "fixture {id}");
    assert_eq!(
        error.to_string(),
        string(expected, "diagnostic"),
        "fixture {id}"
    );
}

fn assert_frame(actual: &Frame, expected: &Value, id: &str) {
    assert_eq!(
        actual.is_final(),
        expected["fin"]
            .as_bool()
            .expect("frame fin must be boolean"),
        "fixture {id} final bit"
    );
    assert_eq!(
        actual.opcode(),
        opcode(&expected["opcode"]),
        "fixture {id} opcode"
    );
    assert_eq!(
        actual.payload(),
        hex(string(expected, "payload_hex")),
        "fixture {id} payload"
    );
}

#[test]
fn shared_accept_key_cases_match_rust() {
    for case in operation_cases("derive_accept") {
        let id = string(&case, "id");
        assert_eq!(
            derive_accept(string(&case["input"], "key")),
            string(&case["expected"], "accept"),
            "fixture {id}"
        );
    }
}

#[test]
fn shared_client_request_cases_match_rust() {
    for case in operation_cases("build_client_request") {
        let id = string(&case, "id");
        let input = &case["input"];
        let nonce: [u8; 16] = hex(string(input, "nonce_hex"))
            .try_into()
            .expect("nonce fixture must contain sixteen bytes");
        let result = build_client_request(string(input, "host"), string(input, "target"), nonce);
        if case["expected"].get("error").is_some() {
            assert_error(&case, result);
            continue;
        }
        let handshake = result.unwrap_or_else(|error| panic!("fixture {id}: {error}"));
        assert_eq!(
            handshake.bytes(),
            string(&case["expected"], "request").as_bytes(),
            "fixture {id} request"
        );
        assert_eq!(
            handshake.expected_accept(),
            string(&case["expected"], "expected_accept"),
            "fixture {id} accept"
        );
    }
}

#[test]
fn shared_client_response_cases_match_rust() {
    for case in operation_cases("validate_client_response") {
        let id = string(&case, "id");
        let input = &case["input"];
        let mut bytes = string(input, "response").as_bytes().to_vec();
        bytes.extend(hex(string(input, "tail_hex")));
        let result = validate_client_response(&bytes, string(input, "expected_accept"));
        if case["expected"].get("error").is_some() {
            assert_error(&case, result);
        } else {
            assert_eq!(
                result.unwrap_or_else(|error| panic!("fixture {id}: {error}")),
                case["expected"]["consumed"]
                    .as_u64()
                    .expect("consumed must be unsigned") as usize,
                "fixture {id}"
            );
        }
    }
}

#[test]
fn shared_server_request_cases_match_rust() {
    for case in operation_cases("accept_server_request") {
        let id = string(&case, "id");
        let input = &case["input"];
        let mut bytes = string(input, "request").as_bytes().to_vec();
        bytes.extend(hex(string(input, "tail_hex")));
        let result = accept_server_request(&bytes);
        if case["expected"].get("error").is_some() {
            assert_error(&case, result);
            continue;
        }
        let handshake = result.unwrap_or_else(|error| panic!("fixture {id}: {error}"));
        assert_eq!(
            handshake.consumed(),
            case["expected"]["consumed"]
                .as_u64()
                .expect("consumed must be unsigned") as usize,
            "fixture {id} consumed"
        );
        assert_eq!(
            handshake.response(),
            string(&case["expected"], "response").as_bytes(),
            "fixture {id} response"
        );
    }
}

#[test]
fn shared_frame_encoding_cases_match_rust() {
    for case in operation_cases("encode_frame") {
        let id = string(&case, "id");
        let input = &case["input"];
        let built = frame(&input["frame"]);
        let result = built.and_then(|frame| {
            let mask_key = input["mask_key_hex"].as_str().map(|encoded| {
                hex(encoded)
                    .try_into()
                    .expect("mask fixture must contain four bytes")
            });
            encode_frame(role(&input["role"]), &frame, mask_key)
        });
        if case["expected"].get("error").is_some() {
            assert_error(&case, result);
        } else {
            assert_eq!(
                result.unwrap_or_else(|error| panic!("fixture {id}: {error}")),
                hex(string(&case["expected"], "wire_hex")),
                "fixture {id}"
            );
        }
    }
}

#[test]
fn shared_incremental_decode_cases_match_rust() {
    for case in operation_cases("decode_frames") {
        let id = string(&case, "id");
        let input = &case["input"];
        let mut decoder = FrameDecoder::new(
            role(&input["role"]),
            input["max_frame_payload"]
                .as_u64()
                .expect("max frame payload must be unsigned") as usize,
        );
        let mut actual_frames = Vec::new();
        let mut failure = None;
        for chunk in input["chunks_hex"]
            .as_array()
            .expect("chunks must be an array")
        {
            match decoder.push(&hex(chunk.as_str().expect("chunk must be a string"))) {
                Ok(frames) => actual_frames.extend(frames),
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        if case["expected"].get("error").is_some() {
            let result: Result<(), WebSocketError> = failure.map_or_else(|| Ok(()), Err);
            assert_error(&case, result);
            continue;
        }
        assert!(failure.is_none(), "fixture {id} unexpectedly failed");
        let expected_frames = case["expected"]["frames"]
            .as_array()
            .expect("expected frames must be an array");
        assert_eq!(actual_frames.len(), expected_frames.len(), "fixture {id}");
        for (actual, expected) in actual_frames.iter().zip(expected_frames) {
            assert_frame(actual, expected, id);
        }
        assert_eq!(
            decoder.buffered_len(),
            case["expected"]["buffered_len"]
                .as_u64()
                .expect("buffered length must be unsigned") as usize,
            "fixture {id} buffered length"
        );
    }
}

fn assert_event(actual: &MessageEvent, expected: &Value, id: &str) {
    match (actual, string(expected, "kind")) {
        (MessageEvent::Text(actual), "text") => {
            assert_eq!(actual, string(expected, "text"), "fixture {id}")
        }
        (MessageEvent::Binary(actual), "binary")
        | (MessageEvent::Ping(actual), "ping")
        | (MessageEvent::Pong(actual), "pong") => {
            assert_eq!(
                actual,
                &hex(string(expected, "payload_hex")),
                "fixture {id}"
            )
        }
        (MessageEvent::Close(actual), "close") => {
            let code = expected["code"].as_u64().map(|value| value as u16);
            assert_eq!(actual.code(), code, "fixture {id} close code");
            assert_eq!(
                actual.reason(),
                string(expected, "reason"),
                "fixture {id} close reason"
            );
        }
        (actual, kind) => panic!("fixture {id} expected {kind}, got {actual:?}"),
    }
}

#[test]
fn shared_message_assembly_cases_match_rust() {
    for case in operation_cases("assemble_messages") {
        let id = string(&case, "id");
        let input = &case["input"];
        let mut assembler = MessageAssembler::new(
            input["max_message_payload"]
                .as_u64()
                .expect("max message payload must be unsigned") as usize,
        );
        let mut events = Vec::new();
        let mut replies = Vec::new();
        let mut failure = None;
        for (index, frame_value) in input["frames"]
            .as_array()
            .expect("frames must be an array")
            .iter()
            .enumerate()
        {
            let result = frame(frame_value).and_then(|frame| assembler.push(frame));
            match result {
                Ok(Some(event)) => {
                    replies.push(control_reply(&event));
                    events.push(event);
                }
                Ok(None) => {}
                Err(error) => {
                    failure = Some((index, error));
                    break;
                }
            }
        }
        if case["expected"].get("error").is_some() {
            let (index, error) =
                failure.unwrap_or_else(|| panic!("fixture {id} expected an error"));
            assert_eq!(
                index,
                case["expected"]["error_at"]
                    .as_u64()
                    .expect("error index must be unsigned") as usize,
                "fixture {id} error index"
            );
            assert_error(&case, Err::<(), _>(error));
            continue;
        }
        assert!(failure.is_none(), "fixture {id} unexpectedly failed");
        let expected_events = case["expected"]["events"]
            .as_array()
            .expect("events must be an array");
        assert_eq!(events.len(), expected_events.len(), "fixture {id}");
        for (actual, expected) in events.iter().zip(expected_events) {
            assert_event(actual, expected, id);
        }
        let expected_replies = case["expected"]["replies"]
            .as_array()
            .expect("replies must be an array");
        assert_eq!(replies.len(), expected_replies.len(), "fixture {id}");
        for (actual, expected) in replies.iter().zip(expected_replies) {
            match (actual, expected.is_null()) {
                (None, true) => {}
                (Some(actual), false) => assert_frame(actual, expected, id),
                _ => panic!("fixture {id} control reply mismatch"),
            }
        }
        assert_eq!(
            assembler.is_fragmented(),
            case["expected"]["fragmented"]
                .as_bool()
                .expect("fragmented must be boolean"),
            "fixture {id} fragmented state"
        );
        assert_eq!(
            assembler.is_closed(),
            case["expected"]["closed"]
                .as_bool()
                .expect("closed must be boolean"),
            "fixture {id} closed state"
        );
    }
}
