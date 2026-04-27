# irc-net-stdlib

**Level 1 network implementation for the coding-adventures IRC stack.**

This package provides a concrete TCP networking layer built on Python's
standard-library sockets and OS threads.  It is the first of potentially
several `irc-net-*` packages that all implement the same stable
`Connection` / `Listener` / `Handler` / `EventLoop` interfaces, so the IRC
server can swap networking backends without changing any server logic.

Specification: [`../../../specs/irc-net-stdlib.md`](../../../specs/irc-net-stdlib.md)

---

## What it does

- Opens a TCP server socket (with `SO_REUSEADDR`) on a given host and port.
- Accepts incoming connections in a loop.
- Spawns **one OS thread per connection**.  Each thread owns a blocking
  `recv` loop.
- Delivers raw byte chunks to a `Handler` callback interface.
- Serialises all handler calls behind a lock so the IRC server need not be
  thread-safe.
- Allows any thread to push bytes to any connection via `send_to()`.

---

## Thread-per-connection model

```
accept() loop ──► StdlibConnection
                       │
                       └──► daemon thread
                                 │
                          on_connect()   ─┐
                          on_data()      ─┤  all under _handler_lock
                          on_disconnect()─┘
```

Each accepted connection gets a Python `threading.Thread` (daemon, so it
doesn't prevent process exit).  Inside the thread:

1. Call `handler.on_connect(conn_id, host)` once.
2. Loop: `conn.read()` → `handler.on_data(conn_id, data)` for each chunk.
3. When `read()` returns `b""` (connection closed): call
   `handler.on_disconnect(conn_id)`, clean up, exit.

### Two locks protect shared state

| Lock | Protects | Held by |
|------|----------|---------|
| `_handler_lock` | All `Handler` callbacks | Worker threads |
| `_conns_lock` | `_conns` dict (ConnId → Connection) | Main thread (insert on accept) + worker threads (remove on close) + `send_to` (lookup) |

The two locks are always acquired separately — never both at once — so
deadlock is impossible.

`send_to()` acquires `_conns_lock` for the dict lookup, then releases it
before calling `conn.write()`.  This means a slow TCP write never stalls
other threads from running IRC logic.

---

## Installation

This package is part of the coding-adventures monorepo and is not published
to PyPI.  Install it in development mode:

```bash
pip install -e .
```

Or let the `BUILD` script install all dependencies:

```bash
bash BUILD
```

---

## Quick start

```python
from irc_net_stdlib import create_listener, StdlibEventLoop, ConnId, Handler

class MyHandler:
    def __init__(self, loop: StdlibEventLoop) -> None:
        self.loop = loop

    def on_connect(self, conn_id: ConnId, host: str) -> None:
        print(f"New connection {conn_id} from {host}")

    def on_data(self, conn_id: ConnId, data: bytes) -> None:
        # Echo back (real server would frame + parse here)
        self.loop.send_to(conn_id, data)

    def on_disconnect(self, conn_id: ConnId) -> None:
        print(f"Connection {conn_id} closed")

loop = StdlibEventLoop()
listener = create_listener("0.0.0.0", 6667)
handler = MyHandler(loop)

# Blocks until loop.stop() is called from another thread.
loop.run(listener, handler)
```

---

## Stable interfaces

All `irc-net-*` packages expose the same Protocol types:

```python
from irc_net_stdlib import ConnId, Connection, Listener, Handler, EventLoop
```

| Type | Role |
|------|------|
| `ConnId` | `NewType(int)` — unique connection identity |
| `Connection` | Wraps a socket: `read()`, `write()`, `close()` |
| `Listener` | Wraps a server socket: `accept()`, `close()` |
| `Handler` | Callbacks: `on_connect`, `on_data`, `on_disconnect` |
| `EventLoop` | Drives I/O: `run()`, `stop()`, `send_to()` |

---

## Dependencies

- `irc-proto` — IRC message parsing (not used directly here; consumed by driver)
- `irc-framing` — byte-stream framer (not used directly here; consumed by driver)
- `irc-server` — IRC server state machine (called via the Handler interface)

---

## Running tests

```bash
pip install -e .[dev]
pytest tests/ -v
```

Tests use **real TCP sockets** — no mocking.  Coverage target: 85%+.

---

## Layer diagram

```
ircd (program)
  └── DriverHandler  ← assembles framer + parser + server
        ├── irc-framing  (Framer: bytes → IRC lines)
        ├── irc-proto    (parse/serialize IRC messages)
        └── irc-server   (server state machine)
              ▲
        called via Handler callbacks
              │
irc-net-stdlib  ← THIS PACKAGE
  StdlibEventLoop + StdlibListener + StdlibConnection
```
