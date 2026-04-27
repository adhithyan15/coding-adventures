# irc-net-stdlib — Level 1: Standard Library Sockets + Threads

## Overview

`irc-net-stdlib` is the first and outermost layer of the Russian Nesting Doll. It implements the
stable `Connection`, `Listener`, and `EventLoop` interfaces using only each language's standard
library: OS-managed TCP sockets and one thread per accepted connection.

This is the simplest possible implementation. It is not the most efficient — one thread per
connection does not scale beyond a few thousand connections. But it requires no third-party
dependencies, works on all platforms, and is correct. Every language port starts here.

Once this layer is working, `irc-net-selectors` peels back the first layer of the doll.

---

## Layer Position

```
ircd (program)
    ↓ calls run()
irc-net-stdlib          ← THIS SPEC: Connection, Listener, EventLoop
    ↓ OS manages
TCP sockets (kernel)
    ↓
Physical network
```

---

## The Stable Interfaces

These interfaces are defined here and shared across all `irc-net-*` specs. They do not change
as implementations are swapped. The `ircd` program imports only these types, never a concrete
implementation class directly.

```python
from __future__ import annotations

from typing import Protocol, NewType


ConnId = NewType('ConnId', int)


class Connection(Protocol):
    """A single accepted TCP connection.

    read() returns raw bytes from the socket.
    Returns b"" when the connection has been closed by the peer.
    write() sends bytes to the peer. May block if the send buffer is full.
    close() closes the connection. Safe to call multiple times.
    """

    @property
    def id(self) -> ConnId: ...

    @property
    def peer_addr(self) -> tuple[str, int]: ...

    def read(self) -> bytes: ...

    def write(self, data: bytes) -> None: ...

    def close(self) -> None: ...


class Listener(Protocol):
    """A bound, listening TCP socket.

    accept() blocks until a new connection arrives, then returns a Connection.
    close() stops accepting new connections.
    """

    def accept(self) -> Connection: ...

    def close(self) -> None: ...


class Handler(Protocol):
    """The IRC server's interface to the network layer.

    The EventLoop calls these methods as events occur.
    Implementations must be thread-safe if the EventLoop uses multiple threads.
    """

    def on_connect(self, conn_id: ConnId) -> None: ...

    def on_message(self, conn_id: ConnId, msg: Message) -> None: ...

    def on_disconnect(self, conn_id: ConnId) -> None: ...


class EventLoop(Protocol):
    """Drives the server: accepts connections and dispatches messages.

    run() blocks until the server is shut down (e.g. via stop()).
    """

    def run(self, listener: Listener, handler: Handler) -> None: ...

    def stop(self) -> None: ...
```

---

## Concepts

### Thread-per-Connection Model

The simplest possible concurrent server: the main thread accepts connections in a loop.
For each accepted connection, it spawns a new thread. That thread reads lines in a loop,
calls the handler, and exits when the connection closes.

```
main thread                    worker thread (one per connection)
────────────                   ──────────────────────────────────
listener.accept() ──────────→  while True:
                                   data = conn.read()       # blocks here
                                   if not data: break
                                   framer.feed(data)
                                   for frame in framer.frames():
                                       msg = parse(frame)
                                       responses = handler.on_message(conn.id, msg)
                                       for (target_id, resp) in responses:
                                           target_conn.write(serialize(resp))
                               handler.on_disconnect(conn.id)
```

### Shared Connection Map

All threads need access to `target_conn` to fan out channel messages to other users. The
`EventLoop` maintains a thread-safe map of `ConnId → Connection`. Access is protected by a lock.

```python
import threading

conn_map: dict[ConnId, Connection] = {}
conn_lock = threading.Lock()
```

The `write()` calls in the worker thread acquire the lock on the target connection (or on the
map) before sending. This prevents interleaved writes when two threads fan out to the same
connection simultaneously.

### Thread Safety Contract

`IRCServer` (from `irc-server`) is **not** thread-safe. With a thread-per-connection model,
multiple threads will call `handler.on_message()` concurrently. The `EventLoop` must serialize
these calls with a lock:

```python
server_lock = threading.Lock()

def handle_message(conn_id: ConnId, msg: Message) -> None:
    with server_lock:
        responses = handler.on_message(conn_id, msg)
    # send responses outside the lock
    for target_id, resp in responses:
        send_to(target_id, serialize(resp))
```

This lock is a global bottleneck. IRC traffic is mostly idle, so this is acceptable for stdlib.
The event-loop implementations (`irc-net-selectors` and below) avoid this by being single-threaded.

---

## Implementation

### Python

