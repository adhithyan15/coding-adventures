// TcpClientTests.swift — Tests for the TCP client with buffered I/O
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// Test strategy
// ============================================================================
//
// We cannot test against real internet servers (tests must be fast, offline,
// and deterministic). Instead, we spin up tiny POSIX server sockets on
// localhost using OS-assigned ports (port 0). Each test server lives in a
// background DispatchQueue and handles exactly one connection.
//
// ## Test groups
//
// 1. Echo server tests — write data, read it back
// 2. Timeout tests — verify connect and read timeouts fire
// 3. Error tests — connection refused, DNS failure, unexpected EOF
// 4. Half-close tests — shutdown write while keeping read open
// 5. Edge cases — zero-byte writes, multiple exchanges, address queries
// 6. Options tests — verify defaults and custom options
//
// ============================================================================

import XCTest
@testable import TcpClient
import Foundation

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

// On Darwin, SOCK_STREAM is an Int32 constant. On Linux (Glibc), it's a
// __socket_type enum whose raw value must be extracted explicitly.
#if canImport(Darwin)
private let sockStream = SOCK_STREAM
#elseif canImport(Glibc)
private let sockStream = Int32(SOCK_STREAM.rawValue)
#endif

// Disambiguate POSIX bind() from Swift's Sequence.bind — inside closures like
// withMemoryRebound, the compiler sees the instance method first. Wrapping it
// in a free function at module scope resolves the ambiguity.
private func posixBind(_ fd: Int32, _ addr: UnsafePointer<sockaddr>, _ len: socklen_t) -> Int32 {
    #if canImport(Darwin)
    return Darwin.bind(fd, addr, len)
    #elseif canImport(Glibc)
    return Glibc.bind(fd, addr, len)
    #endif
}

// ============================================================================
// Test helpers: mini TCP servers
// ============================================================================
//
// Each helper binds a server socket to 127.0.0.1:0 (OS picks a free port),
// listens for one connection, and runs its logic in a background thread.
// The assigned port is returned so the test can connect to it.
//
// ## Why port 0?
//
// ```text
// bind("127.0.0.1", 0) → OS assigns an ephemeral port (e.g., 52341)
//   - No conflicts when tests run in parallel
//   - No need to hard-code ports
//   - Works on every platform
// ```

/// Start a local echo server that reads data and sends it back verbatim.
/// Returns the port number the server is listening on.
func startEchoServer() -> (port: UInt16, cleanup: () -> Void) {
    let serverFd = socket(AF_INET, sockStream, 0)
    precondition(serverFd >= 0, "Failed to create server socket")

    // Allow port reuse to prevent "address already in use" errors
    var reuseAddr: Int32 = 1
    setsockopt(serverFd, SOL_SOCKET, SO_REUSEADDR, &reuseAddr,
               socklen_t(MemoryLayout<Int32>.size))

    // Bind to localhost on an OS-assigned port
    var addr = sockaddr_in()
    addr.sin_family = sa_family_t(AF_INET)
    addr.sin_port = UInt16(0).bigEndian
    addr.sin_addr.s_addr = inet_addr("127.0.0.1")

    let bindResult = withUnsafePointer(to: &addr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            posixBind(serverFd, sockPtr, socklen_t(MemoryLayout<sockaddr_in>.size))
        }
    }
    precondition(bindResult == 0, "Failed to bind server socket")

    listen(serverFd, 1)

    // Read back the assigned port
    var boundAddr = sockaddr_in()
    var addrLen = socklen_t(MemoryLayout<sockaddr_in>.size)
    _ = withUnsafeMutablePointer(to: &boundAddr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            getsockname(serverFd, sockPtr, &addrLen)
        }
    }
    let port = UInt16(bigEndian: boundAddr.sin_port)

    // Run the echo logic in a background thread
    let thread = Thread {
        var clientAddr = sockaddr_storage()
        var clientLen = socklen_t(MemoryLayout<sockaddr_storage>.size)
        let clientFd = withUnsafeMutablePointer(to: &clientAddr) { storagePtr in
            storagePtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                accept(serverFd, sockPtr, &clientLen)
            }
        }
        guard clientFd >= 0 else { return }

        // Set a read timeout so we don't hang forever
        var tv = timeval(tv_sec: 5, tv_usec: 0)
        setsockopt(clientFd, SOL_SOCKET, SO_RCVTIMEO, &tv,
                   socklen_t(MemoryLayout<timeval>.size))

        var buf = [UInt8](repeating: 0, count: 65536)
        while true {
            let n = recv(clientFd, &buf, buf.count, 0)
            if n <= 0 { break }
            var sent = 0
            while sent < n {
                let s = buf.withUnsafeBufferPointer { bufPtr in
                    send(clientFd, bufPtr.baseAddress! + sent, n - sent, 0)
                }
                if s <= 0 { break }
                sent += s
            }
        }
        #if canImport(Darwin)
        Darwin.close(clientFd)
        #elseif canImport(Glibc)
        Glibc.close(clientFd)
        #endif
    }
    thread.start()

    // Small delay to let the server start accepting
    Thread.sleep(forTimeInterval: 0.05)

    return (port, {
        #if canImport(Darwin)
        Darwin.close(serverFd)
        #elseif canImport(Glibc)
        Glibc.close(serverFd)
        #endif
    })
}

