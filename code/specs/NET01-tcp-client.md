# NET01 — TCP Client

## Overview

A TCP client is the fundamental building block for any networked application. It
opens a connection to a remote machine, sends bytes, receives bytes, and closes
the connection. Every web browser, email client, chat program, and database
driver starts with a TCP client.

This package wraps Rust's `std::net::TcpStream` with ergonomic defaults:
timeouts, buffering, and clean error handling. It is protocol-agnostic — it
knows nothing about HTTP, SMTP, or Redis. It just moves bytes reliably between
two machines. Higher-level packages (NET05 http1.0-client, DT25 mini-redis)
build application protocols on top of this foundation.

**Analogy: A telephone call.**

```
Making a TCP connection is like making a phone call:

1. DIAL (DNS + connect)
   You look up "Grandma" in your contacts → 555-0123 (DNS resolution)
   You dial the number and wait for it to ring   (TCP three-way handshake)
   If nobody picks up after 30 seconds, you hang up (connect timeout)

2. TALK (read/write)
   You say "Hello, Grandma!"                      (write_all)
   You listen for her response                     (read_line)
   If she goes silent for 30 seconds, you ask
   "Are you still there?"                          (read timeout)

3. HANG UP (shutdown/close)
   You say "Goodbye" and hang up                   (shutdown + close)
   Half-close: you can say "I'm done talking"
   but keep listening for her last words            (shutdown(Write))
```

## Where It Fits

```
User types "http://info.cern.ch/hypertext/WWW/TheProject.html"
     │
     ▼
url-parser (NET00) — parse into scheme, host, port, path
     │  Url { scheme: "http", host: "info.cern.ch",
     │        port: 80, path: "/hypertext/WWW/TheProject.html" }
     ▼
tcp-client (NET01) ← THIS PACKAGE
     │  TcpConnection to info.cern.ch:80
     │  (raw byte stream — no HTTP knowledge)
     ▼
frame-extractor (NET02) — extract HTTP frames from byte stream
     │
     ▼
http1.0-lexer (NET03) — tokenize HTTP response
     │
     ▼
http1.0-parser (NET04) — parse into structured response
     │
     ▼
http1.0-client (NET05) — high-level "fetch this URL" API
```

**Depends on:** nothing (std only)
**Depended on by:** http1.0-client (NET05), future SMTP/FTP clients, RESP protocol (DT23)

**Important:** This package uses real OS sockets via `std::net`. It is NOT
related to the D17 simulated network stack, which is an educational simulation
of TCP/IP internals. This package calls into the operating system's actual
networking APIs.

---

## Concepts

### Why TCP (vs UDP)?

The internet offers two main transport protocols:

```
TCP (Transmission Control Protocol)         UDP (User Datagram Protocol)
─────────────────────────────────           ──────────────────────────────
Reliable: every byte arrives                Best-effort: packets can be lost
Ordered: bytes arrive in send order         Unordered: packets may arrive jumbled
Connection-oriented: dial first, then talk  Connectionless: just send packets
Flow control: won't overwhelm receiver      No flow control
Slower to start (handshake)                 No handshake overhead

Used by: HTTP, SMTP, FTP, SSH, Redis       Used by: DNS, video streaming, games
```

TCP is the right choice for protocols where **every byte matters and order
matters**. When a web browser fetches a page, a missing or reordered byte would
corrupt the HTML. TCP guarantees that the bytes arrive completely and in order,
or the connection fails with an error — there is no silent data loss.

### The Connection Lifecycle

A TCP connection goes through four phases:

```
Phase 1: DNS Resolution
  "info.cern.ch" → 188.184.21.108
  The hostname is a human-readable alias. The OS resolver
  (or /etc/hosts) translates it to an IP address.
  This can fail: the domain might not exist, the DNS
  server might be unreachable, or resolution might time out.

Phase 2: TCP Connect (Three-Way Handshake)
  Client → Server:  SYN        "I want to connect"
  Server → Client:  SYN-ACK    "OK, I accept"
  Client → Server:  ACK        "Great, we're connected"

  This establishes the connection. It can fail if the server
  is not listening (ConnectionRefused), the network is down,
  or the handshake takes too long (Timeout).

Phase 3: Data Transfer (Read/Write)
  Once connected, both sides can send and receive bytes at
  any time. TCP is full-duplex — reading and writing happen
  independently.

  Reads can block until data arrives, or time out.
  Writes can block if the send buffer is full, or time out.

Phase 4: Shutdown and Close
  Either side can initiate shutdown:
  - shutdown(Write) → "I'm done sending" (half-close)
  - drop/close → fully close the connection

  A clean shutdown lets the other side read any remaining
  data before the connection is torn down.
```

