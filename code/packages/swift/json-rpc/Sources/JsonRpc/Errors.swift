// ============================================================================
// Errors.swift — JSON-RPC 2.0 Error Codes
// ============================================================================
//
// JSON-RPC 2.0 defines a set of reserved integer error codes, analogous to
// HTTP status codes. Each code has a standard meaning that the client can
// switch on without parsing the human-readable message string.
//
// Standard error code table:
//
//   Code     Name              When to use
//   ----     ----              -----------
//   -32700   Parse error       Payload is not valid JSON
//   -32600   Invalid Request   Valid JSON but not a JSON-RPC message
//   -32601   Method not found  Method has no registered handler
//   -32602   Invalid params    Wrong parameter shape for a method
//   -32603   Internal error    Unhandled exception inside a handler
//
// Server-defined errors live in the range [-32099, -32000].
// LSP reserves [-32899, -32800] for protocol-level errors; this package
// intentionally leaves that range alone.
//
// Usage:
//   let code = JsonRpcErrorCodes.methodNotFound
//   // => -32601
//
// ============================================================================

import Foundation

/// Standard JSON-RPC 2.0 error codes.
///
/// These codes are reserved by the JSON-RPC specification and must not be
/// used for application-specific errors. Application errors should use
/// codes outside the reserved range.
public enum JsonRpcErrorCodes {
    /// The bytes are not valid JSON at all.
    /// Example: "{broken json" arrives on stdin.
    public static let parseError = -32_700

    /// Valid JSON, but not a recognisable JSON-RPC message shape.
    /// Example: the payload is a JSON array instead of an object.
    public static let invalidRequest = -32_600

    /// The method name in the Request is not registered on the server.
    /// The server MUST send this error — it must not silently drop the request.
    /// (Silently dropping is only allowed for unknown Notifications.)
    public static let methodNotFound = -32_601

    /// The method was found but the supplied params are wrong.
    /// Handlers should return this when required fields are missing or types
    /// do not match what the method expects.
    public static let invalidParams = -32_602

    /// An unexpected error occurred inside the handler.
    /// Catch-all for server-side bugs. Ask: "was the request well-formed?"
    /// If yes → InternalError. If no → InvalidParams.
    public static let internalError = -32_603
}

// ============================================================================
// JsonRpcError — exception raised by the JSON-RPC transport layer
// ============================================================================
//
// Thrown by MessageReader when framing or parsing fails.
// Carries a numeric `code` in addition to the standard message.
//
// Example:
//   do {
//       let msg = try reader.readMessage()
//   } catch let error as JsonRpcError {
//       print("code=\(error.code) message=\(error.message)")
//   }
//

/// A transport-level error thrown by the JSON-RPC reader or parser.
///
/// This is distinct from `ResponseError` (which is a data structure embedded
/// inside a Response). `JsonRpcError` is thrown when framing or JSON parsing
/// fails — before a proper Response can be constructed.
public struct JsonRpcError: Error, Sendable {
    /// One of the `JsonRpcErrorCodes` constants.
    public let code: Int

    /// Human-readable description of what went wrong.
    public let message: String

    public init(code: Int, message: String) {
        self.code = code
        self.message = message
    }
}
