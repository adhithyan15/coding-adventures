import Testing
import Foundation
@testable import ConduitHello
import Conduit

#if canImport(Glibc)
import Glibc
#elseif canImport(Darwin)
import Darwin
#endif

#if canImport(Darwin)
private let SOCK_STREAM_VALUE = SOCK_STREAM
#else
private let SOCK_STREAM_VALUE = Int32(SOCK_STREAM.rawValue)
#endif

// Smoke test: launch the demo app on an OS-assigned port and hit a few routes.

private func get(_ port: UInt16, _ path: String) -> (status: Int, body: String)? {
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
        p.withMemoryRebound(to: sockaddr.self, capacity: 1) {
            connect(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
        }
    }
    if connected != 0 { return nil }
    let req = "GET \(path) HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    _ = Array(req.utf8).withUnsafeBytes { send(fd, $0.baseAddress, $0.count, 0) }
    var data = [UInt8]()
    var buf = [UInt8](repeating: 0, count: 4096)
    while true {
        let n = buf.withUnsafeMutableBytes { recv(fd, $0.baseAddress, $0.count, 0) }
        if n <= 0 { break }
        data.append(contentsOf: buf[0..<n])
    }
    let raw = String(decoding: data, as: UTF8.self)
    guard let sep = raw.range(of: "\r\n\r\n"),
          let statusLine = raw.components(separatedBy: "\r\n").first else { return nil }
    let parts = statusLine.components(separatedBy: " ")
    let status = parts.count >= 2 ? Int(parts[1]) ?? 0 : 0
    return (status, String(raw[sep.upperBound...]))
}

private func request(_ port: UInt16, _ path: String) -> (status: Int, body: String)? {
    for _ in 0..<100 {
        if let r = get(port, path) { return r }
        usleep(50_000)
    }
    return nil
}

@Suite struct SmokeTests {
    @Test func demoServes() throws {
        let server = try makeApp().bind(host: "127.0.0.1", port: 0)
        #expect(server.serveBackground())
        let port = server.localPort
        #expect(port > 0)
        let watchdog = Thread { Thread.sleep(forTimeInterval: 30); server.stop() }
        watchdog.start()
        defer { server.stop() }

        let root = request(port, "/")
        #expect(root?.status == 200)
        #expect(root?.body.contains("Hello from Conduit") == true)

        let hello = request(port, "/hello/Ada")
        #expect(hello?.status == 200)
        #expect(hello?.body.contains("Hello Ada") == true)

        let nf = request(port, "/nope")
        #expect(nf?.status == 404)
    }
}
