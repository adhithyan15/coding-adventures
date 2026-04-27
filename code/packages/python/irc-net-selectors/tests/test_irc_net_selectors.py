"""Tests for irc-net-selectors.

Philosophy: **real sockets only**.  We never mock the network.  A fake socket
cannot reveal timing races, OS buffer behaviour, or selector registration bugs.
Real sockets can.

Test design
===========
The same ``EchoHandler`` pattern used in ``irc-net-stdlib`` tests is reused
here.  When data arrives it echoes it back via ``loop.send_to()``.  This lets
us verify the full round-trip:

    test client ──TCP──► SelectorsListener ──► event loop
                                                    ├── on_connect
                                                    ├── on_data → send_to → enqueue → EVENT_WRITE → flush
    test client ◄──TCP── SelectorsConnection.flush() ◄──────────────────────────────────────────────┘

Selectors-specific tests
========================
Beyond the shared contract tests (echo round-trip, connect/disconnect events,
send_to safety), we add tests specific to the reactor model:

* **Single thread**: after connecting 100 clients, the thread count must be 2
  (main thread + run thread).  ``irc-net-stdlib`` would spawn 100 threads.
* **Write deregistration**: after a write buffer drains, the fd must not remain
  registered for ``EVENT_WRITE`` (which would cause a busy-loop).
* **Rapid connect/disconnect**: 50 rapid connect-and-disconnect cycles produce
  no resource leak (``_conns`` ends up empty).

Port strategy
=============
All tests use port 0 to get an OS-assigned ephemeral port, avoiding conflicts
with other services or previous test runs.  The actual port is read back from
the socket after binding.
"""

from __future__ import annotations

import socket
import threading
import time

from irc_net_selectors import (
    ConnId,
    Handler,
    SelectorsConnection,
    SelectorsEventLoop,
    SelectorsListener,
    __version__,
    create_listener,
)

# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------


def _make_listener(
    host: str = "127.0.0.1",
    port: int = 0,
) -> tuple[SelectorsListener, int]:
    """Create a listener on an OS-assigned port and return (listener, port)."""
    raw = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    raw.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    raw.bind((host, port))
    raw.listen(128)
    assigned_port: int = raw.getsockname()[1]
    return SelectorsListener(raw), assigned_port


def _connect(port: int) -> socket.socket:
    """Open a TCP connection to 127.0.0.1:port."""
    return socket.create_connection(("127.0.0.1", port))


def _recv_all_timeout(sock: socket.socket, n: int, timeout: float = 3.0) -> bytes:
    """Receive exactly *n* bytes, raising AssertionError on timeout."""
    sock.settimeout(timeout)
    buf = b""
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            raise AssertionError(
                f"Connection closed after {len(buf)} bytes (wanted {n})"
            )
        buf += chunk
    return buf


class EchoHandler:
    """Echoes every received byte back to the sender via ``loop.send_to()``.

    Because ``SelectorsEventLoop`` is single-threaded, none of these methods
    are ever called concurrently.  No locking is needed.
    """

    def __init__(self, loop: SelectorsEventLoop) -> None:
        self._loop = loop
        self.connected: set[ConnId] = set()
        self.connect_events: list[tuple[ConnId, str]] = []
        self.disconnect_events: list[ConnId] = []
        # Threading events for synchronising test assertions with the loop.
        self.on_connect_event = threading.Event()
        self.on_disconnect_event = threading.Event()

    def on_connect(self, conn_id: ConnId, host: str) -> None:
        self.connected.add(conn_id)
        self.connect_events.append((conn_id, host))
        self.on_connect_event.set()

    def on_data(self, conn_id: ConnId, data: bytes) -> None:
        self._loop.send_to(conn_id, data)

    def on_disconnect(self, conn_id: ConnId) -> None:
        self.connected.discard(conn_id)
        self.disconnect_events.append(conn_id)
        self.on_disconnect_event.set()


def _start_loop(
    loop: SelectorsEventLoop,
    listener: SelectorsListener,
    handler: Handler,
) -> threading.Thread:
    """Start the event loop in a background daemon thread."""
    t = threading.Thread(target=loop.run, args=(listener, handler), daemon=True)
    t.start()
    return t


# ---------------------------------------------------------------------------
# 1. Echo round-trip
# ---------------------------------------------------------------------------


