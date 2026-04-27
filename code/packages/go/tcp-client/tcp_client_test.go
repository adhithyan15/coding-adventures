package tcpclient

import (
	"fmt"
	"io"
	"net"
	"os"
	"sync"
	"syscall"
	"testing"
	"time"
)

// ============================================================================
// Test helpers — lightweight TCP servers
// ============================================================================
//
// Each helper starts a server on port 0 (OS picks an available port),
// runs it in a goroutine, and returns the port number plus a cleanup
// function. Using port 0 avoids conflicts when tests run in parallel.

// startEchoServer starts a TCP server that echoes back everything it receives.
//
// This is the simplest useful server: read bytes, write them back. It mimics
// the behavior needed for testing round-trip communication.
//
//	Client sends "Hello" → Server receives "Hello" → Server sends "Hello" back
func startEchoServer(t *testing.T) (uint16, func()) {
	t.Helper()

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to start echo server: %v", err)
	}

	port := uint16(listener.Addr().(*net.TCPAddr).Port)
	done := make(chan struct{})

	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()
		conn.SetReadDeadline(time.Now().Add(10 * time.Second))

		buf := make([]byte, 65536)
		for {
			n, err := conn.Read(buf)
			if err != nil {
				return
			}
			if _, err := conn.Write(buf[:n]); err != nil {
				return
			}
		}
	}()

	// Give the listener a moment to be ready.
	time.Sleep(50 * time.Millisecond)

	cleanup := func() {
		listener.Close()
		<-done
	}
	return port, cleanup
}

// startSilentServer starts a server that accepts a connection but never
// sends any data. Used to test read timeouts — the client connects
// successfully but hangs waiting for a response that never comes.
func startSilentServer(t *testing.T) (uint16, func()) {
	t.Helper()

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to start silent server: %v", err)
	}

	port := uint16(listener.Addr().(*net.TCPAddr).Port)
	done := make(chan struct{})

	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		// Hold connection open until cleanup.
		<-done
		conn.Close()
	}()

	time.Sleep(50 * time.Millisecond)

	cleanup := func() {
		listener.Close()
		// Signal the goroutine is being cleaned up. The channel close
		// above means reading from done will unblock.
	}
	return port, cleanup
}

// startPartialServer starts a server that sends exactly the given data,
// then closes the connection. Used to test EOF handling and ReadExact
// with insufficient data.
func startPartialServer(t *testing.T, data []byte) (uint16, func()) {
	t.Helper()

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to start partial server: %v", err)
	}

	port := uint16(listener.Addr().(*net.TCPAddr).Port)
	done := make(chan struct{})

	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		conn.Write(data)
		// Small delay so client can read before we close.
		time.Sleep(100 * time.Millisecond)
		conn.Close()
	}()

	time.Sleep(50 * time.Millisecond)

	cleanup := func() {
		listener.Close()
		<-done
	}
	return port, cleanup
}

// startRequestResponseServer reads one request (waits for some data),
// then sends the given response. Used to test HTTP-like request/response
// patterns.
func startRequestResponseServer(t *testing.T, response []byte) (uint16, func()) {
	t.Helper()

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to start request-response server: %v", err)
	}

	port := uint16(listener.Addr().(*net.TCPAddr).Port)
	done := make(chan struct{})

	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()
		conn.SetReadDeadline(time.Now().Add(5 * time.Second))

		// Read one chunk (the request).
		buf := make([]byte, 4096)
		conn.Read(buf)

		// Send the response.
		conn.Write(response)
	}()

	time.Sleep(50 * time.Millisecond)

	cleanup := func() {
		listener.Close()
		<-done
	}
	return port, cleanup
}

// testOptions returns ConnectOptions with shorter timeouts for testing.
// Production defaults are 30s; tests use 5s to fail faster.
func testOptions() ConnectOptions {
	return ConnectOptions{
		ConnectTimeout: 5 * time.Second,
		ReadTimeout:    5 * time.Second,
		WriteTimeout:   5 * time.Second,
		BufferSize:     4096,
	}
}

// ============================================================================
// Group 1: Connection basics
// ============================================================================