```python
from __future__ import annotations

import socket
import threading
from typing import Iterator

from irc_framing import Framer
from irc_proto import Message, parse, serialize


class StdlibConnection:
    _next_id: int = 0
    _lock: threading.Lock = threading.Lock()

    def __init__(self, sock: socket.socket, addr: tuple[str, int]) -> None:
        with StdlibConnection._lock:
            StdlibConnection._next_id += 1
            self._id = ConnId(StdlibConnection._next_id)
        self._sock = sock
        self._addr = addr

    @property
    def id(self) -> ConnId:
        return self._id

    @property
    def peer_addr(self) -> tuple[str, int]:
        return self._addr

    def read(self) -> bytes:
        try:
            return self._sock.recv(4096)
        except OSError:
            return b""

    def write(self, data: bytes) -> None:
        self._sock.sendall(data)

    def close(self) -> None:
        try:
            self._sock.close()
        except OSError:
            pass


class StdlibListener:
    def __init__(self, host: str, port: int) -> None:
        self._sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._sock.bind((host, port))
        self._sock.listen(128)

    def accept(self) -> StdlibConnection:
        sock, addr = self._sock.accept()
        return StdlibConnection(sock, addr)

    def close(self) -> None:
        self._sock.close()


class StdlibEventLoop:
    def __init__(self) -> None:
        self._running = False
        self._conns: dict[ConnId, StdlibConnection] = {}
        self._conns_lock = threading.Lock()
        self._server_lock = threading.Lock()

    def run(self, listener: StdlibListener, handler: Handler) -> None:
        self._running = True
        while self._running:
            conn = listener.accept()
            with self._conns_lock:
                self._conns[conn.id] = conn
            t = threading.Thread(
                target=self._handle_conn,
                args=(conn, handler),
                daemon=True,
            )
            t.start()

    def stop(self) -> None:
        self._running = False

    def _handle_conn(self, conn: StdlibConnection, handler: Handler) -> None:
        framer = Framer()
        with self._server_lock:
            handler.on_connect(conn.id)
        try:
            while True:
                data = conn.read()
                if not data:
                    break
                framer.feed(data)
                for frame in framer.frames():
                    msg = parse(frame.decode("utf-8", errors="replace"))
                    with self._server_lock:
                        responses = handler.on_message(conn.id, msg)
                    self._send_responses(responses)
        finally:
            with self._server_lock:
                responses = handler.on_disconnect(conn.id)
            self._send_responses(responses)
            with self._conns_lock:
                self._conns.pop(conn.id, None)
            conn.close()

    def _send_responses(self, responses: list[tuple[ConnId, Message]]) -> None:
        for target_id, resp in responses:
            with self._conns_lock:
                target = self._conns.get(target_id)
            if target:
                target.write(serialize(resp))
```

### Language Mappings

| Language | Listener | Connection | Thread |
|---|---|---|---|
| Python | `socket.socket.accept()` | `socket.socket` | `threading.Thread` |
| Go | `net.Listen("tcp", addr)` | `net.Conn` | goroutine |
| TypeScript (Node) | `net.createServer()` | `net.Socket` | event loop (Node is single-threaded) |
| Ruby | `TCPServer.new` | `TCPSocket` | `Thread.new` |
| Elixir | `:gen_tcp.listen` | `:gen_tcp` socket | lightweight process (`spawn`) |
| Rust | `std::net::TcpListener` | `std::net::TcpStream` | `std::thread::spawn` |
| Kotlin | `java.net.ServerSocket` | `java.net.Socket` | coroutine or `Thread` |
| Swift | `Network.framework NWListener` | `NWConnection` | `DispatchQueue` |
| C# | `System.Net.Sockets.TcpListener` | `TcpClient` | `Task` / `async-await` |
| F# | same as C# | same as C# | `async` computation expression |

Node.js / TypeScript note: Node's event loop makes the stdlib implementation effectively
single-threaded. No explicit thread management is needed, but write calls must be careful not
to block.

---

## Trade-offs

| Concern | Impact | Mitigation |
|---|---|---|
| One thread per connection | ~8 MB stack per thread; limits to ~1000 connections | Acceptable for IRC; next doll peels this |
| Global server lock | Serializes all IRC logic | IRCServer is fast; lock held briefly |
| Blocking `recv` | Thread blocks while waiting for data | Fine; threads are cheap for low-concurrency |
| No backpressure | Slow client can fill send buffer | `sendall` blocks sender thread; natural backpressure |

---

## Test Strategy

Tests must verify the interface contract, not the implementation internals. Use the same tests
for all `irc-net-*` implementations.

### Echo server test (implementation-agnostic)

Create a minimal `EchoHandler` that reflects every message back to the sender. Start the event
loop in a thread. Connect a real TCP socket. Send `PING :test\r\n`. Assert the response is
`PONG :test\r\n`. Disconnect. Verify `on_disconnect` was called.

### Concurrent connections

Connect 20 clients simultaneously. Each sends `NICK client_N\r\nUSER ...\r\nJOIN #test\r\n`.
Verify each receives a welcome sequence and the NAMES list grows with each join. All on stdlib
threads.

### Graceful shutdown

Start the server. Connect 5 clients. Call `stop()`. Verify the listener closes and the program
exits cleanly (no hanging threads).

### Write serialization

Connect two clients to the same channel. Have client A send 50 PRIVMSG messages rapidly.
Verify client B receives all 50 messages in order (no interleaving / corruption).

---

## Future: Peeling This Layer

When ready to move to `irc-net-selectors`, the steps are:

1. Implement `SelectorsConnection`, `SelectorsListener`, `SelectorsEventLoop` in `irc-net-selectors`.
2. Verify they pass the same echo server and concurrency tests.
3. In `ircd`, swap `StdlibEventLoop` for `SelectorsEventLoop` (one line change in the wiring).
4. Run integration tests with a real IRC client. Nothing else changes.

The `ircd` program, `irc-server`, `irc-framing`, and `irc-proto` are untouched.
