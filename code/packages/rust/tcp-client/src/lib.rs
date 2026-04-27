//! # tcp-client
//!
//! A TCP client with buffered I/O and configurable timeouts.
//!
//! This crate wraps `std::net::TcpStream` with ergonomic defaults for
//! building network clients. It is **protocol-agnostic** — it knows nothing
//! about HTTP, SMTP, or Redis. It just moves bytes reliably between two
//! machines. Higher-level packages build application protocols on top.
//!
//! ## Analogy: A telephone call
//!
//! ```text
//! Making a TCP connection is like making a phone call:
//!
//! 1. DIAL (DNS + connect)
//!    Look up "Grandma" → 555-0123     (DNS resolution)
//!    Dial and wait for ring            (TCP three-way handshake)
//!    If nobody picks up → hang up      (connect timeout)
//!
//! 2. TALK (read/write)
//!    Say "Hello, Grandma!"             (write_all + flush)
//!    Listen for response               (read_line)
//!    If silence for 30s → "Still there?" (read timeout)
//!
//! 3. HANG UP (shutdown/close)
//!    Say "Goodbye" and hang up         (shutdown_write + drop)
//! ```
//!
//! ## Where it fits
//!
//! ```text
//! url-parser (NET00) → tcp-client (NET01, THIS) → frame-extractor (NET02)
//!                         ↓
//!                    raw byte stream
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use tcp_client::{connect, ConnectOptions};
//!
//! let mut conn = connect("info.cern.ch", 80, ConnectOptions::default()).unwrap();
//! conn.write_all(b"GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n").unwrap();
//! conn.flush().unwrap();
//! let status_line = conn.read_line().unwrap();
//! println!("{}", status_line);
//! ```

pub const VERSION: &str = "0.1.0";

use std::fmt;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

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
/// connect_timeout (30s) — how long to wait for the TCP handshake
///   If a server is down or firewalled, the OS might wait minutes.
///
/// read_timeout (30s) — how long to wait for data after calling read
///   Without this, a stalled server hangs your program forever.
///
/// write_timeout (30s) — how long to wait for the OS send buffer
///   Usually instant, but blocks if the remote side isn't reading.
/// ```
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// Maximum time to wait for the TCP handshake. Default: 30s.
    pub connect_timeout: Duration,
    /// Maximum time to wait for data on read. Default: Some(30s).
    /// `None` means block forever.
    pub read_timeout: Option<Duration>,
    /// Maximum time to wait on write. Default: Some(30s).
    /// `None` means block forever.
    pub write_timeout: Option<Duration>,
    /// Size of internal read and write buffers in bytes. Default: 8192.
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

// ============================================================================
// TcpError — structured error types
// ============================================================================

/// Errors that can occur during TCP connection and communication.
///
/// Each variant maps to a specific failure mode. Match on these to decide
/// how to recover.
///
/// ```text
/// DnsResolutionFailed → hostname typo or no internet
/// ConnectionRefused   → server is up but nothing listening on that port
/// Timeout             → took too long (connect, read, or write)
/// ConnectionReset     → remote side crashed (TCP RST)
/// BrokenPipe          → tried to write after remote closed
/// UnexpectedEof       → connection closed before expected data arrived
/// IoError             → catch-all for other OS-level errors
/// ```
#[derive(Debug)]
pub enum TcpError {
    /// DNS lookup failed — hostname could not be resolved.
    DnsResolutionFailed { host: String, message: String },
    /// Server is reachable but nothing is listening on the port (TCP RST).
    ConnectionRefused { addr: String },
    /// Operation timed out. `phase` is "connect", "read", or "write".
    Timeout { phase: String, duration: Duration },
    /// Remote side reset the connection unexpectedly (TCP RST during transfer).
    ConnectionReset,
    /// Tried to write to a connection the remote side already closed.
    BrokenPipe,
    /// Connection closed before the expected number of bytes arrived.
    UnexpectedEof { expected: usize, received: usize },
    /// A low-level I/O error not covered by the specific variants.
    IoError(io::Error),
}