/// Start a server that accepts a connection but never sends any data.
/// Used for read timeout tests.
func startSilentServer() -> (port: UInt16, cleanup: () -> Void) {
    let serverFd = socket(AF_INET, sockStream, 0)
    precondition(serverFd >= 0)

    var reuseAddr: Int32 = 1
    setsockopt(serverFd, SOL_SOCKET, SO_REUSEADDR, &reuseAddr,
               socklen_t(MemoryLayout<Int32>.size))

    var addr = sockaddr_in()
    addr.sin_family = sa_family_t(AF_INET)
    addr.sin_port = UInt16(0).bigEndian
    addr.sin_addr.s_addr = inet_addr("127.0.0.1")

    _ = withUnsafePointer(to: &addr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            posixBind(serverFd, sockPtr, socklen_t(MemoryLayout<sockaddr_in>.size))
        }
    }
    listen(serverFd, 1)

    var boundAddr = sockaddr_in()
    var addrLen = socklen_t(MemoryLayout<sockaddr_in>.size)
    _ = withUnsafeMutablePointer(to: &boundAddr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            getsockname(serverFd, sockPtr, &addrLen)
        }
    }
    let port = UInt16(bigEndian: boundAddr.sin_port)

    let semaphore = DispatchSemaphore(value: 0)
    let thread = Thread {
        var clientAddr = sockaddr_storage()
        var clientLen = socklen_t(MemoryLayout<sockaddr_storage>.size)
        let clientFd = withUnsafeMutablePointer(to: &clientAddr) { storagePtr in
            storagePtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                accept(serverFd, sockPtr, &clientLen)
            }
        }
        // Hold the connection open until cleanup
        semaphore.wait()
        if clientFd >= 0 {
            #if canImport(Darwin)
            Darwin.close(clientFd)
            #elseif canImport(Glibc)
            Glibc.close(clientFd)
            #endif
        }
    }
    thread.start()
    Thread.sleep(forTimeInterval: 0.05)

    return (port, {
        semaphore.signal()
        #if canImport(Darwin)
        Darwin.close(serverFd)
        #elseif canImport(Glibc)
        Glibc.close(serverFd)
        #endif
    })
}

