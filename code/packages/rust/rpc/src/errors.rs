//! Standard RPC error codes and the `RpcError` type.
//!
//! ## Why These Codes?
//!
//! RPC error codes play the same role as HTTP status codes: they let the
//! *client* understand *why* the call failed without parsing the human-readable
//! message string. The codes below are inherited from the JSON-RPC 2.0 spec
//! (which in turn borrowed them from XML-RPC) and are used unchanged regardless
//! of the wire format.
//!
//! ```text
//! ┌─────────────────────┬─────────────────┬────────────────────────────────────────────┐
//! │ Code                │ Constant        │ Meaning                                    │
//! ├─────────────────────┼─────────────────┼────────────────────────────────────────────┤
//! │ -32700              │ PARSE_ERROR     │ Framed bytes could not be decoded           │
//! │ -32600              │ INVALID_REQUEST │ Decoded but not a valid RPC message         │
//! │ -32601              │ METHOD_NOT_FOUND│ No handler registered for the method        │
//! │ -32602              │ INVALID_PARAMS  │ Handler rejected the params                 │
//! │ -32603              │ INTERNAL_ERROR  │ Unexpected error inside the handler         │
//! │ -32000 to -32099    │ (reserved)      │ Implementation-defined server errors        │
//! └─────────────────────┴─────────────────┴────────────────────────────────────────────┘
//! ```
//!
//! ## The Two Error Types in This Crate
//!
//! There are two distinct "error" concepts here:
//!
//! 1. **`RpcError`** — a low-level I/O or protocol error that occurred *before*
//!    a request could be dispatched (e.g. the framer could not read a frame,
//!    the codec could not decode a message). This is an infrastructure error.
//!
//! 2. **`RpcErrorResponse`** — a structured error *returned by the application
//!    layer* (the handler). This is what goes over the wire back to the caller.
//!    `RpcErrorResponse` lives in [`crate::message`].
//!
//! The distinction matters: `RpcError` is the Rust `Result::Err` type for
//! framing/codec operations. `RpcErrorResponse<V>` is the application-level
//! error sent to the remote peer.
//!
//! ## Usage
//!
//! ```rust
//! use coding_adventures_rpc::errors::{RpcError, PARSE_ERROR, METHOD_NOT_FOUND};
//!
//! // Infrastructure error — something went wrong reading off the wire:
//! let err = RpcError::new("unexpected EOF while reading frame");
//! println!("{}", err);  // "RPC error: unexpected EOF while reading frame"
//!
//! // Application error code constants:
//! assert_eq!(PARSE_ERROR, -32_700);
//! assert_eq!(METHOD_NOT_FOUND, -32_601);
//! ```

// ---------------------------------------------------------------------------
// Error code constants
// ---------------------------------------------------------------------------
//
// We use i64 (the JSON number type for integers) so these round-trip through
// serde_json without loss. JSON numbers are decoded as i64 by default.

/// The framed bytes could not be decoded by the codec.
///
/// The data was received but it is not parseable as the expected format.
/// Analogous to a syntax error in a programming language.
pub const PARSE_ERROR: i64 = -32_700;

/// The bytes decoded successfully, but the result is not a valid RPC message.
///
/// For example: valid JSON, but no `method` or `result` field.
/// Analogous to a type error — the structure is wrong.
pub const INVALID_REQUEST: i64 = -32_600;

/// No handler is registered for the requested method.
///
/// The server received a well-formed request, but its dispatch table has no
/// entry for the method name. The client asked for something the server does
/// not know how to do.
pub const METHOD_NOT_FOUND: i64 = -32_601;

/// The handler rejected the method parameters as invalid.
///
/// The method is known, but the params are the wrong type, shape, or range.
/// Think of this as an argument validation error.
pub const INVALID_PARAMS: i64 = -32_602;

/// An unexpected error occurred inside the handler.
///
/// The server encountered an internal problem — a bug, a dependency failure,
/// a panicking closure. The client should treat this like an HTTP 500.
pub const INTERNAL_ERROR: i64 = -32_603;

// ---------------------------------------------------------------------------
// RpcError — infrastructure-level error
// ---------------------------------------------------------------------------
//
// This is deliberately simple: just a string message. We do not want to
// create complex error hierarchies here — callers that need structured errors
// can build them on top. The primary purpose of RpcError is to carry a
// human-readable message out of framer/codec operations.

/// A low-level RPC infrastructure error.
///
/// Used as the `Err` variant in framer and codec operations — situations where
/// the RPC machinery itself broke down before or after application-level
/// dispatch. This is *not* sent over the wire; it stays inside the Rust
/// process. Application-level errors that do go over the wire are represented
/// as [`crate::message::RpcErrorResponse`].
///
/// # Examples
///
/// ```rust
/// use coding_adventures_rpc::errors::RpcError;
///
/// let e = RpcError::new("failed to read frame: unexpected EOF");
/// assert_eq!(e.message(), "failed to read frame: unexpected EOF");
///
/// // Implements Display and std::error::Error:
/// println!("{}", e);
/// let _: &dyn std::error::Error = &e;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    /// Human-readable description of what went wrong.
    msg: String,
}

impl RpcError {
    /// Create a new `RpcError` with the given message.
    ///
    /// The message should be concise and actionable, e.g.:
    /// - `"failed to read frame header: unexpected EOF"`
    /// - `"codec decode error: invalid UTF-8 sequence"`
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }

    /// Return the error message as a string slice.
    pub fn message(&self) -> &str {
        &self.msg
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RPC error: {}", self.msg)
    }
}

impl std::error::Error for RpcError {}
