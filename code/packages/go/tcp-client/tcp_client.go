// Package tcpclient provides a TCP client with buffered I/O and configurable
// timeouts.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
//
// # Analogy: A telephone call
//
// Making a TCP connection is like making a phone call:
//
//	1. DIAL (DNS + connect)
//	   Look up "Grandma" in contacts → 555-0123     (DNS resolution)
//	   Dial and wait for ring                        (TCP three-way handshake)
//	   If nobody picks up → hang up                  (connect timeout)
//
//	2. TALK (read/write)
//	   Say "Hello, Grandma!"                         (WriteAll + Flush)
//	   Listen for response                           (ReadLine)
//	   If silence for 30s → "Still there?"           (read timeout)
//
//	3. HANG UP (shutdown/close)
//	   Say "Goodbye" and hang up                     (ShutdownWrite + Close)
//
// # Where it fits
//
//	url-parser (NET00) → tcp-client (NET01, THIS) → frame-extractor (NET02)
//	                         ↓
//	                    raw byte stream
//
// # Example
//
//	conn, err := tcpclient.Connect("info.cern.ch", 80, tcpclient.DefaultOptions())
//	if err != nil { log.Fatal(err) }
//	defer conn.Close()
//
//	conn.WriteAll([]byte("GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n"))
//	conn.Flush()
//	line, _ := conn.ReadLine()
//	fmt.Println(line) // "HTTP/1.0 200 OK\r\n"
package tcpclient

import (
	"bufio"
	"fmt"
	"io"
	"net"
	"os"
	"strconv"
	"syscall"
	"time"
)

// ============================================================================
// ConnectOptions — configuration for establishing a connection
// ============================================================================

// ConnectOptions holds timeouts and buffer sizes for a TCP connection.
//
// All timeouts default to 30 seconds. The buffer size defaults to 8192
// bytes (8 KiB), a good balance between memory usage and syscall reduction.
//
// Why separate timeouts?
//
//	ConnectTimeout (30s) — how long to wait for the TCP handshake.
//	  If a server is down or firewalled, the OS might wait minutes.
//
//	ReadTimeout (30s) — how long to wait for data after calling read.
//	  Without this, a stalled server hangs your program forever.
//
//	WriteTimeout (30s) — how long to wait for the OS send buffer.
//	  Usually instant, but blocks if the remote side isn't reading.
//
//	BufferSize (8192) — internal buffer for BufReader/BufWriter.
//	  Larger = fewer syscalls but more memory per connection.
type ConnectOptions struct {
	ConnectTimeout time.Duration // Maximum time to wait for the TCP handshake.
	ReadTimeout    time.Duration // Maximum time to wait for data on read.
	WriteTimeout   time.Duration // Maximum time to wait on write.
	BufferSize     int           // Size of internal read and write buffers in bytes.
}

// DefaultOptions returns a ConnectOptions with sensible defaults:
//
//	ConnectTimeout: 30s
//	ReadTimeout:    30s
//	WriteTimeout:   30s
//	BufferSize:     8192
func DefaultOptions() ConnectOptions {
	return ConnectOptions{
		ConnectTimeout: 30 * time.Second,
		ReadTimeout:    30 * time.Second,
		WriteTimeout:   30 * time.Second,
		BufferSize:     8192,
	}
}

// ============================================================================
// TcpError — structured error types
// ============================================================================

// TcpError represents an error during TCP connection or communication.
//
// Kind is a short classifier; Message provides human-readable detail.
//
// Possible Kind values and what they mean:
//
//	DnsResolutionFailed — hostname could not be resolved
//	ConnectionRefused   — server is up but nothing listening on that port
//	Timeout             — took too long (connect, read, or write)
//	ConnectionReset     — remote side crashed (TCP RST)
//	BrokenPipe          — tried to write after remote closed
//	UnexpectedEof       — connection closed before expected data arrived
//	IoError             — catch-all for other OS-level errors
type TcpError struct {
	Kind    string
	Message string
}

// Error implements the error interface, returning "Kind: Message".
func (e *TcpError) Error() string {
	return fmt.Sprintf("%s: %s", e.Kind, e.Message)
}

// ============================================================================
// Error mapping helpers
// ============================================================================

