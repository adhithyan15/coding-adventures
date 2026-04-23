// TcpClient.swift — TCP client with buffered I/O and configurable timeouts
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// Overview
// ============================================================================
//
// A TCP client wraps POSIX sockets with ergonomic defaults for building
// network clients. It is **protocol-agnostic** — it knows nothing about HTTP,
// SMTP, or Redis. It just moves bytes reliably between two machines. Higher-
// level packages build application protocols on top.
//
// ## Analogy: A telephone call
//
// ```text
// Making a TCP connection is like making a phone call:
//
// 1. DIAL (DNS + connect)
//    Look up "Grandma" → 555-0123     (DNS resolution)
//    Dial and wait for ring            (TCP three-way handshake)
//    If nobody picks up → hang up      (connect timeout)
//
// 2. TALK (read/write)
//    Say "Hello, Grandma!"             (writeAll + flush)
//    Listen for response               (readLine)
//    If silence for 30s → "Still there?" (read timeout)
//
// 3. HANG UP (shutdown/close)
//    Say "Goodbye" and hang up         (shutdownWrite + close)
// ```
//
// ## Where it fits
//
// ```text
// url-parser (NET00) → tcp-client (NET01, THIS) → frame-extractor (NET02)
//                          ↓
//                     raw byte stream
// ```
//
// ## Example
//
// ```swift
// let conn = try tcpConnect(host: "info.cern.ch", port: 80)
// try conn.writeAll(Data("GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n".utf8))
// try conn.flush()
// let statusLine = try conn.readLine()
// print(statusLine)
// conn.close()
// ```
//
// ============================================================================
// Platform imports
// ============================================================================
//
// POSIX sockets are available on all Unix-like systems. On macOS, the C
// standard library is exposed via `Darwin`; on Linux, via `Glibc`. We import
// whichever is available so we can call `socket()`, `connect()`, `recv()`,
// `send()`, `setsockopt()`, `getaddrinfo()`, etc.

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

import Foundation

/// The current version of the tcp-client package.
public let VERSION = "0.1.0"

// ============================================================================
// TcpError — structured error types
// ============================================================================

/// Errors that can occur during TCP connection and communication.
///
/// Each case maps to a specific failure mode. Switch on these to decide
/// how to recover.
///
/// ```text
/// dnsResolutionFailed → hostname typo or no internet
/// connectionRefused   → server is up but nothing listening on that port
/// timeout             → took too long (connect, read, or write)
/// connectionReset     → remote side crashed (TCP RST)
/// brokenPipe          → tried to write after remote closed
/// unexpectedEof       → connection closed before expected data arrived
/// ioError             → catch-all for other OS-level errors
/// ```
public enum TcpError: Error, Equatable {
    /// DNS lookup failed — hostname could not be resolved.
    case dnsResolutionFailed(host: String, message: String)

    /// Server is reachable but nothing is listening on the port (TCP RST).
    case connectionRefused(addr: String)

    /// Operation timed out. `phase` is "connect", "read", or "write".
    case timeout(phase: String, duration: TimeInterval)

    /// Remote side reset the connection unexpectedly (TCP RST during transfer).
    case connectionReset

    /// Tried to write to a connection the remote side already closed.
    case brokenPipe

    /// Connection closed before the expected number of bytes arrived.
    case unexpectedEof(expected: Int, received: Int)

    /// A low-level I/O error not covered by the specific variants.
    case ioError(String)
}

// ============================================================================
// ConnectOptions — configuration for establishing a connection
// ============================================================================

/// Configuration for establishing a TCP connection.
///
/// All timeouts default to 30 seconds. The buffer size defaults to 8192
/// bytes (8 KiB), a good balance between memory usage and syscall reduction.
///
/// ## Why separate timeouts?
///
/// ```text
/// connectTimeout (30s) — how long to wait for the TCP handshake
///   If a server is down or firewalled, the OS might wait minutes.
///
/// readTimeout (30s) — how long to wait for data after calling read
///   Without this, a stalled server hangs your program forever.
///
/// writeTimeout (30s) — how long to wait for the OS send buffer
///   Usually instant, but blocks if the remote side isn't reading.
/// ```
public struct ConnectOptions {
    /// Maximum time to wait for the TCP handshake. Default: 30s.
    public var connectTimeout: TimeInterval

    /// Maximum time to wait for data on read. Default: 30s.
    /// `nil` means block forever.
    public var readTimeout: TimeInterval?

    /// Maximum time to wait on write. Default: 30s.
    /// `nil` means block forever.
    public var writeTimeout: TimeInterval?

