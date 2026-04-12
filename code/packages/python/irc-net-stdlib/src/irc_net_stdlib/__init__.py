"""irc-net-stdlib — Level 1 network implementation: stdlib sockets + threads.

Overview
========
This package provides the **concrete TCP networking layer** for the IRC stack.
It is the first of potentially several ``irc-net-*`` packages, each of which
implements the same stable protocol interfaces using a different I/O strategy:

- ``irc-net-stdlib`` (this package): one OS thread per connection, blocking
  ``recv``/``sendall``.  Simple, readable, works well up to a few hundred
  concurrent clients before thread-switching overhead dominates.
- ``irc-net-asyncio`` (future): a single-threaded async event loop using
  ``asyncio``, appropriate for thousands of concurrent clients.
- ``irc-net-selector`` (future): ``select()``/``epoll()``-based multiplexing,
  illustrating the classical UNIX I/O multiplexing pattern.

All three packages expose the same ``Connection``, ``Listener``, ``Handler``,
and ``EventLoop`` Protocol interfaces, so the IRC server can be swapped to a
new networking backend with zero changes to server logic.

Thread-per-connection model
===========================
Each accepted TCP connection gets its own OS thread.  The thread:

1. Calls ``handler.on_connect()`` to notify the server.
2. Loops calling ``conn.read()`` (blocking ``recv``) and forwards each chunk
   to ``handler.on_data()``.
3. When ``recv`` returns ``b""`` (peer closed), calls ``handler.on_disconnect()``
   and exits.

This is the textbook model taught in every OS/networking course.  Its chief
virtue is clarity: each connection's lifecycle is a simple sequential program,
easy to reason about with no callbacks or coroutines.

Two locks protect shared state
===============================

**``_handler_lock``** (a ``threading.Lock``):
    Serialises *all* calls to the ``Handler`` (``on_connect``, ``on_data``,
    ``on_disconnect``).  The Handler's internals (i.e. the IRC server state
    machine) are **not** thread-safe — nicks, channels, and pending replies
    are plain dicts with no locking.  By funnelling every callback through
    this single lock we guarantee that IRC logic executes in one thread at a
    time.  IRC traffic is mostly idle (clients send at human typing speed),
    so lock contention is negligible in practice.

    Think of it as a mutex around the "IRC brain".

    **``_conns_lock``** (a ``threading.Lock``):
        Protects the ``_conns: dict[ConnId, Connection]`` mapping.  Two
    threads that accept connections simultaneously could both try to insert
    into this dict, and a ``send_to()`` call racing against a worker thread
    removing a closed connection could read a half-updated map.  The lock
    prevents both races.

    Note that ``_conns_lock`` and ``_handler_lock`` are **independent**.  We
    never hold both at the same time, so there is no deadlock risk.

Writes bypass the handler lock
================================
``send_to()`` looks up the connection (under ``_conns_lock``), then calls
``conn.write()`` *without* holding ``_handler_lock``.  This is intentional:

- Writing bytes to a socket is independent of reading server state.
- Allowing two threads to write to *different* connections simultaneously
  is safe — the OS serialises writes to individual sockets internally.
- If we held ``_handler_lock`` during writes, a slow TCP write would stall
  all other connection threads that want to run IRC logic.

The only risk is two threads writing to the *same* connection simultaneously,
which could interleave bytes on the wire.  In practice this cannot happen
because the IRCServer calls ``send_to`` from within ``on_data`` callbacks,
and all ``on_data`` callbacks are serialised by ``_handler_lock``.  But even
if it did happen, the per-socket TCP send buffer in the kernel will simply
concatenate the bytes — and IRC framing handles message boundaries, so a
mixed write would at worst corrupt a single message, not crash the server.
"""

from __future__ import annotations

import contextlib
import socket
import threading
from typing import NewType, Protocol

__version__ = "0.1.0"

