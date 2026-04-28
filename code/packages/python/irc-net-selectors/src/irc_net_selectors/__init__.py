"""irc-net-selectors — Level 2 network implementation: single-threaded event loop.

Overview
========
This package is the **second layer of the IRC networking doll**.  It replaces
the one-thread-per-connection model of ``irc-net-stdlib`` with a
**single-threaded event loop** driven by OS-level I/O readiness notifications.

The key insight: most IRC connections are idle most of the time.  With 10,000
connected clients, 9,990 are doing nothing at any given moment.  Allocating a
full OS thread (8 MB stack on Linux) for each idle connection wastes gigabytes
of virtual memory.  The event loop alternative registers every socket with the
OS and says: *"wake me up when any of them have data."*  One thread handles all
10,000 connections with near-zero idle CPU usage.

This pattern — **the reactor** — is the foundation of:

* **nginx**: one worker process per CPU, epoll-driven event loop
* **Node.js**: libuv's event loop, backed by epoll (Linux) / kqueue (macOS)
* **Redis**: single-threaded, ae event loop over epoll
* **Python asyncio**: event loop backed by ``selectors.DefaultSelector``
* **Rust tokio / mio**: epoll/kqueue abstraction at the core

``irc-net-selectors`` implements the same ``Connection``, ``Listener``,
``Handler``, and ``EventLoop`` interfaces as ``irc-net-stdlib``.  Swapping
the two packages requires changing **one import** in the ``ircd`` program.
The IRC logic (``irc-server``) is completely untouched.

The Reactor Pattern
===================
The reactor pattern has two parts:

1. **Demultiplexing**: ask the OS which of N file descriptors are ready for I/O
   without blocking on any single one.  The OS call for this is ``select()``,
   ``poll()``, ``epoll_wait()``, or ``kqueue()``.  Python's ``selectors`` module
   picks the best available one for the current platform.

2. **Dispatching**: for each ready fd, call the appropriate handler function.

The event loop is a tight while-loop that alternates between steps 1 and 2::

    while running:
        ready_fds = sel.select(timeout=0.1)  # step 1: demultiplex
        for fd, event in ready_fds:
            dispatch(fd, event)              # step 2: dispatch

Non-blocking I/O
================
Every socket in this implementation is set to **non-blocking** mode with
``sock.setblocking(False)``.  In non-blocking mode, ``recv()`` and ``send()``
return immediately:

* ``recv()`` returns ``b""`` (EOF), raises ``BlockingIOError`` (no data yet),
  or returns bytes.
* ``send()`` returns the number of bytes accepted by the kernel's send buffer,
  which may be less than the full payload (partial write).

Because we use ``sel.select()`` to know *when* to call ``recv()`` / ``send()``,
we never hit ``BlockingIOError`` for reads.  The selector tells us the socket
has data; we call ``recv()`` and it returns bytes immediately.

Writes are trickier: the kernel's send buffer may not have room for all our
bytes.  We maintain a **write buffer** per connection and only register for
``EVENT_WRITE`` when there are bytes waiting to be sent.  When the kernel's
buffer has room, ``EVENT_WRITE`` fires, we flush as many bytes as we can, and
deregister from ``EVENT_WRITE`` once the buffer drains.

Level-Triggered Notifications
==============================
Python's ``selectors.DefaultSelector`` uses **level-triggered** notifications
by default (matching the behaviour of ``select()`` and ``poll()``):

* A socket is reported as readable as long as there are bytes in the kernel's
  receive buffer.  You don't have to drain it all in one call — the selector
  will keep reporting it as ready until it's empty.

* A socket is reported as writable as long as there is room in the kernel's
  send buffer.  This is almost always true for an IRC server with modest
  traffic.

Edge-triggered mode (``epoll`` with ``EPOLLET``) is introduced at Level 3
(``irc-net-epoll``).  It requires draining all available data in one shot to
avoid missing subsequent events — a common source of bugs.

Why No Locks?
=============
``irc-net-stdlib`` needs two locks (``_handler_lock``, ``_conns_lock``) because
multiple worker threads race to read and modify shared state.

``irc-net-selectors`` is **single-threaded**: every socket operation, every
handler callback, every connection table update runs in the same thread as
``run()``.  There is no concurrent mutation of shared state.  No locks needed.
This is the first of several performance advantages of the reactor model.

(``stop()`` is the one exception: it sets a ``threading.Event`` that another
thread may have called.  That is thread-safe by construction.)

Interface Contract
==================
The ``Connection``, ``Listener``, ``Handler``, and ``EventLoop`` protocols are
**identical** to those in ``irc-net-stdlib``.  Any code written against those
protocols works transparently with either implementation.

The concrete class names differ intentionally — you can import both packages
in the same project for comparison without name collisions:

* ``StdlibEventLoop`` ↔ ``SelectorsEventLoop``
* ``StdlibListener``  ↔ ``SelectorsListener``
* ``StdlibConnection``  ← not publicly needed; the loop manages connections

The factory function ``create_listener(host, port)`` has the same signature
and behaviour in both packages.
"""