    /// Size of internal read buffer in bytes. Default: 8192.
    public var bufferSize: Int

    /// Create a ConnectOptions with the given values.
    ///
    /// - Parameters:
    ///   - connectTimeout: Max time for TCP handshake (default 30s)
    ///   - readTimeout: Max time to wait for data (default 30s, nil = forever)
    ///   - writeTimeout: Max time to wait for send buffer (default 30s, nil = forever)
    ///   - bufferSize: Internal read buffer size (default 8192)
    public init(
        connectTimeout: TimeInterval = 30,
        readTimeout: TimeInterval? = 30,
        writeTimeout: TimeInterval? = 30,
        bufferSize: Int = 8192
    ) {
        self.connectTimeout = connectTimeout
        self.readTimeout = readTimeout
        self.writeTimeout = writeTimeout
        self.bufferSize = bufferSize
    }

    /// Default options: 30s connect/read/write timeouts, 8 KiB buffer.
    public static var `default`: ConnectOptions {
        return ConnectOptions()
    }
}

// ============================================================================
// Helper: convert errno to TcpError
// ============================================================================
//
// After a POSIX call fails, `errno` tells us what went wrong. This function
// maps common errno values to our structured TcpError enum.
//
// ## errno cheat sheet
//
// ```text
// ECONNREFUSED (61/111) → server sent TCP RST during handshake
// ETIMEDOUT    (60/110) → OS timeout expired
// ECONNRESET   (54/104) → TCP RST during data transfer
// EPIPE        (32)     → write after remote close
// EAGAIN       (35/11)  → resource temporarily unavailable (non-blocking)
// ```

/// Map the current `errno` to the most specific `TcpError` variant.
/// The `context` parameter provides human-readable info for generic errors.
internal func mapErrno(_ context: String) -> TcpError {
    let code = errno
    switch code {
    case ECONNREFUSED:
        return .connectionRefused(addr: context)
    case ETIMEDOUT:
        return .timeout(phase: context, duration: 0)
    case ECONNRESET, ECONNABORTED:
        return .connectionReset
    case EPIPE:
        return .brokenPipe
    case EAGAIN:
        // EAGAIN on a socket with a timeout set means the timeout expired
        return .timeout(phase: context, duration: 0)
    default:
        let message = String(cString: strerror(code))
        return .ioError("\(context): \(message) (errno \(code))")
    }
}

// ============================================================================
// Helper: convert TimeInterval to timeval
// ============================================================================
//
// POSIX `setsockopt` expects timeouts as `struct timeval { seconds, microseconds }`.
// Swift's `TimeInterval` is a `Double` representing seconds, so we split it
// into integer seconds and fractional microseconds.

/// Convert a TimeInterval (seconds as Double) to a POSIX `timeval` struct.
internal func timevalFromInterval(_ interval: TimeInterval) -> timeval {
    let secs = Int(interval)
    let usecs = Int32((interval - Double(secs)) * 1_000_000)
    #if canImport(Darwin)
    return timeval(tv_sec: secs, tv_usec: Int32(usecs))
    #elseif canImport(Glibc)
    return timeval(tv_sec: secs, tv_usec: Int(usecs))
    #endif
}

// ============================================================================
// TcpConnection — buffered I/O over a TCP stream
// ============================================================================

/// A TCP connection with buffered I/O and configured timeouts.
///
/// Wraps a POSIX file descriptor with an internal read buffer for efficient
/// line-oriented or chunk-oriented communication.
///
/// ## Why buffered I/O?
///
/// ```text
/// Without buffering:
///   recv() returns arbitrary chunks: "HT", "TP/", "1.0 2", "00 OK\r\n"
///   100 recv() calls = 100 syscalls (expensive!)
///
/// With an internal buffer (8 KiB):
///   First recv() pulls up to 8 KiB from the OS into memory
///   Subsequent readLine() calls serve data from the buffer
///   100 lines might need only 1-2 syscalls
///
/// Similarly, writeAll() + flush() batches writes to avoid tiny TCP packets.
/// ```
///
/// The connection is automatically closed when the object is deallocated.
public class TcpConnection {

    // ── Private state ─────────────────────────────────────────────────

    /// The underlying POSIX file descriptor for this TCP socket.
    /// A value of -1 means the socket has been closed.
    private var fd: Int32

    /// Internal read buffer. Data from `recv()` accumulates here, and
    /// read methods consume from this buffer before making more syscalls.
    private var readBuffer: Data