# ``accept()`` uses a short socket timeout so ``close()`` can stop a blocked
# accept loop predictably on Linux, macOS, and Windows.
_LISTENER_ACCEPT_TIMEOUT_SECONDS = 0.2

# ---------------------------------------------------------------------------
# Stable interface types — used by ALL irc-net-* packages
# ---------------------------------------------------------------------------

# ``ConnId`` is a distinct integer type used to identify connections.
# Using a NewType instead of a plain ``int`` prevents accidentally passing an
# arbitrary integer where a connection identity is expected.  The runtime cost
# is zero — NewType creates a thin wrapper that mypy understands but Python
# erases at runtime.
ConnId = NewType("ConnId", int)


class Connection(Protocol):
    """A single bidirectional byte-stream connection.

    Implementations wrap an OS socket, a TLS channel, a pipe — anything that
    can read and write bytes.  This Protocol defines the minimum interface that
    the event loop and tests need.

    **All methods are safe to call from any thread.**  ``read()`` blocks until
    data arrives or the connection closes.  ``write()`` blocks until the kernel
    accepts all bytes.  ``close()`` unblocks any concurrent ``read()`` call by
    invalidating the underlying file descriptor.
    """

    @property
    def id(self) -> ConnId:
        """A unique integer identifier for this connection."""
        ...

    @property
    def peer_addr(self) -> tuple[str, int]:
        """The remote (host, port) as a (str, int) tuple."""
        ...

    def read(self) -> bytes:
        """Read the next available chunk of bytes from the peer.

        Returns ``b""`` when the connection has been closed (either by the
        remote end or by a local ``close()`` call).
        """
        ...

    def write(self, data: bytes) -> None:
        """Write *data* to the peer, blocking until all bytes are sent.

        Silently ignores errors caused by the connection already being closed.
        """
        ...

    def close(self) -> None:
        """Close the underlying socket, unblocking any pending ``read()``."""
        ...


class Listener(Protocol):
    """A passive TCP server socket waiting for incoming connections.

    The event loop calls ``accept()`` in a tight loop.  Each call blocks
    until a new client connects, then returns a fresh ``Connection`` for
    that client.  ``close()`` shuts down the listener and causes any
    blocking ``accept()`` to raise, allowing the event loop to exit.
    """

    def accept(self) -> Connection:
        """Block until a new connection arrives and return it."""
        ...

    def close(self) -> None:
        """Close the listening socket, unblocking any pending ``accept()``."""
        ...


class Handler(Protocol):
    """Callback interface that the event loop drives.

    The event loop calls these methods as connection lifecycle events occur.
    All three methods are called **serialised** by the event loop's handler
    lock — the implementation need not be thread-safe.

    This interface deliberately passes **raw bytes** to ``on_data``, not parsed
    ``Message`` objects.  Framing (reassembling byte chunks into complete IRC
    lines) and parsing happen in the driver layer above this one, keeping
    ``irc-net-stdlib`` free of any IRC-specific knowledge.
    """

    def on_connect(self, conn_id: ConnId, host: str) -> None:
        """Called once when a new client connects.

        :param conn_id: Unique identifier for this connection.
        :param host: The peer's hostname or IP address string.
        """
        ...

    def on_data(self, conn_id: ConnId, data: bytes) -> None:
        """Called each time new bytes arrive from *conn_id*.

        The bytes may contain a partial IRC message, multiple complete
        messages, or anything in between — it is the handler's responsibility
        to buffer and frame them.

        :param conn_id: Which connection sent the data.
        :param data: The raw bytes chunk, never empty.
        """
        ...

    def on_disconnect(self, conn_id: ConnId) -> None:
        """Called once when *conn_id* has closed (either end initiated).

        After this call the conn_id is invalid; ``send_to()`` with it is a
        safe no-op.
        """
        ...


