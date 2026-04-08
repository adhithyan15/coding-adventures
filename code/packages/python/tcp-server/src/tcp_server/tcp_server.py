"""
tcp_server.py — Single-threaded TCP server using I/O multiplexing.

This module implements a TCP server from first principles, exposing the
seven fundamental operating system syscalls that every TCP server in every
language ultimately relies on:

  1. socket()  — create a file descriptor (just an integer, e.g. fd=5)
  2. bind()    — attach the fd to an IP address + port
  3. listen()  — mark the fd as a passive listener; OS queues connections
  4. accept()  — dequeue one connection, returning a NEW fd for that client
  5. read()    — receive bytes from a client fd (returns 0 on disconnect)
  6. write()   — send bytes to a client fd
  7. close()   — release the fd and free OS resources

The server knows nothing about RESP or Redis commands. It is a pure I/O
layer: bytes in, handler called, bytes out. All protocol logic lives in
the handler (DT25).

Why use selectors instead of threads?
─────────────────────────────────────
A thread-per-connection server allocates an 8 MB stack per thread. With
10,000 simultaneous clients, that is 80 GB of virtual memory — before a
single byte of application data. Threads also spend most of their time
blocked waiting for network I/O, wasting CPU scheduling cycles.

The event loop alternative registers every file descriptor with the OS
(epoll on Linux, kqueue on macOS/BSD, IOCP on Windows). The OS puts the
process to sleep and wakes it only when a fd has data ready. A single
thread handles thousands of connections with zero idle CPU usage.

Python's `selectors` module abstracts over epoll/kqueue/IOCP and provides
a portable, Pythonic interface to OS-level I/O readiness notifications.

Interrupt chain (how a client message reaches the handler):

  Client sends b"PING\\r\\n"
       │
       ▼  (NIC hardware interrupt)
  OS kernel: "data arrived on client socket fd 7"
       │
       ▼  (kernel wakes our epoll/kqueue waiter)
  sel.select() returns [(key, EVENT_READ)] for fd 7
       │
       ▼
  conn.recv(4096) → raw bytes
       │
       ▼
  handler(raw_bytes) → response bytes
       │
       ▼
  conn.sendall(response)
"""

from __future__ import annotations

import selectors
import socket
import threading
from typing import Any, Callable

# ---------------------------------------------------------------------------
# Public type alias
# ---------------------------------------------------------------------------

# A Handler receives the raw bytes from a client and returns raw bytes to send
# back. The server has no opinion about the content — that is the handler's
# responsibility. The default handler echoes bytes back unchanged.
Handler = Callable[[bytes], bytes]


# ---------------------------------------------------------------------------
# TcpServer
# ---------------------------------------------------------------------------