// TestConnectAndDisconnect verifies that we can establish and tear down
// a connection without error. This is the smoke test — if this fails,
// nothing else will work.
func TestConnectAndDisconnect(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("expected successful connection, got: %v", err)
	}
	defer conn.Close()
}

// TestDefaultOptions verifies that DefaultOptions returns the documented
// defaults. This is a safety net — if someone changes the defaults, tests
// should catch it.
func TestDefaultOptions(t *testing.T) {
	opts := DefaultOptions()

	if opts.ConnectTimeout != 30*time.Second {
		t.Errorf("ConnectTimeout = %v, want 30s", opts.ConnectTimeout)
	}
	if opts.ReadTimeout != 30*time.Second {
		t.Errorf("ReadTimeout = %v, want 30s", opts.ReadTimeout)
	}
	if opts.WriteTimeout != 30*time.Second {
		t.Errorf("WriteTimeout = %v, want 30s", opts.WriteTimeout)
	}
	if opts.BufferSize != 8192 {
		t.Errorf("BufferSize = %d, want 8192", opts.BufferSize)
	}
}

// ============================================================================
// Group 2: Echo server — write/read round trips
// ============================================================================

// TestWriteAndReadBack sends data to the echo server and reads it back,
// verifying the entire write → flush → read pipeline.
func TestWriteAndReadBack(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	err = conn.WriteAll([]byte("Hello, TCP!"))
	if err != nil {
		t.Fatalf("write failed: %v", err)
	}
	err = conn.Flush()
	if err != nil {
		t.Fatalf("flush failed: %v", err)
	}

	got, err := conn.ReadExact(11)
	if err != nil {
		t.Fatalf("read failed: %v", err)
	}
	if string(got) != "Hello, TCP!" {
		t.Errorf("got %q, want %q", string(got), "Hello, TCP!")
	}
}

// TestReadLineFromEcho sends two lines and reads them back one at a time.
// ReadLine should return each line including the trailing newline.
func TestReadLineFromEcho(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	conn.WriteAll([]byte("Hello\r\nWorld\r\n"))
	conn.Flush()

	line1, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine 1 failed: %v", err)
	}
	if line1 != "Hello\r\n" {
		t.Errorf("line1 = %q, want %q", line1, "Hello\r\n")
	}

	line2, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine 2 failed: %v", err)
	}
	if line2 != "World\r\n" {
		t.Errorf("line2 = %q, want %q", line2, "World\r\n")
	}
}

// TestReadExactFromEcho sends a known byte sequence and reads it back
// with ReadExact, verifying byte-level accuracy.
func TestReadExactFromEcho(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	// Generate a 100-byte pattern: 0, 1, 2, ..., 99.
	data := make([]byte, 100)
	for i := range data {
		data[i] = byte(i % 256)
	}

	conn.WriteAll(data)
	conn.Flush()

	got, err := conn.ReadExact(100)
	if err != nil {
		t.Fatalf("ReadExact failed: %v", err)
	}
	for i, b := range got {
		if b != data[i] {
			t.Fatalf("byte %d: got %d, want %d", i, b, data[i])
		}
	}
}

// TestReadUntilFromEcho sends data containing a null terminator and
// reads up to (and including) it.
func TestReadUntilFromEcho(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	conn.WriteAll([]byte("key:value\x00next"))
	conn.Flush()

	got, err := conn.ReadUntil(0x00)
	if err != nil {
		t.Fatalf("ReadUntil failed: %v", err)
	}
	if string(got) != "key:value\x00" {
		t.Errorf("got %q, want %q", string(got), "key:value\x00")
	}
}

// TestLargeDataTransfer sends 64 KiB through the echo server to verify
// that buffered I/O handles data larger than the buffer size correctly.
//
// With a 4 KiB buffer, 64 KiB requires ~16 buffer flushes. This tests
// that the buffering logic handles multiple fill/drain cycles.
func TestLargeDataTransfer(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	// 64 KiB of patterned data.
	data := make([]byte, 65536)
	for i := range data {
		data[i] = byte(i % 256)
	}

	conn.WriteAll(data)
	conn.Flush()

	got, err := conn.ReadExact(65536)
	if err != nil {
		t.Fatalf("ReadExact failed: %v", err)
	}
	if len(got) != 65536 {
		t.Fatalf("got %d bytes, want 65536", len(got))
	}
	for i := range got {
		if got[i] != data[i] {
			t.Fatalf("mismatch at byte %d: got %d, want %d", i, got[i], data[i])
		}
	}
}