class EventLoop(Protocol):
    """Drives connection lifecycle and I/O.

    The event loop owns the thread model.  Callers create a ``Listener``,
    a ``Handler``, and an ``EventLoop``, then call ``run()`` to start serving.
    ``stop()`` can be called from any thread (including a signal handler) to
    request a clean shutdown.
    """

    def run(self, listener: Listener, handler: Handler) -> None:
        """Block, accepting connections and dispatching events to *handler*.

        Returns when ``stop()`` has been called and all in-flight connection
        threads have exited.
        """
        ...

    def stop(self) -> None:
        """Request the event loop to stop accepting new connections.

        In-flight connections are not forcibly closed; they run to completion.
        Safe to call from any thread.
        """
        ...

    def send_to(self, conn_id: ConnId, data: bytes) -> None:
        """Write *data* to connection *conn_id*.

        Safe to call from any thread, including from within handler callbacks.
        If *conn_id* no longer exists (connection closed between the handler
        callback deciding to write and actually calling ``send_to``), this is
        a silent no-op.
        """
        ...


# ---------------------------------------------------------------------------
# Concrete implementation — StdlibConnection
# ---------------------------------------------------------------------------

# Class-level counter for generating unique ConnId values.
# We use a module-level lock rather than relying on the GIL, because:
# 1. The GIL is an implementation detail of CPython, not the Python spec.
# 2. Making the locking explicit documents the intent clearly.
_next_conn_id: ConnId = ConnId(0)
_conn_id_lock: threading.Lock = threading.Lock()


def _alloc_conn_id() -> ConnId:
    """Atomically allocate the next unique ConnId.

    Each new connection gets an integer that never repeats within a process
    lifetime.  We start at 1 (0 is reserved as a sentinel "no connection"
    value in some contexts).
    """
    global _next_conn_id
    with _conn_id_lock:
        _next_conn_id = ConnId(_next_conn_id + 1)
        return _next_conn_id


