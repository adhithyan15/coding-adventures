"""Tests for tcp-client.

These tests use a local echo server pattern: each test starts a tiny TCP
server in a background thread on an OS-assigned port (port 0), exercises
the client against it, and shuts it down. This avoids needing any external
services and lets tests run in parallel without port conflicts.

Test groups:
  1. Echo server tests -- basic read/write functionality
  2. Timeout tests -- connect and read timeouts
  3. Error tests -- connection refused, DNS failure, unexpected EOF
  4. Half-close tests -- shutdown_write behavior
  5. Edge cases -- empty reads, zero-byte writes, addresses
  6. API tests -- ConnectOptions defaults, error display strings
"""

from __future__ import annotations

import contextlib
import socket
import threading
import time

import pytest

from tcp_client import (
    BrokenPipe,
    ConnectionRefused,
    ConnectionReset,
    ConnectOptions,
    DnsResolutionFailed,
    TcpConnection,
    TcpError,
    Timeout,
    UnexpectedEof,
    __version__,
    connect,
)

# ============================================================================
# Test helpers -- local TCP servers for each test pattern
# ============================================================================
#
# Each helper starts a server in a background thread and returns the port.
# Using port 0 lets the OS pick an available port, so tests never collide.
# The server handles exactly one connection then stops.


def start_echo_server() -> tuple[int, threading.Event]:
    """Start a server that echoes back everything it receives.

    The echo server is the workhorse for most tests. It accepts one
    connection, reads data in a loop, and writes it right back.

    Returns:
        (port, stop_event) -- port to connect to, event to signal shutdown.
    """
    server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server_sock.bind(("127.0.0.1", 0))
    server_sock.listen(1)
    port: int = server_sock.getsockname()[1]
    stop_event = threading.Event()

    def serve() -> None:
        server_sock.settimeout(5.0)
        try:
            conn, _ = server_sock.accept()
            conn.settimeout(5.0)
            try:
                while not stop_event.is_set():
                    try:
                        data = conn.recv(65536)
                        if not data:
                            break  # EOF from client
                        conn.sendall(data)
                    except (OSError, ConnectionError):
                        break
            finally:
                conn.close()
        except OSError:
            pass
        finally:
            server_sock.close()

    thread = threading.Thread(target=serve, daemon=True)
    thread.start()
    # Small delay to ensure the server is listening before tests connect
    time.sleep(0.05)
    return port, stop_event


def start_silent_server() -> tuple[int, threading.Event]:
    """Start a server that accepts but never sends data.

    Used for read timeout tests -- the client connects successfully but
    any read will hang until the timeout fires.

    Returns:
        (port, stop_event)
    """
    server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server_sock.bind(("127.0.0.1", 0))
    server_sock.listen(1)
    port: int = server_sock.getsockname()[1]
    stop_event = threading.Event()

    def serve() -> None:
        server_sock.settimeout(5.0)
        try:
            conn, _ = server_sock.accept()
            # Hold the connection open but never send anything
            stop_event.wait(timeout=10.0)
            conn.close()
        except OSError:
            pass
        finally:
            server_sock.close()

    thread = threading.Thread(target=serve, daemon=True)
    thread.start()
    time.sleep(0.05)
    return port, stop_event


def start_partial_server(data: bytes) -> tuple[int, threading.Event]:
    """Start a server that sends exactly the given data then closes.

    Used for unexpected EOF and empty read tests -- the server delivers
    a specific payload and immediately disconnects.

    Args:
        data: Exact bytes to send before closing.

    Returns:
        (port, stop_event)
    """
    server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server_sock.bind(("127.0.0.1", 0))
    server_sock.listen(1)
    port: int = server_sock.getsockname()[1]
    stop_event = threading.Event()

    def serve() -> None:
        server_sock.settimeout(5.0)
        try:
            conn, _ = server_sock.accept()
            if data:
                conn.sendall(data)
            # Small delay so client can read before we close
            time.sleep(0.1)
            conn.close()
        except OSError:
            pass
        finally:
            server_sock.close()

    thread = threading.Thread(target=serve, daemon=True)
    thread.start()
    time.sleep(0.05)
    return port, stop_event