/// Start a server that sends exactly the given data, then closes the connection.
/// Used for partial-read and EOF tests.
func startPartialServer(data: Data) -> (port: UInt16, cleanup: () -> Void) {
    let serverFd = socket(AF_INET, sockStream, 0)
    precondition(serverFd >= 0)

    var reuseAddr: Int32 = 1
    setsockopt(serverFd, SOL_SOCKET, SO_REUSEADDR, &reuseAddr,
               socklen_t(MemoryLayout<Int32>.size))

    var addr = sockaddr_in()
    addr.sin_family = sa_family_t(AF_INET)
    addr.sin_port = UInt16(0).bigEndian
    addr.sin_addr.s_addr = inet_addr("127.0.0.1")

    _ = withUnsafePointer(to: &addr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            posixBind(serverFd, sockPtr, socklen_t(MemoryLayout<sockaddr_in>.size))
        }
    }
    listen(serverFd, 1)

    var boundAddr = sockaddr_in()
    var addrLen = socklen_t(MemoryLayout<sockaddr_in>.size)
    _ = withUnsafeMutablePointer(to: &boundAddr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            getsockname(serverFd, sockPtr, &addrLen)
        }
    }
    let port = UInt16(bigEndian: boundAddr.sin_port)

    let thread = Thread {
        var clientAddr = sockaddr_storage()
        var clientLen = socklen_t(MemoryLayout<sockaddr_storage>.size)
        let clientFd = withUnsafeMutablePointer(to: &clientAddr) { storagePtr in
            storagePtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                accept(serverFd, sockPtr, &clientLen)
            }
        }
        guard clientFd >= 0 else { return }

        // Send the data
        data.withUnsafeBytes { rawBuffer in
            if let ptr = rawBuffer.baseAddress {
                _ = send(clientFd, ptr, data.count, 0)
            }
        }

        // Small delay so client can read before we close
        Thread.sleep(forTimeInterval: 0.1)

        #if canImport(Darwin)
        Darwin.close(clientFd)
        #elseif canImport(Glibc)
        Glibc.close(clientFd)
        #endif
    }
    thread.start()
    Thread.sleep(forTimeInterval: 0.05)

    return (port, {
        #if canImport(Darwin)
        Darwin.close(serverFd)
        #elseif canImport(Glibc)
        Glibc.close(serverFd)
        #endif
    })
}

/// Start a server that reads a request, then sends the given response.
/// Used for request-response pattern tests (like HTTP).
func startRequestResponseServer(response: Data) -> (port: UInt16, cleanup: () -> Void) {
    let serverFd = socket(AF_INET, sockStream, 0)
    precondition(serverFd >= 0)

    var reuseAddr: Int32 = 1
    setsockopt(serverFd, SOL_SOCKET, SO_REUSEADDR, &reuseAddr,
               socklen_t(MemoryLayout<Int32>.size))

    var addr = sockaddr_in()
    addr.sin_family = sa_family_t(AF_INET)
    addr.sin_port = UInt16(0).bigEndian
    addr.sin_addr.s_addr = inet_addr("127.0.0.1")

    _ = withUnsafePointer(to: &addr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            posixBind(serverFd, sockPtr, socklen_t(MemoryLayout<sockaddr_in>.size))
        }
    }
    listen(serverFd, 1)

    var boundAddr = sockaddr_in()
    var addrLen = socklen_t(MemoryLayout<sockaddr_in>.size)
    _ = withUnsafeMutablePointer(to: &boundAddr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            getsockname(serverFd, sockPtr, &addrLen)
        }
    }
    let port = UInt16(bigEndian: boundAddr.sin_port)

    let thread = Thread {
        var clientAddr = sockaddr_storage()
        var clientLen = socklen_t(MemoryLayout<sockaddr_storage>.size)
        let clientFd = withUnsafeMutablePointer(to: &clientAddr) { storagePtr in
            storagePtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                accept(serverFd, sockPtr, &clientLen)
            }
        }
        guard clientFd >= 0 else { return }

        // Set a read timeout
        var tv = timeval(tv_sec: 5, tv_usec: 0)
        setsockopt(clientFd, SOL_SOCKET, SO_RCVTIMEO, &tv,
                   socklen_t(MemoryLayout<timeval>.size))

        // Read the request (just drain it)
        var buf = [UInt8](repeating: 0, count: 4096)
        _ = recv(clientFd, &buf, buf.count, 0)

        // Send the response
        response.withUnsafeBytes { rawBuffer in
            if let ptr = rawBuffer.baseAddress {
                _ = send(clientFd, ptr, response.count, 0)
            }
        }

        Thread.sleep(forTimeInterval: 0.1)

        #if canImport(Darwin)
        Darwin.close(clientFd)
        #elseif canImport(Glibc)
        Glibc.close(clientFd)
        #endif
    }
    thread.start()
    Thread.sleep(forTimeInterval: 0.05)

    return (port, {
        #if canImport(Darwin)
        Darwin.close(serverFd)
        #elseif canImport(Glibc)
        Glibc.close(serverFd)
        #endif
    })
}

