# ircd

IRC server executable — wires `irc-proto`, `irc-framing`, `irc-server`, and
`irc-net-stdlib` together into a runnable IRC server daemon.

This program is part of the [coding-adventures](../../../../README.md) monorepo,
which builds a complete IRC stack from scratch.  See the
[ircd specification](../../../specs/ircd.md) for the full design document.

## What it does

`ircd` is the top layer of the IRC stack.  It owns no business logic of its
own — it is a *wiring module* that connects:

| Layer            | Package            | Role                                                     |
|------------------|--------------------|----------------------------------------------------------|
| Network I/O      | `irc-net-stdlib`   | TCP accept loop, thread-per-connection, `send_to()`      |
| Byte framing     | `irc-framing`      | Reassembles TCP byte-stream into CRLF-terminated lines   |
| Message parsing  | `irc-proto`        | Parses `"PRIVMSG #chan :hello\r\n"` → `Message` objects  |
| IRC logic        | `irc-server`       | Channels, nicks, command dispatch, response generation   |

The central adapter is `DriverHandler`, which implements the `Handler`
protocol from `irc-net-stdlib` and translates raw byte events into parsed
messages for `irc-server`.

## Quick start

```bash
# Install in development mode (from the repo root):
cd code/programs/python/ircd
pip install -e ../../../packages/python/irc-proto \
            -e ../../../packages/python/irc-framing \
            -e ../../../packages/python/irc-server \
            -e ../../../packages/python/irc-net-stdlib \
            -e .

# Run the server (default: 0.0.0.0:6667):
python -m ircd

# Or, after pip install, via the console script:
ircd

# Custom options:
ircd --host 127.0.0.1 --port 6668 --server-name irc.example.com \
     --motd "Welcome to my IRC server." "Enjoy your stay." \
     --oper-password s3cr3t
```

Connect with any IRC client (e.g. `irssi`, `weechat`, `netcat`):

```
/server 127.0.0.1 6667
```

## CLI reference

```
usage: ircd [-h] [--host HOST] [--port PORT] [--server-name SERVER_NAME]
            [--motd [MOTD ...]] [--oper-password OPER_PASSWORD]

IRC server — wires irc-proto, irc-framing, irc-server, and irc-net-stdlib.

options:
  -h, --help            show this help message and exit
  --host HOST           IP address to bind to (default: 0.0.0.0)
  --port PORT           TCP port to listen on (default: 6667)
  --server-name SERVER_NAME
                        Server hostname advertised to clients (default: irc.local)
  --motd [MOTD ...]     Message of the Day lines (default: 'Welcome.')
  --oper-password OPER_PASSWORD
                        Password for the OPER command (default: empty = disabled)
```

## Architecture

```
TCP socket
   ↓ raw bytes
StdlibEventLoop.on_data()        ← irc-net-stdlib
   ↓ conn_id, raw bytes
DriverHandler.on_data()          ← ircd (THIS PACKAGE)
   ↓ feeds bytes into per-connection Framer
Framer.frames()                  ← irc-framing
   ↓ b"NICK alice"
irc_proto.parse()                ← irc-proto
   ↓ Message(command='NICK', ...)
IRCServer.on_message()           ← irc-server
   ↓ list[(ConnId, Message)]
irc_proto.serialize()            ← irc-proto
   ↓ b":irc.local 001 alice :Welcome\r\n"
EventLoop.send_to()              ← irc-net-stdlib
   ↓ bytes on the wire
```

## Running tests

```bash
bash BUILD
```

Or manually:

```bash
pip install -e ../../../packages/python/irc-proto \
            -e ../../../packages/python/irc-framing \
            -e ../../../packages/python/irc-server \
            -e ../../../packages/python/irc-net-stdlib \
            -e .[dev]
pytest tests/ -v
```

## Related packages

- [`irc-proto`](../../../packages/python/irc-proto/) — Message parsing and serialization
- [`irc-framing`](../../../packages/python/irc-framing/) — CRLF framing over TCP byte streams
- [`irc-server`](../../../packages/python/irc-server/) — Pure IRC state machine
- [`irc-net-stdlib`](../../../packages/python/irc-net-stdlib/) — Thread-per-connection event loop
