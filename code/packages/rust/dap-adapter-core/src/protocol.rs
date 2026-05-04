//! DAP message framing and JSON serialisation.
//!
//! ## Implementation plan (LS03 PR A)
//!
//! The Debug Adapter Protocol uses HTTP-style Content-Length framing,
//! identical to LSP. Each message:
//!
//! ```text
//! Content-Length: <N>\r\n
//! \r\n
//! { "seq": N, "type": "request"|"response"|"event", ... }
//! ```
//!
//! ### read_message(reader: &mut impl Read) → Result<serde_json::Value, String>
//!
//! 1. Read "Content-Length: " header line, parse N.
//! 2. Read "\r\n" blank line.
//! 3. Read exactly N bytes.
//! 4. Parse as JSON.
//!
//! ### write_message(writer: &mut impl Write, body: &serde_json::Value)
//!
//! 1. Serialise body to JSON string.
//! 2. Write "Content-Length: <len>\r\n\r\n".
//! 3. Write JSON string.
//!
//! ### Key DAP message shapes (serde structs needed):
//!
//! Request:  { seq, type: "request",  command, arguments? }
//! Response: { seq, type: "response", request_seq, success, command, body? }
//! Event:    { seq, type: "event",    event, body? }
//!
//! See https://microsoft.github.io/debug-adapter-protocol/specification
//! for the full schema. Only implement the commands listed in server.rs.

/// Read one framed DAP message from `reader`.
///
/// ## TODO — implement (LS03 PR A)
pub fn read_message(_reader: &mut dyn std::io::Read) -> Result<serde_json::Value, String> {
    Err("not yet implemented".into())
}

/// Write one framed DAP message to `writer`.
///
/// ## TODO — implement (LS03 PR A)
pub fn write_message(
    _writer: &mut dyn std::io::Write,
    _body: &serde_json::Value,
) -> Result<(), String> {
    Err("not yet implemented".into())
}