from __future__ import annotations

import contextlib
import selectors
import socket
import threading
from typing import NewType, Protocol

__version__ = "0.1.0"

# How long sel.select() sleeps before rechecking the stop flag.
# 100 ms gives ≤100 ms shutdown latency with negligible idle CPU cost.
_SELECTOR_TIMEOUT_SECONDS = 0.1

# ---------------------------------------------------------------------------
# Stable interface types — identical to irc-net-stdlib
# ---------------------------------------------------------------------------

# ``ConnId`` is a distinct integer type that uniquely identifies a connection.
# Using NewType gives mypy a way to distinguish ConnId from a plain int, at
# zero runtime cost (NewType is erased by the Python interpreter).
ConnId = NewType("ConnId", int)


class Connection(Protocol):
    """A single bidirectional byte-stream connection.

    This is the same protocol as in ``irc-net-stdlib``.  Code written against
    this interface works with any ``irc-net-*`` implementation.
    """

    @property
    def id(self) -> ConnId:
        """Unique integer identifier for this connection."""
        ...

    @property
    def peer_addr(self) -> tuple[str, int]:
        """The remote (host, port) pair."""
        ...

    def read(self) -> bytes:
        """Read the next available chunk of bytes.  Returns b"" on close."""
        ...

    def write(self, data: bytes) -> None:
        """Write *data* to the peer."""
        ...

    def close(self) -> None:
        """Close the connection."""
        ...


class Listener(Protocol):
    """A passive TCP server socket waiting for incoming connections."""

    def accept(self) -> Connection:
        """Block until a new connection arrives and return it."""
        ...

    def close(self) -> None:
        """Close the listening socket."""
        ...


class Handler(Protocol):
    """Callback interface that the event loop drives.

    Identical to the protocol in ``irc-net-stdlib``:

    * ``on_connect``   — new TCP connection arrived.
    * ``on_data``      — raw bytes received on an established connection.
    * ``on_disconnect``— TCP connection has closed.

    Because ``SelectorsEventLoop`` is single-threaded, callbacks are never
    called concurrently.  No locking is needed inside the handler.
    """

    def on_connect(self, conn_id: ConnId, host: str) -> None:
        """Called once when a new client connects."""
        ...

    def on_data(self, conn_id: ConnId, data: bytes) -> None:
        """Called with raw bytes from an established connection."""
        ...

    def on_disconnect(self, conn_id: ConnId) -> None:
        """Called once when a connection closes."""
        ...


class EventLoop(Protocol):
    """Drives connection lifecycle and I/O.

    Identical protocol to ``irc-net-stdlib``.  The concrete implementation
    ``SelectorsEventLoop`` uses OS selectors instead of threads.
    """

    def run(self, listener: Listener, handler: Handler) -> None:
        """Block, accepting connections and dispatching events to *handler*."""
        ...

    def stop(self) -> None:
        """Request shutdown.  Safe to call from any thread."""
        ...

    def send_to(self, conn_id: ConnId, data: bytes) -> None:
        """Write *data* to connection *conn_id*.  Silent no-op if not found."""
        ...


# ---------------------------------------------------------------------------
# ConnId allocation — same strategy as irc-net-stdlib
# ---------------------------------------------------------------------------

# Module-level counter.  A lock guards it so stop() / external callers that
# create connections outside the event loop get unique IDs even if they race.
_next_conn_id: ConnId = ConnId(0)
_conn_id_lock: threading.Lock = threading.Lock()


