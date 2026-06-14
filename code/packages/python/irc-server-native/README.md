# coding-adventures-irc-server-native

A **high-performance IRC server for Python** — every line of IRC and TCP logic
runs in Rust (the [`irc-net-reactor`](../../rust/irc-net-reactor) engine on the
home-grown `kqueue`/`epoll` reactor). Python only launches and controls the
server, through a thin native extension built on the zero-dependency
`python-bridge` (no PyO3).

## Why

This is the Python member of a family: one Rust IRC engine, embedded natively in
every language. Because all logic lives in Rust, the binding is a pure lifecycle
control surface — **create, serve, stop** — with no per-message callback into
Python (and therefore none of the GIL-re-acquisition complexity that a
callback-based framework like `conduit` needs).

## Usage

```python
from coding_adventures.irc_server_native import IrcServer

server = IrcServer(host="127.0.0.1", port=6667, server_name="irc.example")
server.serve()   # blocks until another thread calls server.stop()
```

Bind an ephemeral port and read it back (handy for tests):

```python
import threading

server = IrcServer(host="127.0.0.1", port=0)
port = server.local_port()
threading.Thread(target=server.serve, daemon=True).start()
# ... connect IRC clients to 127.0.0.1:port ...
server.stop()
```

Point an IRC client (irssi, WeeChat, `nc`) at the host/port, register with
`NICK`/`USER`, `JOIN #channel`, and chat — the broadcast fan-out to other
channel members happens entirely in Rust.

## API

| method                | description                                            |
|-----------------------|--------------------------------------------------------|
| `IrcServer(host, port, server_name, motd, oper_password, max_connections)` | build + bind |
| `serve()`             | run the event loop, blocking (the GIL is released)     |
| `stop()`              | signal the loop to stop (safe from another thread)     |
| `local_host()` / `local_port()` | the bound address (real port after `port=0`)  |
| `running()`           | whether the loop is currently running                  |
| `dispose()`           | free the listener now (must be stopped first)          |

## How it's built

The `BUILD` script compiles the Rust cdylib in `ext/irc_server_native`, copies it
into the package under the platform's extension suffix, then installs and tests:

```
cd ext/irc_server_native && cargo build --release
# copy target/release/libirc_server_native.* → src/.../irc_server_native<EXT_SUFFIX>
uv venv && uv pip install -e ".[dev]"
pytest
```

## Layer position

```
coding_adventures.irc_server_native.IrcServer   (this package — Python facade)
        ↓ irc_server_native C extension (python-bridge, GIL released on serve)
irc-net-reactor   (Rust IRC engine, in-process broadcast)
        ↓
tcp-runtime → transport-platform → kqueue / epoll / IOCP
```
