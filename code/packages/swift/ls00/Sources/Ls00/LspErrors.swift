// ============================================================================
// LspErrors.swift — LSP-specific error codes
// ============================================================================
//
// The JSON-RPC 2.0 specification reserves error codes in the range [-32768, -32000].
// The LSP specification further reserves [-32899, -32800] for LSP protocol-level errors.
//
// Standard JSON-RPC error codes (from the JsonRpc package):
//   -32700  ParseError
//   -32600  InvalidRequest
//   -32601  MethodNotFound
//   -32602  InvalidParams
//   -32603  InternalError
//
// LSP-specific error codes are listed below.
//
// Reference: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes
//
// ============================================================================

import Foundation

/// LSP-specific error codes, distinct from the standard JSON-RPC codes.
public enum LspErrorCodes {
    /// The server has received a request before the initialize handshake completed.
    /// The server must reject any request (other than initialize) before initialization.
    public static let serverNotInitialized = -32002

    /// A generic error code for unknown errors.
    public static let unknownErrorCode = -32001

    // LSP-specific codes in the range [-32899, -32800]:

    /// A request failed but not due to a protocol problem.
    /// For example, the document requested was not found.
    public static let requestFailed = -32803

    /// The server cancelled the request.
    public static let serverCancelled = -32802

    /// The document content was modified before the request completed.
    /// The client should retry.
    public static let contentModified = -32801

    /// The client cancelled the request.
    public static let requestCancelled = -32800
}