// TestMultipleExchanges sends multiple request/response pairs over a
// single connection. This verifies that the buffered state is correctly
// maintained across multiple read/write cycles.
func TestMultipleExchanges(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	// Exchange 1.
	conn.WriteAll([]byte("ping\n"))
	conn.Flush()
	line1, _ := conn.ReadLine()
	if line1 != "ping\n" {
		t.Errorf("exchange 1: got %q, want %q", line1, "ping\n")
	}

	// Exchange 2.
	conn.WriteAll([]byte("pong\n"))
	conn.Flush()
	line2, _ := conn.ReadLine()
	if line2 != "pong\n" {
		t.Errorf("exchange 2: got %q, want %q", line2, "pong\n")
	}
}

// ============================================================================
// Group 3: Timeout tests
// ============================================================================

// TestConnectTimeout tries to connect to a non-routable IP address
// (10.255.255.1). The connection attempt should time out.
func TestConnectTimeout(t *testing.T) {
	opts := ConnectOptions{
		ConnectTimeout: 1 * time.Second,
		ReadTimeout:    5 * time.Second,
		WriteTimeout:   5 * time.Second,
		BufferSize:     4096,
	}

	_, err := Connect("10.255.255.1", 1, opts)
	if err == nil {
		t.Fatal("expected timeout error, got nil")
	}

	tcpErr, ok := err.(*TcpError)
	if !ok {
		t.Fatalf("expected *TcpError, got %T: %v", err, err)
	}

	// On some platforms, non-routable addresses may return IoError instead
	// of Timeout. Both are acceptable.
	if tcpErr.Kind != "Timeout" && tcpErr.Kind != "IoError" && tcpErr.Kind != "ConnectionRefused" {
		t.Errorf("error kind = %q, want Timeout or IoError", tcpErr.Kind)
	}
}

// TestReadTimeout connects to a server that never sends data, then
// tries to read. The read should time out.
func TestReadTimeout(t *testing.T) {
	port, cleanup := startSilentServer(t)
	defer cleanup()

	opts := ConnectOptions{
		ConnectTimeout: 5 * time.Second,
		ReadTimeout:    1 * time.Second,
		WriteTimeout:   5 * time.Second,
		BufferSize:     4096,
	}

	conn, err := Connect("127.0.0.1", port, opts)
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	_, err = conn.ReadLine()
	if err == nil {
		t.Fatal("expected timeout error, got nil")
	}

	tcpErr, ok := err.(*TcpError)
	if !ok {
		t.Fatalf("expected *TcpError, got %T: %v", err, err)
	}

	if tcpErr.Kind != "Timeout" && tcpErr.Kind != "IoError" {
		t.Errorf("error kind = %q, want Timeout or IoError", tcpErr.Kind)
	}
}

// ============================================================================
// Group 4: Error handling
// ============================================================================

// TestConnectionRefused connects to a port where nothing is listening.
// The server should respond with a TCP RST, which we map to ConnectionRefused.
func TestConnectionRefused(t *testing.T) {
	// Bind a port, then immediately close it so nothing listens.
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to bind port: %v", err)
	}
	port := uint16(listener.Addr().(*net.TCPAddr).Port)
	listener.Close()

	_, err = Connect("127.0.0.1", port, testOptions())
	if err == nil {
		t.Fatal("expected error, got nil")
	}

	tcpErr, ok := err.(*TcpError)
	if !ok {
		t.Fatalf("expected *TcpError, got %T: %v", err, err)
	}

	if tcpErr.Kind != "ConnectionRefused" && tcpErr.Kind != "IoError" {
		t.Errorf("error kind = %q, want ConnectionRefused or IoError", tcpErr.Kind)
	}
}