class TestEchoRoundTrip:
    """Basic send → echo-back verification."""

    def test_echo_single_message(self) -> None:
        """Send b'hello\\r\\n' and receive the same bytes echoed back."""
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

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
            srv.join(timeout=2.0)

    def test_echo_multiple_messages_in_sequence(self) -> None:
        """Two messages sent back-to-back are both echoed correctly."""
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

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
            srv.join(timeout=2.0)

    def test_echo_binary_data(self) -> None:
        """All 256 byte values survive the round-trip unchanged."""
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            try:
                payload = bytes(range(256))
                client.sendall(payload)
                received = _recv_all_timeout(client, len(payload))
                assert received == payload
            finally:
                client.close()
        finally:
            loop.stop()
            srv.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 2. Multiple clients
# ---------------------------------------------------------------------------


class TestMultipleClients:
    """Multiple concurrent connections handled by the single event loop."""

    def test_five_clients_echo_independently(self) -> None:
        """Each client echoes only its own data, not other clients'."""
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        clients: list[socket.socket] = []
        try:
            for _ in range(5):
                clients.append(_connect(port))

            for i, client in enumerate(clients):
                msg = f"CLIENT{i}\r\n".encode()
                client.sendall(msg)
                received = _recv_all_timeout(client, len(msg))
                assert received == msg

        finally:
            for c in clients:
                c.close()
            loop.stop()
            srv.join(timeout=2.0)

    def test_five_clients_all_connected_simultaneously(self) -> None:
        """on_connect fires once for each of the five clients."""
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        clients: list[socket.socket] = []
        n = 5
        try:
            for _ in range(n):
                clients.append(_connect(port))

            deadline = time.monotonic() + 3.0
            while len(handler.connect_events) < n and time.monotonic() < deadline:
                time.sleep(0.01)

            assert len(handler.connect_events) == n

        finally:
            for c in clients:
                c.close()
            loop.stop()
            srv.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 3. on_connect
# ---------------------------------------------------------------------------


class TestOnConnect:
    """Verify on_connect arguments and uniqueness."""

    def test_on_connect_fires_once_per_connection(self) -> None:
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            try:
                fired = handler.on_connect_event.wait(timeout=2.0)
                assert fired, "on_connect was not called within 2 s"
                assert len(handler.connect_events) == 1
            finally:
                client.close()
        finally:
            loop.stop()
            srv.join(timeout=2.0)

    def test_on_connect_receives_loopback_host(self) -> None:
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            try:
                handler.on_connect_event.wait(timeout=2.0)
                conn_id, host = handler.connect_events[0]
                assert isinstance(host, str)
                assert host == "127.0.0.1"
            finally:
                client.close()
        finally:
            loop.stop()
            srv.join(timeout=2.0)

    def test_on_connect_conn_id_unique_across_connections(self) -> None:
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

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
            srv.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 4. on_disconnect
# ---------------------------------------------------------------------------


class TestOnDisconnect:
    """on_disconnect fires correctly after client closes."""

    def test_on_disconnect_fires_after_client_close(self) -> None:
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            handler.on_connect_event.wait(timeout=2.0)
            conn_id, _ = handler.connect_events[0]

            client.close()

            fired = handler.on_disconnect_event.wait(timeout=2.0)
            assert fired, "on_disconnect was not called within 2 s"
            assert conn_id in handler.disconnect_events

        finally:
            loop.stop()
            srv.join(timeout=2.0)

    def test_on_disconnect_fires_for_each_client(self) -> None:
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        n = 3
        clients: list[socket.socket] = []
        try:
            for _ in range(n):
                clients.append(_connect(port))

            deadline = time.monotonic() + 2.0
            while len(handler.connect_events) < n and time.monotonic() < deadline:
                time.sleep(0.01)

            for c in clients:
                c.close()

            deadline = time.monotonic() + 3.0
            while len(handler.disconnect_events) < n and time.monotonic() < deadline:
                time.sleep(0.01)

            assert len(handler.disconnect_events) == n

        finally:
            loop.stop()
            srv.join(timeout=2.0)

    def test_connected_set_empty_after_all_disconnect(self) -> None:
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            handler.on_connect_event.wait(timeout=2.0)
            assert len(handler.connected) == 1

            client.close()
            handler.on_disconnect_event.wait(timeout=2.0)
            assert len(handler.connected) == 0

        finally:
            loop.stop()
            srv.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 5. send_to safety
# ---------------------------------------------------------------------------