impl fmt::Display for TcpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TcpError::DnsResolutionFailed { host, message } => {
                write!(f, "DNS resolution failed for '{}': {}", host, message)
            }
            TcpError::ConnectionRefused { addr } => {
                write!(f, "connection refused by {}", addr)
            }
            TcpError::Timeout { phase, duration } => {
                write!(f, "{} timed out after {:?}", phase, duration)
            }
            TcpError::ConnectionReset => write!(f, "connection reset by peer"),
            TcpError::BrokenPipe => write!(f, "broken pipe (remote closed)"),
            TcpError::UnexpectedEof { expected, received } => {
                write!(
                    f,
                    "unexpected EOF: expected {} bytes, got {}",
                    expected, received
                )
            }
            TcpError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for TcpError {}

/// Map a `std::io::Error` to the most specific `TcpError` variant.
///
/// The IO error kind tells us what went wrong at the OS level:
///
/// ```text
/// ConnectionRefused → server sent TCP RST during handshake
/// TimedOut          → OS timeout expired
/// ConnectionReset   → TCP RST during data transfer
/// BrokenPipe        → write after remote close
/// UnexpectedEof     → connection closed mid-read
/// Other             → wrap in IoError
/// ```
fn map_io_error(err: io::Error) -> TcpError {
    match err.kind() {
        io::ErrorKind::ConnectionRefused => TcpError::ConnectionRefused {
            addr: String::new(),
        },
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => TcpError::Timeout {
            phase: "io".to_string(),
            duration: Duration::ZERO,
        },
        io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted => {
            TcpError::ConnectionReset
        }
        io::ErrorKind::BrokenPipe => TcpError::BrokenPipe,
        io::ErrorKind::UnexpectedEof => TcpError::UnexpectedEof {
            expected: 0,
            received: 0,
        },
        _ => TcpError::IoError(err),
    }
}

// ============================================================================
// connect() — establish a TCP connection
// ============================================================================

/// Establish a TCP connection to the given host and port.
///
/// ## Algorithm
///
/// ```text
/// 1. DNS resolution: (host, port) → [addr1, addr2, ...]
///    Uses the OS resolver (respects /etc/hosts, system DNS).
///
/// 2. Try each address with connect_timeout:
///    addr1 → TcpStream::connect_timeout(addr1, timeout)
///    If that fails → try addr2
///    If all fail → return the last error
///
/// 3. Configure the connected stream:
///    set_read_timeout, set_write_timeout
///
/// 4. Wrap in BufReader + BufWriter → TcpConnection
/// ```
///
/// ## Example
///
/// ```rust,no_run
/// use tcp_client::{connect, ConnectOptions};
/// use std::time::Duration;
///
/// let opts = ConnectOptions {
///     connect_timeout: Duration::from_secs(10),
///     ..ConnectOptions::default()
/// };
/// let mut conn = connect("example.com", 80, opts).unwrap();
/// ```
pub fn connect(host: &str, port: u16, options: ConnectOptions) -> Result<TcpConnection, TcpError> {
    // Step 1: DNS resolution
    //
    // `to_socket_addrs()` calls the OS resolver. It returns an iterator
    // of SocketAddr values — one per resolved IP address. A single hostname
    // can resolve to multiple addresses (IPv4 + IPv6, or multiple A records).
    let addr_string = format!("{}:{}", host, port);
    let addrs: Vec<SocketAddr> = addr_string.to_socket_addrs().map_err(|e| {
        TcpError::DnsResolutionFailed {
            host: host.to_string(),
            message: e.to_string(),
        }
    })?.collect();

    if addrs.is_empty() {
        return Err(TcpError::DnsResolutionFailed {
            host: host.to_string(),
            message: "no addresses found".to_string(),
        });
    }

    // Step 2: Try each resolved address in order
    //
    // If the hostname resolved to [IPv4, IPv6], we try IPv4 first. If it
    // fails (e.g., timeout), we try IPv6. This is a simplified version of
    // the "Happy Eyeballs" algorithm (RFC 6555).
    let mut last_err = None;
    for addr in &addrs {
        match TcpStream::connect_timeout(addr, options.connect_timeout) {
            Ok(stream) => {
                // Step 3: Configure timeouts on the connected stream
                stream
                    .set_read_timeout(options.read_timeout)
                    .map_err(TcpError::IoError)?;
                stream
                    .set_write_timeout(options.write_timeout)
                    .map_err(TcpError::IoError)?;

                // Step 4: Wrap in buffered reader/writer
                //
                // We need two independent handles to the same socket:
                // one for reading, one for writing. `try_clone()` creates
                // a second file descriptor pointing to the same socket.
                let reader_stream = stream.try_clone().map_err(TcpError::IoError)?;
                let reader = BufReader::with_capacity(options.buffer_size, reader_stream);
                let writer = BufWriter::with_capacity(options.buffer_size, stream);

                return Ok(TcpConnection { reader, writer });
            }
            Err(e) => {
                last_err = Some((addr, e));
            }
        }
    }

    // All addresses failed — return the most informative error
    let (addr, err) = last_err.unwrap();
    match err.kind() {
        io::ErrorKind::ConnectionRefused => Err(TcpError::ConnectionRefused {
            addr: addr.to_string(),
        }),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => Err(TcpError::Timeout {
            phase: "connect".to_string(),
            duration: options.connect_timeout,
        }),
        _ => Err(map_io_error(err)),
    }
}

