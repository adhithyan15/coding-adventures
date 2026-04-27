# irc-net-selectors

Level 2 of the IRC networking doll. Replaces one-thread-per-connection with a
**single-threaded event loop** using OS I/O readiness notifications.

## Position in the stack

```
ircd (program)
    ↓
irc-net-selectors       ← THIS PACKAGE
    ↓
selectors.DefaultSelector (epoll on Linux, kqueue on macOS, select on Windows)
    ↓
Kernel TCP stack
```

## Why this matters

`irc-net-stdlib` spawns one OS thread (8 MB stack) per connected client. At
10,000 connections that is 80 GB of virtual memory before a single IRC message
is processed.

`irc-net-selectors` uses **one thread for all connections**. The single
event-loop thread asks the OS: *"which of these N sockets has data?"* and wakes
up only when work is available. Memory per idle connection: ~400 bytes instead
of 8 MB — a 20,000× improvement.

This is the **reactor pattern**: the same model used by nginx, Node.js, Redis,
and Python's `asyncio`.

## Usage

```python
from irc_net_selectors import SelectorsEventLoop, create_listener

listener = create_listener("0.0.0.0", 6667)
loop = SelectorsEventLoop()
loop.run(listener, my_handler)   # blocks until loop.stop()
```

The `Handler` protocol is identical to `irc-net-stdlib`:

```python
class MyHandler:
    def on_connect(self, conn_id, host): ...
    def on_data(self, conn_id, data: bytes): ...
    def on_disconnect(self, conn_id): ...
```

Send data back to a client:

```python
loop.send_to(conn_id, b"hello\r\n")
```

## Swapping from irc-net-stdlib

In `ircd/__init__.py`, change:

```python
# Before:
from irc_net_stdlib import StdlibEventLoop, create_listener
loop = StdlibEventLoop()

# After:
from irc_net_selectors import SelectorsEventLoop, create_listener
loop = SelectorsEventLoop()
```

That's the only change. The IRC logic (`irc-server`, `irc-proto`, `irc-framing`)
is completely untouched.

## What you learn here

| Concept | Where |
|---|---|
| Reactor pattern | `SelectorsEventLoop.run()` |
| Non-blocking sockets | `SelectorsConnection.__init__()` (`setblocking(False)`) |
| Level-triggered readiness | `selectors.DefaultSelector` |
| Write buffer + backpressure | `SelectorsConnection.enqueue()` / `flush()` |
| Selector re-registration | `_handle_write()` — remove `EVENT_WRITE` when drained |
| OS selector abstractions | `selectors.DefaultSelector` → epoll / kqueue |

## Next layer

`irc-net-epoll` — remove the `selectors` abstraction and call `epoll_create1`,
`epoll_ctl`, and `epoll_wait` directly via Python's `ctypes` or the `select`
module's raw interface. Introduces edge-triggered mode.
