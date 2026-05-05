//! DAP message framing and JSON serialisation.
//!
//! ## Wire format
//!
//! The Debug Adapter Protocol uses LSP-style framing — each message is a
//! `Content-Length`-prefixed JSON body:
//!
//! ```text
//! Content-Length: <N>\r\n
//! \r\n
//! { "seq": N, "type": "request"|"response"|"event", ... }
//! ```
//!
//! `read_message` reads exactly one such message and returns the JSON body.
//! `write_message` serialises a JSON body and writes the framed envelope.
//!
//! ## Message shapes
//!
//! | type       | required keys                                              |
//! |------------|------------------------------------------------------------|
//! | `request`  | `seq`, `type`, `command`, optional `arguments`             |
//! | `response` | `seq`, `type`, `request_seq`, `success`, `command`, `body?` |
//! | `event`    | `seq`, `type`, `event`, optional `body`                    |
//!
//! See <https://microsoft.github.io/debug-adapter-protocol/specification>.

use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Framing — read_message / write_message
// ---------------------------------------------------------------------------

/// Read one framed DAP message from `reader`.
///
/// Returns the JSON body of the message.  The framing header is consumed
/// and discarded.  An EOF before any header is reported as `Err("eof")`.
pub fn read_message(reader: &mut dyn BufRead) -> Result<Value, String> {
    // ----- 1. Read headers ---------------------------------------------
    //
    // Headers are CRLF-terminated.  We only care about Content-Length
    // (DAP defines no other required header).  A blank line ends the
    // header block.
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)
            .map_err(|e| format!("read header: {e}"))?;
        if n == 0 {
            return Err("eof".into());
        }
        // Strip trailing \r\n (or \n).
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            // Blank line marks end of headers.
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            content_length = Some(
                rest.trim()
                    .parse()
                    .map_err(|e| format!("invalid Content-Length: {e}"))?,
            );
        }
        // Other headers (Content-Type) are ignored.
    }
    let n = content_length.ok_or("missing Content-Length header")?;

    // ----- 2. Read exactly N bytes for the body ------------------------
    let mut buf = vec![0u8; n];
    reader.read_exact(&mut buf).map_err(|e| format!("read body: {e}"))?;

    // ----- 3. Parse JSON -----------------------------------------------
    serde_json::from_slice(&buf).map_err(|e| format!("invalid JSON body: {e}"))
}

/// Write one framed DAP message to `writer`.
///
/// Serialises `body` to JSON, then writes
/// `Content-Length: <N>\r\n\r\n<body>` and flushes.
pub fn write_message(writer: &mut dyn Write, body: &Value) -> Result<(), String> {
    let payload = serde_json::to_vec(body).map_err(|e| format!("serialise: {e}"))?;
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    writer.write_all(header.as_bytes()).map_err(|e| format!("write header: {e}"))?;
    writer.write_all(&payload).map_err(|e| format!("write body: {e}"))?;
    writer.flush().map_err(|e| format!("flush: {e}"))
}

// ---------------------------------------------------------------------------
// Sequence numbers
// ---------------------------------------------------------------------------

/// Monotonic source of `seq` numbers for outgoing messages.
///
/// DAP requires every outgoing message (response or event) to carry a unique
/// monotonically-increasing `seq` integer starting at 1.
#[derive(Debug, Default)]
pub struct SeqCounter {
    next: u64,
}

impl SeqCounter {
    /// Build a fresh counter starting at 1.
    pub fn new() -> Self {
        SeqCounter { next: 1 }
    }
    /// Allocate the next sequence number.
    pub fn next(&mut self) -> u64 {
        let n = self.next;
        self.next = self.next.saturating_add(1);
        n
    }
}

// ---------------------------------------------------------------------------
// Message builders
// ---------------------------------------------------------------------------

/// Build a `response` JSON value for the request `req`.
///
/// `body` is the response payload (use `serde_json::json!({})` for empty).
/// `success` indicates whether the request succeeded; failures carry the
/// supplied `message` string.
pub fn build_response(
    req_seq: u64,
    seq: u64,
    command: &str,
    success: bool,
    message: Option<&str>,
    body: Value,
) -> Value {
    let mut v = serde_json::json!({
        "seq":         seq,
        "type":        "response",
        "request_seq": req_seq,
        "success":     success,
        "command":     command,
        "body":        body,
    });
    if let Some(m) = message {
        v.as_object_mut().unwrap()
            .insert("message".to_string(), Value::String(m.to_string()));
    }
    v
}

/// Build an `event` JSON value.
pub fn build_event(seq: u64, event: &str, body: Value) -> Value {
    serde_json::json!({
        "seq":   seq,
        "type":  "event",
        "event": event,
        "body":  body,
    })
}

// ---------------------------------------------------------------------------
// Request envelope (typed extraction)
// ---------------------------------------------------------------------------