    /// How many bytes to request per `recv()` call.
    private let bufferCapacity: Int

    /// Whether `close()` has been called. Prevents double-close.
    private var closed: Bool = false

    // ── Initializer (internal — only tcpConnect creates these) ────────

    /// Create a TcpConnection wrapping the given file descriptor.
    ///
    /// - Parameters:
    ///   - fd: An already-connected POSIX socket file descriptor
    ///   - bufferSize: The read buffer capacity (default 8192)
    internal init(fd: Int32, bufferSize: Int) {
        self.fd = fd
        self.readBuffer = Data()
        self.bufferCapacity = bufferSize
    }

    /// When the object is deallocated, ensure the socket is closed.
    /// This prevents file descriptor leaks if the caller forgets to close.
    deinit {
        close()
    }

    // ── Private: fill the read buffer from the socket ─────────────────
    //
    // This is the only place we call `recv()`. All read methods go through
    // this to keep buffering logic centralized.

    /// Read more data from the socket into the internal buffer.
    /// Returns the number of bytes read (0 = EOF).
    ///
    /// ## How recv() works
    ///
    /// ```text
    /// recv(fd, buffer, count, 0)
    ///   fd     — which socket to read from
    ///   buffer — where to store the bytes
    ///   count  — maximum bytes to read (our bufferCapacity)
    ///   0      — no special flags
    ///
    /// Returns:
    ///   > 0 → number of bytes read (might be less than count)
    ///   0   → EOF (remote closed the connection)
    ///   -1  → error (check errno)
    /// ```
    private func fillBuffer() throws -> Int {
        var tmp = [UInt8](repeating: 0, count: bufferCapacity)
        let bytesRead = recv(fd, &tmp, bufferCapacity, 0)

        if bytesRead > 0 {
            readBuffer.append(contentsOf: tmp[0..<bytesRead])
            return bytesRead
        } else if bytesRead == 0 {
            // EOF — remote closed cleanly
            return 0
        } else {
            // bytesRead == -1 → error
            throw mapErrno("read")
        }
    }

    // ── Public API: Reading ───────────────────────────────────────────

    /// Read bytes until a newline (`\n`) is found.
    ///
    /// Returns the line **including** the trailing `\n` (and `\r\n` if
    /// present). Returns an empty string at EOF (remote closed cleanly).
    ///
    /// This is the workhorse for line-oriented protocols like HTTP/1.0,
    /// SMTP, and RESP (Redis protocol).
    ///
    /// ## Algorithm
    ///
    /// ```text
    /// 1. Search the read buffer for \n
    /// 2. If found → extract everything up to and including \n, return it
    /// 3. If not found → recv() more data into the buffer, go to step 1
    /// 4. If recv() returns 0 (EOF) and buffer is non-empty → return what we have
    /// 5. If recv() returns 0 (EOF) and buffer is empty → return ""
    /// ```
    ///
    /// ## Errors
    ///
    /// - `TcpError.timeout` if no data arrives within the read timeout
    /// - `TcpError.connectionReset` if the remote side closed unexpectedly
    public func readLine() throws -> String {
        while true {
            // Step 1-2: Check if the buffer already contains a newline
            if let newlineIndex = readBuffer.firstIndex(of: UInt8(ascii: "\n")) {
                let lineEnd = readBuffer.index(after: newlineIndex)
                let lineData = readBuffer[readBuffer.startIndex..<lineEnd]
                readBuffer = Data(readBuffer[lineEnd...])
                return String(data: Data(lineData), encoding: .utf8) ?? ""
            }

            // Step 3: No newline yet — recv() more data
            let bytesRead = try fillBuffer()

            // Step 4-5: EOF
            if bytesRead == 0 {
                if readBuffer.isEmpty {
                    return ""
                }
                // Return remaining data even without a newline
                let remaining = readBuffer
                readBuffer = Data()
                return String(data: remaining, encoding: .utf8) ?? ""
            }
        }
    }

    /// Read exactly `n` bytes from the connection.
    ///
    /// Blocks until all `n` bytes have been received. Useful for protocols
    /// that specify an exact content length (e.g., HTTP Content-Length).
    ///
    /// ## Algorithm
    ///
    /// ```text
    /// 1. If the buffer already has >= n bytes → extract and return
    /// 2. Otherwise → recv() more data until buffer has >= n bytes
    /// 3. If EOF before n bytes → throw unexpectedEof
    /// ```
    ///
    /// ## Errors
    ///
    /// - `TcpError.unexpectedEof` if the connection closes before `n` bytes
    public func readExact(_ n: Int) throws -> Data {
        while readBuffer.count < n {
            let bytesRead = try fillBuffer()
            if bytesRead == 0 {
                // EOF before we have enough bytes
                let received = readBuffer.count
                throw TcpError.unexpectedEof(expected: n, received: received)
            }
        }

        // Extract exactly n bytes from the front of the buffer
        let result = Data(readBuffer[readBuffer.startIndex..<readBuffer.startIndex + n])
        readBuffer = Data(readBuffer[(readBuffer.startIndex + n)...])
        return result
    }

