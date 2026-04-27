// ============================================================================
// Server.swift — JSON-RPC 2.0 Server with method dispatch
// ============================================================================
//
// The Server combines a MessageReader and MessageWriter with a
// method dispatch table. It drives the read-dispatch-write loop,
// isolating the application from all framing, parsing, and
// error-handling details.
//
// Lifecycle:
//   1. Create a Server with input Data and an OutputTarget.
//   2. Register handlers with `onRequest` and `onNotification`.
//   3. Call `serve()` — it blocks until the stream is exhausted.
//
// Dispatch rules:
//
//   Request received:
//     handler found    → call it; send result or ResponseError as Response
//     no handler       → send -32601 (Method not found) Response
//     handler raises   → send -32603 (Internal error) Response
//
//   Notification received:
//     handler found    → call it; send NOTHING
//     no handler       → silently ignore
//
//   Response received:
//     discarded in server-only mode
//
// Concurrency:
//   `serve()` is single-threaded — it processes one message at a time.
//   This is correct for LSP, where editors send one request and wait
//   before sending the next.
//
// Example:
//   let server = Server(inputData: data, output: output)
//   server.onRequest("initialize") { id, params in
//       return (["capabilities": [:]], nil)
//   }
//   server.onNotification("textDocument/didOpen") { params in }
//   server.serve()
//
// ============================================================================

import Foundation

/// Signature for a JSON-RPC request handler.
///
/// The handler receives the request id and params, and must return either:
/// - `(result, nil)` — any JSON-serializable value as a success result.
/// - `(nil, ResponseError)` — an error to send back to the client.
public typealias RequestHandler = (_ id: Any, _ params: Any?) -> (Any?, ResponseError?)

/// Signature for a JSON-RPC notification handler.
///
/// The handler receives the notification params. No return value is needed
/// because notifications never get a response.
public typealias NotificationHandler = (_ params: Any?) -> Void

/// JSON-RPC 2.0 server combining a reader + writer with method dispatch.
///
/// Typical usage:
///
///   let server = Server(inputData: stdinBytes, output: FileHandleOutput(.standardOutput))
///   server.onRequest("initialize") { id, params in
///       return (["capabilities": [:]], nil)
///   }
///   server.serve()
public final class Server: @unchecked Sendable {
    private let reader: MessageReader
    private let writer: MessageWriter
    private var requestHandlers: [String: RequestHandler] = [:]
    private var notificationHandlers: [String: NotificationHandler] = [:]

    /// Create a server that reads from input data and writes to the given output.
    ///
    /// - Parameters:
    ///   - inputData: The raw bytes containing incoming JSON-RPC messages.
    ///   - output: The target for outgoing framed messages.
    public init(inputData: Data, output: OutputTarget) {
        self.reader = MessageReader(inputData)
        self.writer = MessageWriter(output)
    }

    // ----------------------------------------------------------------
    // onRequest — register a handler for a Request method
    // ----------------------------------------------------------------
    //
    // The closure receives (id, params) and must return either:
    //   - (result, nil) for a successful response
    //   - (nil, ResponseError) for an error response
    //
    // Returns `self` for chaining.
    //

    /// Register a handler for a JSON-RPC request method.
    ///
    /// - Parameters:
    ///   - method: The method name to handle.
    ///   - handler: The closure called when a request with this method arrives.
    /// - Returns: Self, for chaining.
    @discardableResult
    public func onRequest(_ method: String, handler: @escaping RequestHandler) -> Self {
        requestHandlers[method] = handler
        return self
    }

    // ----------------------------------------------------------------
    // onNotification — register a handler for a Notification method
    // ----------------------------------------------------------------
    //
    // The closure receives (params) and returns nothing. Even if it
    // raises, no response is sent.
    //
    // Returns `self` for chaining.
    //

    /// Register a handler for a JSON-RPC notification method.
    ///
    /// - Parameters:
    ///   - method: The method name to handle.
    ///   - handler: The closure called when a notification arrives.
    /// - Returns: Self, for chaining.
    @discardableResult
    public func onNotification(_ method: String, handler: @escaping NotificationHandler) -> Self {
        notificationHandlers[method] = handler
        return self
    }

    // ----------------------------------------------------------------
    // serve — run the read-dispatch-write loop
    // ----------------------------------------------------------------
    //
    // Reads messages until the input is exhausted. Processes one
    // message per iteration.
    //

    /// Start the blocking read-dispatch-write loop.
    ///
    /// Reads messages until EOF (input exhausted). Each message is
    /// dispatched to the appropriate handler. Errors during framing
    /// or parsing produce error responses.
    public func serve() {
        while true {
            let msg: JsonRpcMessage?
            do {
                msg = try reader.readMessage()
            } catch let error as JsonRpcError {
                sendError(id: nil, code: error.code, message: error.message)
                continue
            } catch {
                sendError(id: nil, code: JsonRpcErrorCodes.internalError, message: "Internal error: \(error)")
                continue
            }

            guard let msg else { break } // clean EOF

            dispatch(msg)
        }
    }

    // ----------------------------------------------------------------
    // Private dispatch and handler methods
    // ----------------------------------------------------------------

    private func dispatch(_ msg: JsonRpcMessage) {
        switch msg {
        case .request(let request):
            handleRequest(request)
        case .notification(let notification):
            handleNotification(notification)
        case .response:
            // Server mode: discard incoming Responses.
            break
        }
    }

    private func handleRequest(_ request: Request) {
        guard let handler = requestHandlers[request.method] else {
            sendError(
                id: request.id.value,
                code: JsonRpcErrorCodes.methodNotFound,
                message: "Method not found: \(request.method)"
            )
            return
        }

        let (result, error) = handler(request.id.value, request.params?.value)

        if let error {
            let resp = Response(
                id: AnySendable(request.id.value),
                error: error
            )
            try? writer.writeMessage(.response(resp))
        } else {
            let resp = Response(
                id: AnySendable(request.id.value),
                result: result.map { AnySendable($0) }
            )
            try? writer.writeMessage(.response(resp))
        }
    }

    private func handleNotification(_ notification: Notification) {
        guard let handler = notificationHandlers[notification.method] else {
            // Per spec: silently ignore unknown notifications.
            return
        }

        // Notifications are fire-and-forget. Errors are swallowed.
        handler(notification.params?.value)
    }

    private func sendError(id: Any?, code: Int, message: String) {
        let error = ResponseError(code: code, message: message)
        let resp = Response(
            id: id.map { AnySendable($0) },
            error: error
        )
        try? writer.writeMessage(.response(resp))
    }
}
