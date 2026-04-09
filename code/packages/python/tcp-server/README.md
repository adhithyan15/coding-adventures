# tcp-server — DT24

Single-threaded TCP server with pluggable handler. Part of the
[coding-adventures](https://github.com/coding-adventures) series.

## What It Does

`tcp-server` implements the I/O layer of a Redis-like server. It:

1. Accepts TCP connections on a configurable host and port
2. Reads raw bytes from each connected client
3. Passes those bytes to a user-supplied **handler function**
4. Writes back whatever the handler returns
5. Handles multiple clients sequentially using OS-level I/O multiplexing
6. Shuts down cleanly when `stop()` is called from any thread

The server has **no opinion about the bytes it carries**. It does not know
about RESP, HTTP, or any other protocol. That knowledge lives in the handler
(DT25, the mini-Redis layer).

## Where It Fits

```
DT23: resp-protocol   ← pure encode/decode, no I/O
DT24: tcp-server      ← [THIS PACKAGE] I/O only, no protocol knowledge
DT25: mini-redis      ← uses DT24 for connections + DT23 for RESP framing
```

## Installation

```bash
pip install coding-adventures-tcp-server
```

## Quick Start

```python
from tcp_server import TcpServer

# Echo server: returns every byte unchanged
server = TcpServer(host="127.0.0.1", port=6380)
server.serve_forever()
```

```python
# Shout server: uppercase everything
server = TcpServer(
    host="127.0.0.1",
    port=6380,
    handler=lambda data: data.upper(),
)
server.serve_forever()
```

```python
# Controlled shutdown from another thread
import threading
from tcp_server import TcpServer

server = TcpServer(host="127.0.0.1", port=6380, handler=lambda d: d)
t = threading.Thread(target=server.serve_forever, daemon=True)
t.start()

# ... do work ...

server.stop()
t.join()
```

## API

### `TcpServer`

```python
TcpServer(
    host: str = "127.0.0.1",
    port: int = 6380,
    handler: Callable[[bytes], bytes] | None = None,
    backlog: int = 128,
    buffer_size: int = 4096,
)
```

| Method / Property | Description |
|---|---|
| `start()` | Bind and listen. Does not block. |
| `serve()` | Enter event loop. Blocks until `stop()`. |
| `serve_forever()` | `start()` + `serve()` in one call. |
| `stop()` | Signal the event loop to exit. Thread-safe. |
| `address` | `(host, port)` the server is bound to. Valid after `start()`. |
| `is_running` | `True` while the event loop is active. |
| `__enter__` / `__exit__` | Context manager support. |
| `__repr__` | `TcpServer(host='...', port=..., status='running')` |

### `Handler` type alias

```python
Handler = Callable[[bytes], bytes]
```

The handler receives the raw bytes from one `recv()` call and must return
the raw bytes to send back. Returning `b""` sends nothing.

## How It Works

Under the hood, every TCP server calls the same seven OS syscalls regardless
of language. Python's `socket` and `selectors` modules wrap them:

| Syscall | Python | Purpose |
|---|---|---|
| `socket()` | `socket.socket(AF_INET, SOCK_STREAM)` | Create a file descriptor |
| `bind()` | `sock.bind((host, port))` | Attach fd to address + port |
| `listen()` | `sock.listen(backlog)` | Mark fd as passive listener |
| `accept()` | `sock.accept()` | Dequeue a pending connection |
| `read()` | `conn.recv(buffer_size)` | Receive bytes from client |
| `write()` | `conn.sendall(response)` | Send bytes to client |
| `close()` | `conn.close()` | Release the file descriptor |

The event loop uses Python's `selectors.DefaultSelector` which maps to
`epoll` on Linux, `kqueue` on macOS/BSD, and `select` on Windows. The OS
puts the process to sleep when no fd has data; it wakes exactly when a client
sends bytes or a new connection arrives. This is how Redis achieves over one
million operations per second on a single thread.

## Development

```bash
# Install dependencies and run tests
uv venv .venv --no-project
uv pip install --python .venv -e ../resp-protocol
uv pip install --python .venv -e .[dev]
uv run --no-project python -m pytest tests/ -v
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
