"""
Tests for the DT24 TCP server.

Strategy
────────
Because the server blocks in serve_forever(), every test starts the server
in a daemon thread. A daemon thread is killed automatically when the main
thread exits, so no test can hang the test suite even if cleanup fails.

We use ports in the 16380–16399 range to avoid conflicts with system services
and with each other. Each test class uses a distinct port.

A short ``time.sleep(0.05)`` after starting the server gives the event loop
time to enter sel.select() before the test client tries to connect. Without
this, there is a race between the test client's connect() and the server's
bind()/listen().

Fixtures
────────
``start_server_thread``
    Starts the server in a background daemon thread and yields it to the test.
    Calls server.stop() in teardown.

``send_recv``
    Opens a TCP connection, sends data, receives the response, closes. Used by
    most tests that need a simple request-response exchange.
"""

from __future__ import annotations

import socket
import threading
import time

import pytest

from tcp_server import TcpServer


# ---------------------------------------------------------------------------
# Helper utilities
# ---------------------------------------------------------------------------


def send_recv(port: int, data: bytes, host: str = "127.0.0.1") -> bytes:
    """
    Open a TCP connection to host:port, send ``data``, receive the response.

    Uses a fresh socket each call so tests are independent. The server's
    event loop will process the request and close the connection when the
    client socket closes (TCP FIN triggers the recv()-returns-b"" path in
    _handle_client).
    """
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect((host, port))
        s.sendall(data)
        # Allow enough time for the round-trip through the event loop
        s.settimeout(5.0)
        return s.recv(4096)


def start_server(server: TcpServer) -> threading.Thread:
    """
    Start a TcpServer in a background daemon thread and return the thread.

    The thread is marked daemon=True so Python does not wait for it to
    finish when the test process exits. The thread runs serve_forever()
    which calls start() then serve() internally.
    """
    t = threading.Thread(target=server.serve_forever, daemon=True)
    t.start()
    # Give the server time to bind and enter the event loop before tests
    # start connecting. 50 ms is more than enough on any modern machine.
    time.sleep(0.05)
    return t


# ---------------------------------------------------------------------------
# Basic functionality
# ---------------------------------------------------------------------------


class TestTcpServerBasic:
    """Core request-response behaviour."""

    def test_echo_server(self) -> None:
        """
        Default handler echoes every byte back unchanged.

        The server reads b"hello world" and returns b"hello world".
        This verifies the complete read → handler → write pipeline.
        """
        server = TcpServer(host="127.0.0.1", port=16380, handler=lambda d: d)
        start_server(server)
        try:
            response = send_recv(16380, b"hello world")
            assert response == b"hello world"
        finally:
            server.stop()

    def test_custom_handler(self) -> None:
        """
        A custom handler receives the raw bytes and can transform them.

        This test uses an uppercase handler to verify the handler's return
        value is sent back to the client.
        """

        def shout(data: bytes) -> bytes:
            # Transform: every ASCII letter becomes uppercase.
            return data.upper()

        server = TcpServer(host="127.0.0.1", port=16381, handler=shout)
        start_server(server)
        try:
            response = send_recv(16381, b"hello")
            assert response == b"HELLO"
        finally:
            server.stop()

    def test_multiple_sequential_clients(self) -> None:
        """
        Multiple sequential client connections are all handled correctly.

        Each connection goes through the full accept → read → handle → write
        → close lifecycle. This tests that the server correctly cleans up
        after each client and accepts the next one.
        """
        server = TcpServer(host="127.0.0.1", port=16382, handler=lambda d: d)
        start_server(server)
        try:
            for i in range(5):
                msg = f"message {i}".encode()
                assert send_recv(16382, msg) == msg
        finally:
            server.stop()

    def test_default_echo_handler(self) -> None:
        """
        When handler=None, the server defaults to an echo handler.

        This tests the default argument code path: the lambda inside __init__
        is used when no handler is supplied.
        """
        server = TcpServer(host="127.0.0.1", port=16383)
        start_server(server)
        try:
            assert send_recv(16383, b"ping") == b"ping"
        finally:
            server.stop()

    def test_binary_data(self) -> None:
        """
        Handler receives and returns binary (non-ASCII) data correctly.

        The server must not attempt to decode or encode bytes — it passes
        them through unchanged. This verifies the raw-bytes contract.
        """
        server = TcpServer(host="127.0.0.1", port=16384, handler=lambda d: d)
        start_server(server)
        try:
            binary_data = bytes(range(256))
            response = send_recv(16384, binary_data)
            assert response == binary_data
        finally:
            server.stop()

    def test_empty_response_from_handler(self) -> None:
        """
        A handler returning b"" results in no data being sent to the client.

        sendall(b"") is a no-op, so the client receives nothing. This is
        useful for fire-and-forget protocols where the server does not reply.
        """
        server = TcpServer(host="127.0.0.1", port=16385, handler=lambda d: b"")
        start_server(server)
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.connect(("127.0.0.1", 16385))
                s.sendall(b"hello")
                s.settimeout(0.2)
                # Server sends nothing back; recv() should time out or return b""
                try:
                    data = s.recv(4096)
                    # If recv returns immediately it should be b""
                    assert data == b""
                except TimeoutError:
                    pass  # timeout is expected: no data was sent
        finally:
            server.stop()


