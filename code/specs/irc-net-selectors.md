# irc-net-selectors — Level 2: Event-Driven I/O (Reactor Pattern)

## Overview

`irc-net-selectors` is the second layer of the Russian Nesting Doll. It replaces the
one-thread-per-connection model of `irc-net-stdlib` with a **single-threaded event loop**
that can handle thousands of connections using OS readiness notifications.

The key insight: most IRC connections are idle most of the time. A server with 10,000 connected
clients has 9,990 connections doing nothing at any given moment. Allocating a thread (and its
8 MB stack) for each idle connection wastes memory. Instead, one thread registers interest in
all file descriptors (fds) and asks the OS: "wake me up when any of them have data."

This is the **reactor pattern**. It is the foundation of every high-performance network server:
Node.js, nginx, Redis, Erlang's scheduler, and `tokio` are all reactors underneath.

`irc-net-selectors` implements the same `Connection`, `Listener`, `EventLoop`, and `Handler`
interfaces defined in `irc-net-stdlib`. The `ircd` program swaps implementations by changing
one import. The IRC logic is untouched.

---

## Layer Position

```
ircd (program)
    ↓
irc-net-selectors       ← THIS SPEC: event loop using OS selector
    ↓
OS selector (select/poll/epoll/kqueue — abstracted by language stdlib)
    ↓
Kernel TCP stack
```

This layer still uses the OS selector abstraction (Python `selectors`, Go `net.Listener` +
goroutine pool, Rust `mio`). The raw epoll/kqueue syscalls are exposed in the next layer
(`irc-net-epoll`).

---

## Concepts

### The Readiness Model

In the thread model, each thread blocks on `recv()`, waiting for data. In the readiness model,
a single thread asks the OS a different question:

> "Which of these file descriptors are ready to be read without blocking?"

The OS maintains readiness state for every open fd. When data arrives on a socket, the kernel
marks it as readable. The selector returns a list of ready fds. The event loop reads from each
one, then loops back to ask again.

```
event loop
    │
    ▼
selector.select(timeout=1.0)   ← blocks until at least one fd is ready
    │
    ├── fd 5 is readable → read from connection 5
    ├── fd 12 is readable → read from connection 12
    └── fd 3 is readable → this is the listener → accept() new connection
    │
    ▼
selector.select(timeout=1.0)   ← repeat
```

### Level-Triggered vs Edge-Triggered

The OS selector has two notification modes:

- **Level-triggered** (default in `select()`, `poll()`, `epoll` LT mode): the selector keeps
  returning a fd as ready as long as there is unread data. Safe: if you don't read all the data,
  the fd stays in the ready set and you'll process it next iteration.

- **Edge-triggered** (epoll `EPOLLET`): the selector notifies once when readiness *changes* (from
  not-ready to ready). You MUST drain all available data in one shot (loop until `EAGAIN`),
  otherwise you'll never be notified again. Faster but requires careful implementation.

The `selectors` module uses level-triggered by default. `irc-net-epoll` introduces edge-triggered
mode.

### Non-blocking Writes

In the single-threaded model, blocking on a write would freeze the entire event loop. Writes
should use non-blocking mode and, if the send buffer is full (`EAGAIN`), queue the data for
later. For IRC servers with modest traffic, writes rarely block — but the implementation must
handle it.

A simple write buffer per connection:

```python
pending_writes: dict[ConnId, bytearray] = {}

def send_to(conn_id: ConnId, data: bytes) -> None:
    pending_writes[conn_id].extend(data)
    sel.modify(fds[conn_id], selectors.EVENT_READ | selectors.EVENT_WRITE)

# in the event loop, when a fd is writable:
buf = pending_writes[conn_id]
sent = conn.sock.send(buf)
del buf[:sent]
if not buf:
    sel.modify(fds[conn_id], selectors.EVENT_READ)  # stop watching for writes
```

---

## Public API

The interfaces are identical to those in `irc-net-stdlib`. Only the concrete class names differ.