def _alloc_conn_id() -> ConnId:
    """Return the next unique ConnId, atomically.

    IDs start at 1.  We use a module-level counter so IDs are unique across
    multiple ``SelectorsEventLoop`` instances in the same process — useful in
    tests that spin up many servers.
    """
    global _next_conn_id
    with _conn_id_lock:
        _next_conn_id = ConnId(_next_conn_id + 1)
        return _next_conn_id


# ---------------------------------------------------------------------------
# SelectorsConnection — a non-blocking socket with a write buffer
# ---------------------------------------------------------------------------


class SelectorsConnection:
    """A non-blocking TCP connection managed by the selector event loop.

    Unlike ``StdlibConnection`` (which exposes blocking ``read()`` / ``write()``
    for use in a dedicated thread), ``SelectorsConnection`` exposes two paths:

    **Read path**  — ``read_available()``
        Called by the event loop when the selector reports ``EVENT_READ``.
        Drains all available bytes from the kernel receive buffer without
        blocking.  Returns the concatenated bytes, or ``b""`` if the peer
        closed the connection.

    **Write path** — ``enqueue(data)`` / ``flush()``
        * ``enqueue(data)``: append bytes to the connection's write buffer.
          Does NOT touch the socket.  Called by the event loop when
          ``send_to()`` is called.
        * ``flush()``: called by the event loop when ``EVENT_WRITE`` fires.
          Tries to push as many buffered bytes as the kernel's send buffer
          will accept in one ``send()`` call.  Returns True when the buffer
          is fully drained (so the event loop can stop watching for
          writability).

    Why drain in a loop for reads?
    ===============================
    When using level-triggered notifications, a single ``recv(4096)`` per
    event works: if more data remains in the receive buffer, the selector will
    report the fd as readable again on the next iteration.

    But draining in a loop until ``BlockingIOError`` is more efficient: it
    processes all available data in one event-loop iteration instead of N.
    For IRC messages (max 512 bytes), a single recv usually captures
    everything anyway.  The loop is a no-op overhead when a single recv gets
    it all.

    Why a write buffer?
    ===================
    When ``send_to()`` is called, the kernel's send buffer may be full (because
    the client is reading slowly — a slow consumer).  If we called ``sendall()``
    directly, the event loop thread would **block**, stalling all other
    connections.

    Instead, we enqueue bytes to a per-connection ``bytearray`` and register
    the socket for ``EVENT_WRITE``.  The kernel signals writability when its
    send buffer has room.  We flush then.  If the buffer isn't fully drained
    (still full), we keep the socket registered for ``EVENT_WRITE`` and retry
    next iteration.  Only when the buffer is empty do we deregister, avoiding
    a busy-loop.

    Thread safety
    =============
    ``SelectorsConnection`` is NOT thread-safe.  All its methods are designed
    to be called from the single event-loop thread.  The ``_write_buf`` is a
    plain ``bytearray`` with no locking.
    """

    def __init__(
        self,
        sock: socket.socket,
        addr: tuple[str, int],
        conn_id: ConnId,
    ) -> None:
        self._id: ConnId = conn_id
        self._addr: tuple[str, int] = addr
        self._sock: socket.socket = sock
        self._write_buf: bytearray = bytearray()

        # Non-blocking mode is critical.  In non-blocking mode:
        # - recv() raises BlockingIOError instead of sleeping when no data.
        # - send() raises BlockingIOError instead of sleeping when buffer full.
        # The selector tells us WHEN the socket is ready, so we never need to
        # block — we just react to readiness events.
        sock.setblocking(False)

    # ── Identity ──────────────────────────────────────────────────────────

    @property
    def id(self) -> ConnId:
        return self._id

    @property
    def peer_addr(self) -> tuple[str, int]:
        return self._addr

    # ── File descriptor ───────────────────────────────────────────────────

    def fileno(self) -> int:
        """The underlying OS file descriptor number.

        The selector uses this integer to identify the socket in kernel data
        structures (the epoll interest list, the kqueue changelist, etc.).
        Two different socket objects may have the same fileno() if one was
        closed and the OS reused the fd number — always keep the object alive
        while it is registered.
        """
        return self._sock.fileno()

    # ── Read path ─────────────────────────────────────────────────────────

    def read_available(self) -> bytes:
        """Drain all available bytes from the kernel receive buffer.

        Called by the event loop when ``EVENT_READ`` fires.  The selector
        guarantees there is at least one byte waiting, so the first ``recv``
        will not block.

        Returns:
            The concatenated bytes from all recv calls, or ``b""`` if the
            peer sent a TCP FIN (graceful close) or an OS error occurred
            (e.g. ``ECONNRESET``).

        Implementation note — why loop until BlockingIOError:
            A single ``recv(4096)`` would work with level-triggered
            notifications, but draining the whole buffer avoids N selector
            wake-ups for N consecutive messages.  This matters at high
            throughput.
        """
        chunks: list[bytes] = []
        while True:
            try:
                chunk = self._sock.recv(4096)
                if not chunk:
                    # TCP FIN received — peer closed its write end.
                    # Return b"" to signal EOF to the event loop.
                    return b""
                chunks.append(chunk)
            except BlockingIOError:
                # No more data in the kernel buffer right now.
                # The selector will fire again when more arrives.
                break
            except OSError:
                # ECONNRESET, ETIMEDOUT, etc. — treat as disconnect.
                return b""
        return b"".join(chunks)

    # ── Write path ────────────────────────────────────────────────────────

    def enqueue(self, data: bytes) -> None:
        """Append *data* to the connection's pending write buffer.

        Does NOT attempt to send anything.  The event loop will register this
        connection for ``EVENT_WRITE`` after calling ``enqueue()``, and will
        call ``flush()`` when the kernel's send buffer has room.

        This separation (enqueue vs flush) is the key to non-blocking writes:
        the event loop thread never sleeps waiting for a send buffer.
        """
        self._write_buf.extend(data)

    def flush(self) -> bool:
        """Flush as many buffered bytes as the kernel's send buffer will accept.

        Called by the event loop when ``EVENT_WRITE`` fires.

        Returns True if the write buffer is now empty (no more bytes to send),
        False if there are still bytes waiting (the kernel's buffer was full).

        Implementation note — why not sendall()?
            ``sendall()`` retries until all bytes are sent, blocking the thread
            if the kernel's buffer is full.  In a single-threaded event loop,
            that would stall all other connections.  We use ``send()`` instead,
            which performs a single partial write and returns the byte count.
            If the buffer isn't fully drained, ``flush()`` returns False and
            the event loop retries on the next ``EVENT_WRITE`` notification.
        """
        if not self._write_buf:
            return True

        try:
            # send() returns the number of bytes actually accepted by the kernel.
            # It may accept fewer bytes than requested if the send buffer is
            # nearly full — that's the partial-write case we handle here.
            sent = self._sock.send(self._write_buf)
            # Remove the bytes that were successfully sent.
            # del buf[:sent] is O(n) for bytearray but avoids a copy.
            del self._write_buf[:sent]
        except BlockingIOError:
            # The kernel's send buffer is completely full.
            # The selector will fire EVENT_WRITE again when there is room.
            pass
        except OSError:
            # Connection broken mid-write (ECONNRESET, EPIPE, etc.).
            # Clear the buffer — there is no point retrying on a broken socket.
            # The next recv() will return b"" and trigger the disconnect path.
            del self._write_buf[:]

        return len(self._write_buf) == 0

    @property
    def has_pending_writes(self) -> bool:
        """True if there are bytes waiting to be flushed to the kernel."""
        return bool(self._write_buf)

    # ── Lifecycle ─────────────────────────────────────────────────────────

    def close(self) -> None:
        """Close the underlying socket.

        Safe to call multiple times (subsequent calls are no-ops).  After
        ``close()``, the file descriptor is released back to the OS and may
        be reused for a new connection.
        """
        with contextlib.suppress(OSError):
            self._sock.close()