// TestDnsFailure tries to connect to a hostname that does not exist.
// DNS resolution should fail.
func TestDnsFailure(t *testing.T) {
	_, err := Connect("this.host.does.not.exist.example", 80, testOptions())
	if err == nil {
		t.Fatal("expected error, got nil")
	}

	tcpErr, ok := err.(*TcpError)
	if !ok {
		t.Fatalf("expected *TcpError, got %T: %v", err, err)
	}

	// Some ISP DNS resolvers hijack NXDOMAIN, causing a connection error
	// instead of a DNS error. Accept several error kinds.
	validKinds := map[string]bool{
		"DnsResolutionFailed": true,
		"ConnectionRefused":   true,
		"Timeout":             true,
		"IoError":             true,
	}
	if !validKinds[tcpErr.Kind] {
		t.Errorf("error kind = %q, want one of DnsResolutionFailed/ConnectionRefused/Timeout/IoError", tcpErr.Kind)
	}
}

// TestUnexpectedEof: server sends 50 bytes then closes; client tries
// to read 100 bytes. Should get an UnexpectedEof error.
func TestUnexpectedEof(t *testing.T) {
	data := make([]byte, 50)
	for i := range data {
		data[i] = byte(i)
	}
	port, cleanup := startPartialServer(t, data)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	// Wait for server to send data and close.
	time.Sleep(200 * time.Millisecond)

	_, err = conn.ReadExact(100)
	if err == nil {
		t.Fatal("expected error, got nil")
	}

	tcpErr, ok := err.(*TcpError)
	if !ok {
		t.Fatalf("expected *TcpError, got %T: %v", err, err)
	}

	if tcpErr.Kind != "UnexpectedEof" {
		t.Errorf("error kind = %q, want UnexpectedEof", tcpErr.Kind)
	}
}

// TestBrokenPipe: server accepts then immediately closes. Client writes
// repeatedly until the OS detects the closed connection.
func TestBrokenPipe(t *testing.T) {
	port, cleanup := startPartialServer(t, []byte{})
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	// Wait for server to close.
	time.Sleep(300 * time.Millisecond)

	// Write repeatedly. The first write may succeed (data goes to the
	// OS send buffer). Eventually the OS notices the RST and returns
	// an error.
	gotError := false
	bigData := make([]byte, 65536)
	for i := 0; i < 10; i++ {
		if err := conn.WriteAll(bigData); err != nil {
			gotError = true
			break
		}
		if err := conn.Flush(); err != nil {
			gotError = true
			break
		}
		time.Sleep(50 * time.Millisecond)
	}

	if !gotError {
		t.Error("expected write error after server closed")
	}
}

// ============================================================================
// Group 5: Half-close (ShutdownWrite)
// ============================================================================

// TestClientHalfClose verifies the half-close pattern:
//  1. Client sends data
//  2. Client calls ShutdownWrite (sends TCP FIN)
//  3. Server detects EOF on read
//  4. Server sends a response
//  5. Client reads the response
//
// This pattern is used in protocols where the server needs to know the
// client is done sending before it responds.
func TestClientHalfClose(t *testing.T) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to bind: %v", err)
	}
	port := uint16(listener.Addr().(*net.TCPAddr).Port)

	var serverReceived []byte
	var mu sync.Mutex
	done := make(chan struct{})

	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()
		conn.SetReadDeadline(time.Now().Add(5 * time.Second))

		// Read until EOF (client's ShutdownWrite).
		var buf []byte
		tmp := make([]byte, 1024)
		for {
			n, err := conn.Read(tmp)
			if n > 0 {
				buf = append(buf, tmp[:n]...)
			}
			if err != nil {
				break
			}
		}

		mu.Lock()
		serverReceived = buf
		mu.Unlock()

		// Send response after client is done.
		conn.Write([]byte("DONE\n"))
	}()

	time.Sleep(50 * time.Millisecond)

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	conn.WriteAll([]byte("request data"))
	if err := conn.ShutdownWrite(); err != nil {
		t.Fatalf("ShutdownWrite failed: %v", err)
	}

	// Read the server's response.
	response, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine failed: %v", err)
	}
	if response != "DONE\n" {
		t.Errorf("response = %q, want %q", response, "DONE\n")
	}

	// Verify the server got our data.
	<-done
	mu.Lock()
	defer mu.Unlock()
	if string(serverReceived) != "request data" {
		t.Errorf("server received %q, want %q", string(serverReceived), "request data")
	}

	listener.Close()
}