class StdlibConnection:
    """A ``Connection`` backed by a plain ``socket.socket``.

    The socket is assumed to be a connected TCP socket handed to us by
    ``StdlibListener.accept()`` or ``socket.create_connection()``.  We take
    ownership of the socket — callers must not use the socket directly after
    constructing a ``StdlibConnection``.

    Thread safety
    -------------
    ``read()`` is meant to be called from a single dedicated worker thread.
    ``write()`` is called from the event loop's main thread (via ``send_to``).
    ``close()`` may be called from any thread.

    The OS kernel serialises concurrent writes to the same socket file
    descriptor, so two simultaneous ``write()`` calls will not corrupt the
    stream — they will interleave at byte boundaries, which is harmless for
    IRC given that the server never writes to a connection from two threads
    simultaneously (all writes go through ``send_to``, which is called from
    within serialised handler callbacks).
    """

    def __init__(self, sock: socket.socket, peer_addr: tuple[str, int]) -> None:
        # Assign a globally unique identity before doing anything else.
        # This must be the very first thing we do so that the connection is
        # identifiable even if the constructor later raises.
        self._id: ConnId = _alloc_conn_id()

        # We store the peer address at construction time rather than calling
        # sock.getpeername() later, because getpeername() raises after the
        # socket is closed.  Caching it here means we can always report the
        # peer's address in log messages or error output even post-close.
        self._peer_addr: tuple[str, int] = peer_addr

        # The underlying OS socket.  All reads and writes go through here.
        self._sock: socket.socket = sock

    # ── Protocol: identity and address ────────────────────────────────────

    @property
    def id(self) -> ConnId:
        """Unique integer id for this connection."""
        return self._id

    @property
    def peer_addr(self) -> tuple[str, int]:
        """The remote (host, port) pair, cached at construction time."""
        return self._peer_addr

    # ── Protocol: I/O ─────────────────────────────────────────────────────

    def read(self) -> bytes:
        """Receive the next chunk of bytes from the peer.

        Blocks until data arrives or the connection closes.

        Returns ``b""`` in two situations:
        - The peer closed the connection gracefully (TCP FIN received).  The
          kernel returns 0 bytes from recv to signal EOF.
        - An ``OSError`` occurred (e.g. ``ECONNRESET``, ``EBADF`` after
          ``close()`` was called from another thread).  We treat all socket
          errors as "connection gone" and return ``b""`` so the caller's loop
          exits cleanly.

        We receive up to 4096 bytes at a time.  This is a common choice:
        - Large enough to amortise syscall overhead (syscall ~1 µs on Linux).
        - Small enough to fit comfortably in L1/L2 cache.
        - IRC messages are at most 512 bytes, so one recv often captures
          several complete messages.
        """
        try:
            return self._sock.recv(4096)
        except OSError:
            # OSError covers all socket errors: ECONNRESET, ETIMEDOUT,
            # EBADF (bad file descriptor after close()), etc.
            # Returning b"" tells the caller's read loop to stop.
            return b""

    def write(self, data: bytes) -> None:
        """Send *data* to the peer, blocking until all bytes are accepted.

        ``sendall()`` is used rather than ``send()`` because ``send()`` may
        perform a short write (accepting fewer bytes than requested) when the
        kernel's send buffer is full.  ``sendall()`` retries internally until
        all bytes are consumed or an error occurs.

        Errors are silently swallowed because:
        - The connection may have already been closed by the time the server
          gets around to writing a reply (e.g. the client disconnected mid-
          command).
        - There is no meaningful recovery at this layer — the connection is
          broken regardless.  The worker thread's next ``read()`` call will
          return ``b""`` and trigger the disconnect path.
        """
        # contextlib.suppress is the idiomatic Python way to swallow a specific
        # exception class.  It is equivalent to try/except/pass but signals
        # the reader that the suppression is deliberate, not an oversight.
        # Silently ignore: broken pipe, connection reset, etc.
        # The read loop will detect the closure on the next recv().
        with contextlib.suppress(OSError):
            self._sock.sendall(data)

    def close(self) -> None:
        """Close the socket.

        Calling ``socket.close()`` invalidates the file descriptor.  Any
        thread blocked in ``recv()`` will get an ``OSError`` (typically
        ``EBADF``), which our ``read()`` catches and converts to ``b""``.
        This means closing the socket from the main thread is the correct way
        to signal a worker thread that it should stop.
        """
        # Already closed — harmless.
        with contextlib.suppress(OSError):
            self._sock.close()


# ---------------------------------------------------------------------------
# Concrete implementation — StdlibListener
# ---------------------------------------------------------------------------