```python
from __future__ import annotations

import selectors
import socket
import threading
from collections.abc import Iterator

from irc_framing import Framer
from irc_proto import Message, parse, serialize


class SelectorsConnection:
    """A non-blocking TCP connection managed by the selector event loop."""

    def __init__(self, sock: socket.socket, addr: tuple[str, int], conn_id: ConnId) -> None:
        self._sock = sock
        self._addr = addr
        self._id = conn_id
        self._write_buf = bytearray()
        self._framer = Framer()
        sock.setblocking(False)

    @property
    def id(self) -> ConnId: ...

    @property
    def peer_addr(self) -> tuple[str, int]: ...

    def read_available(self) -> bytes:
        """Read all available data without blocking. Returns b"" on close."""
        chunks: list[bytes] = []
        while True:
            try:
                chunk = self._sock.recv(4096)
                if not chunk:
                    return b""
                chunks.append(chunk)
            except BlockingIOError:
                break
        return b"".join(chunks)

    def enqueue_write(self, data: bytes) -> None:
        """Queue bytes for writing. Actual send happens in the event loop."""
        self._write_buf.extend(data)

    def flush_writes(self) -> bool:
        """Attempt to send buffered data. Returns True if buffer drained."""
        if not self._write_buf:
            return True
        try:
            sent = self._sock.send(self._write_buf)
            del self._write_buf[:sent]
        except BlockingIOError:
            pass
        return len(self._write_buf) == 0

    def close(self) -> None: ...


class SelectorsListener:
    def __init__(self, host: str, port: int) -> None:
        self._sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._sock.setblocking(False)
        self._sock.bind((host, port))
        self._sock.listen(128)

    def fileno(self) -> int:
        return self._sock.fileno()

    def accept_connection(self, next_id: ConnId) -> SelectorsConnection:
        sock, addr = self._sock.accept()
        return SelectorsConnection(sock, addr, next_id)

    def close(self) -> None: ...


class SelectorsEventLoop:
    """Single-threaded event loop using Python's selectors module.

    Handles thousands of connections with one thread by using OS readiness
    notifications. No thread spawned per connection.
    """

    def __init__(self) -> None:
        self._running = False
        self._conns: dict[ConnId, SelectorsConnection] = {}
        self._next_id = 1

    def run(self, listener: SelectorsListener, handler: Handler) -> None: ...

    def stop(self) -> None:
        self._running = False
```

---

## Event Loop Internals

```python
def run(self, listener: SelectorsListener, handler: Handler) -> None:
    self._running = True
    sel = selectors.DefaultSelector()

    # Register the listener socket for read events (new connections)
    sel.register(listener.fileno(), selectors.EVENT_READ, data="listener")

    while self._running:
        events = sel.select(timeout=1.0)
        for key, mask in events:

            if key.data == "listener":
                # New connection
                conn_id = ConnId(self._next_id)
                self._next_id += 1
                conn = listener.accept_connection(conn_id)
                self._conns[conn_id] = conn
                sel.register(conn.fileno(), selectors.EVENT_READ, data=conn_id)
                handler.on_connect(conn_id)

            else:
                conn_id: ConnId = key.data
                conn = self._conns[conn_id]

                if mask & selectors.EVENT_READ:
                    data = conn.read_available()
                    if not data:
                        # Connection closed
                        sel.unregister(conn.fileno())
                        responses = handler.on_disconnect(conn_id)
                        self._dispatch(responses)
                        conn.close()
                        del self._conns[conn_id]
                        continue
                    conn._framer.feed(data)
                    for frame in conn._framer.frames():
                        msg = parse(frame.decode("utf-8", errors="replace"))
                        responses = handler.on_message(conn_id, msg)
                        self._dispatch(responses)

                if mask & selectors.EVENT_WRITE:
                    drained = conn.flush_writes()
                    if drained:
                        # No more pending writes; stop watching for writability
                        sel.modify(conn.fileno(), selectors.EVENT_READ, data=conn_id)

    sel.close()

def _dispatch(self, responses: list[tuple[ConnId, Message]]) -> None:
    for target_id, msg in responses:
        target = self._conns.get(target_id)
        if target:
            target.enqueue_write(serialize(msg))
            # Register for write readiness if not already
            sel.modify(target.fileno(),
                       selectors.EVENT_READ | selectors.EVENT_WRITE,
                       data=target_id)
```

