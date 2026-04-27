# ircd — The IRC Server Program

## Overview

`ircd` is the program that wires all packages together. It is the only place in the system
that knows about the specific `irc-net-*` implementation being used. Everything else — the IRC
logic, framing, and parsing — is isolated in packages with stable interfaces.

`ircd` is not a library. It is an executable. Its job is small:
1. Parse CLI arguments and build a configuration
2. Create a `Listener` from the chosen `irc-net-*` implementation
3. Create an `IRCServer` from `irc-server`
4. Wire them together through a `DriverHandler` adapter
5. Call `EventLoop.run()` and block until shutdown

Because `ircd` is thin, porting it to a new language is fast. The work is in the packages.

---

## Layer Position

```
ircd                    ← THIS SPEC: the program; wires all packages
    ├── irc-server      (IRC state machine)
    ├── irc-proto       (parsing/serialization)
    ├── irc-framing     (byte stream → frames)
    └── irc-net-*       (chosen network implementation)
```

---

## CLI Interface

```
ircd [OPTIONS]

Options:
  --host HOST          IP address to listen on (default: 0.0.0.0)
  --port PORT          TCP port to listen on (default: 6667)
  --server-name NAME   IRC server name sent in responses (default: irc.local)
  --motd TEXT          Message of the Day (default: "Welcome.")
  --net-impl IMPL      Network implementation to use (default: stdlib)
                       Choices: stdlib, selectors, epoll, smoltcp
  --oper-pass PASS     IRC operator password for OPER command (default: none)
  --max-conns N        Maximum simultaneous connections (default: unlimited)
  --help               Show this help message and exit
  --version            Show version and exit
```

Example:
```bash
ircd --host 0.0.0.0 --port 6667 --server-name irc.example.com \
     --motd "Welcome to my IRC server." --net-impl selectors
```

---

## Configuration

```python
from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class Config:
    host: str = "0.0.0.0"
    port: int = 6667
    server_name: str = "irc.local"
    motd: list[str] = field(default_factory=lambda: ["Welcome."])
    net_impl: str = "stdlib"
    oper_password: str = ""
    max_connections: int | None = None


def parse_args(argv: list[str]) -> Config:
    """Parse sys.argv[1:] into a Config.

    Raises SystemExit with an error message on invalid arguments.
    """
    ...
```

---

## Wiring: The DriverHandler Adapter

`IRCServer` (from `irc-server`) implements the `Handler` protocol. But the `EventLoop` calls
`on_connect`, `on_message`, and `on_disconnect` with `ConnId` values, while `IRCServer` returns
`list[tuple[ConnId, Message]]` responses that need to be written to the network.

The `DriverHandler` adapter sits between them:

```python
from __future__ import annotations

from irc_framing import Framer
from irc_net_stdlib import StdlibConnection, EventLoop  # or whichever impl
from irc_proto import Message, parse, serialize
from irc_server import IRCServer, ConnId


class DriverHandler:
    """Adapts IRCServer to the EventLoop.Handler protocol.

    Responsibilities:
    - Maintains per-connection Framer instances
    - Calls IRCServer methods and dispatches responses
    - Serializes access to IRCServer (if the event loop is multi-threaded)
    - Handles encoding errors gracefully
    """

    def __init__(self, server: IRCServer, send_fn: SendFn) -> None:
        self._server = server
        self._send = send_fn          # callable: (ConnId, bytes) -> None
        self._framers: dict[ConnId, Framer] = {}

    def on_connect(self, conn_id: ConnId, host: str) -> None:
        self._framers[conn_id] = Framer()
        responses = self._server.on_connect(conn_id, host)
        self._dispatch(responses)

    def on_data(self, conn_id: ConnId, data: bytes) -> None:
        """Called by the event loop when raw bytes arrive."""
        framer = self._framers.get(conn_id)
        if framer is None:
            return
        framer.feed(data)
        for frame in framer.frames():
            try:
                msg = parse(frame.decode("utf-8", errors="replace"))
            except Exception:
                continue   # malformed message; skip
            responses = self._server.on_message(conn_id, msg)
            self._dispatch(responses)

    def on_disconnect(self, conn_id: ConnId) -> None:
        self._framers.pop(conn_id, None)
        responses = self._server.on_disconnect(conn_id)
        self._dispatch(responses)

    def _dispatch(self, responses: list[tuple[ConnId, Message]]) -> None:
        for target_id, msg in responses:
            self._send(target_id, serialize(msg))


# Type alias for the send callback
from typing import Callable
SendFn = Callable[[ConnId, bytes], None]
```

---

## Main Entry Point

```python
from __future__ import annotations

import signal
import sys

from irc_net_stdlib import StdlibEventLoop, StdlibListener
from irc_server import IRCServer


def make_event_loop(net_impl: str) -> tuple[object, object]:
    """Construct the appropriate Listener and EventLoop for net_impl."""
    if net_impl == "stdlib":
        from irc_net_stdlib import StdlibEventLoop, StdlibListener
        return StdlibListener, StdlibEventLoop
    elif net_impl == "selectors":
        from irc_net_selectors import SelectorsEventLoop, SelectorsListener
        return SelectorsListener, SelectorsEventLoop
    elif net_impl == "epoll":
        from irc_net_epoll import EpollEventLoop, EpollListener
        return EpollListener, EpollEventLoop
    else:
        print(f"Unknown net-impl: {net_impl}", file=sys.stderr)
        sys.exit(1)


def main(argv: list[str] | None = None) -> None:
    config = parse_args(argv or sys.argv[1:])

    ListenerClass, EventLoopClass = make_event_loop(config.net_impl)

    listener = ListenerClass(config.host, config.port)
    event_loop = EventLoopClass()

    server = IRCServer(
        server_name=config.server_name,
        motd=config.motd,
        oper_password=config.oper_password,
    )

    handler = DriverHandler(server, send_fn=event_loop.send_to)

    # Graceful shutdown on SIGINT / SIGTERM
    def shutdown(signum: int, frame: object) -> None:
        print("\nShutting down...", flush=True)
        event_loop.stop()

    signal.signal(signal.SIGINT, shutdown)
    signal.signal(signal.SIGTERM, shutdown)

    print(f"ircd listening on {config.host}:{config.port} [{config.net_impl}]", flush=True)
    event_loop.run(listener, handler)
    listener.close()
    print("ircd stopped.", flush=True)


if __name__ == "__main__":
    main()
```