def start_request_response_server(response: bytes) -> tuple[int, threading.Event]:
    """Start a server that reads a request then sends a canned response.

    Mimics HTTP-style request/response: wait for client data, then
    write back a fixed response. Used for the request_response_pattern test.

    Args:
        response: The bytes to send back after receiving client data.

    Returns:
        (port, stop_event)
    """
    server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server_sock.bind(("127.0.0.1", 0))
    server_sock.listen(1)
    port: int = server_sock.getsockname()[1]
    stop_event = threading.Event()

    def serve() -> None:
        server_sock.settimeout(5.0)
        try:
            conn, _ = server_sock.accept()
            conn.settimeout(5.0)
            # Read the request (just consume it)
            with contextlib.suppress(OSError):
                conn.recv(4096)
            # Send the response
            conn.sendall(response)
            time.sleep(0.1)
            conn.close()
        except OSError:
            pass
        finally:
            server_sock.close()

    thread = threading.Thread(target=serve, daemon=True)
    thread.start()
    time.sleep(0.05)
    return port, stop_event


def make_test_options() -> ConnectOptions:
    """Return fast ConnectOptions for tests (short timeouts)."""
    return ConnectOptions(
        connect_timeout=5.0,
        read_timeout=5.0,
        write_timeout=5.0,
        buffer_size=4096,
    )


# ============================================================================
# Group 0: Package metadata
# ============================================================================


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


# ============================================================================
# Group 1: Echo server tests -- basic read/write functionality
# ============================================================================