# ---------------------------------------------------------------------------
# Context manager
# ---------------------------------------------------------------------------


class TestContextManager:
    """__enter__ / __exit__ protocol."""

    def test_context_manager_starts_and_stops(self) -> None:
        """
        The context manager yields the server and calls stop() on exit.

        Inside the ``with`` block we start serve() in a thread and verify
        the server handles requests. After the block, stop() has been called.
        """
        received: list[bytes] = []

        def capture(data: bytes) -> bytes:
            received.append(data)
            return b"ok"

        with TcpServer(host="127.0.0.1", port=16390, handler=capture) as server:
            t = threading.Thread(target=server.serve_forever, daemon=True)
            t.start()
            time.sleep(0.05)
            result = send_recv(16390, b"test")
            assert result == b"ok"
            server.stop()
            t.join(timeout=1.0)

        assert b"test" in received

    def test_context_manager_stops_on_exception(self) -> None:
        """
        Even if the body of the ``with`` block raises, stop() is called.

        This verifies __exit__ always runs (Python's context manager guarantee).
        We check that the server is not running after the exception is caught.
        """
        server = TcpServer(host="127.0.0.1", port=16391)
        try:
            with server:
                raise ValueError("simulated failure")
        except ValueError:
            pass
        # stop() was called; the stop event should be set
        assert server._stop_event.is_set()


# ---------------------------------------------------------------------------
# Properties and repr
# ---------------------------------------------------------------------------


class TestProperties:
    """address, is_running, and __repr__."""

    def test_address_property_after_start(self) -> None:
        """
        address returns the (host, port) the server socket is bound to.

        After start(), getsockname() on the server socket reflects the actual
        bound address. This works even for port=0 (OS-assigned port).
        """
        server = TcpServer(host="127.0.0.1", port=16392)
        server.start()
        try:
            assert server.address == ("127.0.0.1", 16392)
        finally:
            server.stop()

    def test_address_property_before_start_raises(self) -> None:
        """
        address raises RuntimeError before start() is called.

        There is no socket yet, so getsockname() cannot be called.
        """
        server = TcpServer(host="127.0.0.1", port=16393)
        with pytest.raises(RuntimeError, match="not been started"):
            _ = server.address

    def test_is_running_lifecycle(self) -> None:
        """
        is_running transitions: False → True (after start) → False (after cleanup).

        The cleanup happens when serve() exits (after stop() is called).
        We use a separate thread to run serve() so we can observe is_running
        from the test thread.
        """
        server = TcpServer(host="127.0.0.1", port=16394)
        assert not server.is_running

        server.start()
        assert server.is_running

        # Run serve() in a thread so we can call stop() from here
        t = threading.Thread(target=server.serve, daemon=True)
        t.start()
        time.sleep(0.05)
        assert server.is_running

        server.stop()
        t.join(timeout=1.0)
        assert not server.is_running

    def test_repr_contains_host_port_status(self) -> None:
        """
        __repr__ includes the host, port, and current status.

        This makes debugging easier: print(server) gives actionable info.
        """
        server = TcpServer(host="127.0.0.1", port=16395)
        r = repr(server)
        assert "TcpServer" in r
        assert "127.0.0.1" in r
        assert "16395" in r
        assert "stopped" in r

    def test_repr_shows_running_when_started(self) -> None:
        """repr status changes to 'running' after start()."""
        server = TcpServer(host="127.0.0.1", port=16396)
        server.start()
        try:
            r = repr(server)
            assert "running" in r
        finally:
            server.stop()


# ---------------------------------------------------------------------------
# Shutdown behaviour
# ---------------------------------------------------------------------------


class TestShutdown:
    """stop() and clean shutdown."""

    def test_stop_ends_serve_loop(self) -> None:
        """
        stop() causes serve() to exit within a short time.

        We join the server thread with a 1-second timeout. If serve() does not
        exit, t.is_alive() will be True and the assertion fails.
        """
        server = TcpServer(host="127.0.0.1", port=16397)
        t = threading.Thread(target=server.serve_forever, daemon=True)
        t.start()
        time.sleep(0.05)

        server.stop()
        t.join(timeout=1.0)

        assert not t.is_alive(), "serve() did not exit after stop()"

    def test_server_handles_client_disconnect_gracefully(self) -> None:
        """
        A client that connects and immediately closes does not crash the server.

        When recv() returns b"", the server must unregister and close the fd
        without raising an exception. The server then accepts the next client.
        """
        server = TcpServer(host="127.0.0.1", port=16398, handler=lambda d: d)
        start_server(server)
        try:
            # Connect and immediately disconnect (no data sent)
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.connect(("127.0.0.1", 16398))
            s.close()

            # Wait for the server to process the disconnect
            time.sleep(0.1)

            # Server must still be running and accepting connections
            response = send_recv(16398, b"still alive")
            assert response == b"still alive"
        finally:
            server.stop()

    def test_multiple_stop_calls_are_safe(self) -> None:
        """
        Calling stop() multiple times does not raise.

        threading.Event.set() is idempotent; this test verifies our stop()
        is too.
        """
        server = TcpServer(host="127.0.0.1", port=16399)
        t = threading.Thread(target=server.serve_forever, daemon=True)
        t.start()
        time.sleep(0.05)

        server.stop()
        server.stop()  # should not raise
        server.stop()  # should not raise

        t.join(timeout=1.0)
        assert not t.is_alive()