---

## Graceful Shutdown

On SIGINT or SIGTERM:
1. `event_loop.stop()` signals the event loop to exit its run loop
2. The event loop sends `ERROR :Server shutting down` to all connected clients
3. The event loop closes all connection sockets
4. `listener.close()` closes the listening socket
5. The program exits with code 0

The `ERROR` message is required by RFC 1459 when a server disconnects a client.

---

## Startup Validation

Before calling `event_loop.run()`, `ircd` validates the configuration:

```python
def validate_config(config: Config) -> None:
    """Raise ValueError with a descriptive message if config is invalid."""
    if not 1 <= config.port <= 65535:
        raise ValueError(f"Port must be 1–65535, got {config.port}")
    if not config.server_name:
        raise ValueError("Server name must not be empty")
    if config.max_connections is not None and config.max_connections < 1:
        raise ValueError("max-connections must be at least 1")
```

Port binding failures (address already in use, permission denied for ports < 1024) are caught
at listener construction and reported with a clear message before exit.

---

## Language Mappings

| Language | CLI parsing | Signal handling | Entry point |
|---|---|---|---|
| Python | `argparse` | `signal.signal` | `if __name__ == "__main__"` |
| Go | `flag` package | `signal.Notify` | `func main()` |
| TypeScript | `process.argv`, `minimist` | `process.on('SIGINT')` | top-level module |
| Ruby | `OptionParser` | `Signal.trap` | executable script |
| Elixir | `OptionParser` | `System.at_exit` | `mix run` / escript |
| Rust | `clap` | `ctrlc` crate | `fn main()` |
| Kotlin | `kotlinx.cli` | `Runtime.getRuntime().addShutdownHook` | `fun main()` |
| Swift | `ArgumentParser` | `signal(SIGINT, ...)` | `@main struct` |
| C# | `System.CommandLine` | `Console.CancelKeyPress` | `static void Main()` |
| F# | `Argu` | `Console.CancelKeyPress` | `[<EntryPoint>] let main` |

---

## Test Strategy

### Unit tests

- `parse_args`: valid args, missing required args, invalid port number, unknown net-impl
- `validate_config`: port out of range, empty server name, zero max-connections
- `DriverHandler`: feed raw bytes, verify correct IRCServer methods called; verify responses
  dispatched to correct ConnIds; verify framer per-connection isolation

### Integration tests

The integration test suite starts a real `ircd` process and connects with raw TCP sockets.
It does not depend on any IRC client library — it sends and parses raw protocol text.

```python
def test_registration() -> None:
    """Full registration flow over a real TCP socket."""
    with socket.create_connection(("127.0.0.1", 6667), timeout=5) as s:
        s.sendall(b"NICK testbot\r\nUSER testbot 0 * :Test Bot\r\n")
        buf = b""
        while b"376" not in buf:  # wait for end of MOTD
            buf += s.recv(4096)
        assert b"001" in buf
        assert b"testbot" in buf


def test_channel_message() -> None:
    """Two clients in a channel; message delivered to both."""
    with socket.create_connection(("127.0.0.1", 6667)) as a, \
         socket.create_connection(("127.0.0.1", 6667)) as b:
        register(a, "alice")
        register(b, "bob")
        a.sendall(b"JOIN #test\r\n")
        b.sendall(b"JOIN #test\r\n")
        wait_for(a, "366")   # end of names
        wait_for(b, "366")
        a.sendall(b"PRIVMSG #test :hello\r\n")
        response = recv_until(b, "PRIVMSG")
        assert b"hello" in response
```

### End-to-end test with WeeChat

Manual test script:
```
1. Start ircd on port 6667
2. Open WeeChat: /connect localhost 6667
3. Verify: welcome message received, MOTD displayed
4. /join #test
5. Open second WeeChat session, join #test
6. Send messages from both sessions; verify delivery
7. /quit from one session; verify QUIT notice in other
8. Kill ircd with Ctrl-C; verify WeeChat shows disconnect
```

---

## Related Specs

- [irc-architecture.md](irc-architecture.md) — System overview and layer diagram
- [irc-proto.md](irc-proto.md) — Message parsing and serialization
- [irc-framing.md](irc-framing.md) — Byte stream framing
- [irc-server.md](irc-server.md) — IRC state machine
- [irc-net-stdlib.md](irc-net-stdlib.md) — Default network implementation
- [irc-net-selectors.md](irc-net-selectors.md) — Event-driven network implementation
- [irc-net-epoll.md](irc-net-epoll.md) — Raw syscall implementation
- [irc-net-smoltcp.md](irc-net-smoltcp.md) — Userspace TCP (Rust)
- [irc-unikernel.md](irc-unikernel.md) — Bare metal unikernel (Rust)
