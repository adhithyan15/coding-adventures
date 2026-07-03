import CConduit

/// A read-only view of an incoming HTTP request. Valid only inside the handler
/// it is passed to — its accessors read directly from the native request, whose
/// memory the engine reclaims when the handler returns.
public struct Request {
    private let ptr: OpaquePointer

    init(_ ptr: OpaquePointer) { self.ptr = ptr }

    public var method: String { cstr(conduit_request_method(ptr)) }
    public var path: String { cstr(conduit_request_path(ptr)) }
    public var queryString: String { cstr(conduit_request_query_string(ptr)) }
    public var contentType: String { cstr(conduit_request_content_type(ptr)) }
    public var remoteAddr: String { cstr(conduit_request_remote_addr(ptr)) }

    /// Inside an error handler, the message describing what failed; "" otherwise.
    public var error: String { cstr(conduit_request_error(ptr)) }

    /// The raw request body bytes.
    public var body: [UInt8] {
        var len = 0
        guard let p = conduit_request_body(ptr, &len), len > 0 else { return [] }
        return Array(UnsafeBufferPointer(start: p, count: len))
    }

    /// The request body decoded as UTF-8 (lossy).
    public var bodyText: String { String(decoding: body, as: UTF8.self) }

    /// A named route parameter (`:name`), or nil if absent.
    public func param(_ name: String) -> String? { optCstr(conduit_request_param(ptr, name)) }

    /// A query-string value, or nil if absent.
    public func query(_ name: String) -> String? { optCstr(conduit_request_query(ptr, name)) }

    /// A request header (case-insensitive), or nil if absent.
    public func header(_ name: String) -> String? { optCstr(conduit_request_header(ptr, name)) }
}

private func cstr(_ p: UnsafePointer<CChar>?) -> String {
    guard let p else { return "" }
    return String(cString: p)
}

private func optCstr(_ p: UnsafePointer<CChar>?) -> String? {
    guard let p else { return nil }
    return String(cString: p)
}