/// Start a server that reads until EOF (client shutdown), then sends a response.
/// Used for half-close tests.
func startHalfCloseServer(response: Data) -> (port: UInt16, receivedData: () -> Data, cleanup: () -> Void) {
    let serverFd = socket(AF_INET, sockStream, 0)
    precondition(serverFd >= 0)

    var reuseAddr: Int32 = 1
    setsockopt(serverFd, SOL_SOCKET, SO_REUSEADDR, &reuseAddr,
               socklen_t(MemoryLayout<Int32>.size))

    var addr = sockaddr_in()
    addr.sin_family = sa_family_t(AF_INET)
    addr.sin_port = UInt16(0).bigEndian
    addr.sin_addr.s_addr = inet_addr("127.0.0.1")

    _ = withUnsafePointer(to: &addr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            posixBind(serverFd, sockPtr, socklen_t(MemoryLayout<sockaddr_in>.size))
        }
    }
    listen(serverFd, 1)

    var boundAddr = sockaddr_in()
    var addrLen = socklen_t(MemoryLayout<sockaddr_in>.size)
    _ = withUnsafeMutablePointer(to: &boundAddr) { addrPtr in
        addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            getsockname(serverFd, sockPtr, &addrLen)
        }
    }
    let port = UInt16(bigEndian: boundAddr.sin_port)

    // Shared state: what the server received from the client
    let receivedLock = NSLock()
    var receivedBytes = Data()
    let semaphore = DispatchSemaphore(value: 0)

    let thread = Thread {
        var clientAddr = sockaddr_storage()
        var clientLen = socklen_t(MemoryLayout<sockaddr_storage>.size)
        let clientFd = withUnsafeMutablePointer(to: &clientAddr) { storagePtr in
            storagePtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                accept(serverFd, sockPtr, &clientLen)
            }
        }
        guard clientFd >= 0 else {
            semaphore.signal()
            return
        }

        var tv = timeval(tv_sec: 5, tv_usec: 0)
        setsockopt(clientFd, SOL_SOCKET, SO_RCVTIMEO, &tv,
                   socklen_t(MemoryLayout<timeval>.size))

        // Read until EOF (client calls shutdownWrite)
        var buf = [UInt8](repeating: 0, count: 4096)
        while true {
            let n = recv(clientFd, &buf, buf.count, 0)
            if n <= 0 { break }
            receivedLock.lock()
            receivedBytes.append(contentsOf: buf[0..<n])
            receivedLock.unlock()
        }

        // Client is done writing — send our response
        response.withUnsafeBytes { rawBuffer in
            if let ptr = rawBuffer.baseAddress {
                _ = send(clientFd, ptr, response.count, 0)
            }
        }

        Thread.sleep(forTimeInterval: 0.1)

        #if canImport(Darwin)
        Darwin.close(clientFd)
        #elseif canImport(Glibc)
        Glibc.close(clientFd)
        #endif

        semaphore.signal()
    }
    thread.start()
    Thread.sleep(forTimeInterval: 0.05)

    return (
        port,
        {
            // Wait for the server thread to finish processing
            semaphore.wait()
            receivedLock.lock()
            let result = receivedBytes
            receivedLock.unlock()
            return result
        },
        {
            #if canImport(Darwin)
            Darwin.close(serverFd)
            #elseif canImport(Glibc)
            Glibc.close(serverFd)
            #endif
        }
    )
}

// ============================================================================
// Helper: test-friendly connect options with short timeouts
// ============================================================================

func testOptions() -> ConnectOptions {
    return ConnectOptions(
        connectTimeout: 5,
        readTimeout: 5,
        writeTimeout: 5,
        bufferSize: 4096
    )
}

