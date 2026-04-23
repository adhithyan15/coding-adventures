"""Tests for irc-net-stdlib.

All tests use **real TCP sockets** — no mocking.  This philosophy follows the
coding-adventures convention that mocking hides real bugs.  A fake socket
cannot reveal timing issues, OS buffer behaviour, or shutdown races.  Real
sockets can.

Test design
===========
The ``EchoHandler`` is the simplest possible ``Handler`` implementation: when
it receives data on a connection, it echoes those bytes right back.  This lets
us verify the full round-trip:

    test client ──TCP──► StdlibListener ──► worker thread ──► EchoHandler
                                                                      │
    test client ◄─TCP──  StdlibConnection.write() ◄── loop.send_to() ◄┘

Each test class isolates one aspect of the event loop's contract.  Tests that
need a running server spin it up in a background thread and coordinate with
``threading.Event`` objects.

Port selection
==============
We use port 0 for all tests, which asks the OS to assign an ephemeral port
from the high range (usually 49152–65535).  This avoids conflicts with other
tests or services running on the machine.  We read the actual assigned port
back from the listener socket with ``sock.getsockname()[1]``.

Why port 0?  Because fixed ports like 16667 fail when the port is already in
use (e.g. a previous test crashed and left the socket open, or another process
is listening).  Port 0 never fails.
"""

from __future__ import annotations

import socket
import threading
import time

from irc_net_stdlib import (
    ConnId,
    Handler,  # used in _start_loop type annotation
    StdlibConnection,
    StdlibEventLoop,
    StdlibListener,
    __version__,
    create_listener,
)

# ---------------------------------------------------------------------------
# Helpers and shared fixtures
# ---------------------------------------------------------------------------


def _make_listener(
    host: str = "127.0.0.1",
    port: int = 0,
) -> tuple[StdlibListener, int]:
    """Create a listener on an OS-assigned port and return (listener, port).

    Using port 0 lets the OS pick a free ephemeral port, eliminating port
    conflicts between tests and with other services.
    """
    # We need to peek at the assigned port before handing the socket to
    # StdlibListener, so we create the socket ourselves here.
    raw = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    raw.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    raw.bind((host, port))
    raw.listen(128)
    assigned_port: int = raw.getsockname()[1]
    return StdlibListener(raw), assigned_port


def _connect(port: int) -> socket.socket:
    """Open a TCP connection to 127.0.0.1:port and return the client socket."""
    return socket.create_connection(("127.0.0.1", port))


def _recv_all_timeout(sock: socket.socket, n: int, timeout: float = 2.0) -> bytes:
    """Receive exactly *n* bytes from *sock*, timing out after *timeout* seconds.

    IRC is line-oriented, but for test purposes we know exactly how many bytes
    we expect to receive.  This helper loops until we have all of them or time
    runs out, raising AssertionError on timeout.

    We set a recv timeout on the socket so we do not block forever if the
    server fails to send data.
    """
    sock.settimeout(timeout)
    buf = b""
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            raise AssertionError(
                f"Connection closed after receiving {len(buf)!r} bytes "
                f"(wanted {n})"
            )
        buf += chunk
    return buf


