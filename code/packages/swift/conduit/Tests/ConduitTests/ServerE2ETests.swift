import Testing
import Foundation
@testable import Conduit

#if canImport(Glibc)
import Glibc
#elseif canImport(Darwin)
import Darwin
#endif

// ============================================================================
// End-to-end: build a real Conduit app, serve it on a background thread, and
// drive it with a tiny blocking HTTP/1.0 client over a POSIX socket. A watchdog
// thread stops the server after a deadline so a wedged run fails fast instead of
// hanging. Sockets carry a receive timeout for the same reason.
// ============================================================================

private struct HTTPResult {
    let status: Int
    let headers: [String: String]
    let body: String
}

private enum DemoError: Error { case boom }

/// One blocking HTTP/1.0 request. Returns nil if the connection couldn't be made
/// (e.g. the server isn't accepting yet — callers retry).
private func httpRequest(
    port: UInt16, method: String, path: String, body: String? = nil, contentType: String? = nil
) -> HTTPResult? {
    let fd = socket(AF_INET, SOCK_STREAM_VALUE, 0)
    if fd < 0 { return nil }
    defer { close(fd) }

    var tv = timeval(tv_sec: 5, tv_usec: 0)
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size))

    var addr = sockaddr_in()
    addr.sin_family = sa_family_t(AF_INET)
    addr.sin_port = port.bigEndian
    addr.sin_addr.s_addr = inet_addr("127.0.0.1")

    let connected = withUnsafePointer(to: &addr) { p in
        p.withMemoryRebound(to: sockaddr.self, capacity: 1) { sa in
            connect(fd, sa, socklen_t(MemoryLayout<sockaddr_in>.size))
        }
    }
    if connected != 0 { return nil }

    var req = "\(method) \(path) HTTP/1.0\r\nHost: 127.0.0.1\r\n"
    if let body {
        req += "Content-Type: \(contentType ?? "text/plain")\r\nContent-Length: \(body.utf8.count)\r\n"
    }
    req += "Connection: close\r\n\r\n"
    if let body { req += body }

    let reqBytes = Array(req.utf8)
    _ = reqBytes.withUnsafeBytes { send(fd, $0.baseAddress, $0.count, 0) }

    var data = [UInt8]()
    var buf = [UInt8](repeating: 0, count: 4096)
    while true {
        let n = buf.withUnsafeMutableBytes { recv(fd, $0.baseAddress, $0.count, 0) }
        if n <= 0 { break }
        data.append(contentsOf: buf[0..<n])
    }

    let raw = String(decoding: data, as: UTF8.self)
    guard let sep = raw.range(of: "\r\n\r\n") else { return nil }
    let head = String(raw[..<sep.lowerBound])
    let bodyStr = String(raw[sep.upperBound...])
    let lines = head.components(separatedBy: "\r\n")
    guard let statusLine = lines.first else { return nil }
    let parts = statusLine.components(separatedBy: " ")
    let status = parts.count >= 2 ? Int(parts[1]) ?? 0 : 0
    var headers = [String: String]()
    for line in lines.dropFirst() {
        if let c = line.range(of: ": ") {
            headers[String(line[..<c.lowerBound]).lowercased()] = String(line[c.upperBound...])
        }
    }
    return HTTPResult(status: status, headers: headers, body: bodyStr)
}

/// Connect-retry wrapper: the background reactor may need a moment after bind.
private func request(
    port: UInt16, _ method: String, _ path: String, body: String? = nil, contentType: String? = nil
) -> HTTPResult? {
    for _ in 0..<100 {
        if let r = httpRequest(port: port, method: method, path: path, body: body, contentType: contentType) {
            return r
        }
        usleep(50_000) // 50ms
    }
    return nil
}

@Suite struct ServerE2ETests {
    @Test func fullDispatch() throws {
        let app = Application()
        app.set("app_name", "conduit-test")

        app.before { req in
            if req.path == "/down" { try halt(503, "maintenance") }
            return nil
        }

        // Transforming after-hook: stamps every response with a header. This
        // exercises the full response round-trip (read current → rebuild).
        app.after { _, resp in
            var r = resp
            r.headers.append(("x-served-by", "conduit-swift"))
            return r
        }

        app.get("/") { _ in .html("<h1>OK</h1>") }
        app.get("/hello/:name") { req in .json("{\"hi\":\"\(req.param("name") ?? "")\"}") }
        app.post("/echo") { req in
            .respond(200, req.bodyText,
                     headers: [("content-type", req.contentType.isEmpty ? "text/plain" : req.contentType)])
        }
        app.get("/q") { req in .text("a=\(req.query("a") ?? "")") }
        app.get("/boom") { _ in throw DemoError.boom }
        app.get("/redir") { _ in try .redirect("/", status: 302) }
        app.notFound { req in .text("no route: \(req.path)", status: 404) }
        app.onError { _ in .json("{\"error\":\"server\"}", status: 500) }

        let server = try app.bind(host: "127.0.0.1", port: 0)
        #expect(server.serveBackground())
        let port = server.localPort
        #expect(port > 0)

        // Watchdog: force the server down after a deadline so nothing hangs.
        let watchdog = Thread {
            Thread.sleep(forTimeInterval: 30)
            server.stop()
        }
        watchdog.start()
        defer { server.stop() }

        let root = request(port: port, "GET", "/")
        #expect(root?.status == 200)
        #expect(root?.body == "<h1>OK</h1>")
        #expect(root?.headers["content-type"]?.contains("text/html") == true)
        #expect(root?.headers["x-served-by"] == "conduit-swift") // after-hook stamped it

        let hello = request(port: port, "GET", "/hello/world")
        #expect(hello?.status == 200)
        #expect(hello?.body == "{\"hi\":\"world\"}")

        let echo = request(port: port, "POST", "/echo", body: "ping-pong", contentType: "application/octet-stream")
        #expect(echo?.status == 200)
        #expect(echo?.body == "ping-pong")
        #expect(echo?.headers["content-type"]?.contains("octet-stream") == true)

        let q = request(port: port, "GET", "/q?a=42")
        #expect(q?.status == 200)
        #expect(q?.body == "a=42")

        let down = request(port: port, "GET", "/down")
        #expect(down?.status == 503)
        #expect(down?.body == "maintenance")

        let boom = request(port: port, "GET", "/boom")
        #expect(boom?.status == 500)
        #expect(boom?.body == "{\"error\":\"server\"}")

        let nf = request(port: port, "GET", "/nope")
        #expect(nf?.status == 404)
        #expect(nf?.body == "no route: /nope")

        let redir = request(port: port, "GET", "/redir")
        #expect(redir?.status == 302)
        #expect(redir?.headers["location"] == "/")
    }
}

// SOCK_STREAM is an enum on Darwin and a macro on Glibc; normalize the value.
#if canImport(Darwin)
private let SOCK_STREAM_VALUE = SOCK_STREAM
#else
private let SOCK_STREAM_VALUE = Int32(SOCK_STREAM.rawValue)
#endif
