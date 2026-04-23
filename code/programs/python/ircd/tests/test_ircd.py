"""Integration and unit tests for ``ircd``.

Test strategy
=============
Two layers of testing are used:

**Integration tests** (``TestRegistration``, ``TestMOTD``, ``TestPrivmsg``,
``TestPingPong``) start a *real* ``ircd`` server in a background thread using
the production ``SelectorsEventLoop`` and connect to it via real TCP sockets.
This exercises the full stack: framing → parsing → IRC logic → serialization
→ network delivery.  Port 0 is used so the OS picks a free ephemeral port,
which avoids conflicts between parallel test runs.

**Unit tests** (``TestDriverHandlerUnit``, ``TestConfig``) exercise
``DriverHandler`` and ``parse_args`` in isolation, with mocked or minimal
collaborators.  These run without binding any network sockets.

Concurrency note
================
The server runs on a daemon thread.  ``loop.stop()`` is called in
``teardown_method()`` / the fixture finaliser to cleanly stop it.  We use a
short ``socket.settimeout()`` on test sockets so tests fail quickly rather
than hanging indefinitely if the server does not respond.
"""

from __future__ import annotations

import socket
import threading
import time
from typing import TYPE_CHECKING
from unittest.mock import MagicMock, patch

import pytest

from ircd import Config, DriverHandler, parse_args

if TYPE_CHECKING:
    from irc_net_selectors import EventLoop


# ---------------------------------------------------------------------------
# Helpers shared across tests
# ---------------------------------------------------------------------------

_TIMEOUT = 5.0  # seconds — generous but not infinite


def _start_server(motd: list[str] | None = None) -> tuple[int, EventLoop]:
    """Start an ircd server on a free port and return (port, loop).

    The caller is responsible for calling ``loop.stop()`` when done.
    The server binds to 127.0.0.1 (loopback only) so no real network traffic
    escapes the test machine.
    """
    from irc_net_selectors import SelectorsEventLoop, create_listener
    from irc_server import IRCServer

    # Port 0 tells the OS to assign a free ephemeral port.
    listener = create_listener("127.0.0.1", 0)

    # Retrieve the OS-assigned port so we know where to connect.
    port: int = listener._sock.getsockname()[1]

    loop: SelectorsEventLoop = SelectorsEventLoop()
    server = IRCServer(
        server_name="irc.test",
        motd=motd if motd is not None else ["Hello from ircd tests."],
    )
    handler = DriverHandler(server, loop)

    # Run the event loop in a daemon thread so tests don't block on it.
    t = threading.Thread(
        target=loop.run,
        args=(listener, handler),
        daemon=True,
        name="test-ircd-server",
    )
    t.start()

    # Give the thread a moment to enter the accept loop before we connect.
    # (In practice it's instant, but a small sleep avoids a spurious race on
    # very slow CI runners.)
    time.sleep(0.05)

    return port, loop


def _connect(port: int) -> socket.socket:
    """Create a connected TCP socket to the test server."""
    sock = socket.create_connection(("127.0.0.1", port))
    sock.settimeout(_TIMEOUT)
    return sock


def _recv_until(sock: socket.socket, marker: bytes, max_bytes: int = 8192) -> bytes:
    """Receive bytes until *marker* appears in the accumulated buffer.

    Returns the full buffer (which may contain bytes *beyond* the marker if
    the OS coalesced multiple server responses into a single ``recv`` chunk).

    Raises ``TimeoutError`` (via the socket timeout) if the marker never
    arrives.
    """
    buf = b""
    while marker not in buf:
        chunk = sock.recv(4096)
        if not chunk:
            break
        buf += chunk
        if len(buf) > max_bytes:
            break
    return buf


def register_client(
    sock: socket.socket,
    nick: str,
    user: str = "user",
    realname: str = "Test User",
) -> bytes:
    """Send NICK + USER and collect the server's welcome sequence.

    Reads until numeric 376 (``RPL_ENDOFMOTD``) is seen.  Returns the raw
    accumulated bytes, which the caller can assert against.
    """
    sock.sendall(f"NICK {nick}\r\n".encode())
    sock.sendall(f"USER {user} 0 * :{realname}\r\n".encode())
    return _recv_until(sock, b"376")