class EchoHandler:
    """A Handler that echoes every received byte back to the sender.

    This is the simplest non-trivial Handler: it exercises on_connect,
    on_data, and on_disconnect without any IRC-specific logic.

    Thread safety note: the Handler is called from worker threads, but always
    under StdlibEventLoop._handler_lock.  So only one thread at a time calls
    any of these methods — no additional locking is needed here.
    """

    def __init__(self, loop: StdlibEventLoop) -> None:
        # Reference to the event loop so we can call send_to() from on_data.
        self._loop = loop

        # Set of currently connected conn_ids.  Updated in on_connect and
        # on_disconnect.  Safe to read/write without a lock because all handler
        # callbacks are serialised by _handler_lock.
        self.connected: set[ConnId] = set()

        # Record every on_connect call as (conn_id, host) for test assertions.
        self.connect_events: list[tuple[ConnId, str]] = []

        # Record every on_disconnect call as conn_id for test assertions.
        self.disconnect_events: list[ConnId] = []

        # Threading events used to synchronise test assertions.
        # Tests can wait on these rather than sleeping.
        self.on_connect_event = threading.Event()
        self.on_disconnect_event = threading.Event()

    def on_connect(self, conn_id: ConnId, host: str) -> None:
        """Record the connection and signal any waiting tests."""
        self.connected.add(conn_id)
        self.connect_events.append((conn_id, host))
        self.on_connect_event.set()

    def on_data(self, conn_id: ConnId, data: bytes) -> None:
        """Echo the data back to the sender."""
        # send_to is safe to call here even though we are inside a handler
        # callback — it acquires _conns_lock (not _handler_lock), so there is
        # no deadlock risk.
        self._loop.send_to(conn_id, data)

    def on_disconnect(self, conn_id: ConnId) -> None:
        """Record the disconnection and signal any waiting tests."""
        self.connected.discard(conn_id)
        self.disconnect_events.append(conn_id)
        self.on_disconnect_event.set()


def _start_loop(
    loop: StdlibEventLoop,
    listener: StdlibListener,
    handler: Handler,
) -> threading.Thread:
    """Start the event loop in a daemon background thread and return the thread.

    Using a daemon thread means the thread will not prevent the test process
    from exiting if a test hangs (pytest can still report and exit).
    """
    t = threading.Thread(target=loop.run, args=(listener, handler), daemon=True)
    t.start()
    return t


# ---------------------------------------------------------------------------
# 1. Echo test — basic round-trip
# ---------------------------------------------------------------------------


class TestEchoRoundTrip:
    """A single client sends data and gets the exact same bytes echoed back."""

    def test_echo_single_message(self) -> None:
        """Send b"hello\\r\\n" and receive b"hello\\r\\n" echoed back."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            try:
                client.sendall(b"hello\r\n")
                received = _recv_all_timeout(client, len(b"hello\r\n"))
                assert received == b"hello\r\n"
            finally:
                client.close()
        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)

    def test_echo_multiple_messages_in_sequence(self) -> None:
        """Send two distinct messages and verify both are echoed in order."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            try:
                msgs = [b"NICK alice\r\n", b"USER alice 0 * :Alice\r\n"]
                total = b"".join(msgs)
                client.sendall(total)
                received = _recv_all_timeout(client, len(total))
                assert received == total
            finally:
                client.close()
        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)

    def test_echo_binary_data(self) -> None:
        """Echo works for arbitrary byte values, not just ASCII text."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            try:
                payload = bytes(range(256))  # all 256 byte values
                client.sendall(payload)
                received = _recv_all_timeout(client, len(payload))
                assert received == payload
            finally:
                client.close()
        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 2. Multiple clients
# ---------------------------------------------------------------------------


class TestMultipleClients:
    """Five clients connect simultaneously; each gets its own echo stream."""

    def test_five_clients_echo_independently(self) -> None:
        """Each client sees only its own echoed data, not other clients'."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        n_clients = 5
        clients: list[socket.socket] = []

        try:
            # Open all connections before sending any data.
            for _ in range(n_clients):
                clients.append(_connect(port))

            # Send a unique message from each client and verify the echo.
            # We do this sequentially (one client at a time) so that the
            # received bytes are unambiguously from this client's echo.
            for i, client in enumerate(clients):
                msg = f"CLIENT{i}\r\n".encode()
                client.sendall(msg)
                received = _recv_all_timeout(client, len(msg))
                assert received == msg, (
                    f"Client {i}: expected {msg!r}, got {received!r}"
                )

        finally:
            for c in clients:
                c.close()
            loop.stop()
            srv_thread.join(timeout=2.0)

    def test_five_clients_all_connected_simultaneously(self) -> None:
        """on_connect fires for each of the five clients."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        n_clients = 5
        clients: list[socket.socket] = []

        try:
            for _ in range(n_clients):
                clients.append(_connect(port))

            # Give the server a moment to process all accept() calls and fire
            # all on_connect callbacks.  We poll rather than sleep to keep
            # tests fast.
            deadline = time.monotonic() + 2.0
            while (
                len(handler.connect_events) < n_clients
                and time.monotonic() < deadline
            ):
                time.sleep(0.01)

            assert len(handler.connect_events) == n_clients

        finally:
            for c in clients:
                c.close()
            loop.stop()
            srv_thread.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 3. on_connect called correctly
# ---------------------------------------------------------------------------


class TestOnConnect:
    """Verify on_connect is called with the right arguments."""

    def test_on_connect_fires_once_per_connection(self) -> None:
        """on_connect is called exactly once when a client connects."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            try:
                # Wait for the server to call on_connect.
                fired = handler.on_connect_event.wait(timeout=2.0)
                assert fired, "on_connect was not called within 2 seconds"
                assert len(handler.connect_events) == 1
            finally:
                client.close()
        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)

    def test_on_connect_receives_host_string(self) -> None:
        """on_connect is called with a non-empty host string."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            try:
                handler.on_connect_event.wait(timeout=2.0)
                # The host should be the loopback address (127.0.0.1).
                conn_id, host = handler.connect_events[0]
                assert isinstance(host, str)
                assert host  # non-empty
                assert host == "127.0.0.1"
            finally:
                client.close()
        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)

    def test_on_connect_conn_id_is_unique(self) -> None:
        """Each connection gets a distinct ConnId."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        n = 3
        clients: list[socket.socket] = []

        try:
            for _ in range(n):
                clients.append(_connect(port))

            deadline = time.monotonic() + 2.0
            while len(handler.connect_events) < n and time.monotonic() < deadline:
                time.sleep(0.01)

            ids = [cid for cid, _ in handler.connect_events]
            assert len(set(ids)) == n, f"Expected {n} unique ids, got {ids}"

        finally:
            for c in clients:
                c.close()
            loop.stop()
            srv_thread.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 4. on_disconnect called after client closes