# ---------------------------------------------------------------------------
# SelectorsListener — a non-blocking server socket
# ---------------------------------------------------------------------------


class SelectorsListener:
    """A passive TCP server socket managed by the selector event loop.

    Like ``StdlibListener``, this wraps a bound, listening socket.  Unlike
    ``StdlibListener``, the socket is set to **non-blocking** so the event
    loop can call ``accept_connection()`` immediately when the selector reports
    ``EVENT_READ`` without any risk of blocking.

    The public ``accept()`` method (for the ``Listener`` protocol) is kept for
    interface compatibility but is an alias for ``accept_connection()`` with
    auto-allocated IDs.  The event loop uses ``accept_connection()`` directly
    to supply the next ``ConnId`` itself.
    """

    def __init__(self, sock: socket.socket) -> None:
        self._sock: socket.socket = sock
        # Non-blocking mode: accept() returns immediately rather than
        # sleeping until a client connects.  The selector tells us when
        # a connection is waiting.
        self._sock.setblocking(False)

    def fileno(self) -> int:
        """OS file descriptor — used to register with the selector."""
        return self._sock.fileno()

    def accept_connection(self, conn_id: ConnId) -> SelectorsConnection:
        """Accept a pending connection and wrap it in a ``SelectorsConnection``.

        The event loop calls this when ``EVENT_READ`` fires on the server socket.
        Because the selector guarantees a connection is waiting, ``accept()``
        will not block.

        :param conn_id: The ID to assign to the new connection.
        :returns: A ready-to-use ``SelectorsConnection``.
        """
        # accept() is syscall 4 of the TCP lifecycle:
        #   socket() → bind() → listen() → accept() → ...
        # It dequeues one connection from the kernel's accept queue and
        # returns a *new* fd for that specific client.  The listening socket
        # continues to receive new connections independently.
        client_sock, raw_addr = self._sock.accept()
        addr: tuple[str, int] = (str(raw_addr[0]), int(raw_addr[1]))
        return SelectorsConnection(client_sock, addr, conn_id)

    def accept(self) -> SelectorsConnection:
        """Accept a connection with an auto-allocated ID.

        Satisfies the ``Listener`` protocol.  The event loop calls
        ``accept_connection()`` directly; this method is for external callers
        (e.g. tests) that want a connection without managing IDs themselves.
        """
        return self.accept_connection(_alloc_conn_id())

    def close(self) -> None:
        """Close the server socket."""
        with contextlib.suppress(OSError):
            self._sock.close()