    /// Read bytes until the given delimiter byte is found.
    ///
    /// Returns all bytes up to **and including** the delimiter. Useful for
    /// protocols with custom delimiters (RESP uses `\r\n`, null-terminated
    /// strings use `\0`).
    ///
    /// ## Algorithm
    ///
    /// ```text
    /// 1. Search the buffer for the delimiter byte
    /// 2. If found → extract everything up to and including delimiter, return it
    /// 3. If not found → recv() more data, go to step 1
    /// 4. If EOF → return whatever is in the buffer (may be empty)
    /// ```
    public func readUntil(_ delimiter: UInt8) throws -> Data {
        while true {
            // Search for the delimiter in the buffer
            if let delimIndex = readBuffer.firstIndex(of: delimiter) {
                let end = readBuffer.index(after: delimIndex)
                let result = Data(readBuffer[readBuffer.startIndex..<end])
                readBuffer = Data(readBuffer[end...])
                return result
            }

            // Not found — read more
            let bytesRead = try fillBuffer()
            if bytesRead == 0 {
                // EOF — return whatever we have
                let remaining = readBuffer
                readBuffer = Data()
                return remaining
            }
        }
    }

    // ── Public API: Writing ───────────────────────────────────────────

    /// Write all bytes to the connection.
    ///
    /// Unlike a single `send()` call, which might send fewer bytes than
    /// requested (a "short write"), this function loops until every byte
    /// has been delivered to the OS.
    ///
    /// ## Algorithm
    ///
    /// ```text
    /// offset = 0
    /// while offset < data.count:
    ///     sent = send(fd, data[offset:], remaining, 0)
    ///     if sent == -1 → throw error
    ///     offset += sent
    /// ```
    ///
    /// You **must** call `flush()` after writing a complete message to ensure
    /// data is actually sent over the wire. (In this implementation, writeAll
    /// sends directly, so flush is a no-op — but calling it maintains the
    /// contract for future buffered-write implementations.)
    ///
    /// ## Errors
    ///
    /// - `TcpError.brokenPipe` if the remote side has closed the connection
    public func writeAll(_ data: Data) throws {
        guard !data.isEmpty else { return }

        var offset = 0
        while offset < data.count {
            let remaining = data.count - offset
            let sent = data.withUnsafeBytes { rawBuffer -> Int in
                let ptr = rawBuffer.baseAddress!.advanced(by: offset)
                return send(fd, ptr, remaining, 0)
            }

            if sent < 0 {
                throw mapErrno("write")
            }
            offset += sent
        }
    }

    /// Flush the write buffer, sending all buffered data to the network.
    ///
    /// In this implementation, `writeAll()` sends data directly via `send()`,
    /// so `flush()` is a no-op. However, calling it after every complete
    /// message is good practice — it documents intent and ensures correctness
    /// if the implementation ever adds write buffering.
    public func flush() throws {
        // No-op: writeAll() sends directly. Future implementations might
        // batch writes in a buffer and flush here.
    }

    // ── Public API: Connection management ─────────────────────────────

    /// Shut down the write half of the connection (half-close).
    ///
    /// Signals to the remote side that no more data will be sent. The
    /// read half remains open — you can still receive data.
    ///
    /// ```text
    /// Before shutdownWrite():
    ///   Client <-> Server  (full-duplex, both directions open)
    ///
    /// After shutdownWrite():
    ///   Client <- Server   (client can still READ)
    ///   Client X  Server   (client can no longer WRITE)
    /// ```
    ///
    /// This is the TCP equivalent of saying "I'm done talking, but I'm still
    /// listening." It causes the remote side's next `recv()` to return 0 (EOF).
    public func shutdownWrite() throws {
        let result = shutdown(fd, Int32(SHUT_WR))
        if result < 0 {
            throw mapErrno("shutdown")
        }
    }