# ---------------------------------------------------------------------------


class TestOnDisconnect:
    """on_disconnect fires after the client socket is closed."""

    def test_on_disconnect_fires_after_client_close(self) -> None:
        """Close the client socket; on_disconnect fires within 1 second."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)

            # Wait for the connection to be established.
            handler.on_connect_event.wait(timeout=2.0)
            assert len(handler.connect_events) == 1

            # Get the conn_id that was assigned.
            conn_id, _ = handler.connect_events[0]

            # Close the client socket.  This sends TCP FIN to the server.
            # The server's worker thread will next recv() return b"", which
            # triggers on_disconnect.
            client.close()

            # on_disconnect should fire within 1 second.
            fired = handler.on_disconnect_event.wait(timeout=1.0)
            assert fired, "on_disconnect was not called within 1 second"
            assert conn_id in handler.disconnect_events

        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)

    def test_on_disconnect_fires_for_each_client(self) -> None:
        """on_disconnect fires once per client that disconnects."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        n = 3
        clients: list[socket.socket] = []

        try:
            for _ in range(n):
                clients.append(_connect(port))

            # Wait for all connections to be established.
            deadline = time.monotonic() + 2.0
            while len(handler.connect_events) < n and time.monotonic() < deadline:
                time.sleep(0.01)

            # Close all clients.
            for c in clients:
                c.close()

            # All disconnects should fire within 2 seconds.
            deadline = time.monotonic() + 2.0
            while len(handler.disconnect_events) < n and time.monotonic() < deadline:
                time.sleep(0.01)

            assert len(handler.disconnect_events) == n

        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)

    def test_connected_set_empty_after_all_disconnect(self) -> None:
        """EchoHandler.connected is empty once all clients have disconnected."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            handler.on_connect_event.wait(timeout=2.0)
            assert len(handler.connected) == 1

            client.close()
            handler.on_disconnect_event.wait(timeout=1.0)

            # After disconnect, the conn_id should have been removed.
            assert len(handler.connected) == 0

        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 5. send_to unknown ConnId — must not crash
# ---------------------------------------------------------------------------


class TestSendToUnknown:
    """send_to with an unknown ConnId is a silent no-op."""

    def test_send_to_nonexistent_conn_id_does_not_raise(self) -> None:
        """Calling send_to with a made-up ConnId raises no exception."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        try:
            # ConnId(99999) almost certainly does not exist.
            loop.send_to(ConnId(99999), b"phantom\r\n")
            # If we reach here without an exception, the test passes.

        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)

    def test_send_to_after_client_disconnect_does_not_raise(self) -> None:
        """send_to a closed-and-removed conn_id is harmless."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            handler.on_connect_event.wait(timeout=2.0)
            conn_id, _ = handler.connect_events[0]

            # Close the client so the server cleans it up.
            client.close()
            handler.on_disconnect_event.wait(timeout=1.0)

            # Now conn_id should be gone from _conns.  send_to must not crash.
            loop.send_to(conn_id, b"too late\r\n")
            # Pass if no exception raised.

        finally:
            loop.stop()
            srv_thread.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 6. stop() causes run() to exit
# ---------------------------------------------------------------------------


class TestStop:
    """Calling stop() causes run() to return within a reasonable time."""

    def test_stop_exits_run_within_two_seconds(self) -> None:
        """After stop() is called, the run() thread should finish in ≤2 s."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        # Give the event loop a moment to start accepting.
        time.sleep(0.05)

        loop.stop()

        # join with a timeout — if run() does not exit we fail the test.
        srv_thread.join(timeout=2.0)
        assert not srv_thread.is_alive(), (
            "run() did not exit within 2 seconds after stop()"
        )

    def test_stop_idempotent(self) -> None:
        """Calling stop() twice does not raise."""
        loop = StdlibEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv_thread = _start_loop(loop, listener, handler)

        time.sleep(0.05)
        loop.stop()
        loop.stop()  # second call must not raise

        srv_thread.join(timeout=2.0)

    def test_stop_before_run_does_not_crash(self) -> None:
        """stop() is safe to call before run() has been invoked."""
        loop = StdlibEventLoop()
        loop.stop()  # _listener is None — must not raise


