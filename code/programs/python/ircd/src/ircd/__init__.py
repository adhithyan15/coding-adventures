"""ircd — IRC server executable.

This module is the wiring layer — the topmost layer of the IRC stack.  It
connects the pure IRC logic (``irc-server``) to the TCP transport layer
(``irc-net-selectors``) via two adapter objects:

    ``DriverHandler``
        Implements the ``Handler`` protocol expected by ``irc-net-selectors``.
        Translates raw byte chunks from the network into ``Message`` objects,
        feeds them to ``IRCServer``, and sends the resulting responses back
        over the network.

    ``main()`` / ``parse_args()`` / ``Config``
        Entry-point glue: parse command-line arguments, construct the server
        and event loop, install signal handlers for graceful shutdown, and
        call ``loop.run()``.

Wiring diagram::

    TCP socket
       ↓ raw bytes
    SelectorsEventLoop.on_data()     ← irc-net-selectors
       ↓ conn_id, raw bytes
    DriverHandler.on_data()          ← THIS MODULE
       ↓ feeds bytes into per-connection Framer
    Framer.frames()                  ← irc-framing
       ↓ b"NICK alice"
    irc_proto.parse()                ← irc-proto
       ↓ Message(command='NICK', ...)
    IRCServer.on_message()           ← irc-server
       ↓ list[(ConnId, Message)]
    irc_proto.serialize()            ← irc-proto
       ↓ b":irc.local 001 alice :Welcome\r\n"
    EventLoop.send_to()              ← irc-net-selectors
       ↓ bytes on the wire

None of the four dependency packages know about each other — only this module
imports all four and wires them together.  This is the Dependency Inversion
Principle at work: higher-level modules (``irc-server``) know nothing about
lower-level infrastructure (sockets), because both talk through a common
message interface.
"""

from __future__ import annotations

import argparse
import logging
import os
import sys
import threading
from dataclasses import dataclass, field

# Use a module-level logger so operators can configure log level and output
# destination independently of stdout.  In production, redirect to a file or
# structured log aggregator; in tests the default NullHandler keeps tests quiet.
log = logging.getLogger(__name__)

from irc_framing import Framer
from irc_net_selectors import (
    ConnId as NetConnId,
)
from irc_net_selectors import (
    EventLoop,
    SelectorsEventLoop,
    create_listener,
)
from irc_proto import Message, ParseError, parse, serialize
from irc_server import ConnId as ServerConnId
from irc_server import IRCServer

__version__ = "0.1.0"


def _to_server_conn_id(conn_id: NetConnId) -> ServerConnId:
    """Convert a transport-layer connection id into an IRC-server connection id."""
    return ServerConnId(int(conn_id))


def _to_net_conn_id(conn_id: ServerConnId) -> NetConnId:
    """Convert an IRC-server connection id back into a transport-layer id."""
    return NetConnId(int(conn_id))


# ---------------------------------------------------------------------------
# DriverHandler — bridges irc-net-selectors and irc-server
# ---------------------------------------------------------------------------