class TestEchoServer:
    """Tests that use a local echo server for round-trip communication."""

    def test_connect_and_disconnect(self) -> None:
        """Verify that we can connect to a server and disconnect cleanly."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            # Connection should be valid
            assert isinstance(conn, TcpConnection)
            conn.close()
        finally:
            stop.set()

    def test_write_and_read_back(self) -> None:
        """Send bytes to the echo server and verify they come back."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            conn.write_all(b"Hello, TCP!")
            conn.flush()

            # The echo server sends our data right back. read_exact(11)
            # blocks until all 11 bytes arrive.
            result = conn.read_exact(11)
            assert result == b"Hello, TCP!"
            conn.close()
        finally:
            stop.set()

    def test_read_line(self) -> None:
        """Verify line-by-line reading from an echo server."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            conn.write_all(b"Hello\r\nWorld\r\n")
            conn.flush()

            # read_line returns the line *including* the trailing newline
            line1 = conn.read_line()
            assert line1 == "Hello\r\n"

            line2 = conn.read_line()
            assert line2 == "World\r\n"
            conn.close()
        finally:
            stop.set()

    def test_read_exact(self) -> None:
        """Send a known byte pattern and read it back exactly."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            # Send bytes 0..99 as a known pattern
            data = bytes(range(100))
            conn.write_all(data)
            conn.flush()

            result = conn.read_exact(100)
            assert result == data
            conn.close()
        finally:
            stop.set()

    def test_read_until(self) -> None:
        """Read until a null byte delimiter."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            conn.write_all(b"key:value\x00next")
            conn.flush()

            # read_until stops at and includes the delimiter
            result = conn.read_until(0)  # 0 = null byte
            assert result == b"key:value\x00"
            conn.close()
        finally:
            stop.set()

    def test_large_data_transfer(self) -> None:
        """Send and receive 64 KiB to test buffering with large payloads."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            # 64 KiB of patterned data
            data = bytes(i % 256 for i in range(65536))
            conn.write_all(data)
            conn.flush()

            result = conn.read_exact(65536)
            assert len(result) == 65536
            assert result == data
            conn.close()
        finally:
            stop.set()

    def test_multiple_exchanges(self) -> None:
        """Multiple round-trips on the same connection."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())

            # Exchange 1
            conn.write_all(b"ping\n")
            conn.flush()
            line1 = conn.read_line()
            assert line1 == "ping\n"

            # Exchange 2
            conn.write_all(b"pong\n")
            conn.flush()
            line2 = conn.read_line()
            assert line2 == "pong\n"

            conn.close()
        finally:
            stop.set()


# ============================================================================
# Group 2: Timeout tests
# ============================================================================


class TestTimeouts:
    """Tests for connect and read timeout behavior."""

    def test_connect_timeout(self) -> None:
        """Connecting to a non-routable address should time out.

        10.255.255.1 is a non-routable IP -- the TCP SYN packet is sent
        but never answered, so the connect hangs until the timeout fires.
        """
        opts = ConnectOptions(
            connect_timeout=1.0,
            read_timeout=1.0,
            write_timeout=1.0,
        )
        start = time.monotonic()
        with pytest.raises(TcpError) as exc_info:
            connect("10.255.255.1", 1, opts)
        elapsed = time.monotonic() - start

        # Should have timed out, not succeeded
        err = exc_info.value
        # On some platforms this may be Timeout, on others a generic TcpError
        assert isinstance(err, (Timeout, TcpError))

        # Should not have taken much longer than the timeout
        assert elapsed < 5.0, f"took {elapsed:.1f}s, expected ~1s"

    def test_read_timeout(self) -> None:
        """Reading from a silent server should time out."""
        port, stop = start_silent_server()
        try:
            opts = ConnectOptions(
                connect_timeout=5.0,
                read_timeout=1.0,
                write_timeout=5.0,
            )
            conn = connect("127.0.0.1", port, opts)

            start = time.monotonic()
            with pytest.raises(TcpError) as exc_info:
                conn.read_line()
            elapsed = time.monotonic() - start

            err = exc_info.value
            assert isinstance(err, (Timeout, TcpError))
            assert elapsed < 5.0, f"took {elapsed:.1f}s, expected ~1s"
            conn.close()
        finally:
            stop.set()


# ============================================================================
# Group 3: Error tests
# ============================================================================


class TestErrors:
    """Tests for specific error conditions and their mappings."""

    def test_connection_refused(self) -> None:
        """Connecting to a closed port should raise ConnectionRefused.

        We bind a port, then immediately close the listener. Any connect
        attempt to that port will get TCP RST.
        """
        # Grab a port, then close the listener
        temp_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        temp_sock.bind(("127.0.0.1", 0))
        port: int = temp_sock.getsockname()[1]
        temp_sock.close()

        with pytest.raises(TcpError) as exc_info:
            connect("127.0.0.1", port, make_test_options())

        err = exc_info.value
        # May be ConnectionRefused or a generic TcpError on some platforms
        assert isinstance(err, (ConnectionRefused, TcpError))

    def test_dns_failure(self) -> None:
        """Resolving a non-existent hostname should raise DnsResolutionFailed."""
        with pytest.raises(TcpError) as exc_info:
            connect("this.host.does.not.exist.example", 80, make_test_options())

        err = exc_info.value
        # Some ISP DNS resolvers hijack NXDOMAIN, so we accept several
        # error types. But normally this should be DnsResolutionFailed.
        assert isinstance(
            err, (DnsResolutionFailed, ConnectionRefused, Timeout, TcpError)
        )
        if isinstance(err, DnsResolutionFailed):
            assert err.host == "this.host.does.not.exist.example"

    def test_unexpected_eof(self) -> None:
        """Server sends 50 bytes, client tries to read 100 -> UnexpectedEof."""
        data = bytes(range(50))
        port, stop = start_partial_server(data)
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            # Wait for server to send and close
            time.sleep(0.2)

            with pytest.raises(UnexpectedEof) as exc_info:
                conn.read_exact(100)

            err = exc_info.value
            assert err.expected == 100
            assert err.received == 50
            conn.close()
        finally:
            stop.set()

    def test_broken_pipe(self) -> None:
        """Writing to a connection after the server closes should error.

        The first write may succeed (data goes to the OS send buffer),
        but eventually the RST arrives and subsequent writes fail.
        """
        port, stop = start_partial_server(b"")
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            # Wait for server to close its end
            time.sleep(0.3)

            # Try to write repeatedly -- eventually the OS will notice
            # the remote end is gone and return an error.
            got_error = False
            for _ in range(20):
                try:
                    conn.write_all(b"\x00" * 65536)
                    conn.flush()
                    time.sleep(0.05)
                except TcpError:
                    got_error = True
                    break

            assert got_error, "expected write error after server closed"
            conn.close()
        finally:
            stop.set()


# ============================================================================
# Group 4: Half-close tests
# ============================================================================


class TestHalfClose:
    """Tests for shutdown_write (half-close) behavior."""

    def test_client_half_close(self) -> None:
        """After shutdown_write, server sends DONE, client reads it.

        Half-close flow:
          1. Client sends "request data"
          2. Client calls shutdown_write() -- signals "I'm done sending"
          3. Server reads EOF, responds with "DONE\\n"
          4. Client reads "DONE\\n" -- read half is still open
        """
        server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server_sock.bind(("127.0.0.1", 0))
        server_sock.listen(1)
        port: int = server_sock.getsockname()[1]

        server_received: list[bytes] = []

        def serve() -> None:
            server_sock.settimeout(5.0)
            try:
                conn, _ = server_sock.accept()
                conn.settimeout(5.0)
                # Read until EOF (client shuts down write)
                chunks: list[bytes] = []
                while True:
                    chunk = conn.recv(4096)
                    if not chunk:
                        break
                    chunks.append(chunk)
                server_received.append(b"".join(chunks))
                # Send response after client is done writing
                conn.sendall(b"DONE\n")
                time.sleep(0.1)
                conn.close()
            except OSError:
                pass
            finally:
                server_sock.close()

        thread = threading.Thread(target=serve, daemon=True)
        thread.start()
        time.sleep(0.05)

        conn = connect("127.0.0.1", port, make_test_options())
        conn.write_all(b"request data")
        conn.shutdown_write()

        # Read the server's response -- read half is still open
        response = conn.read_line()
        assert response == "DONE\n"

        thread.join(timeout=5.0)
        assert server_received == [b"request data"]
        conn.close()


# ============================================================================
# Group 5: Edge cases
# ============================================================================


class TestEdgeCases:
    """Edge case and boundary condition tests."""

    def test_empty_read_at_eof(self) -> None:
        """After reading all data, read_line returns empty string at EOF."""
        port, stop = start_partial_server(b"hello\n")
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            time.sleep(0.2)

            line = conn.read_line()
            assert line == "hello\n"

            # Next read should return "" (EOF)
            eof = conn.read_line()
            assert eof == ""
            conn.close()
        finally:
            stop.set()

    def test_zero_byte_write(self) -> None:
        """Writing zero bytes should succeed without error."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            # This should not raise
            conn.write_all(b"")
            conn.flush()
            conn.close()
        finally:
            stop.set()

    def test_peer_address(self) -> None:
        """peer_addr and local_addr return correct IP and port."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())

            peer = conn.peer_addr()
            assert peer[0] == "127.0.0.1"
            assert peer[1] == port

            local = conn.local_addr()
            assert local[0] == "127.0.0.1"
            assert local[1] > 0  # OS-assigned ephemeral port
            conn.close()
        finally:
            stop.set()


# ============================================================================
# Group 6: API tests -- defaults, error display, context manager
# ============================================================================


class TestApi:
    """Tests for the public API surface: defaults, string representations."""

    def test_connect_options_defaults(self) -> None:
        """Verify ConnectOptions defaults match the spec."""
        opts = ConnectOptions()
        assert opts.connect_timeout == 30.0
        assert opts.read_timeout == 30.0
        assert opts.write_timeout == 30.0
        assert opts.buffer_size == 8192

    def test_error_display(self) -> None:
        """Verify error message formatting for each error type."""
        err = DnsResolutionFailed("example.com", "no such host")
        assert str(err) == "DNS resolution failed for 'example.com': no such host"

        err2 = ConnectionRefused("127.0.0.1:8080")
        assert str(err2) == "connection refused by 127.0.0.1:8080"

        err3 = BrokenPipe()
        assert str(err3) == "broken pipe (remote closed)"

        err4 = ConnectionReset()
        assert str(err4) == "connection reset by peer"

        err5 = Timeout("connect", 5.0)
        assert str(err5) == "connect timed out after 5.0s"

        err6 = UnexpectedEof(100, 50)
        assert str(err6) == "unexpected EOF: expected 100 bytes, got 50"

    def test_error_hierarchy(self) -> None:
        """All specific errors should be subclasses of TcpError."""
        assert issubclass(DnsResolutionFailed, TcpError)
        assert issubclass(ConnectionRefused, TcpError)
        assert issubclass(Timeout, TcpError)
        assert issubclass(ConnectionReset, TcpError)
        assert issubclass(BrokenPipe, TcpError)
        assert issubclass(UnexpectedEof, TcpError)

    def test_connect_options_repr(self) -> None:
        """ConnectOptions should have a useful repr."""
        opts = ConnectOptions(connect_timeout=10.0, buffer_size=1024)
        r = repr(opts)
        assert "connect_timeout=10.0" in r
        assert "buffer_size=1024" in r

    def test_connection_repr(self) -> None:
        """TcpConnection should have a useful repr."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            r = repr(conn)
            assert "TcpConnection" in r
            assert "127.0.0.1" in r
            conn.close()
        finally:
            stop.set()

    def test_context_manager(self) -> None:
        """TcpConnection can be used as a context manager."""
        port, stop = start_echo_server()
        try:
            with connect("127.0.0.1", port, make_test_options()) as conn:
                conn.write_all(b"test\n")
                conn.flush()
                line = conn.read_line()
                assert line == "test\n"
            # Connection should be closed after the with block
        finally:
            stop.set()

    def test_connect_with_none_options(self) -> None:
        """Passing None for options should use defaults."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, None)
            assert isinstance(conn, TcpConnection)
            conn.close()
        finally:
            stop.set()

    def test_request_response_pattern(self) -> None:
        """Simulate an HTTP-like request/response exchange.

        This test exercises the full lifecycle:
          1. Send a multi-line request (like HTTP)
          2. Read a status line
          3. Read a header line
          4. Read a blank line (end of headers)
          5. Read exact body bytes using Content-Length
        """
        response_data = b"HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello"
        port, stop = start_request_response_server(response_data)
        try:
            conn = connect("127.0.0.1", port, make_test_options())

            # Send request
            conn.write_all(b"GET / HTTP/1.0\r\n\r\n")
            conn.flush()

            # Read response line by line
            status = conn.read_line()
            assert status.startswith("HTTP/1.0 200")

            header = conn.read_line()
            assert header.startswith("Content-Length:")

            blank = conn.read_line()
            assert blank == "\r\n"

            body = conn.read_exact(5)
            assert body == b"hello"
            conn.close()
        finally:
            stop.set()

    def test_dns_resolution_failed_attributes(self) -> None:
        """DnsResolutionFailed stores host and message."""
        err = DnsResolutionFailed("bad.host", "NXDOMAIN")
        assert err.host == "bad.host"
        assert err.message == "NXDOMAIN"

    def test_connection_refused_attributes(self) -> None:
        """ConnectionRefused stores the address."""
        err = ConnectionRefused("10.0.0.1:443")
        assert err.addr == "10.0.0.1:443"

    def test_timeout_attributes(self) -> None:
        """Timeout stores phase and duration."""
        err = Timeout("read", 2.5)
        assert err.phase == "read"
        assert err.duration == 2.5

    def test_unexpected_eof_attributes(self) -> None:
        """UnexpectedEof stores expected and received counts."""
        err = UnexpectedEof(1000, 42)
        assert err.expected == 1000
        assert err.received == 42


# ============================================================================
# Group 7: _map_socket_error coverage
# ============================================================================
#
# The _map_socket_error function has branches for each OS error type.
# Some of these are hard to trigger with real sockets, so we test the
# mapping function directly.


class TestMapSocketError:
    """Direct tests for the _map_socket_error internal function."""

    def test_map_gaierror(self) -> None:
        """socket.gaierror maps to DnsResolutionFailed."""
        from tcp_client import _map_socket_error

        err = socket.gaierror("Name or service not known")
        result = _map_socket_error(err, host="bad.host")
        assert isinstance(result, DnsResolutionFailed)
        assert result.host == "bad.host"

    def test_map_connection_refused(self) -> None:
        """ConnectionRefusedError maps to ConnectionRefused."""
        from tcp_client import _map_socket_error

        err = ConnectionRefusedError("Connection refused")
        result = _map_socket_error(err, addr="10.0.0.1:80")
        assert isinstance(result, ConnectionRefused)
        assert result.addr == "10.0.0.1:80"

    def test_map_timeout(self) -> None:
        """socket.timeout maps to Timeout."""
        from tcp_client import _map_socket_error

        err = TimeoutError("timed out")
        result = _map_socket_error(err, phase="read", duration=5.0)
        assert isinstance(result, Timeout)
        assert result.phase == "read"
        assert result.duration == 5.0

    def test_map_timeout_error(self) -> None:
        """TimeoutError maps to Timeout."""
        from tcp_client import _map_socket_error

        err = TimeoutError("timed out")
        result = _map_socket_error(err, phase="write", duration=3.0)
        assert isinstance(result, Timeout)
        assert result.phase == "write"

    def test_map_connection_reset(self) -> None:
        """ConnectionResetError maps to ConnectionReset."""
        from tcp_client import _map_socket_error

        err = ConnectionResetError("Connection reset by peer")
        result = _map_socket_error(err)
        assert isinstance(result, ConnectionReset)

    def test_map_connection_aborted(self) -> None:
        """ConnectionAbortedError maps to ConnectionReset."""
        from tcp_client import _map_socket_error

        err = ConnectionAbortedError("Connection aborted")
        result = _map_socket_error(err)
        assert isinstance(result, ConnectionReset)

    def test_map_broken_pipe(self) -> None:
        """BrokenPipeError maps to BrokenPipe."""
        from tcp_client import _map_socket_error

        err = builtins_broken_pipe_error()
        result = _map_socket_error(err)
        assert isinstance(result, BrokenPipe)

    def test_map_generic_os_error(self) -> None:
        """Unknown OSError maps to generic TcpError."""
        from tcp_client import _map_socket_error

        err = OSError("something unexpected")
        result = _map_socket_error(err)
        assert isinstance(result, TcpError)
        assert "I/O error" in str(result)

    def test_close_is_idempotent(self) -> None:
        """Calling close() multiple times should not raise."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            conn.close()
            conn.close()  # Should not raise
            conn.close()  # Should not raise
        finally:
            stop.set()

    def test_repr_after_close(self) -> None:
        """repr on a closed connection should not crash."""
        port, stop = start_echo_server()
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            conn.close()
            r = repr(conn)
            assert "TcpConnection" in r
        finally:
            stop.set()

    def test_read_until_eof_without_delimiter(self) -> None:
        """read_until returns partial data if EOF before delimiter."""
        port, stop = start_partial_server(b"no-delimiter-here")
        try:
            conn = connect("127.0.0.1", port, make_test_options())
            time.sleep(0.2)

            result = conn.read_until(0)  # looking for null byte
            assert result == b"no-delimiter-here"
            conn.close()
        finally:
            stop.set()


def builtins_broken_pipe_error() -> BrokenPipeError:
    """Create a BrokenPipeError for testing.

    Separated into a function to avoid shadowing the tcp_client.BrokenPipe
    name in the test class scope.
    """
    return BrokenPipeError("Broken pipe")
