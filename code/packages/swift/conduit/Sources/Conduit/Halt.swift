import CConduit

/// Errors the framework itself raises.
public enum ConduitError: Error {
    /// A redirect location contained CR or LF (response-splitting guard).
    case invalidRedirect(String)
    /// `bind` was called on an application that was already bound/consumed.
    case alreadyBound
    /// The native bind failed; carries the engine's error message.
    case bindFailed(String)
}

/// Thrown by `halt(...)` for a Sinatra-style non-local exit from a handler. The
/// trampoline catches it and returns the carried response.
public struct ConduitHalt: Error {
    public let response: Response

    public init(_ status: Int, _ body: String = "") {
        self.response = Response.text(body, status: status)
    }

    public init(response: Response) {
        self.response = response
    }
}

/// Immediately stop handling the current request and return `status`/`body`.
///
///     app.before { req in
///         if req.path == "/down" { try halt(503, "maintenance") }
///         return nil
///     }
public func halt(_ status: Int, _ body: String = "") throws -> Never {
    throw ConduitHalt(status, body)
}
