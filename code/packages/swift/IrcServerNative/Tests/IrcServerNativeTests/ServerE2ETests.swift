import Testing
import Foundation
@testable import IrcServerNative

#if canImport(Glibc)
import Glibc
#elseif canImport(Darwin)
import Darwin
#endif

// ============================================================================
// End-to-end: start the real Rust IRC engine on an ephemeral port and drive two
// live IRC clients over POSIX TCP sockets. The headline assertion is the
// in-process broadcast: alice PRIVMSGs a channel and bob (a *different*
// connection) must receive it — exercising the Rust engine's mailbox fan-out.
//
// A watchdog thread stops the server after a deadline and every socket carries a
// receive timeout, so a wedged run fails fast instead of hanging.
// ============================================================================

/// A connected IRC client over a raw POSIX socket.
private final class IRCClient {
    let fd: Int32

    init?(port: UInt16) {
        let s = socket(AF_INET, SOCK_STREAM_VALUE, 0)
        if s < 0 { return nil }
        var tv = timeval(tv_sec: 5, tv_usec: 0)
        setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size))

        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = port.bigEndian
        addr.sin_addr.s_addr = inet_addr("127.0.0.1")
        let connected = withUnsafePointer(to: &addr) { p in
            p.withMemoryRebound(to: sockaddr.self, capacity: 1) { sa in
                connect(s, sa, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }
        if connected != 0 {
            _ = Glibc_or_Darwin_close(s)
            return nil
        }
        self.fd = s
    }

    func send(_ text: String) {
        let bytes = Array(text.utf8)
        _ = bytes.withUnsafeBytes { Glibc_or_Darwin_send(fd, $0.baseAddress, $0.count) }
    }

    /// Read until `needle` appears in the accumulated stream, or a deadline.
    func recvUntil(_ needle: String, timeout: TimeInterval = 5) -> String {
        let deadline = Date().addingTimeInterval(timeout)
        var data = [UInt8]()
        var buf = [UInt8](repeating: 0, count: 4096)
        while Date() < deadline {
            if String(decoding: data, as: UTF8.self).contains(needle) { break }
            let n = buf.withUnsafeMutableBytes { recv(fd, $0.baseAddress, $0.count, 0) }
            if n <= 0 { continue } // timeout slice — poll again until deadline
            data.append(contentsOf: buf[0..<n])
        }
        return String(decoding: data, as: UTF8.self)
    }

    func close() { _ = Glibc_or_Darwin_close(fd) }
}

/// `send`/`close` are ambiguous against the IRCClient methods; wrap the C calls.
private func Glibc_or_Darwin_send(_ fd: Int32, _ p: UnsafeRawPointer?, _ n: Int) -> Int {
    send(fd, p, n, 0)
}
private func Glibc_or_Darwin_close(_ fd: Int32) -> Int32 { close(fd) }

/// Connect with retry — the background reactor may need a moment after bind.
private func connectClient(port: UInt16) -> IRCClient? {
    for _ in 0..<100 {
        if let c = IRCClient(port: port) { return c }
        usleep(50_000) // 50ms
    }
    return nil
}

@Suite struct ServerE2ETests {
    @Test func broadcastBetweenClients() throws {
        let server = try IrcServer(port: 0, serverName: "irc.test")
        #expect(server.localHost == "127.0.0.1")
        #expect(server.localPort > 0)
        #expect(server.localAddr == "127.0.0.1:\(server.localPort)")
        #expect(!server.running)

        #expect(server.serveBackground())
        // Wait for the loop to come up.
        for _ in 0..<200 where !server.running { usleep(5_000) }
        #expect(server.running)

        // Watchdog: force the server down after a deadline so nothing hangs.
        let watchdog = Thread {
            Thread.sleep(forTimeInterval: 30)
            server.stop()
        }
        watchdog.start()
        defer { server.stop() }

        let port = server.localPort
        let alice = try #require(connectClient(port: port))
        let bob = try #require(connectClient(port: port))
        defer { alice.close(); bob.close() }

        // Register both clients and confirm the 001 welcome numeric.
        alice.send("NICK alice\r\nUSER alice 0 * :alice\r\n")
        #expect(alice.recvUntil("001").contains("001"))
        bob.send("NICK bob\r\nUSER bob 0 * :bob\r\n")
        #expect(bob.recvUntil("001").contains("001"))

        // PING/PONG liveness.
        alice.send("PING :liveness\r\n")
        #expect(alice.recvUntil("PONG").contains("PONG"))

        // Join the channel from both clients.
        alice.send("JOIN #test\r\n")
        bob.send("JOIN #test\r\n")
        _ = alice.recvUntil("JOIN")
        _ = bob.recvUntil("JOIN")

        // The headline: alice speaks, bob (a different connection) must receive
        // it — proving the Rust engine's in-process mailbox fan-out.
        alice.send("PRIVMSG #test :hello bob\r\n")
        let received = bob.recvUntil("hello bob")
        #expect(received.contains("PRIVMSG"))
        #expect(received.contains("hello bob"))

        server.stop()
        #expect(!server.running)
    }

    @Test func errorTypeDescribesItself() {
        // The bind-failure path surfaces an `IrcServerError`; confirm it carries
        // its message through `description` (used when the error is logged).
        let err = IrcServerError(message: "irc_server_new: failed to bind 127.0.0.1:6667")
        #expect(err.description == "irc_server_new: failed to bind 127.0.0.1:6667")
        #expect("\(err)" == err.message)
    }
}

// SOCK_STREAM is an enum on Darwin and a macro on Glibc; normalize the value.
#if canImport(Darwin)
private let SOCK_STREAM_VALUE = SOCK_STREAM
#else
private let SOCK_STREAM_VALUE = Int32(SOCK_STREAM.rawValue)
#endif