// ============================================================================
// TcpConnection — buffered I/O over a TCP stream
// ============================================================================

/// A TCP connection with buffered I/O and configured timeouts.
///
/// Wraps a `TcpStream` in `BufReader` and `BufWriter` for efficient,
/// line-oriented or chunk-oriented communication.
///
/// ## Why buffered I/O?
///
/// ```text
/// Without buffering:
///   read() returns arbitrary chunks: "HT", "TP/", "1.0 2", "00 OK\r\n"
///   100 read() calls = 100 syscalls (expensive!)
///
/// With BufReader (8 KiB internal buffer):
///   First read() pulls 8 KiB from the OS into memory
///   Subsequent read_line() calls serve data from the buffer
///   100 lines might need only 1-2 syscalls
///
/// Similarly, BufWriter batches small writes into larger chunks
/// before flushing to the OS, avoiding tiny TCP packets.
/// ```
///
/// The connection is automatically closed when dropped.
pub struct TcpConnection {
    /// Buffered reader wrapping the read half of the TCP stream.
    reader: BufReader<TcpStream>,
    /// Buffered writer wrapping the write half of the TCP stream.
    writer: BufWriter<TcpStream>,
}

impl fmt::Debug for TcpConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TcpConnection")
            .field("peer_addr", &self.writer.get_ref().peer_addr().ok())
            .field("local_addr", &self.writer.get_ref().local_addr().ok())
            .finish()
    }
}