# ---------------------------------------------------------------------------
# Integration: registration
# ---------------------------------------------------------------------------


class TestRegistration:
    """A new client can register with NICK + USER and receives welcome numerics."""

    def setup_method(self) -> None:
        self.port, self.loop = _start_server()

    def teardown_method(self) -> None:
        self.loop.stop()

    def test_001_welcome_in_response(self) -> None:
        """Server sends numeric 001 (RPL_WELCOME) after registration."""
        sock = _connect(self.port)
        try:
            resp = register_client(sock, "alice")
            assert b"001" in resp, f"Expected 001 in welcome response, got: {resp!r}"
        finally:
            sock.close()

    def test_registration_contains_nick(self) -> None:
        """The welcome sequence references the client's chosen nick."""
        sock = _connect(self.port)
        try:
            resp = register_client(sock, "bob")
            assert b"bob" in resp
        finally:
            sock.close()


# ---------------------------------------------------------------------------
# Integration: MOTD
# ---------------------------------------------------------------------------


class TestMOTD:
    """Server sends 375/372/376 MOTD numerics during registration."""

    def setup_method(self) -> None:
        self.port, self.loop = _start_server(motd=["Line one.", "Line two."])

    def teardown_method(self) -> None:
        self.loop.stop()

    def test_motd_start(self) -> None:
        """Numeric 375 (RPL_MOTDSTART) is present in the welcome sequence."""
        sock = _connect(self.port)
        try:
            resp = register_client(sock, "carol")
            assert b"375" in resp, f"Expected 375 in: {resp!r}"
        finally:
            sock.close()

    def test_motd_line(self) -> None:
        """Numeric 372 (RPL_MOTD) carries the MOTD content."""
        sock = _connect(self.port)
        try:
            resp = register_client(sock, "dave")
            assert b"372" in resp, f"Expected 372 in: {resp!r}"
        finally:
            sock.close()

    def test_motd_end(self) -> None:
        """Numeric 376 (RPL_ENDOFMOTD) terminates the MOTD section."""
        sock = _connect(self.port)
        try:
            resp = register_client(sock, "eve")
            assert b"376" in resp, f"Expected 376 in: {resp!r}"
        finally:
            sock.close()

    def test_motd_content(self) -> None:
        """The MOTD lines we configured appear in the welcome sequence."""
        sock = _connect(self.port)
        try:
            resp = register_client(sock, "frank")
            assert b"Line one." in resp
            assert b"Line two." in resp
        finally:
            sock.close()


# ---------------------------------------------------------------------------
# Integration: PRIVMSG channel delivery
# ---------------------------------------------------------------------------


class TestPrivmsgChannel:
    """Two clients join a channel; one sends PRIVMSG, the other receives it."""

    def setup_method(self) -> None:
        self.port, self.loop = _start_server()

    def teardown_method(self) -> None:
        self.loop.stop()

    def test_channel_message_delivered(self) -> None:
        """PRIVMSG to a channel is forwarded to all other channel members."""
        sock1 = _connect(self.port)
        sock2 = _connect(self.port)
        try:
            register_client(sock1, "alice")
            register_client(sock2, "bob")

            # Both clients join #test.
            sock1.sendall(b"JOIN #test\r\n")
            sock2.sendall(b"JOIN #test\r\n")

            # Give the server a moment to process both JOINs.
            time.sleep(0.1)

            # Drain any JOIN responses from sock2 before sending the message.
            sock2.settimeout(0.2)
            try:
                while sock2.recv(4096):
                    pass
            except (TimeoutError, OSError):
                pass
            sock2.settimeout(_TIMEOUT)

            # alice sends a message to #test.
            sock1.sendall(b"PRIVMSG #test :hello channel\r\n")

            # bob should receive it.
            resp = _recv_until(sock2, b"hello channel")
            assert b"PRIVMSG" in resp
            assert b"hello channel" in resp
        finally:
            sock1.close()
            sock2.close()