### Buffered I/O: Why You Need It

TCP is a **byte stream**, not a message stream. When you call `read()` on a raw
`TcpStream`, you might get 1 byte, or 1000 bytes, or any amount — it depends on
how the OS batched the network packets.

```
What the server sends (logically):     "HTTP/1.0 200 OK\r\n"
What raw read() might return:          "HTTP/1"  then  ".0 200"  then  " OK\r\n"
```

This makes raw `read()` painful to use. You would need to manually accumulate
bytes in a buffer and scan for delimiters. **BufReader** solves this:

```
BufReader wraps TcpStream and adds an internal buffer (default: 8 KiB).
It reads large chunks from the OS and serves them to your code in
convenient pieces:

  read_line()       → reads until \n, returns the whole line
  read_until(b)     → reads until delimiter byte b appears
  read_exact(n)     → reads exactly n bytes, blocking until all arrive

Without buffering, reading 100 lines means 100+ system calls.
With buffering, it might be just 1-2 system calls, with the rest
served from the in-memory buffer.
```

Similarly, **BufWriter** batches small writes into larger chunks before flushing
to the OS, reducing the number of system calls and avoiding tiny TCP packets.

### Timeouts: Connect vs Read vs Write

Three different things can time out, and they need independent settings:

```
Connect timeout (default: 30s)
  How long to wait for the TCP handshake to complete.
  If a server is down or firewalled, the OS might wait
  minutes before giving up. A 30s timeout is reasonable.

Read timeout (default: 30s)
  How long to wait for data after calling read().
  A well-behaved server should respond promptly, but
  a slow or overloaded server might stall. Without a
  read timeout, your program hangs forever.

Write timeout (default: 30s)
  How long to wait for the OS to accept your data.
  This usually completes instantly (data goes to the
  OS send buffer), but can block if the buffer is full
  because the remote side is not reading.
```

### Half-Close: "I'm Done Talking, But Still Listening"

TCP connections are full-duplex: data flows in both directions independently.
A **half-close** shuts down one direction while keeping the other open:

```
Normal close:
  Client: shutdown(Write) + shutdown(Read)  → connection fully closed

Half-close:
  Client: shutdown(Write)                   → "I have nothing more to send"
  Server: reads remaining data, sends final response
  Client: reads the final response
  Client: connection fully closed when both sides are done

Why this matters:
  HTTP/1.0 uses this pattern. The client sends the request,
  then signals "I'm done" with a half-close. The server reads
  the complete request, sends the response, and closes. Without
  half-close, the server would not know when the request ends.
```

---

## Public API

```rust
use std::time::Duration;

// ─── Connection Options ───────────────────────────────────────────────

/// Configuration for establishing a TCP connection.
///
/// All timeouts default to 30 seconds. The buffer size defaults to 8192
/// bytes (8 KiB), which is a good balance between memory usage and
/// system call reduction.
///
/// # Example
///
/// ```rust
/// let opts = ConnectOptions::default()
///     .connect_timeout(Duration::from_secs(10))
///     .read_timeout(Duration::from_secs(60));
/// ```
pub struct ConnectOptions {
    /// Maximum time to wait for the TCP handshake to complete.
    /// Default: 30 seconds.
    pub connect_timeout: Duration,

    /// Maximum time to wait for data when reading.
    /// Default: 30 seconds. Set to None for no timeout (blocks forever).
    pub read_timeout: Option<Duration>,

    /// Maximum time to wait when writing data.
    /// Default: 30 seconds. Set to None for no timeout (blocks forever).
    pub write_timeout: Option<Duration>,

    /// Size of the internal read and write buffers in bytes.
    /// Default: 8192 (8 KiB).
    pub buffer_size: usize,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        ConnectOptions {
            connect_timeout: Duration::from_secs(30),
            read_timeout: Some(Duration::from_secs(30)),
            write_timeout: Some(Duration::from_secs(30)),
            buffer_size: 8192,
        }
    }
}

// ─── Connect ──────────────────────────────────────────────────────────

/// Establish a TCP connection to the given host and port.
///
/// This function:
/// 1. Resolves the hostname to one or more IP addresses using the OS
///    DNS resolver (via `std::net::ToSocketAddrs`).
/// 2. Attempts to connect to each resolved address in order, using
///    the configured connect timeout.
/// 3. Configures read/write timeouts on the resulting socket.
/// 4. Wraps the socket in buffered reader/writer.
///
/// # Arguments
///
/// * `host` — A hostname ("info.cern.ch") or IP address ("93.184.216.34")
/// * `port` — The TCP port number (e.g., 80 for HTTP, 25 for SMTP)
/// * `options` — Connection configuration (timeouts, buffer size)
///
/// # Example
///
/// ```rust
/// let conn = connect("info.cern.ch", 80, ConnectOptions::default())?;
/// conn.write_all(b"GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n")?;
/// let response = conn.read_line()?;
/// ```
pub fn connect(
    host: &str,
    port: u16,
    options: ConnectOptions,
) -> Result<TcpConnection, TcpError> { ... }