// ============================================================================
// Group 6: Edge cases
// ============================================================================

// TestEmptyReadAtEof: server sends one line then closes. First ReadLine
// returns the line; second ReadLine returns empty string (EOF).
func TestEmptyReadAtEof(t *testing.T) {
	port, cleanup := startPartialServer(t, []byte("hello\n"))
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	// Wait for server to send and close.
	time.Sleep(200 * time.Millisecond)

	line, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine failed: %v", err)
	}
	if line != "hello\n" {
		t.Errorf("line = %q, want %q", line, "hello\n")
	}

	// EOF should return empty string with no error.
	eof, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine at EOF failed: %v", err)
	}
	if eof != "" {
		t.Errorf("expected empty string at EOF, got %q", eof)
	}
}

// TestZeroByteWrite verifies that writing zero bytes succeeds without error.
// This is an edge case that some implementations handle incorrectly.
func TestZeroByteWrite(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	err = conn.WriteAll([]byte{})
	if err != nil {
		t.Errorf("WriteAll(empty) failed: %v", err)
	}
}

// TestPeerAddress verifies that PeerAddr and LocalAddr return the
// expected addresses after connecting to localhost.
func TestPeerAddress(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	peer, err := conn.PeerAddr()
	if err != nil {
		t.Fatalf("PeerAddr failed: %v", err)
	}
	tcpAddr := peer.(*net.TCPAddr)
	if tcpAddr.IP.String() != "127.0.0.1" {
		t.Errorf("peer IP = %s, want 127.0.0.1", tcpAddr.IP)
	}
	if tcpAddr.Port != int(port) {
		t.Errorf("peer port = %d, want %d", tcpAddr.Port, port)
	}

	local, err := conn.LocalAddr()
	if err != nil {
		t.Fatalf("LocalAddr failed: %v", err)
	}
	localAddr := local.(*net.TCPAddr)
	if localAddr.IP.String() != "127.0.0.1" {
		t.Errorf("local IP = %s, want 127.0.0.1", localAddr.IP)
	}
	if localAddr.Port <= 0 {
		t.Error("local port should be > 0")
	}
}

// TestErrorDisplay verifies that TcpError.Error() returns the expected
// format "Kind: Message".
func TestErrorDisplay(t *testing.T) {
	tests := []struct {
		name string
		err  TcpError
		want string
	}{
		{
			name: "dns error",
			err:  TcpError{Kind: "DnsResolutionFailed", Message: "no such host"},
			want: "DnsResolutionFailed: no such host",
		},
		{
			name: "connection refused",
			err:  TcpError{Kind: "ConnectionRefused", Message: "127.0.0.1:8080"},
			want: "ConnectionRefused: 127.0.0.1:8080",
		},
		{
			name: "broken pipe",
			err:  TcpError{Kind: "BrokenPipe", Message: "broken pipe (remote closed)"},
			want: "BrokenPipe: broken pipe (remote closed)",
		},
		{
			name: "timeout",
			err:  TcpError{Kind: "Timeout", Message: "read timed out after 30s"},
			want: "Timeout: read timed out after 30s",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := tt.err.Error()
			if got != tt.want {
				t.Errorf("Error() = %q, want %q", got, tt.want)
			}
		})
	}
}

// TestConnectWithLocalhostHostname verifies that "localhost" resolves
// correctly via the OS resolver.
func TestConnectWithLocalhostHostname(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("localhost", port, testOptions())
	if err != nil {
		t.Fatalf("expected successful connection via 'localhost', got: %v", err)
	}
	defer conn.Close()
}