# ---------------------------------------------------------------------------
# Integration: PRIVMSG nick delivery
# ---------------------------------------------------------------------------


class TestPrivmsgNick:
    """Direct PRIVMSG between two registered clients."""

    def setup_method(self) -> None:
        self.port, self.loop = _start_server()

    def teardown_method(self) -> None:
        self.loop.stop()

    def test_direct_message_delivered(self) -> None:
        """PRIVMSG to a nick is forwarded to that nick's connection."""
        sock1 = _connect(self.port)
        sock2 = _connect(self.port)
        try:
            register_client(sock1, "alice")
            register_client(sock2, "bob")

            # Drain any residual data from bob's welcome.
            sock2.settimeout(0.2)
            try:
                while sock2.recv(4096):
                    pass
            except (TimeoutError, OSError):
                pass
            sock2.settimeout(_TIMEOUT)

            # alice sends a direct message to bob.
            sock1.sendall(b"PRIVMSG bob :hi bob\r\n")

            # bob receives it.
            resp = _recv_until(sock2, b"hi bob")
            assert b"PRIVMSG" in resp
            assert b"hi bob" in resp
        finally:
            sock1.close()
            sock2.close()


# ---------------------------------------------------------------------------
# Integration: PING/PONG
# ---------------------------------------------------------------------------


class TestPingPong:
    """Server echoes PONG in response to PING."""

    def setup_method(self) -> None:
        self.port, self.loop = _start_server()

    def teardown_method(self) -> None:
        self.loop.stop()

    def test_pong_response(self) -> None:
        """After registration, sending PING :test yields PONG :test."""
        sock = _connect(self.port)
        try:
            register_client(sock, "pingpong")
            sock.sendall(b"PING :test\r\n")
            resp = _recv_until(sock, b"PONG")
            assert b"PONG" in resp
            assert b"test" in resp
        finally:
            sock.close()


# ---------------------------------------------------------------------------
# Unit: Config and parse_args
# ---------------------------------------------------------------------------


class TestConfig:
    """``parse_args`` converts argv lists into ``Config`` objects."""

    def test_defaults(self) -> None:
        """With no arguments, all fields have their documented defaults."""
        cfg = parse_args([])
        assert cfg.host == "0.0.0.0"
        assert cfg.port == 6667
        assert cfg.server_name == "irc.local"
        assert cfg.motd == ["Welcome."]
        assert cfg.oper_password == ""

    def test_custom_port(self) -> None:
        """--port sets the port field."""
        cfg = parse_args(["--port", "6668"])
        assert cfg.port == 6668

    def test_custom_host(self) -> None:
        """--host sets the bind address."""
        cfg = parse_args(["--host", "127.0.0.1"])
        assert cfg.host == "127.0.0.1"

    def test_custom_server_name(self) -> None:
        """--server-name sets the server_name field."""
        cfg = parse_args(["--server-name", "irc.example.com"])
        assert cfg.server_name == "irc.example.com"

    def test_custom_motd(self) -> None:
        """--motd sets the motd list."""
        cfg = parse_args(["--motd", "Hello", "World"])
        assert cfg.motd == ["Hello", "World"]

    def test_custom_oper_password(self) -> None:
        """--oper-password sets the oper_password field."""
        cfg = parse_args(["--oper-password", "secret"])
        assert cfg.oper_password == "secret"

    def test_invalid_port_too_high(self) -> None:
        """A port > 65535 raises SystemExit (argparse error)."""
        with pytest.raises(SystemExit):
            parse_args(["--port", "99999"])

    def test_invalid_port_negative(self) -> None:
        """A negative port raises SystemExit (argparse error)."""
        with pytest.raises(SystemExit):
            parse_args(["--port", "-1"])

    def test_invalid_port_non_numeric(self) -> None:
        """A non-numeric port raises SystemExit (argparse error)."""
        with pytest.raises(SystemExit):
            parse_args(["--port", "abc"])

    def test_config_dataclass_fields(self) -> None:
        """Config can be constructed directly as a dataclass."""
        cfg = Config(host="localhost", port=7000, server_name="test", motd=["hi"])
        assert cfg.host == "localhost"
        assert cfg.port == 7000
        assert cfg.server_name == "test"
        assert cfg.motd == ["hi"]