    /// Returns the remote address (IP string and port) of this connection.
    ///
    /// Calls `getpeername()` on the underlying socket to retrieve the address
    /// the socket is connected to.
    public func peerAddr() throws -> (String, UInt16) {
        var addr = sockaddr_storage()
        var addrLen = socklen_t(MemoryLayout<sockaddr_storage>.size)
        let result = withUnsafeMutablePointer(to: &addr) { storagePtr in
            storagePtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                getpeername(fd, sockPtr, &addrLen)
            }
        }
        if result < 0 {
            throw mapErrno("getpeername")
        }
        return extractAddress(from: &addr)
    }

    /// Returns the local address (IP string and port) of this connection.
    ///
    /// Calls `getsockname()` on the underlying socket to retrieve the
    /// local endpoint address.
    public func localAddr() throws -> (String, UInt16) {
        var addr = sockaddr_storage()
        var addrLen = socklen_t(MemoryLayout<sockaddr_storage>.size)
        let result = withUnsafeMutablePointer(to: &addr) { storagePtr in
            storagePtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                getsockname(fd, sockPtr, &addrLen)
            }
        }
        if result < 0 {
            throw mapErrno("getsockname")
        }
        return extractAddress(from: &addr)
    }

    /// Close the connection, releasing the file descriptor.
    ///
    /// Safe to call multiple times — subsequent calls are no-ops.
    /// Also called automatically in `deinit` if the caller forgets.
    public func close() {
        guard !closed else { return }
        closed = true
        #if canImport(Darwin)
        Darwin.close(fd)
        #elseif canImport(Glibc)
        Glibc.close(fd)
        #endif
        fd = -1
    }
}

// ============================================================================
// Helper: extract IP and port from a sockaddr_storage
// ============================================================================
//
// POSIX socket addresses come in different flavors:
//
// ```text
// sockaddr_in  (AF_INET)  → IPv4: 4-byte address + 2-byte port
// sockaddr_in6 (AF_INET6) → IPv6: 16-byte address + 2-byte port
// sockaddr_storage        → big enough to hold either
// ```
//
// We check the `ss_family` field to determine which type, then cast and
// extract the address and port.

/// Extract an IP string and port number from a sockaddr_storage.
internal func extractAddress(from storage: inout sockaddr_storage) -> (String, UInt16) {
    if storage.ss_family == sa_family_t(AF_INET) {
        // IPv4
        return withUnsafePointer(to: &storage) { ptr in
            ptr.withMemoryRebound(to: sockaddr_in.self, capacity: 1) { addrPtr in
                var ipBuffer = [CChar](repeating: 0, count: Int(INET_ADDRSTRLEN))
                var inAddr = addrPtr.pointee.sin_addr
                inet_ntop(AF_INET, &inAddr, &ipBuffer, socklen_t(INET_ADDRSTRLEN))
                let ip = String(cString: ipBuffer)
                let port = UInt16(bigEndian: addrPtr.pointee.sin_port)
                return (ip, port)
            }
        }
    } else {
        // IPv6
        return withUnsafePointer(to: &storage) { ptr in
            ptr.withMemoryRebound(to: sockaddr_in6.self, capacity: 1) { addrPtr in
                var ipBuffer = [CChar](repeating: 0, count: Int(INET6_ADDRSTRLEN))
                var in6Addr = addrPtr.pointee.sin6_addr
                inet_ntop(AF_INET6, &in6Addr, &ipBuffer, socklen_t(INET6_ADDRSTRLEN))
                let ip = String(cString: ipBuffer)
                let port = UInt16(bigEndian: addrPtr.pointee.sin6_port)
                return (ip, port)
            }
        }
    }
}

// ============================================================================
// tcpConnect() — establish a TCP connection
// ============================================================================