# ---------------------------------------------------------------------------
# 7. StdlibConnection unit tests
# ---------------------------------------------------------------------------


class TestStdlibConnection:
    """Unit tests for StdlibConnection in isolation (without the event loop)."""

    def _make_pair(self) -> tuple[StdlibConnection, socket.socket]:
        """Create a connected socket pair and wrap one end in StdlibConnection."""
        # socketpair gives us two connected sockets without needing a listener.
        # This is faster and simpler for testing connection behaviour in isolation.
        a, b = socket.socketpair()
        conn = StdlibConnection(a, ("127.0.0.1", 0))
        return conn, b

    def test_conn_id_is_unique(self) -> None:
        """Two StdlibConnections have distinct ids."""
        a_sock, b_sock = socket.socketpair()
        c_sock, d_sock = socket.socketpair()
        try:
            conn1 = StdlibConnection(a_sock, ("127.0.0.1", 1))
            conn2 = StdlibConnection(c_sock, ("127.0.0.1", 2))
            assert conn1.id != conn2.id
        finally:
            b_sock.close()
            d_sock.close()

    def test_peer_addr_matches_construction(self) -> None:
        """peer_addr returns the address passed to the constructor."""
        a_sock, b_sock = socket.socketpair()
        try:
            conn = StdlibConnection(a_sock, ("192.168.1.1", 12345))
            assert conn.peer_addr == ("192.168.1.1", 12345)
        finally:
            b_sock.close()

    def test_write_and_read_roundtrip(self) -> None:
        """Bytes written on one end arrive on the other."""
        conn, peer = self._make_pair()
        try:
            conn.write(b"hello")
            received = peer.recv(100)
            assert received == b"hello"
        finally:
            peer.close()

    def test_read_returns_empty_bytes_after_peer_close(self) -> None:
        """read() returns b"" when the peer has closed its end."""
        conn, peer = self._make_pair()
        peer.close()  # close the peer — conn's recv will return b""
        result = conn.read()
        assert result == b""

    def test_close_makes_subsequent_read_return_empty(self) -> None:
        """After close(), read() returns b"" (OSError → b"")."""
        conn, peer = self._make_pair()
        try:
            conn.close()
            result = conn.read()
            assert result == b""
        finally:
            peer.close()

    def test_write_after_close_does_not_raise(self) -> None:
        """write() after close() silently swallows the OSError."""
        conn, peer = self._make_pair()
        try:
            conn.close()
            conn.write(b"after close")  # must not raise
        finally:
            peer.close()

    def test_close_twice_does_not_raise(self) -> None:
        """Calling close() twice is safe."""
        conn, peer = self._make_pair()
        try:
            conn.close()
            conn.close()  # second call must not raise
        finally:
            peer.close()