// ─── TcpConnection ───────────────────────────────────────────────────

/// A TCP connection with buffered I/O and configured timeouts.
///
/// TcpConnection wraps a `TcpStream` in a `BufReader` and `BufWriter`
/// for efficient, line-oriented or chunk-oriented communication.
///
/// The connection is automatically closed when dropped.
pub struct TcpConnection { /* internal: BufReader<TcpStream>, BufWriter<TcpStream> */ }

impl TcpConnection {
    /// Read bytes until a newline (`\n`) is found.
    ///
    /// Returns the line INCLUDING the trailing `\n` (and `\r\n` if
    /// present). Returns an empty string at EOF (remote closed).
    ///
    /// This is the workhorse for line-oriented protocols like HTTP/1.0,
    /// SMTP, and RESP.
    ///
    /// # Errors
    ///
    /// Returns `TcpError::Timeout` if no data arrives within the read
    /// timeout. Returns `TcpError::ConnectionReset` if the remote side
    /// closed unexpectedly.
    pub fn read_line(&mut self) -> Result<String, TcpError> { ... }

    /// Read exactly `n` bytes from the connection.
    ///
    /// Blocks until all `n` bytes have been received. This is useful
    /// for protocols that specify an exact content length (e.g.,
    /// HTTP Content-Length header).
    ///
    /// # Errors
    ///
    /// Returns `TcpError::UnexpectedEof` if the connection closes
    /// before `n` bytes are read.
    pub fn read_exact(&mut self, n: usize) -> Result<Vec<u8>, TcpError> { ... }

    /// Read bytes until the given delimiter byte is found.
    ///
    /// Returns all bytes up to AND including the delimiter.
    /// Useful for protocols with custom delimiters (e.g., RESP uses
    /// `\r\n`, null-terminated strings use `\0`).
    pub fn read_until(&mut self, delimiter: u8) -> Result<Vec<u8>, TcpError> { ... }

    /// Write all bytes to the connection.
    ///
    /// Buffers the data internally and flushes when the buffer is full
    /// or when `flush()` is called explicitly. For request/response
    /// protocols, call `flush()` after writing the complete request
    /// to ensure it is actually sent.
    ///
    /// # Errors
    ///
    /// Returns `TcpError::BrokenPipe` if the remote side has closed
    /// the connection.
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), TcpError> { ... }

    /// Flush the write buffer, sending all buffered data to the network.
    ///
    /// You MUST call this after writing a complete message. Without
    /// flushing, data may sit in the BufWriter and never reach the
    /// server.
    pub fn flush(&mut self) -> Result<(), TcpError> { ... }

    /// Shut down the write half of the connection (half-close).
    ///
    /// Signals to the remote side that no more data will be sent.
    /// The read half remains open — you can still read responses.
    ///
    /// This is equivalent to calling `shutdown(Shutdown::Write)` on
    /// the underlying socket.
    pub fn shutdown_write(&mut self) -> Result<(), TcpError> { ... }

    /// Returns the remote address (IP and port) of this connection.
    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, TcpError> { ... }

    /// Returns the local address (IP and port) of this connection.
    pub fn local_addr(&self) -> Result<std::net::SocketAddr, TcpError> { ... }
}
```

### Error Types

```rust
/// Errors that can occur during TCP connection and communication.
///
/// Each variant corresponds to a specific failure mode. Application
/// code should match on these to decide how to recover.
pub enum TcpError {
    /// DNS resolution failed — the hostname could not be resolved
    /// to any IP address.
    ///
    /// Common causes: typo in hostname, no internet connection,
    /// DNS server unreachable.
    DnsResolutionFailed {
        host: String,
        message: String,
    },

    /// The remote server actively refused the connection.
    ///
    /// This means the server is reachable but nothing is listening
    /// on the requested port. The server's OS sent a TCP RST packet.
    ConnectionRefused {
        addr: String,
    },

    /// The connection or read/write operation timed out.
    ///
    /// The `phase` field distinguishes where the timeout occurred:
    /// - "connect" — TCP handshake did not complete in time
    /// - "read" — no data received within the read timeout
    /// - "write" — send buffer did not drain within the write timeout
    Timeout {
        phase: String,
        duration: Duration,
    },

    /// The connection was reset by the remote side.
    ///
    /// The server closed the connection unexpectedly (TCP RST),
    /// typically because the server process crashed or was killed.
    ConnectionReset,

