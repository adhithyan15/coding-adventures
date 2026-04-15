# ircd (Go)

RFC 1459 IRC server — the full stack wired together.

## Overview

`ircd` is the runnable IRC server. It glues the four library packages into a
working server:

```
irc-net-stdlib  (TCP event loop)
      |
      v
DriverHandler   (adapter in main.go)
      |
      +---> irc-framing  (byte stream -> IRC frames)
      +---> irc-proto    (frame -> Message struct)
      +---> irc-server   (Message -> []Response)
      +---> irc-proto    (Message -> wire bytes)
      +---> irc-net-stdlib.SendTo (write responses)
```

## Usage

```
# Default: listen on 0.0.0.0:6667, server name irc.local
ircd

# Custom options
ircd --host 0.0.0.0 --port 6697 --name irc.example.com \
     --motd "Welcome!" --motd "Enjoy your stay." \
     --oper-password s3cr3t
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--host` | `0.0.0.0` | Bind address |
| `--port` | `6667` | TCP port |
| `--name` | `irc.local` | Server name sent in 001 |
| `--motd` | _(empty)_ | MOTD line (repeat for multi-line) |
| `--oper-password` | _(empty)_ | Password for OPER command |

## Architecture

The `DriverHandler` struct implements `irc_net_stdlib.Handler`. For each
connection it maintains a dedicated `irc_framing.Framer` to accumulate bytes
into complete IRC lines. When a frame is complete it is parsed with
`irc_proto.Parse` and forwarded to `irc_server.OnMessage`. Each response from
the server is serialized with `irc_proto.Serialize` and written back via
`loop.SendTo`.

`runLoop` is the testable entry point — it accepts a `stopCh` for clean
shutdown without OS signals.

## Coverage

70%+ statement coverage (signal-handling `run`/`main` are excluded by design).