class TestSendToSafety:
    """send_to with an unknown or closed ConnId must not raise."""

    def test_send_to_nonexistent_conn_id_does_not_raise(self) -> None:
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        try:
            loop.send_to(ConnId(99999), b"phantom\r\n")
        finally:
            loop.stop()
            srv.join(timeout=2.0)

    def test_send_to_after_client_disconnect_does_not_raise(self) -> None:
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            handler.on_connect_event.wait(timeout=2.0)
            conn_id, _ = handler.connect_events[0]

            client.close()
            handler.on_disconnect_event.wait(timeout=2.0)

            loop.send_to(conn_id, b"too late\r\n")  # must not raise

        finally:
            loop.stop()
            srv.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 6. stop()
# ---------------------------------------------------------------------------


class TestStop:
    """stop() causes run() to exit promptly."""

    def test_stop_exits_run_within_two_seconds(self) -> None:
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        time.sleep(0.05)
        loop.stop()
        srv.join(timeout=2.0)
        assert not srv.is_alive(), "run() did not exit within 2 s after stop()"

    def test_stop_idempotent(self) -> None:
        """Calling stop() twice does not raise."""
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        time.sleep(0.05)
        loop.stop()
        loop.stop()
        srv.join(timeout=2.0)

    def test_stop_before_run_does_not_crash(self) -> None:
        """stop() is safe to call before run() is invoked."""
        loop = SelectorsEventLoop()
        loop.stop()  # _stop_event set, _sel is None — must not raise


# ---------------------------------------------------------------------------
# 7. Selectors-specific: no extra threads
# ---------------------------------------------------------------------------


class TestSingleThread:
    """The event loop must not spawn any threads — a key advantage over stdlib."""

    def test_no_extra_threads_with_ten_clients(self) -> None:
        """Connect 10 clients; thread count must be 2 (main + run thread)."""
        baseline = threading.active_count()
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        clients: list[socket.socket] = []
        try:
            for _ in range(10):
                clients.append(_connect(port))

            # Wait for all connections to be registered.
            deadline = time.monotonic() + 3.0
            while len(handler.connect_events) < 10 and time.monotonic() < deadline:
                time.sleep(0.01)

            # Thread count must be baseline + 1 (the run thread only).
            # irc-net-stdlib would be baseline + 10 here.
            after = threading.active_count()
            assert after == baseline + 1, (
                f"Expected {baseline + 1} threads, got {after} — "
                "the event loop must not spawn one thread per connection"
            )

        finally:
            for c in clients:
                c.close()
            loop.stop()
            srv.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 8. Selectors-specific: write deregistration
# ---------------------------------------------------------------------------


class TestWriteDeregistration:
    """After the write buffer drains, EVENT_WRITE must be removed."""

    def test_send_echo_and_write_drains(self) -> None:
        """After a send_to echo, the connection should drain and stop watching writes.

        We cannot directly inspect the selector's internal state, but we can
        verify the echo arrives correctly and the event loop continues
        accepting new events after the write (no busy-loop or hang).
        """
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        try:
            client = _connect(port)
            try:
                # Send a message and receive the echo.
                client.sendall(b"PING :test\r\n")
                received = _recv_all_timeout(client, len(b"PING :test\r\n"))
                assert received == b"PING :test\r\n"

                # Send a second message to confirm the loop is still responsive
                # (not stuck in a write busy-loop).
                client.sendall(b"PING :second\r\n")
                received2 = _recv_all_timeout(client, len(b"PING :second\r\n"))
                assert received2 == b"PING :second\r\n"

            finally:
                client.close()
        finally:
            loop.stop()
            srv.join(timeout=2.0)


# ---------------------------------------------------------------------------
# 9. Selectors-specific: rapid connect / disconnect
# ---------------------------------------------------------------------------


class TestRapidConnectDisconnect:
    """50 rapid connect-and-disconnect cycles leave no resource leaks."""

    def test_rapid_connect_disconnect_no_leak(self) -> None:
        """_conns must be empty after all 50 clients disconnect."""
        loop = SelectorsEventLoop()
        listener, port = _make_listener()
        handler = EchoHandler(loop)
        srv = _start_loop(loop, listener, handler)

        n = 50
        try:
            for _ in range(n):
                c = _connect(port)
                c.close()

            # Allow the event loop time to process all disconnect events.
            deadline = time.monotonic() + 5.0
            while len(handler.disconnect_events) < n and time.monotonic() < deadline:
                time.sleep(0.02)

            assert len(handler.disconnect_events) == n, (
                f"Expected {n} disconnect events, got {len(handler.disconnect_events)}"
            )
            # The connection table should be empty.
            assert len(loop._conns) == 0, (
                f"_conns should be empty; got {len(loop._conns)} entries"
            )

        finally:
            loop.stop()
            srv.join(timeout=3.0)