class DriverHandler:
    """Adapts ``IRCServer`` to the ``Handler`` interface expected by ``irc-net-selectors``.

    The ``irc-net-selectors`` event loop calls three lifecycle callbacks on a
    ``Handler``:

    * ``on_connect(conn_id, host)`` — a new TCP connection arrived.
    * ``on_data(conn_id, data)``    — raw bytes from an established connection.
    * ``on_disconnect(conn_id)``    — the TCP connection has closed.

    ``DriverHandler`` translates these raw-bytes events into structured
    ``Message`` objects that ``IRCServer`` can process, and sends the resulting
    ``(ConnId, Message)`` responses back over the wire via ``loop.send_to()``.

    Per-connection framing
    ----------------------
    IRC uses CRLF-terminated text lines.  TCP, however, delivers an arbitrary
    byte stream — a single ``recv()`` call may return half a message, one
    complete message, or five messages concatenated together.  To reassemble
    byte chunks into complete lines, each connection gets its own ``Framer``
    instance (from ``irc-framing``).  The ``Framer`` is stored in a dict keyed
    by ``ConnId``, created in ``on_connect`` and removed in ``on_disconnect``.

    Concurrency
    -----------
    The ``irc-net-selectors`` event loop already holds its ``_handler_lock`` before
    calling any ``Handler`` method.  This means all three callbacks here run
    serially — we never have two threads in ``on_data`` simultaneously.
    ``IRCServer`` is therefore safe without an additional lock.

    We do, however, need to guard ``_framers`` against races between the
    ``send_to`` path (which calls ``loop.send_to()`` — safe, it holds its own
    lock internally) and the connect/disconnect callbacks.  Because all three
    callbacks are already serialised by the handler lock, we can use a simple
    ``threading.Lock`` on the framers dict as a defensive measure in case
    a future caller removes the serialisation guarantee.
    """

    def __init__(self, server: IRCServer, loop: EventLoop) -> None:
        # The IRC state machine — pure, no I/O.
        self._server = server

        # The event loop — used only for ``loop.send_to(conn_id, bytes)``.
        # We never call ``loop.run()`` or ``loop.stop()`` from here; that
        # responsibility belongs to ``main()``.
        self._loop = loop

        # One Framer per live connection.  Framers accumulate partial IRC lines
        # across multiple ``on_data`` calls until a full CRLF-terminated line is
        # available.  Protected by ``_framers_lock`` for defensive thread safety.
        self._framers: dict[NetConnId, Framer] = {}
        self._framers_lock = threading.Lock()

    # ------------------------------------------------------------------
    # Handler protocol — called by the event loop
    # ------------------------------------------------------------------

    def on_connect(self, conn_id: NetConnId, host: str) -> None:
        """Record a new connection and notify the IRC state machine.

        We create a ``Framer`` for this connection so subsequent ``on_data``
        calls can assemble complete IRC lines.  We also tell ``IRCServer``
        about the new connection so it can create a ``Client`` record with
        the peer's host address (used in the ``nick!user@host`` mask).

        The ``IRCServer.on_connect`` return value is always an empty list
        (no server response is sent until the client speaks), but we still
        dispatch it through ``_send_responses`` for uniformity.
        """
        # Create a per-connection framer before registering with the server,
        # so that if on_message somehow fires before on_connect returns
        # (impossible in practice, but defensive) we have a framer ready.
        with self._framers_lock:
            self._framers[conn_id] = Framer()

        # Notify the server.  Returns [] (no initial responses).
        responses = self._server.on_connect(_to_server_conn_id(conn_id), host)
        self._send_responses(responses)

    def on_data(self, conn_id: NetConnId, data: bytes) -> None:
        """Feed raw bytes into the per-connection framer and dispatch messages.

        This is the hot path — called for every TCP ``recv()`` that returns
        data.  The sequence is:

        1. Feed raw bytes into the ``Framer``.
        2. Extract all complete lines (``Framer.frames()``).
        3. Decode each line from UTF-8 (IRC is nominally ASCII but many
           clients send UTF-8 freely; ``errors="replace"`` avoids crashes
           on malformed input).
        4. Parse each line with ``irc_proto.parse()``; skip unparseable lines
           (``ParseError``) without closing the connection — IRC servers
           traditionally silently ignore garbage commands.
        5. Pass the parsed ``Message`` to ``IRCServer.on_message()``.
        6. Send any resulting ``(ConnId, Message)`` responses.
        """
        with self._framers_lock:
            framer = self._framers.get(conn_id)

        if framer is None:
            # Defensive: data arrived for a connection we have no framer for.
            # This should be impossible (the event loop guarantees on_connect
            # fires before on_data) but we handle it gracefully.
            return

        # Absorb the raw bytes into the framer's internal buffer.
        framer.feed(data)

        # Drain all complete lines from the framer.
        for raw_line in framer.frames():
            # IRC is specified as ASCII but UTF-8 is universally accepted.
            # ``errors="replace"`` substitutes U+FFFD for undecodable bytes
            # rather than raising — we never want a single bad byte to crash
            # the connection.
            line = raw_line.decode("utf-8", errors="replace")

            try:
                msg = parse(line)
            except ParseError:
                # Malformed or empty line — skip silently.  IRC servers
                # traditionally ignore unparseable input rather than
                # disconnecting the client.
                continue

            responses = self._server.on_message(_to_server_conn_id(conn_id), msg)
            self._send_responses(responses)

    def on_disconnect(self, conn_id: NetConnId) -> None:
        """Clean up state for a closed connection.

        We notify ``IRCServer`` (which broadcasts a QUIT to all channels the
        client was in), dispatch those responses, and then discard the framer
        for this connection to free memory.  After this point, any queued
        ``send_to`` for this ``conn_id`` will be a silent no-op (handled by
        the event loop itself).
        """
        responses = self._server.on_disconnect(_to_server_conn_id(conn_id))
        self._send_responses(responses)

        with self._framers_lock:
            self._framers.pop(conn_id, None)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _send_responses(self, responses: list[tuple[ServerConnId, Message]]) -> None:
        """Serialize and deliver a list of ``(ConnId, Message)`` responses.

        ``IRCServer`` returns responses as a list of ``(ConnId, Message)``
        tuples.  We serialize each ``Message`` to bytes using ``irc_proto``
        and forward it to the event loop's ``send_to()`` method.

        This indirection (serialize here, not in irc-server) keeps ``irc-server``
        free of any dependency on ``irc-proto``'s serialization side — the server
        only needs to *construct* ``Message`` objects, not wire-encode them.
        """
        for target_conn_id, msg in responses:
            wire = serialize(msg)
            self._loop.send_to(_to_net_conn_id(target_conn_id), wire)