/// Establish a TCP connection to the given host and port.
///
/// ## Algorithm
///
/// ```text
/// 1. DNS resolution: (host, port) → [addr1, addr2, ...]
///    Uses getaddrinfo() — the OS resolver (respects /etc/hosts, system DNS).
///
/// 2. Create a socket: socket(AF_INET, SOCK_STREAM, 0)
///    SOCK_STREAM = TCP (reliable, ordered byte stream)
///
/// 3. Non-blocking connect with timeout:
///    a. Set socket to non-blocking via fcntl(O_NONBLOCK)
///    b. Call connect() — returns immediately with EINPROGRESS
///    c. Use select() to wait for the socket to become writable
///    d. Check for connection errors via getsockopt(SO_ERROR)
///    e. Restore blocking mode
///
/// 4. Configure the connected socket:
///    Set SO_RCVTIMEO and SO_SNDTIMEO via setsockopt()
///
/// 5. Return a TcpConnection wrapping the file descriptor
/// ```
///
/// ## Why non-blocking connect?
///
/// ```text
/// Blocking connect (the default):
///   connect() blocks for up to ~75 seconds on Linux
///   No way to control the timeout
///
/// Non-blocking connect:
///   connect() returns immediately → EINPROGRESS
///   select() waits with our chosen timeout
///   If select() times out → we control the error
/// ```
///
/// ## Example
///
/// ```swift
/// let opts = ConnectOptions(connectTimeout: 10)
/// let conn = try tcpConnect(host: "example.com", port: 80, options: opts)
/// ```
public func tcpConnect(
    host: String,
    port: UInt16,
    options: ConnectOptions = .default
) throws -> TcpConnection {
    // ── Step 1: DNS resolution ────────────────────────────────────────
    //
    // `getaddrinfo()` is the POSIX standard for hostname resolution. It
    // supports both IPv4 and IPv6, and respects the system's resolver
    // configuration (/etc/resolv.conf, /etc/hosts, mDNS, etc.).
    //
    // ```text
    // hints.ai_family   = AF_UNSPEC   → accept both IPv4 and IPv6
    // hints.ai_socktype = SOCK_STREAM → TCP only (not UDP)
    // ```

    var hints = addrinfo()
    hints.ai_family = AF_UNSPEC
    // On Darwin, SOCK_STREAM is an Int32 constant. On Linux (Glibc),
    // it's a __socket_type enum — use .rawValue to extract the Int32.
    #if canImport(Darwin)
    hints.ai_socktype = SOCK_STREAM
    hints.ai_protocol = IPPROTO_TCP
    #elseif canImport(Glibc)
    hints.ai_socktype = Int32(SOCK_STREAM.rawValue)
    hints.ai_protocol = Int32(IPPROTO_TCP)
    #endif

    var resultPtr: UnsafeMutablePointer<addrinfo>?
    let portStr = String(port)
    let status = getaddrinfo(host, portStr, &hints, &resultPtr)

    if status != 0 {
        let message: String
        if let errStr = gai_strerror(status) {
            message = String(cString: errStr)
        } else {
            message = "unknown error (code \(status))"
        }
        throw TcpError.dnsResolutionFailed(host: host, message: message)
    }

    guard let firstResult = resultPtr else {
        throw TcpError.dnsResolutionFailed(host: host, message: "no addresses found")
    }

    // Ensure we free the addrinfo linked list when done, no matter what.
    defer { freeaddrinfo(firstResult) }

    // Collect all resolved addresses into an array for iteration.
    // getaddrinfo() returns a linked list of addrinfo structs — one per
    // resolved address. A single hostname can resolve to multiple addresses
    // (IPv4 + IPv6, or multiple A/AAAA records).
    var addresses: [addrinfo] = []
    var current: UnsafeMutablePointer<addrinfo>? = firstResult
    while let addr = current {
        addresses.append(addr.pointee)
        current = addr.pointee.ai_next
    }

    if addresses.isEmpty {
        throw TcpError.dnsResolutionFailed(host: host, message: "no addresses found")
    }

    // ── Step 2-3: Try each address with timeout ───────────────────────
    //
    // If the hostname resolved to [IPv4, IPv6], we try each in order.
    // This is a simplified version of the "Happy Eyeballs" algorithm
    // (RFC 6555).

    var lastError: TcpError = .ioError("no addresses to try")

    for addrInfo in addresses {
        do {
            let conn = try attemptConnect(addrInfo: addrInfo, options: options)
            return conn
        } catch let error as TcpError {
            lastError = error
            continue
        }
    }

    throw lastError
}

// ============================================================================
// attemptConnect — try connecting to a single resolved address
// ============================================================================
//
// This function implements the non-blocking connect pattern:
//
// ```text
// 1. socket()           → create file descriptor
// 2. fcntl(O_NONBLOCK)  → make non-blocking
// 3. connect()          → starts handshake (returns EINPROGRESS)
// 4. select()           → wait for completion with timeout
// 5. getsockopt(SO_ERROR) → check if connect succeeded
// 6. fcntl(~O_NONBLOCK) → restore blocking mode
// 7. setsockopt(timeouts) → configure read/write timeouts
// ```