class TcpServer:
    """
    Single-threaded TCP server using I/O multiplexing via ``selectors``.

    The server listens on a host/port, accepts client connections, reads raw
    bytes, passes them to a pluggable ``handler`` function, and writes back
    whatever the handler returns. Multiple clients are handled sequentially
    (one at a time per event loop iteration) — the server never blocks waiting
    for a single slow client because the selector wakes it only when a fd is
    actually ready.

    Protocol overview:

    .. code-block::

        ┌─────────────────────────────────────────────────────┐
        │ TcpServer event loop                                │
        │                                                     │
        │  sel.select()  ←──── OS: fd N is readable          │
        │       │                                             │
        │  if fd == server_socket:                            │
        │      accept() → new client fd                       │
        │      register client fd with selector               │
        │                                                     │
        │  if fd == client_socket:                            │
        │      recv(buffer_size) → raw bytes                  │
        │      if empty: client disconnected → close          │
        │      else: handler(raw_bytes) → response            │
        │            sendall(response)                        │
        └─────────────────────────────────────────────────────┘

    Usage::

        server = TcpServer(host="127.0.0.1", port=6380, handler=lambda d: d)
        server.serve_forever()   # blocks until stop() is called

    Context manager::

        with TcpServer(host="127.0.0.1", port=6380) as server:
            threading.Thread(target=server.serve, daemon=True).start()
            ...
            server.stop()
    """

    def __init__(
        self,
        host: str = "127.0.0.1",
        port: int = 6380,
        handler: Handler | None = None,
        backlog: int = 128,
        buffer_size: int = 4096,
    ) -> None:
        """
        Initialise the server (does not bind or listen yet — call start()).

        Parameters
        ----------
        host:
            IP address to bind to. ``"127.0.0.1"`` accepts connections only
            from the same machine; ``"0.0.0.0"`` accepts from all interfaces.
        port:
            TCP port number (1–65535). Ports below 1024 typically require root.
        handler:
            Called with the raw bytes received from a client. Must return the
            raw bytes to send back. Defaults to an echo handler (returns the
            input unchanged) when ``None``.
        backlog:
            Size of the OS-level connection queue. The kernel accepts incoming
            TCP handshakes and queues them here; ``accept()`` dequeues them one
            at a time. 128 is a sensible default for most workloads.
        buffer_size:
            Maximum bytes to read per ``recv()`` syscall. 4096 bytes matches a
            typical OS page size and is a good balance between memory and
            efficiency for most protocols.
        """
        self._host = host
        self._port = port

        # Default handler: echo. The lambda receives bytes and returns bytes.
        # This makes the server immediately useful for basic connectivity tests
        # without any configuration.
        self._handler: Handler = handler if handler is not None else (lambda data: data)

        self._backlog = backlog
        self._buffer_size = buffer_size

        # The listening socket, created in start(). None until start() runs.
        self._server_socket: socket.socket | None = None

        # DefaultSelector wraps the best available mechanism for the current
        # platform: epoll on Linux, kqueue on macOS/BSD, select on Windows.
        # We create it here so it persists across start/serve cycles.
        self._sel: selectors.BaseSelector = selectors.DefaultSelector()

        # _running tracks whether start() has been called and the socket is
        # open. It becomes False after _cleanup() runs.
        self._running: bool = False

        # _stop_event is set by stop() and checked each event loop iteration.
        # Using threading.Event ensures stop() is safe to call from any thread.
        self._stop_event: threading.Event = threading.Event()

    # -----------------------------------------------------------------------
    # Lifecycle
    # -----------------------------------------------------------------------

    def start(self) -> None:
        """
        Bind and listen. Does not block — call ``serve()`` to enter the loop.

        This method performs the first three of the seven TCP syscalls:

        .. code-block:: text

            socket()  → create AF_INET/SOCK_STREAM fd
            bind()    → attach fd to (host, port)
            listen()  → mark fd as passive; kernel queues connections

        After ``start()`` the OS is accepting TCP handshakes and queuing them;
        ``accept()`` (called inside ``serve()``) dequeues them.

        SO_REUSEADDR
        ~~~~~~~~~~~~
        Without this option, a port stays in TIME_WAIT for ~60 seconds after
        the server closes. Setting SO_REUSEADDR lets us rebind immediately —
        essential during development when you restart the server frequently.

        Non-blocking mode
        ~~~~~~~~~~~~~~~~~
        ``setblocking(False)`` is critical for the event loop. In blocking mode
        ``accept()`` would halt the process until a client connects; in non-
        blocking mode it returns immediately with EAGAIN if no client is ready,
        letting the event loop continue checking other fds.
        """
        # syscall 1: socket() — create a TCP socket file descriptor
        self._server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

        # SO_REUSEADDR: skip the TIME_WAIT delay so we can rebind immediately
        # after a restart. Without this, "address already in use" errors are
        # common during development.
        self._server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

        # syscall 2: bind() — attach the socket to our chosen address + port
        self._server_socket.bind((self._host, self._port))

        # syscall 3: listen() — mark as passive listener; OS queues up to
        # `backlog` connections before refusing new ones
        self._server_socket.listen(self._backlog)

        # Non-blocking mode: recv/accept return immediately rather than
        # blocking the thread. The selector tells us WHEN they have data,
        # so we never actually need to block.
        self._server_socket.setblocking(False)

        # Register the server socket with the selector. EVENT_READ fires
        # whenever a new client connection is waiting to be accepted.
        # We use the string "server" as the data tag so _serve() can
        # distinguish the server socket from client sockets.
        self._sel.register(self._server_socket, selectors.EVENT_READ, data="server")

        self._running = True

    def serve(self) -> None:
        """
        Enter the event loop. Blocks the calling thread until ``stop()`` is called.

        The loop executes the last four of the seven TCP syscalls:

        .. code-block:: text

            accept()  → dequeue a pending connection, get a new client fd
            read()    → recv bytes from a client fd
            write()   → sendall response bytes to a client fd
            close()   → unregister and close the client fd on disconnect

        Timeout-based polling
        ~~~~~~~~~~~~~~~~~~~~~
        ``sel.select(timeout=0.1)`` wakes up at least every 100 ms to check
        ``_stop_event``. Without a timeout the select would block indefinitely
        even after ``stop()`` is called, making the server impossible to shut
        down from another thread unless a client connects.

        Calling order
        ~~~~~~~~~~~~~
        ``serve()`` assumes ``start()`` has already been called. Use
        ``serve_forever()`` to do both in one call.
        """
        self._stop_event.clear()
        try:
            while not self._stop_event.is_set():
                # sel.select() blocks until at least one registered fd is
                # ready, OR until the timeout expires. Returns a list of
                # (key, events) pairs where key.fileobj is the ready socket
                # and key.data is our tag ("server" or the client addr dict).
                events = self._sel.select(timeout=0.1)
                for key, mask in events:
                    if key.data == "server":
                        # The server socket is readable: a new client is
                        # waiting. Call accept() to dequeue it.
                        self._accept(key.fileobj)  # type: ignore[arg-type]
                    else:
                        # A client socket is readable: bytes are waiting.
                        self._handle_client(key, mask)
        finally:
            self._cleanup()

    def serve_forever(self) -> None:
        """
        Convenience: ``start()`` then ``serve()``.

        This is the typical entry point when you want the server to
        bind and block in a single call::

            server = TcpServer(port=6380, handler=my_handler)
            server.serve_forever()   # blocks here
        """
        self.start()
        self.serve()

    def stop(self) -> None:
        """
        Signal the event loop to stop after the current ``select()`` returns.

        Thread-safe: may be called from any thread, a signal handler, or even
        the handler function itself. The event loop checks ``_stop_event``
        after each ``select()`` timeout (at most 100 ms latency).
        """
        self._stop_event.set()

    # -----------------------------------------------------------------------
    # Internal helpers
    # -----------------------------------------------------------------------

    def _accept(self, server_sock: socket.socket) -> None:
        """
        Accept a pending connection from the OS queue.

        syscall 4: accept() — dequeue one connection from the listen queue.
        Returns a NEW file descriptor representing this specific client.
        The server socket continues listening for the next client.

        We immediately set the client socket to non-blocking and register it
        with the selector for EVENT_READ. The data tag is a dict containing
        the client's address so we can log or report it later.

        .. code-block:: text

            Server socket fd=5 (listening) ──────────────────────────► keeps listening
            accept() ────► New client fd=7  (only for this connection)
        """
        # syscall 4: accept() — blocks until a client connects (but we are
        # in non-blocking mode, so it raises BlockingIOError if the queue is
        # empty; the selector guarantees it is NOT empty when we reach here)
        conn, addr = server_sock.accept()

        # Non-blocking mode for the client socket: recv() returns EAGAIN
        # instead of blocking when no data is available yet.
        conn.setblocking(False)

        # Register the client socket for readability events. The data dict
        # stores the client address for reference.
        self._sel.register(conn, selectors.EVENT_READ, data={"addr": addr, "buf": b""})

    def _handle_client(self, key: selectors.SelectorKey, mask: int) -> None:
        """
        Handle a readable event on a client socket.

        syscall 5: recv() — receive up to ``buffer_size`` bytes from the fd.

        What recv() returns:
        ~~~~~~~~~~~~~~~~~~~~
        - n > 0 bytes: actual data from the client
        - b""  (0 bytes): the client closed the connection (TCP FIN received)
        - BlockingIOError: no data yet (should not happen after select says ready)

        When we have data, we call the handler and send the response with
        sendall(). ``sendall()`` wraps write() in a loop until all bytes are
        sent, handling the partial-write case automatically.

        syscall 6: sendall() → write() in a loop until all bytes sent
        syscall 7: close()   → release client fd on disconnect
        """
        conn: socket.socket = key.fileobj  # type: ignore[assignment]

        if mask & selectors.EVENT_READ:
            # syscall 5: recv() — receive bytes from the client
            raw = conn.recv(self._buffer_size)

            if raw:
                # We have data: call the handler and send the response.
                # The handler is a pure function: bytes → bytes. It knows
                # nothing about sockets or the event loop.
                response = self._handler(raw)

                # syscall 6: sendall() — Python wraps write() in a retry loop
                # until every byte of `response` has been sent to the OS
                # kernel's send buffer. This handles the partial-write case
                # where write() might accept fewer bytes than requested.
                conn.sendall(response)
            else:
                # recv() returned b"": the client sent a TCP FIN (graceful
                # close). We must unregister the fd and close it to free
                # OS resources. Failing to close leaks file descriptors.
                self._sel.unregister(conn)

                # syscall 7: close() — release the file descriptor
                conn.close()

    def _cleanup(self) -> None:
        """
        Release all OS resources: unregister all fds, close sockets, close
        the selector.

        Called automatically when ``serve()`` exits (normally or via exception).
        After cleanup, the server may be restarted by calling ``start()`` again
        with a fresh selector.
        """
        self._running = False

        if self._server_socket is not None:
            try:
                self._sel.unregister(self._server_socket)
            except Exception:
                pass
            self._server_socket.close()
            self._server_socket = None

        # Close all remaining registered file descriptors (connected clients
        # that were open when stop() was called). Iterating over a copy
        # because unregister modifies the internal map.
        for key in list(self._sel.get_map().values()):  # type: ignore[union-attr]
            if key.data != "server":
                try:
                    self._sel.unregister(key.fileobj)
                    key.fileobj.close()  # type: ignore[union-attr]
                except Exception:
                    pass

        self._sel.close()

        # Reset the selector so the server can be restarted
        self._sel = selectors.DefaultSelector()

    # -----------------------------------------------------------------------
    # Properties
    # -----------------------------------------------------------------------

    @property
    def address(self) -> tuple[str, int]:
        """
        Return the ``(host, port)`` this server is bound to.

        Valid after ``start()`` is called. Raises ``RuntimeError`` if the
        server has not been started.

        Example::

            server = TcpServer(host="127.0.0.1", port=0)  # port=0: OS picks
            server.start()
            host, port = server.address   # e.g. ("127.0.0.1", 54321)
        """
        if self._server_socket is None:
            raise RuntimeError("Server has not been started. Call start() first.")
        return self._server_socket.getsockname()

    @property
    def is_running(self) -> bool:
        """
        True if the server socket is open and the event loop has not yet
        been cleaned up.
        """
        return self._running

    # -----------------------------------------------------------------------
    # Context manager
    # -----------------------------------------------------------------------

    def __enter__(self) -> "TcpServer":
        """
        Support ``with TcpServer(...) as server:`` usage.

        Does NOT call start() — the caller may want to start the server
        in a background thread before entering the event loop.
        """
        return self

    def __exit__(self, *args: Any) -> None:
        """
        Ensure the server is stopped and cleaned up when the ``with`` block
        exits, even if an exception was raised.
        """
        self.stop()
        # Give the event loop a moment to notice the stop signal before
        # the context exits, but do not force-close (stop() sets the event).

    def __repr__(self) -> str:
        status = "running" if self._running else "stopped"
        return f"TcpServer(host={self._host!r}, port={self._port!r}, status={status!r})"