# ---------------------------------------------------------------------------
# 10. SelectorsConnection unit tests
# ---------------------------------------------------------------------------


class TestSelectorsConnection:
    """Unit tests for SelectorsConnection in isolation."""

    def _make_pair(self) -> tuple[SelectorsConnection, socket.socket]:
        """Two connected sockets: one wrapped in SelectorsConnection, one raw."""
        a, b = socket.socketpair()
        conn = SelectorsConnection(a, ("127.0.0.1", 0), ConnId(1))
        return conn, b

    def test_id_is_what_was_passed(self) -> None:
        a, b = socket.socketpair()
        try:
            conn = SelectorsConnection(a, ("127.0.0.1", 0), ConnId(42))
            assert conn.id == ConnId(42)
        finally:
            b.close()

    def test_peer_addr_matches_construction(self) -> None:
        a, b = socket.socketpair()
        try:
            conn = SelectorsConnection(a, ("10.0.0.1", 9999), ConnId(1))
            assert conn.peer_addr == ("10.0.0.1", 9999)
        finally:
            b.close()

    def test_enqueue_and_flush_round_trip(self) -> None:
        """Bytes enqueued then flushed arrive at the peer socket."""
        conn, peer = self._make_pair()
        # Make peer blocking so recv() works simply in the test.
        peer.setblocking(True)
        try:
            conn.enqueue(b"hello")
            conn.flush()
            received = peer.recv(100)
            assert received == b"hello"
        finally:
            peer.close()

    def test_flush_returns_true_when_buffer_empty(self) -> None:
        conn, peer = self._make_pair()
        try:
            assert conn.flush() is True  # nothing to flush
            conn.enqueue(b"data")
            conn.flush()
            assert conn.flush() is True  # buffer drained
        finally:
            peer.close()

    def test_read_available_returns_empty_on_peer_close(self) -> None:
        conn, peer = self._make_pair()
        peer.close()
        result = conn.read_available()
        assert result == b""

    def test_close_twice_does_not_raise(self) -> None:
        conn, peer = self._make_pair()
        try:
            conn.close()
            conn.close()  # second close must not raise
        finally:
            peer.close()

    def test_has_pending_writes_reflects_buffer_state(self) -> None:
        conn, peer = self._make_pair()
        try:
            assert not conn.has_pending_writes
            conn.enqueue(b"something")
            assert conn.has_pending_writes
            conn.flush()
            assert not conn.has_pending_writes
        finally:
            peer.close()


# ---------------------------------------------------------------------------
# 11. SelectorsListener unit tests
# ---------------------------------------------------------------------------


class TestSelectorsListener:
    """Unit tests for SelectorsListener in isolation."""

    def test_accept_returns_selectors_connection(self) -> None:
        listener, port = _make_listener()
        client = _connect(port)
        try:
            conn = listener.accept()
            assert isinstance(conn, SelectorsConnection)
        finally:
            conn.close()
            client.close()
            listener.close()

    def test_accepted_connection_peer_port_matches_client(self) -> None:
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


# ---------------------------------------------------------------------------
# 12. create_listener factory
# ---------------------------------------------------------------------------


class TestCreateListener:
    """create_listener() returns a working SelectorsListener."""

    def test_returns_selectors_listener(self) -> None:
        # Get a free port first.
        probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        probe.bind(("127.0.0.1", 0))
        free_port: int = probe.getsockname()[1]
        probe.close()

        listener = create_listener("127.0.0.1", free_port)
        try:
            assert isinstance(listener, SelectorsListener)
        finally:
            listener.close()

    def test_can_accept_real_connection(self) -> None:
        probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        probe.bind(("127.0.0.1", 0))
        free_port: int = probe.getsockname()[1]
        probe.close()

        listener = create_listener("127.0.0.1", free_port)
        # Make listener blocking for this test so accept() works without selector.
        listener._sock.setblocking(True)
        client = _connect(free_port)
        try:
            conn = listener.accept()
            assert isinstance(conn, SelectorsConnection)
        finally:
            conn.close()
            client.close()
            listener.close()


# ---------------------------------------------------------------------------
# 13. Version
# ---------------------------------------------------------------------------


class TestVersion:
    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"

    def test_version_is_str(self) -> None:
        assert isinstance(__version__, str)
