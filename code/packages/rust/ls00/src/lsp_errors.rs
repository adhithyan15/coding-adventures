//! LSP-specific error codes.
//!
//! The JSON-RPC 2.0 specification reserves error codes in the range
//! `[-32768, -32000]`. The LSP specification further reserves `[-32899, -32800]`
//! for LSP protocol-level errors.
//!
//! Standard JSON-RPC error codes (from the `json-rpc` crate):
//! - `-32700` ParseError
//! - `-32600` InvalidRequest
//! - `-32601` MethodNotFound
//! - `-32602` InvalidParams
//! - `-32603` InternalError
//!
//! LSP-specific codes are listed below.
//!
//! Reference: <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes>

/// The server has received a request before the initialize handshake was
/// completed. The server must reject any request (other than initialize)
/// before it has been initialized.
pub const SERVER_NOT_INITIALIZED: i64 = -32002;

/// A generic error code for unknown errors.
pub const UNKNOWN_ERROR_CODE: i64 = -32001;

/// A request failed but not due to a protocol problem. For example, the
/// document requested was not found.
pub const REQUEST_FAILED: i64 = -32803;

/// The server cancelled the request.
pub const SERVER_CANCELLED: i64 = -32802;

/// The document content was modified before the request completed.
/// The client should retry.
pub const CONTENT_MODIFIED: i64 = -32801;

/// The client cancelled the request.
pub const REQUEST_CANCELLED: i64 = -32800;