class StdlibListener:
    """A ``Listener`` backed by a bound, listening ``socket.socket``.

    Construction creates and binds the listening socket.  Call ``accept()``
    inside the event loop to get incoming connections.  Call ``close()`` to
    stop accepting and wake up any thread blocked in ``accept()``.

    SO_REUSEADDR
    ------------
    We set ``SO_REUSEADDR`` before calling ``bind()``.  Without it, the OS
    keeps the port in ``TIME_WAIT`` state for up to 4 minutes after the server
    process exits.  This means restarting the server quickly (common during
    development) would fail with ``Address already in use``.  ``SO_REUSEADDR``
    tells the OS that it is safe to reuse the address immediately, because
    we are deliberately binding the same port again.

    Note: ``SO_REUSEADDR`` does NOT allow two processes to bind the same port
    simultaneously on Linux — it only affects the ``TIME_WAIT`` window.
    (``SO_REUSEPORT`` would allow simultaneous binding, but that is a
    different option with different semantics.)
    """

    def __init__(self, sock: socket.socket) -> None:
        # We take ownership of an already-bound, already-listening socket.
        # The factory function ``create_listener()`` does the bind/listen setup
        # so the constructor stays simple and testable in isolation.
        self._sock: socket.socket = sock
        self._closed: bool = False

        # Some platforms do not immediately unblock a thread in accept() when
        # another thread closes the listening socket.  A short timeout gives us
        # a bounded poll interval so close() reliably takes effect everywhere.
        self._sock.settimeout(_LISTENER_ACCEPT_TIMEOUT_SECONDS)

    def accept(self) -> StdlibConnection:
        """Block until a client connects and return a StdlibConnection for it.

        ``socket.accept()`` returns a (socket, address) pair where address is
        an ``(ip_string, port_number)`` tuple for IPv4/IPv6.

        This call blocks indefinitely.  The only way to unblock it is to call
        ``close()`` from another thread, which causes accept() to raise
        ``OSError``.  The event loop catches that error to detect shutdown.
        """
        while True:
            try:
                client_sock, addr = self._sock.accept()
                break
            except TimeoutError as exc:
                if self._closed:
                    raise OSError("listener closed") from exc
            except OSError:
                if self._closed:
                    raise
                raise

        # addr is (host, port) for AF_INET sockets.
        # We cast to the typed tuple here so the rest of the code sees the
        # correct types without needing runtime assertions.
        peer: tuple[str, int] = (str(addr[0]), int(addr[1]))

        return StdlibConnection(client_sock, peer)

    def close(self) -> None:
        """Close the listening socket.

        This unblocks any thread currently waiting in ``accept()``, causing it
        to raise ``OSError``.  The event loop's main loop catches that error
        as the stop signal.
        """
        self._closed = True
        with contextlib.suppress(OSError):
            self._sock.shutdown(socket.SHUT_RDWR)
        with contextlib.suppress(OSError):
            self._sock.close()


# ---------------------------------------------------------------------------
# Concrete implementation — StdlibEventLoop
# ---------------------------------------------------------------------------


