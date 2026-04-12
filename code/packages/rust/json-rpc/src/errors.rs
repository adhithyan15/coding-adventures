//! Standard JSON-RPC 2.0 error codes and the `ResponseError` type.
//!
//! ## The Error Code Table
//!
//! JSON-RPC 2.0 reserves a specific integer range for protocol-level errors.
//! These codes are analogous to HTTP status codes — they tell the client *why*
//! the request failed without requiring the client to parse the error message.
//!
//! | Code              | Constant           | Meaning                                   |
//! |-------------------|--------------------|-------------------------------------------|
//! | `-32700`          | `PARSE_ERROR`      | The framed bytes are not valid JSON       |
//! | `-32600`          | `INVALID_REQUEST`  | Valid JSON but not a valid request object |
//! | `-32601`          | `METHOD_NOT_FOUND` | No handler registered for the method     |
//! | `-32602`          | `INVALID_PARAMS`   | Handler rejected the params as malformed  |
//! | `-32603`          | `INTERNAL_ERROR`   | An unexpected error inside the handler   |
//! | `-32000`–`-32099` | *(reserved)*       | Implementation-defined server errors      |
//!
//! ## LSP Reserved Range
//!
//! The Language Server Protocol reserves `-32899` to `-32800` for its own
//! error codes (e.g. `ContentModified = -32801`). This JSON-RPC layer does
//! NOT use that range — it belongs to the LSP layer above.
//!
//! ## Usage
//!
//! ```rust
//! use coding_adventures_json_rpc::errors::{self, ResponseError};
//!
//! // Build from a constructor:
//! let err = ResponseError::method_not_found("textDocument/hover");
//! assert_eq!(err.code, errors::METHOD_NOT_FOUND);
//!
//! // Build manually:
//! let err = ResponseError {
//!     code: errors::INTERNAL_ERROR,
//!     message: "handler panicked".to_string(),
//!     data: Some(serde_json::json!("stack trace here")),
//! };
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Error code constants
// ---------------------------------------------------------------------------
//
// We define these as `i64` (the JSON number type) rather than i32, because
// serde_json deserializes JSON integers as i64 by default.

/// The framed bytes are not valid JSON.
pub const PARSE_ERROR: i64 = -32_700;

/// Valid JSON, but not a valid JSON-RPC Request object.
pub const INVALID_REQUEST: i64 = -32_600;

/// No handler is registered for the requested method.
pub const METHOD_NOT_FOUND: i64 = -32_601;

/// The handler rejected the method parameters as invalid.
pub const INVALID_PARAMS: i64 = -32_602;

/// An unexpected error occurred inside the handler.
pub const INTERNAL_ERROR: i64 = -32_603;

// ---------------------------------------------------------------------------
// ResponseError struct
// ---------------------------------------------------------------------------
//
// This is the value that goes inside `"error"` in a Response message.
//
// We derive Serialize and Deserialize so serde_json can round-trip it.
// The `skip_serializing_if` attribute omits `data` from the JSON output when
// it is None — keeping the wire format lean for the common case.

/// A JSON-RPC error object, carried inside a `Response` when the handler fails.
///
/// # JSON Representation
///
/// ```json
/// {
///   "code": -32601,
///   "message": "Method not found",
///   "data": "textDocument/hover is not registered"
/// }
/// ```
///
/// The `data` field is optional. If absent, it is not emitted in the JSON output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseError {
    /// Integer error code from the reserved table above.
    pub code: i64,

    /// Short, human-readable description. Should be stable across versions
    /// (clients may display it directly).
    pub message: String,

    /// Optional additional context — can be any JSON value.
    /// Useful for debugging: stack traces, bad input snippets, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ResponseError {
    // -----------------------------------------------------------------------
    // Constructors for the standard error codes
    // -----------------------------------------------------------------------

    /// Create a `Parse error (-32700)` with optional data.
    ///
    /// ```rust
    /// use coding_adventures_json_rpc::errors::{ResponseError, PARSE_ERROR};
    /// let err = ResponseError::parse_error(None);
    /// assert_eq!(err.code, PARSE_ERROR);
    /// ```
    pub fn parse_error(data: Option<Value>) -> Self {
        Self {
            code: PARSE_ERROR,
            message: "Parse error".to_string(),
            data,
        }
    }

    /// Create an `Invalid Request (-32600)` with optional data.
    pub fn invalid_request(data: Option<Value>) -> Self {
        Self {
            code: INVALID_REQUEST,
            message: "Invalid Request".to_string(),
            data,
        }
    }

    /// Create a `Method not found (-32601)`.
    ///
    /// `method` is the unrecognised method name; it is included as `data`
    /// so the client can display a helpful error message.
    ///
    /// ```rust
    /// use coding_adventures_json_rpc::errors::ResponseError;
    /// let err = ResponseError::method_not_found("textDocument/hover");
    /// assert_eq!(err.data, Some(serde_json::json!("textDocument/hover")));
    /// ```
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: METHOD_NOT_FOUND,
            message: "Method not found".to_string(),
            data: Some(Value::String(method.to_string())),
        }
    }

    /// Create an `Invalid params (-32602)` with optional data.
    pub fn invalid_params(data: Option<Value>) -> Self {
        Self {
            code: INVALID_PARAMS,
            message: "Invalid params".to_string(),
            data,
        }
    }

    /// Create an `Internal error (-32603)` with optional data.
    ///
    /// Use this when a handler panics or returns an unexpected error.
    pub fn internal_error(data: Option<Value>) -> Self {
        Self {
            code: INTERNAL_ERROR,
            message: "Internal error".to_string(),
            data,
        }
    }
}

// ---------------------------------------------------------------------------
// std::error::Error impl
// ---------------------------------------------------------------------------
//
// Implementing the standard Error trait lets ResponseError be used with the
// `?` operator and stored in `Box<dyn Error>` error chains.

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for ResponseError {}
