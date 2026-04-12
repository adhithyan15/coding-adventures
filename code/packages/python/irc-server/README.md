# irc-server

Pure IRC server state machine — channels, nicks, command dispatch.

## What it does

`irc-server` implements the brain of an IRC server as a pure Python state
machine.  It knows nothing about sockets, threads, I/O, or timers.  The
transport layer owns all of that — this package only answers the question
*"given this connection event or parsed message, what responses should be sent
to which connections?"*

All interaction happens through three methods on `IRCServer`:

```
on_connect(conn_id, host)  → list[Response]   # new TCP connection
on_message(conn_id, msg)   → list[Response]   # parsed IRC message arrived
on_disconnect(conn_id)     → list[Response]   # TCP connection closed
```

Each `Response` is a `(ConnId, Message)` pair.  The transport layer iterates
these and writes each message to the corresponding connection.

## Where it fits in the stack

```
irc-proto         # RFC 1459 message parsing and serialization
    └── irc-server    # ← you are here: state machine
            └── irc-transport  # (future) asyncio / TCP framing
```

`irc-server` depends only on `irc-proto`.  It imports `Message` from there
and returns `Message` values in all responses.

## Quick start

```python
from irc_proto import parse
from irc_server import ConnId, IRCServer

server = IRCServer(
    server_name="irc.example.com",
    version="1.0",
    motd=["Welcome to the example IRC server!"],
    oper_password="supersecret",
)

# New connection from 192.168.1.10
cid = ConnId(1)
server.on_connect(cid, "192.168.1.10")

# Client registers
server.on_message(cid, parse("NICK alice"))
responses = server.on_message(cid, parse("USER alice 0 * :Alice Smith"))
# responses now contains the 001–376 welcome sequence

# Client joins a channel
responses = server.on_message(cid, parse("JOIN #general"))
# responses: JOIN broadcast + 331 RPL_NOTOPIC + 353 NAMREPLY + 366 ENDOFNAMES

# Send each response to its target connection
for target_conn_id, msg in responses:
    # transport.send(target_conn_id, serialize(msg))
    pass

# Connection dropped unexpectedly
responses = server.on_disconnect(cid)
# responses: QUIT broadcast to channel peers
```

## Supported commands

| Category     | Commands                                         |
|--------------|--------------------------------------------------|
| Registration | `NICK`, `USER`, `CAP`, `QUIT`, `PASS`           |
| Channels     | `JOIN`, `PART`, `NAMES`, `LIST`, `TOPIC`        |
|              | `KICK`, `INVITE`, `MODE`                        |
| Messaging    | `PRIVMSG`, `NOTICE`                             |
| Server       | `PING`, `PONG`, `AWAY`, `WHOIS`, `WHO`, `OPER` |

Unknown commands return `421 ERR_UNKNOWNCOMMAND`.  Commands sent before
completing the NICK+USER handshake return `451 ERR_NOTREGISTERED`.

## Running tests

```bash
pip install -e ".[dev]"
pytest
```

Coverage is required to exceed 80% (currently ~94%).

## Related

- Spec: [`../../../specs/irc-server.md`](../../../specs/irc-server.md)
- Dependency: [`../irc-proto`](../irc-proto) — message parsing and serialization