/// Attempt a TCP connection to a single address with a timeout.
///
/// - Parameters:
///   - addrInfo: The resolved address to connect to
///   - options: Connection options (timeouts, buffer size)
/// - Returns: A configured TcpConnection
/// - Throws: TcpError if the connection fails
internal func attemptConnect(
    addrInfo: addrinfo,
    options: ConnectOptions
) throws -> TcpConnection {

    // Step 1: Create the socket
    //
    // socket(family, type, protocol) creates a new file descriptor.
    // ```text
    // family   = AF_INET or AF_INET6 (from DNS resolution)
    // type     = SOCK_STREAM (TCP — reliable byte stream)
    // protocol = 0 (let the OS pick — always TCP for SOCK_STREAM)
    // ```
    let fd = socket(addrInfo.ai_family, addrInfo.ai_socktype, addrInfo.ai_protocol)
    if fd < 0 {
        throw mapErrno("socket")
    }

    // Prevent SIGPIPE from killing the process when writing to a closed socket.
    // On Darwin, SO_NOSIGPIPE is a per-socket option. On Linux, we ignore
    // SIGPIPE globally (MSG_NOSIGNAL is the per-send alternative, but we use
    // POSIX write() not send(), so the global ignore is simpler).
    #if canImport(Darwin)
    var noSigPipe: Int32 = 1
    setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &noSigPipe, socklen_t(MemoryLayout<Int32>.size))
    #elseif canImport(Glibc)
    signal(SIGPIPE, SIG_IGN)
    #endif

    // If anything fails from here, close the socket to prevent leaks.
    var success = false
    defer {
        if !success {
            #if canImport(Darwin)
            Darwin.close(fd)
            #elseif canImport(Glibc)
            Glibc.close(fd)
            #endif
        }
    }

    // Step 2: Set non-blocking mode for connect with timeout
    //
    // By default, `connect()` blocks until the TCP handshake completes
    // or the OS gives up (which can take minutes). We want to control
    // the timeout ourselves, so we set the socket to non-blocking mode
    // first, then use `select()` to wait with our timeout.
    let flags = fcntl(fd, F_GETFL)
    if flags < 0 {
        throw mapErrno("fcntl(F_GETFL)")
    }
    if fcntl(fd, F_SETFL, flags | O_NONBLOCK) < 0 {
        throw mapErrno("fcntl(F_SETFL)")
    }

    // Step 3: Start the TCP handshake
    //
    // In non-blocking mode, connect() returns immediately with:
    //   0           → connection completed instantly (rare, usually localhost)
    //   -1 + EINPROGRESS → handshake started, use select() to wait
    //   -1 + other  → actual error
    let connectResult = connect(fd, addrInfo.ai_addr, addrInfo.ai_addrlen)

    if connectResult < 0 && errno != EINPROGRESS {
        throw mapErrno("connect")
    }

    if connectResult != 0 {
        // Step 4: Wait for connection with timeout using select()
        //
        // select() monitors file descriptors for readability/writability.
        // A non-blocking connect() signals writability when the handshake
        // completes (either success or failure).
        //
        // ```text
        // select(nfds, readfds, writefds, exceptfds, timeout)
        //   nfds     = fd + 1 (highest fd number + 1)
        //   readfds  = NULL (we don't care about readability)
        //   writefds = {fd} (we want to know when connect finishes)
        //   exceptfds = NULL
        //   timeout  = our connectTimeout
        //
        // Returns:
        //   > 0 → fd is ready (connect finished — check SO_ERROR)
        //   0   → timeout expired
        //   -1  → error
        // ```
        var writeSet = fd_set()
        fdZero(&writeSet)
        fdSet(fd, &writeSet)

        var tv = timevalFromInterval(options.connectTimeout)

        let selectResult = select(fd + 1, nil, &writeSet, nil, &tv)

        if selectResult == 0 {
            // Timeout — the TCP handshake didn't complete in time
            throw TcpError.timeout(phase: "connect", duration: options.connectTimeout)
        } else if selectResult < 0 {
            throw mapErrno("select")
        }

        // Step 5: Check if the connect actually succeeded
        //
        // select() returning > 0 just means the connect *finished* — it
        // might have finished with an error. We check with getsockopt().
        var connectError: Int32 = 0
        var errorLen = socklen_t(MemoryLayout<Int32>.size)
        if getsockopt(fd, SOL_SOCKET, SO_ERROR, &connectError, &errorLen) < 0 {
            throw mapErrno("getsockopt")
        }

        if connectError != 0 {
            // The connect failed — map the error code
            errno = connectError
            throw mapErrno("connect")
        }
    }

    // Step 6: Restore blocking mode
    //
    // Now that connect() succeeded, we switch back to blocking mode.
    // This way, subsequent recv() and send() calls will block (with
    // timeouts set via SO_RCVTIMEO/SO_SNDTIMEO).
    if fcntl(fd, F_SETFL, flags & ~O_NONBLOCK) < 0 {
        throw mapErrno("fcntl(restore blocking)")
    }

    // Step 7: Configure read/write timeouts
    //
    // SO_RCVTIMEO: if recv() blocks longer than this → EAGAIN/EWOULDBLOCK
    // SO_SNDTIMEO: if send() blocks longer than this → EAGAIN/EWOULDBLOCK
    if let readTimeout = options.readTimeout {
        var tv = timevalFromInterval(readTimeout)
        if setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size)) < 0 {
            throw mapErrno("setsockopt(SO_RCVTIMEO)")
        }
    }

    if let writeTimeout = options.writeTimeout {
        var tv = timevalFromInterval(writeTimeout)
        if setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size)) < 0 {
            throw mapErrno("setsockopt(SO_SNDTIMEO)")
        }
    }

    // Success — transfer ownership of the fd to TcpConnection
    success = true
    return TcpConnection(fd: fd, bufferSize: options.bufferSize)
}