# ---------------------------------------------------------------------------
# Config — command-line configuration
# ---------------------------------------------------------------------------


@dataclass
class Config:
    """All runtime configuration for ``ircd``.

    Values are populated by ``parse_args()`` from ``sys.argv`` (or a supplied
    argv list).  Default values match the conventional IRC server setup:

    * ``host``          — bind address; ``"0.0.0.0"`` means all interfaces.
    * ``port``          — IRC standard port 6667 (no TLS).
    * ``server_name``   — hostname shown in the 001 welcome message and as
                          the prefix of all server-generated messages.
    * ``motd``          — Message of the Day lines; at least one line required
                          for a well-formed MOTD (RFC 1459 §4.1).
    * ``oper_password`` — password for the OPER command; empty string disables.
    """

    host: str = "0.0.0.0"
    port: int = 6667
    server_name: str = "irc.local"
    motd: list[str] = field(default_factory=lambda: ["Welcome."])
    oper_password: str = ""


def parse_args(argv: list[str]) -> Config:
    """Parse *argv* (e.g. ``sys.argv[1:]``) into a :class:`Config`.

    Uses ``argparse`` for standard ``--flag value`` syntax.  Any invalid
    combination raises ``SystemExit`` (the ``argparse`` default).

    Examples::

        >>> parse_args([])
        Config(host='0.0.0.0', port=6667, server_name='irc.local', ...)

        >>> parse_args(['--port', '6668', '--server-name', 'irc.example.com'])
        Config(host='0.0.0.0', port=6668, server_name='irc.example.com', ...)
    """
    parser = argparse.ArgumentParser(
        prog="ircd",
        description=(
            "IRC server — wires irc-proto, irc-framing, irc-server, and irc-net-selectors."
        ),
    )
    parser.add_argument(
        "--host",
        default="0.0.0.0",
        help="IP address to bind to (default: 0.0.0.0 — all interfaces).",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=6667,
        help="TCP port to listen on (default: 6667).",
    )
    parser.add_argument(
        "--server-name",
        default="irc.local",
        dest="server_name",
        help="Server hostname advertised to clients (default: irc.local).",
    )
    parser.add_argument(
        "--motd",
        nargs="*",
        default=["Welcome."],
        help="Message of the Day lines (default: 'Welcome.').",
    )
    parser.add_argument(
        "--oper-password",
        default="",
        dest="oper_password",
        help=(
            "Password for the OPER command (default: empty = disabled).  "
            "WARNING: CLI arguments are visible in the process list (ps aux, "
            "/proc/<pid>/cmdline).  Prefer the IRCD_OPER_PASSWORD environment "
            "variable for production deployments."
        ),
    )

    ns = parser.parse_args(argv)

    # Validate port range: TCP ports are 1–65535.  Port 0 is valid in tests
    # (OS assigns a free ephemeral port), but we allow it here so integration
    # tests can bind without specifying a port.
    if not (0 <= ns.port <= 65535):
        parser.error(f"--port must be between 0 and 65535, got {ns.port}")

    # SECURITY: Prefer environment variable for the OPER password.
    # If the env var is set it takes priority over the CLI argument, so the
    # secret never appears in /proc/<pid>/cmdline or ``ps aux`` output.
    # The CLI flag is kept for convenience in development but carries a warning
    # in its help text above.
    #
    # Use explicit ``is not None`` (not ``or``) so that setting
    # IRCD_OPER_PASSWORD="" intentionally disables oper and does NOT fall
    # back to any CLI-supplied value.  If we used ``or``, an empty env var
    # would be falsy and the CLI password would reactivate unexpectedly.
    _env_oper = os.environ.get("IRCD_OPER_PASSWORD")
    oper_password = _env_oper if _env_oper is not None else ns.oper_password

    return Config(
        host=ns.host,
        port=ns.port,
        server_name=ns.server_name,
        motd=ns.motd if ns.motd else ["Welcome."],
        oper_password=oper_password,
    )