Key observation: because the event loop is single-threaded, `handler.on_message()` is always
called from the same thread. **No server lock needed.** This eliminates the bottleneck from
`irc-net-stdlib`.

---

## Language Mappings

| Language | Selector API | Notes |
|---|---|---|
| Python | `selectors.DefaultSelector` | Uses `epoll` on Linux, `kqueue` on macOS, `select` on Windows |
| Go | `net.Listener` + goroutine-per-conn + channel fan-out | Go's runtime has its own poller under goroutines; "selectors" in Go means goroutine pool |
| TypeScript (Node) | Already single-threaded event loop | `net.createServer()` with `on('data')` callbacks IS the reactor |
| Rust | `mio::Poll` | Direct epoll/kqueue abstraction; no async runtime |
| Ruby | `IO.select` or `nio4r` gem | Ruby GIL makes this more important than Go goroutines |

### Go Note

Go's goroutines are not OS threads; the runtime multiplexes them onto a small thread pool using
its own internal poller (which uses `epoll` on Linux). A goroutine-per-connection in Go is
already more efficient than a thread-per-connection in Python. "Level 2" for Go means using
explicit channel-based fan-out rather than per-goroutine locks, or using `net.Poller` directly.

### Node.js Note

Node.js is inherently single-threaded and event-driven. The `irc-net-stdlib` TypeScript
implementation already used the event loop (no threads). For TypeScript, "Level 2" means
explicitly structuring the code around the event loop (`EventEmitter`, `Readable` streams)
rather than using higher-level abstractions like `readline`.

---

## Trade-offs vs irc-net-stdlib

| Concern | stdlib | selectors |
|---|---|---|
| Concurrency model | One thread per connection | Single thread, OS multiplexing |
| Memory per connection | ~8 MB (stack) | ~few KB (write buffer, framer) |
| Scalability | ~1,000 connections | ~10,000–100,000 connections |
| Complexity | Simple: threads are familiar | Moderate: callback/event discipline needed |
| Latency | Low (thread runs immediately) | Low (selector returns quickly) |
| Handler thread safety | Requires lock | None needed (single thread) |
| Write blocking | Blocks calling thread | Buffered; event loop retries |

---

## Test Strategy

Use the same test suite as `irc-net-stdlib` — the echo server, concurrent connections, and
graceful shutdown tests. The interface contract is identical; only the mechanism differs.

### Additional tests for selectors

- **No threads spawned**: after connecting 100 clients, assert `threading.active_count()` is 1
  (only the main thread).
- **Write backpressure**: create a slow-consumer connection (don't read from it). Send it 1 MB
  of data. Verify the event loop does not block and other connections continue operating normally.
- **Selector re-registration**: verify that after a write buffer is drained, the fd is removed
  from `EVENT_WRITE` interest (prevents busy-loop).
- **Rapid connect/disconnect**: connect and immediately disconnect 1000 connections. Verify no
  resource leaks (`len(self._conns) == 0` after all disconnects).

---

## Future: Peeling to irc-net-epoll

The `selectors` module is an abstraction over the OS's native event notification:
- On Linux: `epoll`
- On macOS/BSD: `kqueue`
- On Windows: `select` (limited)

The next layer (`irc-net-epoll`) removes this abstraction and calls `epoll_create1`,
`epoll_ctl`, and `epoll_wait` directly. The interface remains the same. The swap is one import
change in `ircd`.

What you learn by peeling: why the `selectors` abstraction exists, what `EPOLLIN`, `EPOLLOUT`,
and `EPOLLET` mean at the syscall level, and what bugs edge-triggered mode introduces.