// ============================================================================
// fd_set helpers — portable wrappers for FD_ZERO, FD_SET, FD_ISSET
// ============================================================================
//
// The POSIX fd_set type and its macros (FD_ZERO, FD_SET, FD_ISSET) are
// implemented as C macros, which Swift cannot call directly. We provide
// portable Swift equivalents.
//
// ## How fd_set works (simplified)
//
// ```text
// An fd_set is a bitmask — each bit position represents a file descriptor.
//
// fd_set for fds 0, 2, 5:
//   bit:  7 6 5 4 3 2 1 0
//   val:  0 0 1 0 0 1 0 1
//                ^     ^   ^
//               fd5   fd2 fd0
//
// FD_ZERO → clear all bits
// FD_SET  → set bit N
// FD_ISSET → test bit N
// ```

/// Clear all bits in an fd_set (equivalent to FD_ZERO).
internal func fdZero(_ set: inout fd_set) {
    #if canImport(Darwin)
    // Darwin's fd_set uses an array of Int32 called fds_bits
    set.fds_bits = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    #elseif canImport(Glibc)
    // Glibc's fd_set uses __fds_bits
    set.__fds_bits = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    #endif
}

/// Set a file descriptor's bit in an fd_set (equivalent to FD_SET).
internal func fdSet(_ fd: Int32, _ set: inout fd_set) {
    #if canImport(Darwin)
    // On Darwin, fds_bits is a tuple of 32 Int32 values.
    // Each Int32 holds 32 bits, so fd N is in element (N/32), bit (N%32).
    let intOffset = Int(fd) / 32
    let bitOffset = Int(fd) % 32
    let mask = Int32(1 << bitOffset)
    withUnsafeMutablePointer(to: &set.fds_bits) { tuplePtr in
        tuplePtr.withMemoryRebound(to: Int32.self, capacity: 32) { ptr in
            ptr[intOffset] |= mask
        }
    }
    #elseif canImport(Glibc)
    // On Glibc, __fds_bits is a tuple of 16 Int values (64-bit each).
    let intOffset = Int(fd) / 64
    let bitOffset = Int(fd) % 64
    let mask = Int(1 << bitOffset)
    withUnsafeMutablePointer(to: &set.__fds_bits) { tuplePtr in
        tuplePtr.withMemoryRebound(to: Int.self, capacity: 16) { ptr in
            ptr[intOffset] |= mask
        }
    }
    #endif
}

/// Test if a file descriptor's bit is set in an fd_set (equivalent to FD_ISSET).
internal func fdIsSet(_ fd: Int32, _ set: inout fd_set) -> Bool {
    #if canImport(Darwin)
    let intOffset = Int(fd) / 32
    let bitOffset = Int(fd) % 32
    let mask = Int32(1 << bitOffset)
    return withUnsafeMutablePointer(to: &set.fds_bits) { tuplePtr in
        tuplePtr.withMemoryRebound(to: Int32.self, capacity: 32) { ptr in
            (ptr[intOffset] & mask) != 0
        }
    }
    #elseif canImport(Glibc)
    let intOffset = Int(fd) / 64
    let bitOffset = Int(fd) % 64
    let mask = Int(1 << bitOffset)
    return withUnsafeMutablePointer(to: &set.__fds_bits) { tuplePtr in
        tuplePtr.withMemoryRebound(to: Int.self, capacity: 16) { ptr in
            (ptr[intOffset] & mask) != 0
        }
    }
    #endif
}