# ---------------------------------------------------------------------------
# Unit: DriverHandler
# ---------------------------------------------------------------------------


class TestDriverHandlerUnit:
    """Unit tests for ``DriverHandler`` with a mock EventLoop.

    These tests do not start any TCP server.  They call ``DriverHandler``
    methods directly with raw bytes and assert that the expected bytes are
    passed to ``loop.send_to``.
    """

    def _make_handler(self) -> tuple[DriverHandler, MagicMock]:
        """Create a DriverHandler with a real IRCServer and a mock EventLoop."""
        from irc_server import IRCServer

        mock_loop = MagicMock()
        server = IRCServer(
            server_name="irc.unit",
            motd=["unit test motd"],
        )
        handler = DriverHandler(server, mock_loop)
        return handler, mock_loop

    def test_on_connect_creates_framer(self) -> None:
        """on_connect stores a Framer for the given conn_id."""
        from irc_net_selectors import ConnId

        handler, _ = self._make_handler()
        conn_id = ConnId(1)
        handler.on_connect(conn_id, "127.0.0.1")
        assert conn_id in handler._framers

    def test_on_disconnect_removes_framer(self) -> None:
        """on_disconnect removes the Framer for the given conn_id."""
        from irc_net_selectors import ConnId

        handler, _ = self._make_handler()
        conn_id = ConnId(2)
        handler.on_connect(conn_id, "127.0.0.1")
        handler.on_disconnect(conn_id)
        assert conn_id not in handler._framers

    def test_nick_user_triggers_send_to(self) -> None:
        """Sending NICK + USER causes send_to to be called with 001 wire bytes."""
        from irc_net_selectors import ConnId

        handler, mock_loop = self._make_handler()
        conn_id = ConnId(3)
        handler.on_connect(conn_id, "127.0.0.1")

        # Feed NICK and USER as a single raw byte chunk (simulating TCP coalescing).
        raw = b"NICK testuser\r\nUSER testuser 0 * :Test User\r\n"
        handler.on_data(conn_id, raw)

        # send_to should have been called at least once.
        assert mock_loop.send_to.called

        # Collect all the data that was sent.
        all_data = b"".join(
            c.args[1] for c in mock_loop.send_to.call_args_list
        )
        # The welcome sequence must include numeric 001.
        assert b"001" in all_data

    def test_partial_line_buffered(self) -> None:
        """A partial line is buffered and only dispatched once complete."""
        from irc_net_selectors import ConnId

        handler, mock_loop = self._make_handler()
        conn_id = ConnId(4)
        handler.on_connect(conn_id, "127.0.0.1")
        initial_call_count = mock_loop.send_to.call_count

        # Feed half a NICK command — no CRLF yet.
        handler.on_data(conn_id, b"NICK partial")
        # No additional send_to calls should have occurred.
        assert mock_loop.send_to.call_count == initial_call_count

        # Now complete the line.
        handler.on_data(conn_id, b"\r\n")
        # Still no response (NICK alone does not trigger a reply — we need USER
        # too for the welcome sequence, and an unregistered NICK gets no ACK).
        # But if the server does respond, it must have processed the full line.
        # The key assertion: the framer flushed the complete line.
        # We check the framer's buffer is now empty.
        assert handler._framers[conn_id].buffer_size == 0

    def test_garbage_line_skipped(self) -> None:
        """A completely unparseable line does not crash the server."""
        from irc_net_selectors import ConnId

        handler, mock_loop = self._make_handler()
        conn_id = ConnId(5)
        handler.on_connect(conn_id, "127.0.0.1")

        # Register first so we can send meaningful follow-ups.
        handler.on_data(conn_id, b"NICK g\r\nUSER g 0 * :G\r\n")
        mock_loop.send_to.reset_mock()

        # Feed a line that is entirely whitespace — ParseError territory.
        handler.on_data(conn_id, b"   \r\n")

        # The server should still be alive and able to respond to a PING.
        handler.on_data(conn_id, b"PING :alive\r\n")
        all_data = b"".join(c.args[1] for c in mock_loop.send_to.call_args_list)
        assert b"PONG" in all_data

    def test_on_data_unknown_conn_id_is_noop(self) -> None:
        """on_data for a conn_id with no framer silently does nothing."""
        from irc_net_selectors import ConnId

        handler, mock_loop = self._make_handler()
        # Never called on_connect — no framer exists.
        handler.on_data(ConnId(999), b"NICK x\r\n")
        # No send_to should have been called.
        assert not mock_loop.send_to.called

    def test_privmsg_dispatched_to_correct_connection(self) -> None:
        """A PRIVMSG to a nick sends the message to the correct conn_id."""
        from irc_net_selectors import ConnId

        handler, mock_loop = self._make_handler()
        conn1 = ConnId(10)
        conn2 = ConnId(11)

        handler.on_connect(conn1, "127.0.0.1")
        handler.on_connect(conn2, "127.0.0.2")

        # Register both clients.
        handler.on_data(conn1, b"NICK sender\r\nUSER sender 0 * :Sender\r\n")
        handler.on_data(conn2, b"NICK recvr\r\nUSER recvr 0 * :Receiver\r\n")

        mock_loop.send_to.reset_mock()

        # sender sends a direct message to recvr.
        handler.on_data(conn1, b"PRIVMSG recvr :direct message\r\n")

        # At least one send_to call must have targeted conn2.
        target_ids = [c.args[0] for c in mock_loop.send_to.call_args_list]
        assert conn2 in target_ids, (
            f"Expected conn2={conn2} in send_to targets, got {target_ids}"
        )

    def test_motd_numeric_376_in_welcome(self) -> None:
        """The welcome sequence includes 376 (RPL_ENDOFMOTD)."""
        from irc_net_selectors import ConnId

        handler, mock_loop = self._make_handler()
        conn_id = ConnId(20)
        handler.on_connect(conn_id, "127.0.0.1")
        handler.on_data(conn_id, b"NICK m\r\nUSER m 0 * :M\r\n")

        all_data = b"".join(c.args[1] for c in mock_loop.send_to.call_args_list)
        assert b"376" in all_data