// TestRequestResponsePattern simulates an HTTP-like request/response
// exchange: send a request, read the response line by line, then read
// the body with ReadExact.
func TestRequestResponsePattern(t *testing.T) {
	response := []byte("HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello")
	port, cleanup := startRequestResponseServer(t, response)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	// Send request.
	conn.WriteAll([]byte("GET / HTTP/1.0\r\n\r\n"))
	conn.Flush()

	// Read status line.
	status, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine (status) failed: %v", err)
	}
	if status != "HTTP/1.0 200 OK\r\n" {
		t.Errorf("status = %q, want %q", status, "HTTP/1.0 200 OK\r\n")
	}

	// Read header.
	header, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine (header) failed: %v", err)
	}
	if header != "Content-Length: 5\r\n" {
		t.Errorf("header = %q, want %q", header, "Content-Length: 5\r\n")
	}

	// Read blank line (end of headers).
	blank, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine (blank) failed: %v", err)
	}
	if blank != "\r\n" {
		t.Errorf("blank = %q, want %q", blank, "\r\n")
	}

	// Read body.
	body, err := conn.ReadExact(5)
	if err != nil {
		t.Fatalf("ReadExact (body) failed: %v", err)
	}
	if string(body) != "hello" {
		t.Errorf("body = %q, want %q", string(body), "hello")
	}
}

// TestReadUntilAtEof: server sends data without the delimiter, then
// closes. ReadUntil should return whatever was received.
func TestReadUntilAtEof(t *testing.T) {
	port, cleanup := startPartialServer(t, []byte("no-delimiter-here"))
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	time.Sleep(200 * time.Millisecond)

	got, err := conn.ReadUntil(0x00)
	if err != nil {
		t.Fatalf("ReadUntil failed: %v", err)
	}
	if string(got) != "no-delimiter-here" {
		t.Errorf("got %q, want %q", string(got), "no-delimiter-here")
	}
}

// TestCloseIsIdempotent verifies that calling Close multiple times
// does not panic. The second call may return an error but should not crash.
func TestCloseIsIdempotent(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}

	// First close should succeed.
	err = conn.Close()
	if err != nil {
		t.Errorf("first Close failed: %v", err)
	}

	// Second close may return an error but should not panic.
	_ = conn.Close()
}

// TestFlushWithoutWrite verifies that calling Flush on a fresh connection
// (no buffered data) succeeds.
func TestFlushWithoutWrite(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	err = conn.Flush()
	if err != nil {
		t.Errorf("Flush on fresh connection failed: %v", err)
	}
}

// ============================================================================
// Group 7: containsAny helper
// ============================================================================

// TestContainsAny tests the internal helper used for error classification.
func TestContainsAny(t *testing.T) {
	if !containsAny("broken pipe error", "broken pipe") {
		t.Error("expected true for 'broken pipe' in 'broken pipe error'")
	}
	if !containsAny("ECONNRESET happened", "ECONNRESET") {
		t.Error("expected true for 'ECONNRESET' in 'ECONNRESET happened'")
	}
	if containsAny("no match here", "EPIPE", "broken pipe") {
		t.Error("expected false for unmatched substrings")
	}
	if containsAny("", "something") {
		t.Error("expected false for empty string")
	}
	if containsAny("test", "") {
		t.Error("expected false for empty substring")
	}
}

// ============================================================================
// Group 8: mapIOError coverage
// ============================================================================

// TestMapIOErrorNil verifies that mapIOError(nil) returns nil.
func TestMapIOErrorNil(t *testing.T) {
	if mapIOError(nil) != nil {
		t.Error("mapIOError(nil) should return nil")
	}
}

// TestMapIOErrorUnexpectedEOF verifies that io.ErrUnexpectedEOF maps
// to UnexpectedEof.
func TestMapIOErrorUnexpectedEOF(t *testing.T) {
	tcpErr := mapIOError(io.ErrUnexpectedEOF)
	if tcpErr.Kind != "UnexpectedEof" {
		t.Errorf("kind = %q, want UnexpectedEof", tcpErr.Kind)
	}
}

// TestMapIOErrorGeneric verifies that an unrecognized error maps to IoError.
func TestMapIOErrorGeneric(t *testing.T) {
	tcpErr := mapIOError(fmt.Errorf("some random error"))
	if tcpErr.Kind != "IoError" {
		t.Errorf("kind = %q, want IoError", tcpErr.Kind)
	}
}

