// ============================================================================
// Message.swift — JSON-RPC 2.0 Message Types
// ============================================================================
//
// JSON-RPC 2.0 defines four message shapes. All carry "jsonrpc": "2.0".
// The shape is determined by which fields are present:
//
//   ┌──────────────┬──────┬──────────┬────────┬───────┐
//   │ Shape        │  id  │  method  │ result │ error │
//   ├──────────────┼──────┼──────────┼────────┼───────┤
//   │ Request      │  yes │   yes    │   —    │   —   │
//   │ Notification │   —  │   yes    │   —    │   —   │
//   │ Response OK  │  yes │    —     │  yes   │   —   │
//   │ Response Err │  yes │    —     │   —    │  yes  │
//   └──────────────┴──────┴──────────┴────────┴───────┘
//
// Public API:
//   Request(id:, method:, params:)
//   Notification(method:, params:)
//   Response(id:, result:, error:)
//   ResponseError(code:, message:, data:)
//
//   parseMessage(_:) → Message  (throws JsonRpcError on invalid input)
//   messageToMap(_:) → [String: Any]  (wire-format dictionary for JSONSerialization)
//
// ============================================================================

import Foundation

// ============================================================================
// ResponseError — the structured error inside an error Response
// ============================================================================
//
// Example wire object:
//   { "code": -32601, "message": "Method not found", "data": "..." }
//
// `code`    — integer; see JsonRpcErrorCodes for standard values
// `message` — short human-readable description
// `data`    — optional additional context (any JSON value)
//

/// The structured error object embedded inside a failed JSON-RPC Response.
///
/// This is a data structure, not a thrown exception. It rides inside a
/// `Response` when the server needs to report an error to the client.
/// For transport-level errors (framing, JSON parsing), see `JsonRpcError`.
public struct ResponseError: Sendable, Equatable, Error {
    /// Integer error code; see `JsonRpcErrorCodes` for standard values.
    public let code: Int

    /// Short human-readable description of the error.
    public let message: String

    /// Optional additional context. Any JSON-serializable value.
    public let data: AnySendable?

    public init(code: Int, message: String, data: AnySendable? = nil) {
        self.code = code
        self.message = message
        self.data = data
    }

    public static func == (lhs: ResponseError, rhs: ResponseError) -> Bool {
        return lhs.code == rhs.code && lhs.message == rhs.message
    }
}

// ============================================================================
// AnySendable — type-erased Sendable wrapper for "any JSON value"
// ============================================================================
//
// Swift 6 concurrency requires all values crossing concurrency boundaries to
// conform to Sendable. Since JSON values can be Any, we wrap them in a
// Sendable struct that holds the underlying value.
//

/// A type-erased wrapper that satisfies Sendable for arbitrary JSON values.
///
/// JSON-RPC messages can carry arbitrary data (params, result, error data).
/// Swift 6 requires Sendable conformance for concurrent code. This wrapper
/// allows us to store `Any` values while satisfying the type system.
public struct AnySendable: @unchecked Sendable, Equatable {
    /// The wrapped value. Can be any JSON-representable type:
    /// String, Int, Double, Bool, [Any], [String: Any], or nil.
    public let value: Any

    public init(_ value: Any) {
        self.value = value
    }

    public static func == (lhs: AnySendable, rhs: AnySendable) -> Bool {
        // Best-effort equality for common JSON types.
        switch (lhs.value, rhs.value) {
        case (let a as String, let b as String): return a == b
        case (let a as Int, let b as Int): return a == b
        case (let a as Double, let b as Double): return a == b
        case (let a as Bool, let b as Bool): return a == b
        case (is NSNull, is NSNull): return true
        default: return false
        }
    }
}

// ============================================================================
// Request — a call that expects a Response
// ============================================================================
//
// Example wire object:
//   { "jsonrpc":"2.0", "id":1, "method":"textDocument/hover",
//     "params":{ "position":{"line":0,"character":3} } }
//
// `id`     — String or Int; ties the Response back to this call
// `method` — String; the procedure name
// `params` — optional dictionary or array
//