# ---------------------------------------------------------------------------
# Unit: main() with patched loop
# ---------------------------------------------------------------------------


class TestMainEntryPoint:
    """Verify that main() wires everything up and calls loop.run()."""

    def test_main_calls_loop_run(self) -> None:
        """main() should call loop.run() exactly once before returning."""
        # We patch SelectorsEventLoop so the server never actually binds a socket
        # or blocks waiting for connections.  The mock loop's run() returns
        # immediately.
        with (
            patch("ircd.SelectorsEventLoop") as MockLoop,
            patch("ircd.create_listener") as mock_listener,
            patch("ircd.IRCServer"),
        ):
            mock_loop_inst = MockLoop.return_value
            mock_loop_inst.run = MagicMock()
            mock_loop_inst.stop = MagicMock()
            mock_listener.return_value = MagicMock()

            from ircd import main

            main(["--port", "0", "--host", "127.0.0.1"])

            mock_loop_inst.run.assert_called_once()

    def test_main_uses_config_port(self) -> None:
        """main() passes the configured port to create_listener."""
        with (
            patch("ircd.SelectorsEventLoop") as MockLoop,
            patch("ircd.create_listener") as mock_listener,
            patch("ircd.IRCServer"),
        ):
            MockLoop.return_value.run = MagicMock()

            from ircd import main

            main(["--port", "7777", "--host", "127.0.0.1"])

            mock_listener.assert_called_once_with("127.0.0.1", 7777)