// mapIOError inspects a Go error and returns the most specific TcpError.
//
// Go's net package wraps OS-level errors in layers:
//
//	*net.OpError → contains the inner OS error
//	*net.DNSError → DNS lookup failed
//	os.IsTimeout() → any timeout (connect, read, write)
//	io.ErrUnexpectedEOF → connection closed mid-read
//	syscall.ECONNRESET → TCP RST during data transfer
//	syscall.ECONNREFUSED → nothing listening on the port
//	syscall.EPIPE → write after remote close
//
// We peel the error layer by layer, from most specific to least.
func mapIOError(err error) *TcpError {
	if err == nil {
		return nil
	}

	// Check for DNS errors first — they're wrapped in *net.DNSError.
	if dnsErr, ok := err.(*net.DNSError); ok {
		return &TcpError{
			Kind:    "DnsResolutionFailed",
			Message: dnsErr.Err,
		}
	}

	// Check for timeout. os.IsTimeout handles both net and syscall timeouts.
	if os.IsTimeout(err) {
		return &TcpError{
			Kind:    "Timeout",
			Message: err.Error(),
		}
	}

	// Check for unexpected EOF — connection closed before all data arrived.
	if err == io.ErrUnexpectedEOF {
		return &TcpError{
			Kind:    "UnexpectedEof",
			Message: "connection closed before all expected data arrived",
		}
	}

	// Unwrap *net.OpError to get at the underlying syscall error.
	if opErr, ok := err.(*net.OpError); ok {
		// Recurse into the inner error for syscall-level classification.
		if sysErr, ok := opErr.Err.(*os.SyscallError); ok {
			return mapSyscallError(sysErr)
		}
		// Some OpErrors have a timeout flag.
		if opErr.Timeout() {
			return &TcpError{
				Kind:    "Timeout",
				Message: err.Error(),
			}
		}
	}

	// Fallback: generic I/O error.
	return &TcpError{
		Kind:    "IoError",
		Message: err.Error(),
	}
}

// mapSyscallError classifies a *os.SyscallError into a specific TcpError.
//
// On both Unix and Windows, the syscall error number tells us what happened:
//
//	ECONNREFUSED (10061 on Windows) — nothing listening on the port
//	ECONNRESET   (10054 on Windows) — remote sent TCP RST
//	EPIPE        (10053 on Windows) — write after remote closed
//	ETIMEDOUT    (10060 on Windows) — OS-level timeout
func mapSyscallError(sysErr *os.SyscallError) *TcpError {
	if errno, ok := sysErr.Err.(syscall.Errno); ok {
		switch errno {
		case syscall.ECONNREFUSED:
			return &TcpError{
				Kind:    "ConnectionRefused",
				Message: sysErr.Error(),
			}
		case syscall.ECONNRESET:
			return &TcpError{
				Kind:    "ConnectionReset",
				Message: "connection reset by peer",
			}
		}
		// EPIPE and ECONNABORTED are platform-specific; check the string
		// for portability since the constants may not exist on all platforms.
		errStr := sysErr.Error()
		if containsAny(errStr, "broken pipe", "EPIPE") {
			return &TcpError{
				Kind:    "BrokenPipe",
				Message: "broken pipe (remote closed)",
			}
		}
		if containsAny(errStr, "connection aborted", "ECONNABORTED") {
			return &TcpError{
				Kind:    "ConnectionReset",
				Message: "connection aborted by peer",
			}
		}
	}
	return &TcpError{
		Kind:    "IoError",
		Message: sysErr.Error(),
	}
}

// containsAny returns true if s contains any of the given substrings.
func containsAny(s string, substrs ...string) bool {
	for _, sub := range substrs {
		if len(sub) > 0 && len(s) >= len(sub) {
			for i := 0; i <= len(s)-len(sub); i++ {
				if s[i:i+len(sub)] == sub {
					return true
				}
			}
		}
	}
	return false
}

// ============================================================================
// Connect — establish a TCP connection
// ============================================================================