# ---------------------------------------------------------------------------
# SelectorsEventLoop — the reactor
# ---------------------------------------------------------------------------


class SelectorsEventLoop:
    """Single-threaded event loop using OS I/O readiness notifications.

    This is the heart of ``irc-net-selectors``.  It replaces the N-threads-
    for-N-connections model of ``StdlibEventLoop`` with a single while-loop
    that handles all connections in one thread.

    How it works — the reactor loop
    ================================
    ::

        while not stopping:
            ready = sel.select(timeout=0.1)
            for fd, event in ready:
                if fd is the server socket:
                    accept() → new connection
                    on_connect(conn_id, host)
                elif event is READABLE:
                    data = conn.read_available()
                    if data == b"": on_disconnect, remove conn
                    else:           on_data(conn_id, data)
                elif event is WRITABLE:
                    drained = conn.flush()
                    if drained: stop watching for writes

    Memory model
    ============
    Each connected client requires:

    * One ``SelectorsConnection`` object (~200 bytes)
    * One ``bytearray`` write buffer (empty most of the time, 0 bytes)
    * One entry in ``_conns`` dict (~56 bytes per entry)
    * One entry in the OS selector's interest list (8–16 bytes in the kernel)

    Total: ~300–400 bytes per idle connection.  Compare with ``irc-net-stdlib``
    where each connection needs an 8 MB OS thread stack.  At 10,000 connections:
    ``selectors`` uses ~4 MB total; ``stdlib`` uses ~80 GB virtual memory.

    Scalability ceiling
    ===================
    ``sel.select()`` is O(n) in the number of registered fds for Python's
    ``select``-backed selector (Windows).  ``epoll`` (Linux) and ``kqueue``
    (macOS) are O(1) for adding/removing fds and O(k) where k is the number
    of *ready* fds — much better for sparse activity across many connections.

    ``selectors.DefaultSelector`` picks the best backend automatically, so you
    get epoll on Linux and kqueue on macOS without any code change.

    Thread safety
    =============
    ``run()`` owns the selector and all connection state.  All handler
    callbacks are invoked from the ``run()`` thread.  No locks are needed for
    the event loop internals.

    The one cross-thread operation is ``stop()``: it sets ``_stop_event``, a
    ``threading.Event``, which is designed for cross-thread signalling.

    ``send_to()`` is designed to be called from within handler callbacks
    (same thread as ``run()``).  When called from a different thread (e.g.
    a test that doesn't go through handler callbacks), the ``_conns_lock``
    guards the dict lookup, and modifying the selector is safe as long as
    ``run()`` is between selector calls (which it is in those test scenarios).
    """

    def __init__(self) -> None:
        # Live connection table: conn_id → SelectorsConnection.
        # Guarded by _conns_lock for the dict lookup in send_to(), which may
        # be called from a different thread in test scenarios.
        self._conns: dict[ConnId, SelectorsConnection] = {}
        self._conns_lock: threading.Lock = threading.Lock()

        # The selector instance.  Created and owned by run(); exposed here so
        # that send_to() can register EVENT_WRITE for pending writes.  None
        # when the loop is not running.
        self._sel: selectors.BaseSelector | None = None

        # Stop signal.  Set by stop(); checked by run() each iteration.
        self._stop_event: threading.Event = threading.Event()

        # Next connection ID to allocate.  Starts at 1; 0 is reserved as
        # "no connection" in some calling conventions.
        self._next_id: int = 1

    # ── Public API ─────────────────────────────────────────────────────────

    def run(self, listener: SelectorsListener, handler: Handler) -> None:  # type: ignore[override]
        """Enter the reactor loop.

        Blocks the calling thread until ``stop()`` is called.  Accepts new
        connections, dispatches data to the handler, and flushes write buffers.

        :param listener: A bound, listening ``SelectorsListener``.
        :param handler: The ``Handler`` implementation to receive callbacks.

        The loop has three kinds of events:

        **New connection** (``EVENT_READ`` on the server socket):
            ``accept_connection()`` dequeues it from the kernel's accept queue.
            We assign a ``ConnId``, register the client socket for
            ``EVENT_READ``, store it in ``_conns``, and call
            ``handler.on_connect()``.

        **Incoming data** (``EVENT_READ`` on a client socket):
            ``conn.read_available()`` drains all bytes from the kernel buffer.
            If it returns ``b""``, the peer closed the connection — we
            unregister, call ``handler.on_disconnect()``, and remove the
            connection.  Otherwise we call ``handler.on_data()`` with the
            raw bytes.

        **Writable** (``EVENT_WRITE`` on a client socket):
            ``conn.flush()`` pushes buffered bytes into the kernel's send
            buffer.  When the buffer drains completely, we remove
            ``EVENT_WRITE`` from the registration to avoid a busy-loop.
        """
        # Reset the stop flag so the loop can run again after a previous stop.
        self._stop_event.clear()

        # Create a fresh selector for this run.
        sel = selectors.DefaultSelector()
        self._sel = sel

        # Register the server socket for readability (new connections).
        # We use the string sentinel "listener" as the data tag so we can
        # distinguish it from client socket registrations (which use ConnId).
        sel.register(listener.fileno(), selectors.EVENT_READ, data="listener")

        try:
            while not self._stop_event.is_set():
                # sel.select() blocks until at least one registered fd is
                # ready, or until the timeout expires.  The timeout lets
                # stop() take effect within 100 ms even if no I/O occurs.
                events = sel.select(timeout=_SELECTOR_TIMEOUT_SECONDS)

                for key, mask in events:
                    # ── New connection ────────────────────────────────────
                    if key.data == "listener":
                        self._handle_accept(listener, handler, sel)

                    # ── Existing connection ───────────────────────────────
                    else:
                        conn_id: ConnId = key.data  # type: ignore[assignment]
                        conn = self._conns.get(conn_id)
                        if conn is None:
                            # Should not happen, but be defensive: the conn
                            # may have been removed earlier in this same batch.
                            continue

                        if mask & selectors.EVENT_READ:
                            closed = self._handle_read(conn_id, conn, handler, sel)
                            if closed:
                                # Connection was closed; skip EVENT_WRITE check.
                                continue

                        if mask & selectors.EVENT_WRITE:
                            self._handle_write(conn_id, conn, sel)

        finally:
            # Clean up: unregister listener, close all connections, close sel.
            # This runs whether the loop exited normally (stop() called) or
            # via an unexpected exception.
            self._sel = None
            with contextlib.suppress(Exception):
                sel.unregister(listener.fileno())
            for conn in list(self._conns.values()):
                conn.close()
            self._conns.clear()
            sel.close()

    def stop(self) -> None:
        """Signal the event loop to stop after the current select() returns.

        Thread-safe: may be called from any thread, including signal handlers.
        The loop checks ``_stop_event`` after each ``select()`` call, so
        shutdown takes at most ``_SELECTOR_TIMEOUT_SECONDS`` (100 ms).
        """
        self._stop_event.set()

    def send_to(self, conn_id: ConnId, data: bytes) -> None:
        """Enqueue *data* to connection *conn_id*'s write buffer.

        When called from within a handler callback (the normal case), this is
        on the same thread as ``run()``, so no concurrency issues exist.  The
        connection is found, data is enqueued, and the selector is updated to
        watch for writability.

        When the kernel's send buffer has room, ``EVENT_WRITE`` will fire and
        ``flush()`` will push the bytes out.

        If *conn_id* does not exist (connection already closed, or invalid ID),
        this is a **silent no-op** — the same contract as ``irc-net-stdlib``.

        :param conn_id: The target connection.
        :param data: Raw bytes to send.  NOT parsed — this layer knows nothing
                     about IRC message format.
        """
        # Acquire the lock for the dict lookup.  This is a brief hold: we
        # release before any socket operations.
        conn: SelectorsConnection | None = None
        with self._conns_lock:
            conn = self._conns.get(conn_id)

        if conn is None:
            return

        # Enqueue the bytes to the connection's write buffer.
        conn.enqueue(data)

        # If the selector is running, register EVENT_WRITE so the loop knows
        # to flush this connection.  We use get_key() to check whether
        # EVENT_WRITE is already registered (avoid redundant modify calls).
        if self._sel is not None:
            try:
                key = self._sel.get_key(conn.fileno())
                if not (key.events & selectors.EVENT_WRITE):
                    # Add write-readiness to the existing read-readiness.
                    self._sel.modify(
                        conn.fileno(),
                        selectors.EVENT_READ | selectors.EVENT_WRITE,
                        data=conn_id,
                    )
            except KeyError:
                # The socket was already unregistered (connection closing).
                # Nothing to do — the data will be dropped with the socket.
                pass

    # ── Internal event handlers ────────────────────────────────────────────

    def _handle_accept(
        self,
        listener: SelectorsListener,
        handler: Handler,
        sel: selectors.BaseSelector,
    ) -> None:
        """Accept a new connection from the OS accept queue.

        This is called when ``EVENT_READ`` fires on the server socket.

        Steps:
        1. Allocate the next ``ConnId``.
        2. Call ``listener.accept_connection()`` to dequeue the connection.
        3. Store it in ``_conns``.
        4. Register the client socket for ``EVENT_READ``.
        5. Call ``handler.on_connect()``.

        Note on ordering: we store the connection in ``_conns`` *before*
        calling ``on_connect()`` so that if the handler calls ``send_to()``
        during ``on_connect()`` (e.g. for a server welcome banner), the
        connection is already findable.
        """
        conn_id = ConnId(self._next_id)
        self._next_id += 1

        conn = listener.accept_connection(conn_id)

        with self._conns_lock:
            self._conns[conn_id] = conn

        # Register for readability events on the new client socket.
        # We start with EVENT_READ only; EVENT_WRITE is added lazily when
        # send_to() is called.
        sel.register(conn.fileno(), selectors.EVENT_READ, data=conn_id)

        handler.on_connect(conn_id, conn.peer_addr[0])

    def _handle_read(
        self,
        conn_id: ConnId,
        conn: SelectorsConnection,
        handler: Handler,
        sel: selectors.BaseSelector,
    ) -> bool:
        """Handle incoming data on a client socket.

        Returns True if the connection was closed (caller should skip further
        processing for this fd in this event batch).

        Steps for data arrival:
        1. Drain all available bytes with ``read_available()``.
        2. If b"" returned: unregister, call ``on_disconnect``, clean up.
        3. Otherwise: call ``on_data`` with the raw bytes.

        The data is raw bytes — framing and parsing happen in the ``ircd``
        driver layer above this one, keeping ``irc-net-selectors`` protocol-
        agnostic.

        Disconnect ordering: we call ``on_disconnect`` *before* removing the
        connection from ``_conns``.  This mirrors ``irc-net-stdlib`` and lets
        the handler call ``send_to(conn_id)`` during the disconnect callback
        (e.g. to send a final error message).  The bytes will be enqueued but
        never flushed — that is acceptable; the socket is closed afterwards.
        """
        data = conn.read_available()

        if not data:
            # Peer closed the connection (TCP FIN) or an OS error occurred.
            # Unregister before calling the handler so we don't receive
            # spurious events for a dead socket.
            with contextlib.suppress(Exception):
                sel.unregister(conn.fileno())

            # Notify the handler.  The conn_id is still in _conns so that
            # send_to() calls from within on_disconnect don't silently fail.
            handler.on_disconnect(conn_id)

            # Remove from the live connection table.
            with self._conns_lock:
                self._conns.pop(conn_id, None)

            # Release the OS file descriptor.
            conn.close()
            return True  # connection is gone

        # Data arrived — deliver raw bytes to the handler.
        # The handler (DriverHandler in ircd) feeds them to a per-connection
        # Framer that reassembles IRC lines from the byte stream.
        handler.on_data(conn_id, data)
        return False  # connection is still open

    def _handle_write(
        self,
        conn_id: ConnId,
        conn: SelectorsConnection,
        sel: selectors.BaseSelector,
    ) -> None:
        """Flush buffered write data when the kernel's send buffer has room.

        Called when ``EVENT_WRITE`` fires on a client socket.

        If ``flush()`` drains the entire write buffer, we remove ``EVENT_WRITE``
        from the selector registration.  Leaving it registered when there is
        nothing to write would cause a **busy-loop**: the kernel's send buffer
        is almost always writable, so the selector would immediately return
        this fd as ready on every iteration, spinning at 100% CPU for no
        reason.

        If the buffer is not fully drained (kernel send buffer still full),
        we leave ``EVENT_WRITE`` registered.  The OS will notify us again
        when there is room.
        """
        drained = conn.flush()
        if drained:
            # Buffer empty: stop watching for writability.
            # Keep EVENT_READ so we still notice incoming data.
            with contextlib.suppress(Exception):
                sel.modify(conn.fileno(), selectors.EVENT_READ, data=conn_id)


