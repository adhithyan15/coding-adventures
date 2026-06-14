import CConduit

/// An HTTP response: a status code, ordered headers, and a body.
///
/// Build one directly, or use the Sinatra-style helpers `html`/`json`/`text`/
/// `respond`/`redirect`. Returning a `Response` from a handler hands it to the
/// engine; the framework clamps the status to 100–599 and drops any header whose
/// name/value carries CR/LF/control bytes (response-splitting defense).
public struct Response: Sendable {
    public var status: Int
    public var headers: [(name: String, value: String)]
    public var body: [UInt8]

    public init(
        status: Int = 200,
        body: [UInt8] = [],
        headers: [(name: String, value: String)] = []
    ) {
        self.status = status
        self.body = body
        self.headers = headers
    }

    public init(status: Int = 200, text: String, headers: [(name: String, value: String)] = []) {
        self.init(status: status, body: Array(text.utf8), headers: headers)
    }

    /// The body decoded as UTF-8 (lossy).
    public var bodyText: String { String(decoding: body, as: UTF8.self) }

    // ── Sinatra-style helpers ────────────────────────────────────────────────

    public static func html(_ body: String, status: Int = 200) -> Response {
        Response(status: status, text: body, headers: [("content-type", "text/html; charset=utf-8")])
    }

    public static func json(_ body: String, status: Int = 200) -> Response {
        Response(status: status, text: body, headers: [("content-type", "application/json")])
    }

    public static func text(_ body: String, status: Int = 200) -> Response {
        Response(status: status, text: body, headers: [("content-type", "text/plain; charset=utf-8")])
    }

    public static func respond(
        _ status: Int,
        _ body: String = "",
        headers: [(name: String, value: String)] = []
    ) -> Response {
        Response(status: status, text: body, headers: headers)
    }

    /// A redirect (default 302). Throws if the location contains CR or LF, which
    /// would enable response splitting via the `Location` header.
    public static func redirect(_ location: String, status: Int = 302) throws -> Response {
        // Scan unicode scalars, not Characters: in Swift "\r\n" is a single
        // extended grapheme cluster, so `contains("\r")` would miss a CRLF.
        if location.unicodeScalars.contains(where: { $0 == "\r" || $0 == "\n" }) {
            throw ConduitError.invalidRedirect(location)
        }
        return Response(status: status, headers: [("location", location)])
    }

    // ── C ABI bridging ───────────────────────────────────────────────────────

    /// Build an owned `ConduitResponse*` for handing back to the engine. The
    /// engine takes ownership of the returned pointer.
    func toC() -> OpaquePointer? {
        let clamped = UInt16(min(max(status, 100), 599))
        let resp: OpaquePointer? = body.withUnsafeBufferPointer { buf in
            conduit_response_new(clamped, buf.baseAddress, buf.count)
        }
        guard let resp else { return nil }
        for (name, value) in headers {
            conduit_response_set_header(resp, name, value)
        }
        return resp
    }

    /// Read a response back out of a `ConduitResponse*` (used by after-hooks).
    /// Does not free `ptr`.
    init(reading ptr: OpaquePointer) {
        self.status = Int(conduit_response_status(ptr))

        var len = 0
        if let p = conduit_response_body(ptr, &len), len > 0 {
            self.body = Array(UnsafeBufferPointer(start: p, count: len))
        } else {
            self.body = []
        }

        var hs: [(name: String, value: String)] = []
        let count = conduit_response_header_count(ptr)
        var i = 0
        while i < count {
            if let n = conduit_response_header_name(ptr, i),
               let v = conduit_response_header_value(ptr, i) {
                hs.append((String(cString: n), String(cString: v)))
            }
            i += 1
        }
        self.headers = hs
    }
}