class StdlibEventLoop:
    """Thread-per-connection event loop.

    Lifecycle
    ---------
    1. Caller creates a ``StdlibListener`` and a ``Handler``.
    2. Caller calls ``loop.run(listener, handler)`` — this blocks.
    3. Meanwhile, other threads may call ``loop.send_to()`` to push data to
       connected clients.
    4. When the caller wants to shut down, any thread calls ``loop.stop()``.
    5. ``stop()`` closes the listener, which causes ``accept()`` to raise,
       which exits the main loop, which causes ``run()`` to return.

    Worker thread lifecycle per connection
    ---------------------------------------
    For each accepted connection, a daemon thread is spawned.  Daemon threads
    do not prevent the Python process from exiting even if they are still
    running.  This means:
    - If the main thread finishes (after ``stop()`` returns from ``run()``),
      any remaining worker threads are killed automatically.
    - We do not need a global "all threads done" barrier for normal shutdown.

    The worker thread calls:
    1. ``handler.on_connect()`` under ``_handler_lock``
    2. Loop: ``conn.read()`` → ``handler.on_data()`` under ``_handler_lock``
    3. ``handler.on_disconnect()`` under ``_handler_lock``
    4. Removes conn from ``_conns`` (under ``_conns_lock``) and closes socket.

    Locking discipline
    ------------------
    We never hold both ``_handler_lock`` and ``_conns_lock`` at the same time.
    This guarantees no deadlock is possible regardless of thread interleaving.

    Specifically:
    - ``_handler_lock`` is acquired in worker threads (for handler callbacks).
    - ``_conns_lock`` is acquired in the main thread (accept loop insert) and
      in worker threads (post-disconnect remove), and in ``send_to``.
    - ``send_to`` acquires ``_conns_lock`` to look up the connection, then
      releases it before calling ``conn.write()``.  At no point does it
      hold ``_handler_lock``.
    """

    def __init__(self) -> None:
        # Whether the event loop is currently running.  Set to True by run(),
        # False by stop().  Worker threads check this in their outer loop
        # (though in practice the clean shutdown path is via listener.close()).
        self._running: bool = False

        # Map from ConnId → Connection for all currently-open connections.
        # Protected by _conns_lock.  Must hold _conns_lock to read or write.
        self._conns: dict[ConnId, Connection] = {}

        # Lock protecting _conns.  Use with: with self._conns_lock: ...
        # This is a non-reentrant mutex.  A thread that already holds it must
        # not try to acquire it again (that would deadlock).
        self._conns_lock: threading.Lock = threading.Lock()

        # Lock serialising all Handler callbacks.  The Handler (IRC server)
        # is not thread-safe — all its state is plain Python dicts.  This
        # lock ensures only one thread runs IRC logic at a time.
        self._handler_lock: threading.Lock = threading.Lock()

        # Reference to the listener, kept so stop() can call listener.close().
        # Set at the start of run() and cleared afterwards.
        self._listener: Listener | None = None

    # ── Public API ─────────────────────────────────────────────────────────

    def run(self, listener: Listener, handler: Handler) -> None:
        """Start the accept loop and block until ``stop()`` is called.

        This method is meant to run on the "main" networking thread (or the
        process's main thread).  It never returns under normal operation —
        callers must invoke ``stop()`` from another thread to end it.

        :param listener: Bound, listening socket wrapper.
        :param handler: Callback receiver for connection lifecycle events.
        """
        self._running = True
        self._listener = listener

        # Accept loop: each iteration blocks in listener.accept() until a new
        # client connects.  When stop() is called it closes the listener socket,
        # which causes accept() to raise OSError, which we catch to exit.
        while self._running:
            try:
                conn = listener.accept()
            except OSError:
                # listener.close() was called by stop() — this is our exit
                # signal.  Break out of the accept loop.
                break

            # Register the connection before spawning the thread, so that
            # send_to() can find it immediately (even before on_connect fires).
            with self._conns_lock:
                self._conns[conn.id] = conn

            # Spawn a daemon thread to service this connection.
            # daemon=True means this thread will be killed if the process
            # exits while it is still running (e.g. after run() returns).
            t = threading.Thread(
                target=self._worker,
                args=(conn, handler),
                daemon=True,
                name=f"irc-conn-{conn.id}",
            )
            t.start()

        # After the accept loop exits, clear the listener reference.
        self._listener = None

    def stop(self) -> None:
        """Signal the event loop to stop accepting new connections.

        Safe to call from any thread, including from signal handlers.
        Returns immediately (does not wait for in-flight connections to finish).

        Mechanism: we set _running to False so the accept loop knows to exit,
        then close the listener socket to unblock the accept() call that may
        be currently waiting for a new connection.
        """
        self._running = False
        if self._listener is not None:
            self._listener.close()

    def send_to(self, conn_id: ConnId, data: bytes) -> None:
        """Write *data* to connection *conn_id*.

        Looks up the connection under ``_conns_lock``, then writes **outside**
        the lock.  This is deliberate: we hold the lock for the shortest
        possible time (just the dict lookup), then release it so other threads
        can concurrently look up different connections.

        If *conn_id* is not found (connection closed, or never existed), this
        is a silent no-op.  Callers should not treat the absence as an error —
        it is a normal race condition where the client disconnected between the
        handler deciding to write and actually calling send_to.

        Note: this method does NOT hold ``_handler_lock``.  If it did, a slow
        or blocked ``sendall()`` would stall every other connection thread
        waiting to run IRC logic.
        """
        # Step 1: look up the connection while holding the lock.
        # We release the lock before writing so other threads aren't blocked
        # while we wait for the kernel's send buffer to accept our bytes.
        conn: Connection | None = None
        with self._conns_lock:
            conn = self._conns.get(conn_id)

        # Step 2: write outside the lock.
        # If conn was removed between steps 1 and 2 (the worker thread closed
        # it), conn.write() will silently swallow the resulting OSError.
        if conn is not None:
            conn.write(data)

    # ── Internal: worker thread ────────────────────────────────────────────

    def _worker(self, conn: Connection, handler: Handler) -> None:
        """Service a single connection from its own thread.

        This method is the entry point for every connection's worker thread.
        It runs the full lifecycle: connect → data loop → disconnect → cleanup.

        Error handling philosophy: we never let an exception from the Handler
        crash this thread.  If the handler raises, we log nothing (this is a
        library, not an application) and proceed to the disconnect path.
        In production, the handler should catch its own exceptions.
        """
        host = conn.peer_addr[0]

        # ── Phase 1: notify the handler that the connection opened ─────────
        # We hold _handler_lock for the duration of the callback so the
        # handler's internal state is consistent.  The lock is released
        # before we block in conn.read() below — we only hold it while
        # running IRC logic, not while waiting for network I/O.
        with self._handler_lock:
            handler.on_connect(conn.id, host)

        # ── Phase 2: data receive loop ─────────────────────────────────────
        # conn.read() blocks here, releasing the GIL so other Python threads
        # can run.  This is why the thread-per-connection model works at all
        # in CPython: the blocking syscall releases the GIL, so N connection
        # threads effectively run in parallel at the I/O level.
        try:
            while True:
                data = conn.read()

                if not data:
                    # b"" means the connection is closed (either the peer sent
                    # TCP FIN, or close() was called locally).  Exit the loop.
                    break

                # Dispatch the data to the handler.
                # We hold _handler_lock for the callback duration so the IRC
                # server sees a consistent view of its own state.  Release
                # before the next recv() so other threads can call their
                # callbacks while we are waiting for more bytes.
                with self._handler_lock:
                    handler.on_data(conn.id, data)

        finally:
            # ── Phase 3: cleanup ───────────────────────────────────────────
            # The finally block runs whether the loop exited cleanly (b"" from
            # read) or via an unhandled exception in handler.on_data.
            #
            # First, notify the handler.  We do this before removing the conn
            # from _conns so the handler can still call send_to() during the
            # disconnect callback if needed (e.g. to send a final error reply).
            with self._handler_lock:
                handler.on_disconnect(conn.id)

            # Remove from the connection map so send_to() stops finding it.
            # After this point any send_to(conn.id) is a no-op.
            with self._conns_lock:
                self._conns.pop(conn.id, None)

            # Close the socket.  If it was already closed (e.g. by stop()),
            # conn.close() silently ignores the error.
            conn.close()