/// Minimal typed view of an incoming DAP request.
///
/// The `arguments` field is left as a raw `Value` because each command has a
/// different argument schema — handlers extract what they need.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DapRequest {
    /// Sequence number assigned by the client.
    pub seq: u64,
    /// Always `"request"`.
    #[serde(rename = "type")]
    pub typ: String,
    /// DAP command name (e.g. `"initialize"`, `"setBreakpoints"`).
    pub command: String,
    /// Command-specific arguments (may be absent).
    #[serde(default)]
    pub arguments: Value,
}

impl DapRequest {
    /// Parse a JSON value into a `DapRequest`.
    pub fn from_value(v: Value) -> Result<Self, String> {
        serde_json::from_value(v).map_err(|e| format!("invalid request: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    #[test]
    fn roundtrip_simple_message() {
        let body = serde_json::json!({"seq": 1, "type": "request", "command": "initialize"});
        let mut buf: Vec<u8> = Vec::new();
        write_message(&mut buf, &body).expect("write ok");

        let mut reader = BufReader::new(Cursor::new(buf));
        let got = read_message(&mut reader).expect("read ok");
        assert_eq!(got, body);
    }

    #[test]
    fn read_two_back_to_back() {
        let a = serde_json::json!({"command": "a"});
        let b = serde_json::json!({"command": "b"});
        let mut buf: Vec<u8> = Vec::new();
        write_message(&mut buf, &a).unwrap();
        write_message(&mut buf, &b).unwrap();

        let mut reader = BufReader::new(Cursor::new(buf));
        assert_eq!(read_message(&mut reader).unwrap(), a);
        assert_eq!(read_message(&mut reader).unwrap(), b);
    }

    #[test]
    fn read_eof_reports_eof() {
        let mut reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
        let err = read_message(&mut reader).unwrap_err();
        assert_eq!(err, "eof");
    }

    #[test]
    fn read_missing_content_length_errors() {
        let raw = b"\r\n{}".to_vec();
        let mut reader = BufReader::new(Cursor::new(raw));
        let err = read_message(&mut reader).unwrap_err();
        assert!(err.contains("missing Content-Length"));
    }

    #[test]
    fn read_invalid_content_length_errors() {
        let raw = b"Content-Length: not-a-number\r\n\r\n{}".to_vec();
        let mut reader = BufReader::new(Cursor::new(raw));
        let err = read_message(&mut reader).unwrap_err();
        assert!(err.contains("invalid Content-Length"));
    }

    #[test]
    fn read_short_body_errors() {
        // Header says 100 bytes but only 2 are present.
        let raw = b"Content-Length: 100\r\n\r\n{}".to_vec();
        let mut reader = BufReader::new(Cursor::new(raw));
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn write_includes_content_length() {
        let body = serde_json::json!({"x": 42});
        let mut buf = Vec::new();
        write_message(&mut buf, &body).unwrap();
        let s = String::from_utf8_lossy(&buf);
        assert!(s.starts_with("Content-Length: "));
        assert!(s.contains("\r\n\r\n"));
        assert!(s.contains("\"x\":42"));
    }

    #[test]
    fn seq_counter_starts_at_one_and_monotonic() {
        let mut c = SeqCounter::new();
        assert_eq!(c.next(), 1);
        assert_eq!(c.next(), 2);
        assert_eq!(c.next(), 3);
    }

    #[test]
    fn build_response_success() {
        let v = build_response(7, 1, "initialize", true, None, serde_json::json!({}));
        assert_eq!(v["request_seq"], 7);
        assert_eq!(v["success"], true);
        assert_eq!(v["command"], "initialize");
        assert_eq!(v["type"], "response");
        assert!(v.get("message").is_none());
    }

    #[test]
    fn build_response_failure_carries_message() {
        let v = build_response(3, 1, "launch", false, Some("boom"),
                               serde_json::json!({}));
        assert_eq!(v["success"], false);
        assert_eq!(v["message"], "boom");
    }

    #[test]
    fn build_event_shape() {
        let v = build_event(5, "stopped", serde_json::json!({"reason": "step"}));
        assert_eq!(v["type"], "event");
        assert_eq!(v["event"], "stopped");
        assert_eq!(v["body"]["reason"], "step");
    }

    #[test]
    fn dap_request_parse() {
        let v = serde_json::json!({
            "seq": 1, "type": "request", "command": "launch",
            "arguments": {"program": "foo"}
        });
        let req = DapRequest::from_value(v).unwrap();
        assert_eq!(req.command, "launch");
        assert_eq!(req.arguments["program"], "foo");
    }

    #[test]
    fn dap_request_no_arguments_field() {
        let v = serde_json::json!({"seq": 2, "type": "request", "command": "disconnect"});
        let req = DapRequest::from_value(v).unwrap();
        assert_eq!(req.command, "disconnect");
        assert!(req.arguments.is_null());
    }
}