// Connect establishes a TCP connection to the given host and port.
//
// Algorithm:
//
//  1. Format the address as "host:port".
//  2. Call net.DialTimeout to perform DNS resolution + TCP handshake.
//     The OS resolver handles DNS (respects /etc/hosts, system DNS).
//  3. Set read and write deadlines on the connection.
//  4. Wrap the connection in bufio.Reader and bufio.Writer for efficiency.
//
// If the connection fails, the error is classified into a specific TcpError
// variant (DNS failure, refused, timeout, etc.).
//
// Example:
//
//	conn, err := tcpclient.Connect("example.com", 80, tcpclient.DefaultOptions())
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer conn.Close()
func Connect(host string, port uint16, opts ConnectOptions) (*TcpConnection, error) {
	// Step 1: Format the address string.
	//
	// net.DialTimeout accepts "host:port" and handles both IPv4 and IPv6.
	// For IPv6 literal addresses like "::1", Go's dialer handles brackets
	// internally.
	addr := net.JoinHostPort(host, strconv.Itoa(int(port)))

	// Step 2: Dial with timeout.
	//
	// net.DialTimeout performs DNS resolution and the TCP three-way handshake.
	// If the server doesn't respond within ConnectTimeout, it returns an error.
	rawConn, err := net.DialTimeout("tcp", addr, opts.ConnectTimeout)
	if err != nil {
		// Classify the error. DNS errors get a special variant with the
		// original hostname, which helps callers log useful messages.
		tcpErr := mapIOError(err)
		if tcpErr.Kind == "DnsResolutionFailed" {
			tcpErr.Message = fmt.Sprintf("host '%s': %s", host, tcpErr.Message)
		}
		return nil, tcpErr
	}

	// Step 3: Extract the underlying *net.TCPConn.
	//
	// We need this for ShutdownWrite() later, which calls CloseWrite()
	// on the TCP connection (half-close). The net.Conn interface doesn't
	// expose CloseWrite, but *net.TCPConn does.
	tcpConn, ok := rawConn.(*net.TCPConn)
	if !ok {
		rawConn.Close()
		return nil, &TcpError{
			Kind:    "IoError",
			Message: "connection is not a TCP connection",
		}
	}

	// Step 4: Wrap in buffered reader and writer.
	//
	// Why buffered I/O?
	//
	//   Without buffering:
	//     read() returns arbitrary chunks: "HT", "TP/", "1.0 2", "00 OK\r\n"
	//     100 read() calls = 100 syscalls (expensive!)
	//
	//   With bufio.Reader (8 KiB internal buffer):
	//     First read pulls 8 KiB from the OS into memory.
	//     Subsequent ReadLine() calls serve data from the buffer.
	//     100 lines might need only 1-2 syscalls.
	//
	//   Similarly, bufio.Writer batches small writes into larger chunks
	//   before flushing to the OS, avoiding tiny TCP packets.
	reader := bufio.NewReaderSize(tcpConn, opts.BufferSize)
	writer := bufio.NewWriterSize(tcpConn, opts.BufferSize)

	return &TcpConnection{
		conn:         tcpConn,
		reader:       reader,
		writer:       writer,
		readTimeout:  opts.ReadTimeout,
		writeTimeout: opts.WriteTimeout,
	}, nil
}

// ============================================================================
// TcpConnection — buffered I/O over a TCP stream
// ============================================================================

// TcpConnection wraps a TCP stream with buffered I/O and configured timeouts.
//
// It stores the underlying *net.TCPConn (for address queries and half-close),
// a bufio.Reader (for efficient line/chunk reads), and a bufio.Writer (for
// batching small writes). Timeouts are set as deadlines before each operation.
//
// The connection is safe for sequential use but NOT safe for concurrent use
// from multiple goroutines. If you need concurrent reads and writes, use
// separate goroutines with proper synchronization.
type TcpConnection struct {
	conn         *net.TCPConn   // the raw TCP connection (for CloseWrite, addresses)
	reader       *bufio.Reader  // buffered reader for efficient reads
	writer       *bufio.Writer  // buffered writer for batching writes
	readTimeout  time.Duration  // deadline applied before each read
	writeTimeout time.Duration  // deadline applied before each write
}

// setReadDeadline sets a deadline for the next read operation.
//
// Go's net.Conn uses absolute deadlines, not relative timeouts. We compute
// the deadline from time.Now() + readTimeout before each read. If readTimeout
// is zero, we clear the deadline (block forever).
func (c *TcpConnection) setReadDeadline() error {
	if c.readTimeout > 0 {
		return c.conn.SetReadDeadline(time.Now().Add(c.readTimeout))
	}
	return c.conn.SetReadDeadline(time.Time{}) // zero time = no deadline
}

// setWriteDeadline sets a deadline for the next write operation.
func (c *TcpConnection) setWriteDeadline() error {
	if c.writeTimeout > 0 {
		return c.conn.SetWriteDeadline(time.Now().Add(c.writeTimeout))
	}
	return c.conn.SetWriteDeadline(time.Time{})
}

// ReadLine reads bytes until a newline ('\n') is found.
//
// Returns the line including the trailing '\n' (and '\r\n' if present).
// Returns an empty string at EOF (remote closed cleanly).
//
// This is the workhorse for line-oriented protocols like HTTP/1.0, SMTP,
// and RESP (Redis protocol).
//
// Errors:
//   - TcpError{Kind: "Timeout"} if no data arrives within the read timeout
//   - TcpError{Kind: "ConnectionReset"} if the remote side closed unexpectedly
func (c *TcpConnection) ReadLine() (string, error) {
	if err := c.setReadDeadline(); err != nil {
		return "", mapIOError(err)
	}

	line, err := c.reader.ReadString('\n')
	if err != nil {
		// EOF with partial data — return what we got (like Go's bufio behavior).
		if err == io.EOF {
			return line, nil
		}
		return "", mapIOError(err)
	}
	return line, nil
}