/// A JSON-RPC 2.0 request message that expects a response.
public struct Request: Sendable, Equatable {
    /// Ties the response back to this request. String or Int.
    public let id: AnySendable

    /// The procedure name (e.g. "textDocument/hover").
    public let method: String

    /// Optional parameters. Typically a dictionary.
    public let params: AnySendable?

    public init(id: AnySendable, method: String, params: AnySendable? = nil) {
        self.id = id
        self.method = method
        self.params = params
    }
}

// ============================================================================
// Notification — a one-way message with no Response
// ============================================================================
//
// Example wire object:
//   { "jsonrpc":"2.0", "method":"textDocument/didOpen",
//     "params":{ "textDocument":{"uri":"file:///main.bf"} } }
//
// The server MUST NOT send a response, even on error.
//

/// A JSON-RPC 2.0 notification — fire-and-forget, no response expected.
public struct Notification: Sendable, Equatable {
    /// The procedure name.
    public let method: String

    /// Optional parameters.
    public let params: AnySendable?

    public init(method: String, params: AnySendable? = nil) {
        self.method = method
        self.params = params
    }
}

// ============================================================================
// Response — the server's reply to a Request
// ============================================================================
//
// Exactly one of `result` or `error` is present.
//
// `id`     — matches the originating Request; nil only when the
//            server cannot determine the request id
// `result` — any value on success
// `error`  — ResponseError on failure
//

/// A JSON-RPC 2.0 response to a prior request.
public struct Response: Sendable {
    /// Matches the originating Request's id. Nil only when the server
    /// cannot determine the request id (e.g. parse error).
    public let id: AnySendable?

    /// The success result (any JSON-serializable value).
    public let result: AnySendable?

    /// The error object, if the request failed.
    public let error: ResponseError?

    public init(id: AnySendable?, result: AnySendable? = nil, error: ResponseError? = nil) {
        self.id = id
        self.result = result
        self.error = error
    }
}

// ============================================================================
// parseMessage — raw Dictionary → typed message
// ============================================================================
//
// Converts a dictionary (typically from JSONSerialization) into one of the
// typed message structs. Throws JsonRpcError(-32600) for unrecognised shapes.
//
// Recognition rules (mirrors the table at the top of the file):
//   has "id" AND "method"               → Request
//   has "method" but no "id"            → Notification
//   has "id" AND ("result" OR "error")  → Response
//   anything else                        → throw Invalid Request
//

/// Discriminated union of all JSON-RPC 2.0 message types.
///
/// Handlers receive this enum and switch on the case to determine
/// the message type. The associated values carry the typed payload.
public enum JsonRpcMessage: Sendable {
    case request(Request)
    case notification(Notification)
    case response(Response)
}