impl TcpConnection {
    /// Read bytes until a newline (`\n`) is found.
    ///
    /// Returns the line **including** the trailing `\n` (and `\r\n` if
    /// present). Returns an empty string at EOF (remote closed cleanly).
    ///
    /// This is the workhorse for line-oriented protocols like HTTP/1.0,
    /// SMTP, and RESP (Redis protocol).
    ///
    /// ## Errors
    ///
    /// - `TcpError::Timeout` if no data arrives within the read timeout
    /// - `TcpError::ConnectionReset` if the remote side closed unexpectedly
    pub fn read_line(&mut self) -> Result<String, TcpError> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).map_err(map_io_error)?;
        if bytes_read == 0 {
            // EOF — remote closed the connection cleanly
            return Ok(String::new());
        }
        Ok(line)
    }

    /// Read exactly `n` bytes from the connection.
    ///
    /// Blocks until all `n` bytes have been received. Useful for protocols
    /// that specify an exact content length (e.g., HTTP Content-Length).
    ///
    /// ## Errors
    ///
    /// - `TcpError::UnexpectedEof` if the connection closes before `n` bytes
    pub fn read_exact(&mut self, n: usize) -> Result<Vec<u8>, TcpError> {
        let mut buf = vec![0u8; n];
        match self.reader.read_exact(&mut buf) {
            Ok(()) => Ok(buf),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                // Figure out how many bytes we actually got
                // Unfortunately read_exact doesn't tell us, so we report 0
                Err(TcpError::UnexpectedEof {
                    expected: n,
                    received: 0,
                })
            }
            Err(e) => Err(map_io_error(e)),
        }
    }

    /// Read bytes until the given delimiter byte is found.
    ///
    /// Returns all bytes up to **and including** the delimiter. Useful for
    /// protocols with custom delimiters (RESP uses `\r\n`, null-terminated
    /// strings use `\0`).
    pub fn read_until(&mut self, delimiter: u8) -> Result<Vec<u8>, TcpError> {
        let mut buf = Vec::new();
        self.reader
            .read_until(delimiter, &mut buf)
            .map_err(map_io_error)?;
        Ok(buf)
    }

    /// Write all bytes to the connection.
    ///
    /// Data is buffered internally. You **must** call [`flush()`](Self::flush)
    /// after writing a complete message to ensure it is actually sent.
    ///
    /// ## Errors
    ///
    /// - `TcpError::BrokenPipe` if the remote side has closed the connection
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), TcpError> {
        self.writer.write_all(data).map_err(map_io_error)
    }

    /// Flush the write buffer, sending all buffered data to the network.
    ///
    /// You **must** call this after writing a complete request. Without
    /// flushing, data may sit in the BufWriter and never reach the server.
    pub fn flush(&mut self) -> Result<(), TcpError> {
        self.writer.flush().map_err(map_io_error)
    }

    /// Shut down the write half of the connection (half-close).
    ///
    /// Signals to the remote side that no more data will be sent. The
    /// read half remains open — you can still receive data.
    ///
    /// ```text
    /// Before shutdown_write():
    ///   Client ←→ Server  (full-duplex, both directions open)
    ///
    /// After shutdown_write():
    ///   Client ← Server   (client can still READ)
    ///   Client ✗ Server   (client can no longer WRITE)
    /// ```
    pub fn shutdown_write(&mut self) -> Result<(), TcpError> {
        // Flush any buffered data before shutting down
        self.flush()?;
        self.writer
            .get_ref()
            .shutdown(Shutdown::Write)
            .map_err(map_io_error)
    }

    /// Returns the remote address (IP and port) of this connection.
    pub fn peer_addr(&self) -> Result<SocketAddr, TcpError> {
        self.writer.get_ref().peer_addr().map_err(map_io_error)
    }

    /// Returns the local address (IP and port) of this connection.
    pub fn local_addr(&self) -> Result<SocketAddr, TcpError> {
        self.writer.get_ref().local_addr().map_err(map_io_error)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    // ── Helper: start a local echo server on an OS-assigned port ────────
    //
    // Returns the assigned port. The server runs in a background thread
    // and handles exactly one connection:
    //   1. Accept connection
    //   2. Read data and echo it back
    //   3. Close
    //
    // Using port 0 lets the OS pick an available port, avoiding conflicts
    // when tests run in parallel.

    fn start_echo_server() -> (u16, mpsc::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (stop_tx, stop_rx) = mpsc::channel();

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let mut buf = [0u8; 65536];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            if stream.write_all(&buf[..n]).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
            let _ = stop_rx.try_recv(); // keep channel alive
        });

        // Small delay to let the listener start
        thread::sleep(Duration::from_millis(50));
        (port, stop_tx)
    }

    /// Start a server that accepts but never sends data (for read timeout tests)
    fn start_silent_server() -> (u16, mpsc::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (stop_tx, stop_rx) = mpsc::channel();

        thread::spawn(move || {
            if let Ok((_stream, _)) = listener.accept() {
                // Accept but never send — just hold the connection open
                let _ = stop_rx.recv(); // block until signaled to stop
            }
        });

        thread::sleep(Duration::from_millis(50));
        (port, stop_tx)
    }

    /// Start a server that sends exactly `n` bytes then closes
    fn start_partial_server(data: Vec<u8>) -> (u16, mpsc::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (stop_tx, stop_rx) = mpsc::channel();

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.write_all(&data);
                let _ = stream.flush();
                // Small delay so client can read before we close
                thread::sleep(Duration::from_millis(100));
                drop(stream); // close
            }
            let _ = stop_rx.try_recv();
        });

        thread::sleep(Duration::from_millis(50));
        (port, stop_tx)
    }

    /// Start a server that reads a request, then sends a response
    fn start_request_response_server(response: Vec<u8>) -> (u16, mpsc::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (stop_tx, stop_rx) = mpsc::channel();

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                // Read until we get a blank line (like HTTP)
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                // Send the response
                let _ = stream.write_all(&response);
                let _ = stream.flush();
                thread::sleep(Duration::from_millis(100));
            }
            let _ = stop_rx.try_recv();
        });

        thread::sleep(Duration::from_millis(50));
        (port, stop_tx)
    }

    fn test_options() -> ConnectOptions {
        ConnectOptions {
            connect_timeout: Duration::from_secs(5),
            read_timeout: Some(Duration::from_secs(5)),
            write_timeout: Some(Duration::from_secs(5)),
            buffer_size: 4096,
        }
    }

    // ─── Group 1: Echo server tests ────────────────────────────────────

    #[test]
    fn connect_and_disconnect() {
        let (port, _stop) = start_echo_server();
        let conn = connect("127.0.0.1", port, test_options());
        assert!(conn.is_ok(), "should connect to echo server");
        // Connection is dropped here — auto-closes
    }

    #[test]
    fn write_and_read_back() {
        let (port, _stop) = start_echo_server();
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        conn.write_all(b"Hello, TCP!").unwrap();
        conn.flush().unwrap();

        let mut buf = vec![0u8; 11];
        let result = conn.reader.read_exact(&mut buf);
        assert!(result.is_ok());
        assert_eq!(&buf, b"Hello, TCP!");
    }

    #[test]
    fn read_line_from_echo() {
        let (port, _stop) = start_echo_server();
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        conn.write_all(b"Hello\r\nWorld\r\n").unwrap();
        conn.flush().unwrap();

        let line1 = conn.read_line().unwrap();
        assert_eq!(line1, "Hello\r\n");

        let line2 = conn.read_line().unwrap();
        assert_eq!(line2, "World\r\n");
    }

    #[test]
    fn read_exact_from_echo() {
        let (port, _stop) = start_echo_server();
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        let data: Vec<u8> = (0..100).map(|i| (i % 256) as u8).collect();
        conn.write_all(&data).unwrap();
        conn.flush().unwrap();

        let result = conn.read_exact(100).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn read_until_from_echo() {
        let (port, _stop) = start_echo_server();
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        conn.write_all(b"key:value\0next").unwrap();
        conn.flush().unwrap();

        let result = conn.read_until(b'\0').unwrap();
        assert_eq!(result, b"key:value\0");
    }

    #[test]
    fn large_data_transfer() {
        let (port, _stop) = start_echo_server();
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        // Send 64 KiB (not 1 MiB to keep tests fast)
        let data: Vec<u8> = (0..65536).map(|i| (i % 256) as u8).collect();
        conn.write_all(&data).unwrap();
        conn.flush().unwrap();

        let result = conn.read_exact(65536).unwrap();
        assert_eq!(result.len(), 65536);
        assert_eq!(result, data);
    }

    #[test]
    fn multiple_exchanges() {
        let (port, _stop) = start_echo_server();
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        // Exchange 1
        conn.write_all(b"ping\n").unwrap();
        conn.flush().unwrap();
        let line1 = conn.read_line().unwrap();
        assert_eq!(line1, "ping\n");

        // Exchange 2
        conn.write_all(b"pong\n").unwrap();
        conn.flush().unwrap();
        let line2 = conn.read_line().unwrap();
        assert_eq!(line2, "pong\n");
    }

    // ─── Group 2: Timeout tests ────────────────────────────────────────

    #[test]
    fn connect_timeout() {
        // 10.255.255.1 is a non-routable address — connection will hang
        let opts = ConnectOptions {
            connect_timeout: Duration::from_secs(1),
            ..test_options()
        };
        let result = connect("10.255.255.1", 1, opts);
        assert!(result.is_err());
        match result.unwrap_err() {
            TcpError::Timeout { phase, .. } => assert_eq!(phase, "connect"),
            TcpError::IoError(_) => {} // Some platforms return a generic error
            other => panic!("expected Timeout or IoError, got: {:?}", other),
        }
    }

    #[test]
    fn read_timeout() {
        let (port, _stop) = start_silent_server();
        let opts = ConnectOptions {
            read_timeout: Some(Duration::from_secs(1)),
            ..test_options()
        };
        let mut conn = connect("127.0.0.1", port, opts).unwrap();

        let result = conn.read_line();
        assert!(result.is_err());
        match result.unwrap_err() {
            TcpError::Timeout { .. } => {}
            TcpError::IoError(_) => {} // platform-dependent
            other => panic!("expected Timeout, got: {:?}", other),
        }
    }

    // ─── Group 3: Error tests ──────────────────────────────────────────

    #[test]
    fn connection_refused() {
        // Connect to a port where nothing is listening
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener); // close it immediately — now nothing listens

        let result = connect("127.0.0.1", port, test_options());
        assert!(result.is_err());
        match result.unwrap_err() {
            TcpError::ConnectionRefused { .. } => {}
            TcpError::IoError(_) => {} // some platforms
            other => panic!("expected ConnectionRefused, got: {:?}", other),
        }
    }

    #[test]
    fn dns_failure() {
        let result = connect("this.host.does.not.exist.example", 80, test_options());
        assert!(result.is_err());
        match result.unwrap_err() {
            TcpError::DnsResolutionFailed { host, .. } => {
                assert_eq!(host, "this.host.does.not.exist.example");
            }
            // Some ISP DNS resolvers hijack NXDOMAIN, causing a connection error
            TcpError::ConnectionRefused { .. } => {}
            TcpError::Timeout { .. } => {}
            TcpError::IoError(_) => {}
            other => panic!("expected DnsResolutionFailed, got: {:?}", other),
        }
    }

    #[test]
    fn unexpected_eof() {
        // Server sends 50 bytes then closes, client tries to read 100
        let data: Vec<u8> = (0..50).collect();
        let (port, _stop) = start_partial_server(data);
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        // Wait a moment for the server to send data
        thread::sleep(Duration::from_millis(200));

        let result = conn.read_exact(100);
        assert!(result.is_err());
    }

    #[test]
    fn broken_pipe() {
        // Server accepts then immediately closes
        let (port, _stop) = start_partial_server(vec![]);
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        // Wait for server to close its end
        thread::sleep(Duration::from_millis(300));

        // Try to write — should get BrokenPipe or ConnectionReset
        // We may need multiple writes because the first might succeed
        // (data goes to local OS buffer) before the RST arrives.
        let mut got_error = false;
        for _ in 0..10 {
            let big_data = vec![0u8; 65536];
            if conn.write_all(&big_data).is_err() {
                got_error = true;
                break;
            }
            if conn.flush().is_err() {
                got_error = true;
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert!(got_error, "expected write error after server closed");
    }

    // ─── Group 4: Half-close tests ─────────────────────────────────────

    #[test]
    fn client_half_close() {
        // Server reads until EOF, then sends "DONE\n"
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut buf = Vec::new();
            let mut tmp = [0u8; 1024];
            loop {
                match stream.read(&mut tmp) {
                    Ok(0) => break, // EOF — client shut down write
                    Ok(n) => buf.extend_from_slice(&tmp[..n]),
                    Err(_) => break,
                }
            }
            // Client is done writing. Send final response.
            stream.write_all(b"DONE\n").unwrap();
            stream.flush().unwrap();
            buf
        });

        thread::sleep(Duration::from_millis(50));
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        conn.write_all(b"request data").unwrap();
        conn.shutdown_write().unwrap();

        // Now read the server's response
        let response = conn.read_line().unwrap();
        assert_eq!(response, "DONE\n");

        // Verify server received our data
        let server_received = handle.join().unwrap();
        assert_eq!(server_received, b"request data");
    }

    // ─── Group 5: Edge cases ───────────────────────────────────────────

    #[test]
    fn empty_read_at_eof() {
        let data = b"hello\n".to_vec();
        let (port, _stop) = start_partial_server(data);
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        // Wait for server to send and close
        thread::sleep(Duration::from_millis(200));

        let line = conn.read_line().unwrap();
        assert_eq!(line, "hello\n");

        // Next read should return empty string (EOF)
        let eof = conn.read_line().unwrap();
        assert_eq!(eof, "");
    }

    #[test]
    fn zero_byte_write() {
        let (port, _stop) = start_echo_server();
        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        // Writing zero bytes should succeed without error
        let result = conn.write_all(b"");
        assert!(result.is_ok());
    }

    #[test]
    fn peer_address() {
        let (port, _stop) = start_echo_server();
        let conn = connect("127.0.0.1", port, test_options()).unwrap();

        let peer = conn.peer_addr().unwrap();
        assert_eq!(peer.ip().to_string(), "127.0.0.1");
        assert_eq!(peer.port(), port);

        let local = conn.local_addr().unwrap();
        assert_eq!(local.ip().to_string(), "127.0.0.1");
        assert!(local.port() > 0);
    }

    #[test]
    fn connect_options_defaults() {
        let opts = ConnectOptions::default();
        assert_eq!(opts.connect_timeout, Duration::from_secs(30));
        assert_eq!(opts.read_timeout, Some(Duration::from_secs(30)));
        assert_eq!(opts.write_timeout, Some(Duration::from_secs(30)));
        assert_eq!(opts.buffer_size, 8192);
    }

    #[test]
    fn error_display() {
        let err = TcpError::DnsResolutionFailed {
            host: "example.com".to_string(),
            message: "no such host".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "DNS resolution failed for 'example.com': no such host"
        );

        let err = TcpError::ConnectionRefused {
            addr: "127.0.0.1:8080".to_string(),
        };
        assert_eq!(err.to_string(), "connection refused by 127.0.0.1:8080");

        let err = TcpError::BrokenPipe;
        assert_eq!(err.to_string(), "broken pipe (remote closed)");
    }

    #[test]
    fn connect_with_hostname_localhost() {
        let (port, _stop) = start_echo_server();
        // "localhost" should resolve to 127.0.0.1 via the OS resolver
        let result = connect("localhost", port, test_options());
        assert!(result.is_ok(), "should connect via 'localhost'");
    }

    #[test]
    fn request_response_pattern() {
        let response_data = b"HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello".to_vec();
        let (port, _stop) = start_request_response_server(response_data);

        let mut conn = connect("127.0.0.1", port, test_options()).unwrap();

        // Send request
        conn.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();
        conn.flush().unwrap();

        // Read response line by line
        let status = conn.read_line().unwrap();
        assert!(status.starts_with("HTTP/1.0 200"));

        let header = conn.read_line().unwrap();
        assert!(header.starts_with("Content-Length:"));

        let blank = conn.read_line().unwrap();
        assert_eq!(blank, "\r\n");

        let body = conn.read_exact(5).unwrap();
        assert_eq!(body, b"hello");
    }
}