# ---------------------------------------------------------------------------
# Factory function
# ---------------------------------------------------------------------------


def create_listener(host: str, port: int) -> StdlibListener:
    """Create a TCP server socket bound to *host*:*port* and return a Listener.

    This is the recommended way to create a ``StdlibListener``.  It:

    1. Creates an ``AF_INET`` TCP socket.
    2. Sets ``SO_REUSEADDR`` so the port can be reused immediately after the
       previous server process exits (avoids ``Address already in use`` during
       development).
    3. Binds to the given address.
    4. Calls ``listen()`` to put the socket in passive mode.  The backlog of
       128 means the OS will queue up to 128 pending connections before
       refusing new ones — more than sufficient for an IRC server.

    :param host: IP address to bind to, e.g. ``"0.0.0.0"`` for all interfaces
                 or ``"127.0.0.1"`` for loopback only (useful in tests).
    :param port:  TCP port number, e.g. ``6667`` (the standard IRC port).
    :returns:    A ready-to-use ``StdlibListener``.
    """
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

    # SO_REUSEADDR: allow rebinding this port immediately after the previous
    # process released it.  Without this, the OS holds the port in TIME_WAIT
    # for up to 4 minutes, making rapid server restarts fail with EADDRINUSE.
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

    sock.bind((host, port))

    # Listen with a backlog of 128.  The backlog limits how many connections
    # the OS will queue for us while our accept() loop is busy.  128 is the
    # conventional maximum on Linux (larger values are silently clamped).
    sock.listen(128)

    return StdlibListener(sock)