    /// The write half of the connection is broken.
    ///
    /// You tried to write to a connection that the remote side has
    /// already closed. This is the "you hung up on me" error.
    BrokenPipe,

    /// The connection closed before the expected amount of data
    /// was received.
    ///
    /// Returned by `read_exact()` when the connection closes before
    /// `n` bytes have been read.
    UnexpectedEof {
        expected: usize,
        received: usize,
    },

    /// A low-level I/O error not covered by the specific variants above.
    ///
    /// Wraps `std::io::Error` for edge cases like permission denied,
    /// address already in use, etc.
    IoError(std::io::Error),
}
```

---

## DNS Resolution

DNS resolution is handled by `std::net::ToSocketAddrs`, which calls the
operating system's resolver. This means it respects `/etc/hosts`, the system DNS
configuration, and local DNS caches.

```
connect("info.cern.ch", 80, opts)

Step 1: "info.cern.ch:80".to_socket_addrs()
  → [188.184.21.108:80, 2001:1458:d00:3c::100:47:80]  (IPv4 + IPv6)

Step 2: Try each address in order:
  → Try 188.184.21.108:80 with connect_timeout
  → If that fails, try 2001:1458:d00:3c::100:47:80
  → If all fail, return the last error
```

The `host` parameter can also be a raw IP address:

```
connect("127.0.0.1", 6379, opts)   // no DNS lookup needed
connect("::1", 6379, opts)          // IPv6 loopback, no DNS lookup
```

---

## Testing Strategy

### Echo Server Tests

Spin up a `std::net::TcpListener` on localhost in the test, then connect
to it with the TCP client. This tests real socket behavior without
needing external services.

1. **Connect and disconnect:** verify connection succeeds to a listening port
2. **Write and read back:** send bytes, echo server returns them, verify match
3. **read_line:** send "Hello\r\nWorld\r\n", verify two lines read correctly
4. **read_exact:** send exactly 100 bytes, read_exact(100), verify match
5. **read_until:** send "key:value\0next", read_until(0), verify returns "key:value\0"
6. **Large data transfer:** send and receive 1 MiB of data, verify integrity
7. **Multiple exchanges:** request-response-request-response pattern

### Timeout Tests

8. **Connect timeout:** attempt to connect to a non-routable address (e.g.,
   `10.255.255.1:1`) with a short timeout, verify `TcpError::Timeout`
9. **Read timeout:** connect to echo server that never sends, verify timeout
10. **Write timeout:** fill the send buffer until it blocks (harder to test
    reliably, may need OS-specific tuning)

### Error Tests

11. **Connection refused:** connect to a port with no listener, verify
    `TcpError::ConnectionRefused`
12. **DNS failure:** connect to "this.host.does.not.exist.example", verify
    `TcpError::DnsResolutionFailed`
13. **Connection reset:** server closes abruptly (RST), verify
    `TcpError::ConnectionReset` on next read
14. **Broken pipe:** server closes connection, client writes, verify
    `TcpError::BrokenPipe`
15. **Unexpected EOF:** server sends 50 bytes then closes, client calls
    `read_exact(100)`, verify `TcpError::UnexpectedEof`

### Half-Close Tests

16. **Client half-close:** client calls `shutdown_write()`, server reads EOF,
    server sends final message, client reads it successfully
17. **Server half-close:** server shuts down write, client reads EOF, client
    can still write (server reads it)

### Edge Cases

18. **Empty read_line at EOF:** connection closed cleanly, read_line returns
    empty string
19. **Zero-byte write:** `write_all(b"")` succeeds without error
20. **Peer address:** `peer_addr()` returns the correct remote address
21. **Flush semantics:** data is not sent until `flush()` is called (verify
    with a server that checks timing)

---

## Scope

**In scope:**
- TCP connect with configurable timeout
- DNS resolution via `std::net::ToSocketAddrs`
- Buffered reading: `read_line()`, `read_exact()`, `read_until()`
- Buffered writing: `write_all()`, `flush()`
- Read and write timeouts
- Graceful shutdown (half-close with `shutdown_write()`)
- Structured error types with actionable information

**Out of scope:**
- TLS/SSL encryption (that will be a separate NET-layer package)
- Connection pooling (belongs in higher-level clients)
- Async / non-blocking I/O (this is synchronous, blocking I/O)
- Unix domain sockets (different address family, different use case)
- UDP (different protocol entirely — see concepts section)
- HTTP or any application-level protocol (this is protocol-agnostic)
- The D17 simulated network stack (that is educational; this uses real OS sockets)

---

## Implementation Languages

This package will be implemented in:
- **Rust** (primary, for the Venture browser pipeline)
- Future: other languages as needed for educational comparison