# ---------------------------------------------------------------------------
# main — entry point
# ---------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> None:
    """Parse arguments, wire up all components, and run the IRC server.

    This function:

    1. Parses ``sys.argv[1:]`` (or the supplied *argv* list) into a
       :class:`Config`.
    2. Creates a TCP listener bound to the configured host and port.
    3. Creates an ``IRCServer`` with the configured name, MOTD, and oper
       password.
    4. Creates a ``DriverHandler`` that bridges the network layer and the
       IRC logic.
    5. Installs ``SIGINT``/``SIGTERM`` handlers for graceful shutdown.
    6. Calls ``loop.run()`` — this blocks until ``loop.stop()`` is called
       (by the signal handler).
    7. Closes the listener socket and returns.

    The program can be run two ways::

        # As a module:
        python -m ircd --port 6667

        # As a console script (after ``pip install``):
        ircd --port 6667
    """
    import signal

    # Configure basic logging if no handlers are attached yet.  This ensures
    # security-relevant warnings (e.g. binding to 0.0.0.0) are visible in a
    # plain ``python -m ircd`` invocation.  Library users who configure their
    # own logging before calling main() are unaffected — basicConfig is a no-op
    # when root handlers are already present.
    if not logging.root.handlers:
        logging.basicConfig(
            level=logging.INFO,
            format="%(levelname)s %(name)s: %(message)s",
        )

    config = parse_args(argv if argv is not None else sys.argv[1:])

    # Create the listening socket.  ``create_listener`` sets ``SO_REUSEADDR``
    # so the port is available immediately after the previous server exits.
    listener = create_listener(config.host, config.port)

    loop: SelectorsEventLoop = SelectorsEventLoop()

    # The IRC state machine — knows nothing about sockets or threads.
    server = IRCServer(
        server_name=config.server_name,
        motd=config.motd,
        oper_password=config.oper_password,
    )

    # The adapter that connects the network layer to the IRC logic.
    handler = DriverHandler(server, loop)

    # Graceful shutdown: SIGINT (Ctrl-C) and SIGTERM both call loop.stop(),
    # which closes the listener and lets loop.run() return cleanly.
    def _shutdown(signum: int, frame: object) -> None:  # noqa: ARG001
        loop.stop()

    signal.signal(signal.SIGINT, _shutdown)
    signal.signal(signal.SIGTERM, _shutdown)

    # SECURITY: warn when binding to all interfaces — this exposes the server
    # on every network the host is connected to, including the public internet
    # if the host has a public IP.  Operators must opt in to this explicitly.
    if config.host in ("0.0.0.0", "::"):
        log.warning(
            "ircd binding to %s — server is reachable on ALL network interfaces. "
            "Use --host 127.0.0.1 to restrict to loopback only.",
            config.host,
        )

    # Use the logging module rather than print() so operators can control the
    # log level, format, and destination without modifying this code.
    log.info("ircd listening on %s:%d", config.host, config.port)

    # Block here.  The event loop accepts connections and dispatches events to
    # ``handler`` until ``loop.stop()`` is called (from the signal handler or
    # from a test).
    loop.run(listener, handler)

    # The listener was already closed by loop.stop() → listener.close(), but
    # calling close() twice is safe (it is idempotent).
    listener.close()