# ---------------------------------------------------------------------------
# 8. StdlibListener unit tests
# ---------------------------------------------------------------------------


class TestStdlibListener:
    """Unit tests for StdlibListener in isolation."""

    def test_accept_returns_stdlib_connection(self) -> None:
        """accept() returns a StdlibConnection."""
        listener, port = _make_listener()
        client = _connect(port)

        try:
            conn = listener.accept()
            assert isinstance(conn, StdlibConnection)
        finally:
            conn.close()
            client.close()
            listener.close()

    def test_accepted_connection_has_correct_peer_addr_port(self) -> None:
        """The accepted connection's peer_addr port matches the client's port."""
        listener, port = _make_listener()
        client = _connect(port)

        try:
            conn = listener.accept()
            client_port = client.getsockname()[1]
            assert conn.peer_addr[1] == client_port
        finally:
            conn.close()
            client.close()
            listener.close()

    def test_close_makes_accept_raise(self) -> None:
        """Closing the listener causes a pending accept() to raise OSError."""
        listener, port = _make_listener()

        # Start a thread that blocks in accept().
        error_box: list[Exception] = []

        def _accept() -> None:
            try:
                listener.accept()
            except OSError as e:
                error_box.append(e)

        t = threading.Thread(target=_accept, daemon=True)
        t.start()

        # Give the thread a moment to enter accept().
        time.sleep(0.05)
        listener.close()

        t.join(timeout=2.0)
        assert not t.is_alive()
        assert len(error_box) == 1  # OSError was raised and caught


# ---------------------------------------------------------------------------
# 9. create_listener factory
# ---------------------------------------------------------------------------


class TestCreateListener:
    """create_listener() returns a working StdlibListener."""

    def test_create_listener_returns_listener(self) -> None:
        """create_listener returns a StdlibListener on a usable port."""
        # Use port 0 to get an OS-assigned ephemeral port.
        # We patch the listener's socket to inspect the actual port.
        import socket as _socket

        # Temporarily create and destroy a listener to confirm no exception.
        raw = _socket.socket(_socket.AF_INET, _socket.SOCK_STREAM)
        raw.setsockopt(_socket.SOL_SOCKET, _socket.SO_REUSEADDR, 1)
        raw.bind(("127.0.0.1", 0))
        actual_port: int = raw.getsockname()[1]
        raw.close()

        # Now use create_listener on a different ephemeral port (via 0 trick).
        # create_listener doesn't accept port=0 in its signature via the spec,
        # but our implementation does — so let's verify it works with a real port.
        listener = create_listener("127.0.0.1", actual_port)
        try:
            assert isinstance(listener, StdlibListener)
        finally:
            listener.close()

    def test_create_listener_accepts_connections(self) -> None:
        """A listener created with create_listener can accept a real connection."""
        # Get a free port.
        probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        probe.bind(("127.0.0.1", 0))
        free_port: int = probe.getsockname()[1]
        probe.close()

        listener = create_listener("127.0.0.1", free_port)
        client = _connect(free_port)

        try:
            conn = listener.accept()
            assert isinstance(conn, StdlibConnection)
        finally:
            conn.close()
            client.close()
            listener.close()


# ---------------------------------------------------------------------------
# 10. Version
# ---------------------------------------------------------------------------


class TestVersion:
    """Verify the package exports a correct version string."""

    def test_version_exists(self) -> None:
        """__version__ must be present and match pyproject.toml."""
        assert __version__ == "0.1.0"

    def test_version_is_string(self) -> None:
        """__version__ must be a str, not bytes or None."""
        assert isinstance(__version__, str)