// ============================================================================
// Tests
// ============================================================================

final class TcpClientTests: XCTestCase {

    // ── Test 1: Version ───────────────────────────────────────────────

    func testVersion() {
        XCTAssertEqual(VERSION, "0.1.0")
    }

    // ── Group 1: Echo server tests ────────────────────────────────────

    // Test 2: Basic connect and disconnect
    func testConnectAndDisconnect() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        // Connection should be established — just close it
        conn.close()
    }

    // Test 3: Write data and read it back through echo server
    func testWriteAndReadBack() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        let message = Data("Hello, TCP!".utf8)
        try conn.writeAll(message)
        try conn.flush()

        let result = try conn.readExact(11)
        XCTAssertEqual(result, message)
    }

    // Test 4: Read line from echo server
    func testReadLineFromEcho() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        try conn.writeAll(Data("Hello\r\nWorld\r\n".utf8))
        try conn.flush()

        let line1 = try conn.readLine()
        XCTAssertEqual(line1, "Hello\r\n")

        let line2 = try conn.readLine()
        XCTAssertEqual(line2, "World\r\n")
    }

    // Test 5: Read exact number of bytes from echo
    func testReadExactFromEcho() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        let data = Data((0..<100).map { UInt8($0 % 256) })
        try conn.writeAll(data)
        try conn.flush()

        let result = try conn.readExact(100)
        XCTAssertEqual(result, data)
    }

    // Test 6: Read until delimiter from echo
    func testReadUntilFromEcho() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        try conn.writeAll(Data("key:value\0next".utf8))
        try conn.flush()

        let result = try conn.readUntil(0) // null byte delimiter
        XCTAssertEqual(result, Data("key:value\0".utf8))
    }

    // Test 7: Large data transfer (64 KiB)
    func testLargeDataTransfer() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        // Send 64 KiB
        let data = Data((0..<65536).map { UInt8($0 % 256) })
        try conn.writeAll(data)
        try conn.flush()

        let result = try conn.readExact(65536)
        XCTAssertEqual(result.count, 65536)
        XCTAssertEqual(result, data)
    }

    // Test 8: Multiple exchanges (ping-pong)
    func testMultipleExchanges() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        // Exchange 1
        try conn.writeAll(Data("ping\n".utf8))
        try conn.flush()
        let line1 = try conn.readLine()
        XCTAssertEqual(line1, "ping\n")

        // Exchange 2
        try conn.writeAll(Data("pong\n".utf8))
        try conn.flush()
        let line2 = try conn.readLine()
        XCTAssertEqual(line2, "pong\n")
    }

    // ── Group 2: Timeout tests ────────────────────────────────────────

    // Test 9: Connect timeout with non-routable address
    func testConnectTimeout() {
        // 10.255.255.1 is a non-routable address — connection will hang
        let opts = ConnectOptions(
            connectTimeout: 1,
            readTimeout: 5,
            writeTimeout: 5,
            bufferSize: 4096
        )

        do {
            _ = try tcpConnect(host: "10.255.255.1", port: 1, options: opts)
            XCTFail("Expected timeout error")
        } catch let error as TcpError {
            switch error {
            case .timeout(let phase, _):
                XCTAssertEqual(phase, "connect")
            case .ioError:
                // Some platforms return a generic error
                break
            default:
                XCTFail("Expected timeout or ioError, got: \(error)")
            }
        } catch {
            XCTFail("Unexpected error type: \(error)")
        }
    }

    // Test 10: Read timeout when server never sends data
    func testReadTimeout() throws {
        let (port, cleanup) = startSilentServer()
        defer { cleanup() }

        let opts = ConnectOptions(
            connectTimeout: 5,
            readTimeout: 1,
            writeTimeout: 5,
            bufferSize: 4096
        )

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: opts)
        defer { conn.close() }

        do {
            _ = try conn.readLine()
            XCTFail("Expected timeout error")
        } catch let error as TcpError {
            switch error {
            case .timeout:
                break // expected
            case .ioError:
                break // platform-dependent
            default:
                XCTFail("Expected timeout, got: \(error)")
            }
        }
    }

    // ── Group 3: Error tests ──────────────────────────────────────────

    // Test 11: Connection refused when nothing is listening
    func testConnectionRefused() {
        // Bind a socket, get the port, then immediately close it
        let tempFd = socket(AF_INET, sockStream, 0)
        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = UInt16(0).bigEndian
        addr.sin_addr.s_addr = inet_addr("127.0.0.1")

        _ = withUnsafePointer(to: &addr) { addrPtr in
            addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                posixBind(tempFd, sockPtr, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }
        listen(tempFd, 1)

        var boundAddr = sockaddr_in()
        var addrLen = socklen_t(MemoryLayout<sockaddr_in>.size)
        _ = withUnsafeMutablePointer(to: &boundAddr) { addrPtr in
            addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                getsockname(tempFd, sockPtr, &addrLen)
            }
        }
        let port = UInt16(bigEndian: boundAddr.sin_port)

        // Close the server — now nothing listens on this port
        #if canImport(Darwin)
        Darwin.close(tempFd)
        #elseif canImport(Glibc)
        Glibc.close(tempFd)
        #endif

        do {
            _ = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
            XCTFail("Expected connection refused")
        } catch let error as TcpError {
            switch error {
            case .connectionRefused:
                break // expected
            case .ioError:
                break // some platforms
            default:
                XCTFail("Expected connectionRefused, got: \(error)")
            }
        } catch {
            XCTFail("Unexpected error type: \(error)")
        }
    }

    // Test 12: DNS failure for non-existent host
    func testDnsFailure() {
        do {
            _ = try tcpConnect(
                host: "this.host.does.not.exist.example",
                port: 80,
                options: testOptions()
            )
            XCTFail("Expected DNS resolution failure")
        } catch let error as TcpError {
            switch error {
            case .dnsResolutionFailed(let host, _):
                XCTAssertEqual(host, "this.host.does.not.exist.example")
            case .connectionRefused:
                break // Some ISP DNS resolvers hijack NXDOMAIN
            case .timeout:
                break // DNS might time out
            case .ioError:
                break // platform-dependent
            default:
                XCTFail("Expected dnsResolutionFailed, got: \(error)")
            }
        } catch {
            XCTFail("Unexpected error type: \(error)")
        }
    }

    // Test 13: Unexpected EOF (server sends fewer bytes than client expects)
    func testUnexpectedEof() throws {
        let data = Data((0..<50).map { UInt8($0) })
        let (port, cleanup) = startPartialServer(data: data)
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        // Wait for server to send and close
        Thread.sleep(forTimeInterval: 0.2)

        do {
            _ = try conn.readExact(100)
            XCTFail("Expected unexpected EOF")
        } catch let error as TcpError {
            switch error {
            case .unexpectedEof(let expected, let received):
                XCTAssertEqual(expected, 100)
                XCTAssertLessThanOrEqual(received, 50)
            default:
                XCTFail("Expected unexpectedEof, got: \(error)")
            }
        }
    }

    // Test 14: Broken pipe (write after server closed)
    func testBrokenPipe() throws {
        let (port, cleanup) = startPartialServer(data: Data())
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        // Wait for server to close its end
        Thread.sleep(forTimeInterval: 0.3)

        // Try to write — should eventually get an error
        var gotError = false
        for _ in 0..<10 {
            let bigData = Data(repeating: 0, count: 65536)
            do {
                try conn.writeAll(bigData)
                try conn.flush()
                Thread.sleep(forTimeInterval: 0.05)
            } catch {
                gotError = true
                break
            }
        }
        XCTAssertTrue(gotError, "expected write error after server closed")
    }

    // ── Group 4: Half-close tests ─────────────────────────────────────

    // Test 15: Client half-close — shutdown write, keep reading
    func testClientHalfClose() throws {
        let responseData = Data("DONE\n".utf8)
        let (port, receivedData, cleanup) = startHalfCloseServer(response: responseData)
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        try conn.writeAll(Data("request data".utf8))
        try conn.shutdownWrite()

        // Now read the server's response
        let response = try conn.readLine()
        XCTAssertEqual(response, "DONE\n")

        // Verify server received our data
        let serverReceived = receivedData()
        XCTAssertEqual(serverReceived, Data("request data".utf8))
    }

    // ── Group 5: Edge cases ───────────────────────────────────────────

    // Test 16: Empty read at EOF
    func testEmptyReadAtEof() throws {
        let data = Data("hello\n".utf8)
        let (port, cleanup) = startPartialServer(data: data)
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        // Wait for server to send and close
        Thread.sleep(forTimeInterval: 0.2)

        let line = try conn.readLine()
        XCTAssertEqual(line, "hello\n")

        // Next read should return empty string (EOF)
        let eof = try conn.readLine()
        XCTAssertEqual(eof, "")
    }

    // Test 17: Zero-byte write succeeds without error
    func testZeroByteWrite() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        // Writing zero bytes should succeed without error
        XCTAssertNoThrow(try conn.writeAll(Data()))
    }

    // Test 18: Peer and local address queries
    func testAddressQueries() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        let (peerIP, peerPort) = try conn.peerAddr()
        XCTAssertEqual(peerIP, "127.0.0.1")
        XCTAssertEqual(peerPort, port)

        let (localIP, localPort) = try conn.localAddr()
        XCTAssertEqual(localIP, "127.0.0.1")
        XCTAssertTrue(localPort > 0)
    }

    // ── Group 6: Options tests ────────────────────────────────────────

    // Test 19: Default options have expected values
    func testDefaultOptions() {
        let opts = ConnectOptions.default
        XCTAssertEqual(opts.connectTimeout, 30)
        XCTAssertEqual(opts.readTimeout, 30)
        XCTAssertEqual(opts.writeTimeout, 30)
        XCTAssertEqual(opts.bufferSize, 8192)
    }

    // Test 20: Connect via "localhost" hostname
    func testConnectWithHostnameLocalhost() throws {
        let (port, cleanup) = startEchoServer()
        defer { cleanup() }

        // "localhost" should resolve to 127.0.0.1 via the OS resolver
        let conn = try tcpConnect(host: "localhost", port: port, options: testOptions())
        defer { conn.close() }
        // If we get here, the connection succeeded
    }

    // Test 21: Request-response pattern (like HTTP)
    func testRequestResponsePattern() throws {
        let responseData = Data("HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello".utf8)
        let (port, cleanup) = startRequestResponseServer(response: responseData)
        defer { cleanup() }

        let conn = try tcpConnect(host: "127.0.0.1", port: port, options: testOptions())
        defer { conn.close() }

        // Send request
        try conn.writeAll(Data("GET / HTTP/1.0\r\n\r\n".utf8))
        try conn.flush()

        // Read response line by line
        let status = try conn.readLine()
        XCTAssertTrue(status.hasPrefix("HTTP/1.0 200"))

        let header = try conn.readLine()
        XCTAssertTrue(header.hasPrefix("Content-Length:"))

        let blank = try conn.readLine()
        XCTAssertEqual(blank, "\r\n")

        let body = try conn.readExact(5)
        XCTAssertEqual(body, Data("hello".utf8))
    }

    // Test 22: TcpError equatable conformance
    func testErrorEquatable() {
        let err1 = TcpError.connectionReset
        let err2 = TcpError.connectionReset
        XCTAssertEqual(err1, err2)

        let err3 = TcpError.brokenPipe
        XCTAssertNotEqual(err1, err3)

        let err4 = TcpError.dnsResolutionFailed(host: "example.com", message: "no such host")
        let err5 = TcpError.dnsResolutionFailed(host: "example.com", message: "no such host")
        XCTAssertEqual(err4, err5)

        let err6 = TcpError.unexpectedEof(expected: 100, received: 50)
        let err7 = TcpError.unexpectedEof(expected: 100, received: 50)
        XCTAssertEqual(err6, err7)
    }
}