// TestMapIOErrorDNS verifies that a *net.DNSError maps to DnsResolutionFailed.
func TestMapIOErrorDNS(t *testing.T) {
	dnsErr := &net.DNSError{
		Err:  "no such host",
		Name: "example.invalid",
	}
	tcpErr := mapIOError(dnsErr)
	if tcpErr.Kind != "DnsResolutionFailed" {
		t.Errorf("kind = %q, want DnsResolutionFailed", tcpErr.Kind)
	}
}

// TestMapSyscallErrorConnectionRefused verifies that ECONNREFUSED maps
// to ConnectionRefused.
func TestMapSyscallErrorConnectionRefused(t *testing.T) {
	sysErr := &os.SyscallError{
		Syscall: "connect",
		Err:     syscall.ECONNREFUSED,
	}
	tcpErr := mapSyscallError(sysErr)
	if tcpErr.Kind != "ConnectionRefused" {
		t.Errorf("kind = %q, want ConnectionRefused", tcpErr.Kind)
	}
}

// TestMapSyscallErrorConnectionReset verifies that ECONNRESET maps
// to ConnectionReset.
func TestMapSyscallErrorConnectionReset(t *testing.T) {
	sysErr := &os.SyscallError{
		Syscall: "read",
		Err:     syscall.ECONNRESET,
	}
	tcpErr := mapSyscallError(sysErr)
	if tcpErr.Kind != "ConnectionReset" {
		t.Errorf("kind = %q, want ConnectionReset", tcpErr.Kind)
	}
}

// TestMapSyscallErrorGenericErrno verifies that an unrecognized errno
// falls through to IoError.
func TestMapSyscallErrorGenericErrno(t *testing.T) {
	sysErr := &os.SyscallError{
		Syscall: "read",
		Err:     syscall.ENOENT, // not a network error
	}
	tcpErr := mapSyscallError(sysErr)
	if tcpErr.Kind != "IoError" {
		t.Errorf("kind = %q, want IoError", tcpErr.Kind)
	}
}

// TestZeroTimeoutDeadlines verifies that zero-value timeouts work correctly
// (they mean "no deadline" — block forever).
func TestZeroTimeoutDeadlines(t *testing.T) {
	port, cleanup := startEchoServer(t)
	defer cleanup()

	opts := ConnectOptions{
		ConnectTimeout: 5 * time.Second,
		ReadTimeout:    0, // no deadline
		WriteTimeout:   0, // no deadline
		BufferSize:     4096,
	}

	conn, err := Connect("127.0.0.1", port, opts)
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	// Write and read should work with zero timeouts.
	conn.WriteAll([]byte("test\n"))
	conn.Flush()

	line, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine failed: %v", err)
	}
	if line != "test\n" {
		t.Errorf("got %q, want %q", line, "test\n")
	}
}

// TestReadLinePartialDataAtEof: server sends data without a trailing
// newline, then closes. ReadLine should return the partial data.
func TestReadLinePartialDataAtEof(t *testing.T) {
	port, cleanup := startPartialServer(t, []byte("no-newline"))
	defer cleanup()

	conn, err := Connect("127.0.0.1", port, testOptions())
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer conn.Close()

	time.Sleep(200 * time.Millisecond)

	// ReadLine hits EOF before finding '\n'. It should return partial data.
	line, err := conn.ReadLine()
	if err != nil {
		t.Fatalf("ReadLine failed: %v", err)
	}
	if line != "no-newline" {
		t.Errorf("got %q, want %q", line, "no-newline")
	}
}

// TestMapIOErrorOpErrorTimeout verifies that a *net.OpError with Timeout()
// true maps to Timeout.
func TestMapIOErrorOpErrorTimeout(t *testing.T) {
	opErr := &net.OpError{
		Op:  "read",
		Net: "tcp",
		Err: &timeoutError{},
	}
	tcpErr := mapIOError(opErr)
	// os.IsTimeout should catch this first, yielding Timeout.
	if tcpErr.Kind != "Timeout" {
		t.Errorf("kind = %q, want Timeout", tcpErr.Kind)
	}
}

// timeoutError is a test helper that implements net.Error with Timeout() true.
type timeoutError struct{}

func (e *timeoutError) Error() string   { return "i/o timeout" }
func (e *timeoutError) Timeout() bool   { return true }
func (e *timeoutError) Temporary() bool { return true }