/// Parse a raw JSON dictionary into a typed JSON-RPC message.
///
/// - Parameter data: A dictionary from `JSONSerialization.jsonObject`.
/// - Returns: A `JsonRpcMessage` wrapping the typed message.
/// - Throws: `JsonRpcError` with code -32600 for invalid message shapes.
///
/// Example:
///   let dict = try JSONSerialization.jsonObject(with: bytes) as! [String: Any]
///   let msg = try parseMessage(dict)
public func parseMessage(_ data: [String: Any]) throws -> JsonRpcMessage {
    let hasId = data["id"] != nil && !(data["id"] is NSNull)
    let hasMethod: Bool = {
        if let m = data["method"] as? String, !m.isEmpty { return true }
        return false
    }()
    let hasResult = data.keys.contains("result")
    let hasError = data.keys.contains("error")

    if hasId && hasMethod {
        // ---- Request ----
        let id = normalizeId(data["id"]!)
        let method = data["method"] as! String
        let params: AnySendable? = data["params"].map { AnySendable($0) }
        return .request(Request(id: AnySendable(id), method: method, params: params))
    }

    if hasMethod && !hasId {
        // ---- Notification ----
        let method = data["method"] as! String
        let params: AnySendable? = data["params"].map { AnySendable($0) }
        return .notification(Notification(method: method, params: params))
    }

    if hasId && (hasResult || hasError) {
        // ---- Response ----
        let rawId = data["id"]
        let id: AnySendable? = (rawId is NSNull) ? nil : rawId.map { AnySendable(normalizeId($0)) }

        var errorObj: ResponseError? = nil
        if hasError, let errDict = data["error"] as? [String: Any] {
            guard let code = errDict["code"] as? Int ?? (errDict["code"] as? Double).map({ Int($0) }),
                  let message = errDict["message"] as? String else {
                throw JsonRpcError(
                    code: JsonRpcErrorCodes.invalidRequest,
                    message: "Invalid Request: error must have integer code and string message"
                )
            }
            let errData: AnySendable? = errDict["data"].map { AnySendable($0) }
            errorObj = ResponseError(code: code, message: message, data: errData)
        }

        let result: AnySendable? = hasResult ? AnySendable(data["result"] as Any) : nil
        return .response(Response(id: id, result: errorObj == nil ? result : nil, error: errorObj))
    }

    // Also handle response with null id (error responses from parse failures)
    if (hasResult || hasError) {
        let id: AnySendable? = nil
        var errorObj: ResponseError? = nil
        if hasError, let errDict = data["error"] as? [String: Any] {
            let code = errDict["code"] as? Int ?? (errDict["code"] as? Double).map({ Int($0) }) ?? 0
            let message = errDict["message"] as? String ?? ""
            let errData: AnySendable? = errDict["data"].map { AnySendable($0) }
            errorObj = ResponseError(code: code, message: message, data: errData)
        }
        let result: AnySendable? = hasResult ? AnySendable(data["result"] as Any) : nil
        return .response(Response(id: id, result: errorObj == nil ? result : nil, error: errorObj))
    }

    throw JsonRpcError(
        code: JsonRpcErrorCodes.invalidRequest,
        message: "Invalid Request: unrecognised message shape"
    )
}

/// Convert a typed JSON-RPC message back to a wire-format dictionary.
///
/// Adds "jsonrpc": "2.0" to every message. The resulting dictionary is
/// suitable for `JSONSerialization.data(withJSONObject:)`.
///
/// Example:
///   let req = Request(id: AnySendable(1), method: "ping")
///   let dict = messageToMap(.request(req))
///   // => ["jsonrpc": "2.0", "id": 1, "method": "ping"]
public func messageToMap(_ msg: JsonRpcMessage) -> [String: Any] {
    switch msg {
    case .request(let req):
        var dict: [String: Any] = [
            "jsonrpc": "2.0",
            "id": req.id.value,
            "method": req.method,
        ]
        if let params = req.params {
            dict["params"] = params.value
        }
        return dict

    case .notification(let notif):
        var dict: [String: Any] = [
            "jsonrpc": "2.0",
            "method": notif.method,
        ]
        if let params = notif.params {
            dict["params"] = params.value
        }
        return dict

    case .response(let resp):
        var dict: [String: Any] = ["jsonrpc": "2.0"]
        if let id = resp.id {
            dict["id"] = id.value
        } else {
            dict["id"] = NSNull()
        }
        if let error = resp.error {
            var errDict: [String: Any] = [
                "code": error.code,
                "message": error.message,
            ]
            if let data = error.data {
                errDict["data"] = data.value
            }
            dict["error"] = errDict
        } else {
            dict["result"] = resp.result?.value ?? NSNull()
        }
        return dict
    }
}

// ============================================================================
// Helper: normalizeId
// ============================================================================
//
// JSON numbers decoded by JSONSerialization arrive as NSNumber. If the number
// is a whole number, normalize it to Int for cleaner comparisons.
//

/// Normalizes a JSON id value to Int if it is a whole-number Double.
///
/// JSONSerialization decodes all JSON numbers as NSNumber. For ids that
/// are integers (e.g. "id": 1), we convert the underlying Double to Int
/// so that comparisons work naturally.
func normalizeId(_ id: Any) -> Any {
    if let d = id as? Double {
        if d == d.rounded(.towardZero) && !d.isNaN && !d.isInfinite {
            return Int(d)
        }
        return d
    }
    // NSNumber check for integers that come through as Int already
    if let n = id as? Int {
        return n
    }
    return id
}