// ReadExact reads exactly n bytes from the connection.
//
// Blocks until all n bytes have been received. Useful for protocols that
// specify an exact content length (e.g., HTTP Content-Length header).
//
// Errors:
//   - TcpError{Kind: "UnexpectedEof"} if the connection closes before n bytes
//   - TcpError{Kind: "Timeout"} if the read times out
func (c *TcpConnection) ReadExact(n int) ([]byte, error) {
	if err := c.setReadDeadline(); err != nil {
		return nil, mapIOError(err)
	}

	buf := make([]byte, n)
	_, err := io.ReadFull(c.reader, buf)
	if err != nil {
		if err == io.ErrUnexpectedEOF || err == io.EOF {
			return nil, &TcpError{
				Kind:    "UnexpectedEof",
				Message: fmt.Sprintf("expected %d bytes, connection closed early", n),
			}
		}
		return nil, mapIOError(err)
	}
	return buf, nil
}

// ReadUntil reads bytes until the given delimiter byte is found.
//
// Returns all bytes up to and including the delimiter. Useful for protocols
// with custom delimiters (RESP uses '\r\n', null-terminated strings use '\0').
//
// If EOF is reached before the delimiter, returns whatever was read.
func (c *TcpConnection) ReadUntil(delimiter byte) ([]byte, error) {
	if err := c.setReadDeadline(); err != nil {
		return nil, mapIOError(err)
	}

	buf, err := c.reader.ReadBytes(delimiter)
	if err != nil {
		if err == io.EOF {
			// Return partial data on EOF, like ReadLine.
			return buf, nil
		}
		return nil, mapIOError(err)
	}
	return buf, nil
}

// WriteAll writes all bytes to the connection's internal buffer.
//
// Data is buffered internally. You MUST call Flush() after writing a complete
// message to ensure it is actually sent over the network.
//
// Errors:
//   - TcpError{Kind: "BrokenPipe"} if the remote side has closed the connection
//   - TcpError{Kind: "Timeout"} if the write times out
func (c *TcpConnection) WriteAll(data []byte) error {
	if err := c.setWriteDeadline(); err != nil {
		return mapIOError(err)
	}

	_, err := c.writer.Write(data)
	if err != nil {
		return mapIOError(err)
	}
	return nil
}

// Flush sends all buffered write data to the network.
//
// You MUST call this after writing a complete request. Without flushing,
// data may sit in the bufio.Writer and never reach the server.
//
// Think of it like pressing "send" after typing a text message — the words
// exist in your phone's buffer until you actually send them.
func (c *TcpConnection) Flush() error {
	if err := c.setWriteDeadline(); err != nil {
		return mapIOError(err)
	}

	err := c.writer.Flush()
	if err != nil {
		return mapIOError(err)
	}
	return nil
}

// ShutdownWrite performs a TCP half-close on the write side.
//
// This signals to the remote side that no more data will be sent (sends a
// TCP FIN packet). The read side remains open — you can still receive data.
//
//	Before ShutdownWrite():
//	  Client <-> Server  (full-duplex, both directions open)
//
//	After ShutdownWrite():
//	  Client <- Server   (client can still READ)
//	  Client X  Server   (client can no longer WRITE)
//
// This is essential for protocols where the server waits for the client to
// finish sending before responding (e.g., sending a request body, then
// reading the response).
func (c *TcpConnection) ShutdownWrite() error {
	// Flush any buffered data before shutting down.
	if err := c.Flush(); err != nil {
		return err
	}

	// CloseWrite sends a TCP FIN on the write side.
	err := c.conn.CloseWrite()
	if err != nil {
		return mapIOError(err)
	}
	return nil
}

// PeerAddr returns the remote address (IP and port) of this connection.
func (c *TcpConnection) PeerAddr() (net.Addr, error) {
	addr := c.conn.RemoteAddr()
	if addr == nil {
		return nil, &TcpError{
			Kind:    "IoError",
			Message: "no peer address available",
		}
	}
	return addr, nil
}

// LocalAddr returns the local address (IP and port) of this connection.
func (c *TcpConnection) LocalAddr() (net.Addr, error) {
	addr := c.conn.LocalAddr()
	if addr == nil {
		return nil, &TcpError{
			Kind:    "IoError",
			Message: "no local address available",
		}
	}
	return addr, nil
}

// Close closes the TCP connection, releasing all resources.
//
// After Close, all reads and writes will return errors. Close is idempotent
// — calling it multiple times is safe.
func (c *TcpConnection) Close() error {
	err := c.conn.Close()
	if err != nil {
		return mapIOError(err)
	}
	return nil
}