# ---------------------------------------------------------------------------
# Factory function
# ---------------------------------------------------------------------------


def create_listener(host: str, port: int) -> SelectorsListener:
    """Create a TCP server socket bound to *host*:*port* and return a Listener.

    Identical signature and behaviour to ``irc_net_stdlib.create_listener()``.
    Swapping the import is the only change required in the ``ircd`` program.

    Steps:
    1. ``socket()`` — create an ``AF_INET`` TCP socket.
    2. ``SO_REUSEADDR`` — allow immediate port reuse after server restart.
    3. ``bind()`` — attach socket to (host, port).
    4. ``listen(128)`` — mark as passive; OS queues up to 128 pending connects.

    :param host: IP address to bind to.  ``"0.0.0.0"`` accepts from all
                 interfaces; ``"127.0.0.1"`` accepts loopback only (tests).
    :param port: TCP port number.  Use ``0`` to let the OS pick an ephemeral
                 port (safe in tests; read back the actual port via
                 ``listener._sock.getsockname()[1]``).
    :returns: A ``SelectorsListener`` ready to accept connections.
    """
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

    # Without SO_REUSEADDR, the OS holds the port in TIME_WAIT for ~60 s
    # after the server exits.  During development you restart the server
    # frequently; without this option you'd get "Address already in use".
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

    sock.bind((host, port))

    # Backlog of 128: the OS will queue up to 128 completed TCP handshakes
    # waiting for accept().  If the queue fills, new connection attempts
    # are refused (RST sent).  128 is the conventional Linux maximum.
    sock.listen(128)

    return SelectorsListener(sock)