# ---------------------------------------------------------------------------
# Buffer size configuration
# ---------------------------------------------------------------------------


class TestErrorHandling:
    """Tests for error code-paths: handler exceptions and cleanup with live connections."""

    def test_handler_exception_closes_client_keeps_server_alive(self) -> None:
        """
        A handler that raises an exception must close that client connection
        but keep the server event loop running for subsequent clients.

        This exercises the ``except Exception`` branch in _handle_client
        (lines 387-391 in tcp_server.py).  The server catches the exception,
        unregisters and closes the offending socket, then returns to sel.select()
        to accept new connections.
        """
        call_count = [0]

        def sometimes_bad(data: bytes) -> bytes:
            """Raise on the first call; echo on all subsequent calls."""
            call_count[0] += 1
            if call_count[0] == 1:
                raise RuntimeError("intentional crash on first request")
            return data

        server = TcpServer(host="127.0.0.1", port=16372, handler=sometimes_bad)
        start_server(server)
        try:
            # First connection: handler raises, server closes this client's socket.
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.connect(("127.0.0.1", 16372))
                s.sendall(b"crash me")
                s.settimeout(1.0)
                try:
                    # Server closes the connection after the handler crash.
                    s.recv(4096)
                except (ConnectionResetError, OSError):
                    pass  # platform-specific: some OSes send RST, others FIN

            # Give the event loop a moment to clean up and return to select().
            time.sleep(0.1)

            # Server is still alive; subsequent connections work normally.
            response = send_recv(16372, b"still alive")
            assert response == b"still alive"
        finally:
            server.stop()

    def test_cleanup_closes_active_client_connections(self) -> None:
        """
        _cleanup() must close client sockets that are still registered in the
        selector when stop() is called.

        This exercises the cleanup loop in _cleanup() (lines 424-429 in
        tcp_server.py) that iterates sel.get_map() and closes any remaining
        connected clients.

        Strategy: connect a client and keep the TCP connection open (no FIN).
        After the server processes our data the client socket remains registered
        in the selector waiting for more input.  Calling stop() then exercises
        the cleanup loop.
        """
        server = TcpServer(host="127.0.0.1", port=16373, handler=lambda d: d)
        t = start_server(server)

        client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        try:
            client.connect(("127.0.0.1", 16373))
            # Send data so the server accepts, registers, and processes this client.
            # The client stays connected (no close/FIN) so its socket remains
            # in the selector's fd map when we call stop().
            client.sendall(b"keep-alive")
            # Wait for the server to accept and register the client socket.
            time.sleep(0.1)

            # Stop the server while the client is still in sel.get_map().
            # _cleanup() must unregister and close it (covering lines 424-429).
            server.stop()
            t.join(timeout=2.0)
        finally:
            client.close()

        assert not t.is_alive(), "serve() did not exit after stop()"


class TestConfiguration:
    """Custom buffer_size and backlog parameters."""

    def test_custom_buffer_size(self) -> None:
        """
        A small buffer_size does not break the server for small messages.

        With buffer_size=8, a 5-byte message fits in one recv() call.
        The server still echoes it correctly.
        """
        server = TcpServer(
            host="127.0.0.1",
            port=16370,
            handler=lambda d: d,
            buffer_size=8,
        )
        start_server(server)
        try:
            response = send_recv(16370, b"hello")
            assert response == b"hello"
        finally:
            server.stop()

    def test_large_message(self) -> None:
        """
        A message larger than buffer_size arrives in multiple recv() calls.

        Each recv() call invokes the handler with whatever bytes arrived in
        that chunk. The client receives multiple response chunks. We verify
        the total bytes round-trip correctly.

        Note: with the simple bytes→bytes API, partial reads result in
        partial handler calls. The spec's note on "partial read buffering"
        applies at the DT25 layer, not DT24.
        """
        chunks_received: list[bytes] = []

        def collect(data: bytes) -> bytes:
            chunks_received.append(data)
            return data

        server = TcpServer(
            host="127.0.0.1",
            port=16371,
            handler=collect,
            buffer_size=16,
        )
        start_server(server)
        try:
            payload = b"A" * 64  # 64 bytes, buffer=16 → at least 4 recv() calls
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.connect(("127.0.0.1", 16371))
                s.sendall(payload)
                s.settimeout(2.0)
                # Read until we get all 64 bytes back (may arrive in chunks)
                total = b""
                while len(total) < len(payload):
                    chunk = s.recv(256)
                    if not chunk:
                        break
                    total += chunk
            assert total == payload
        finally:
            server.stop()
