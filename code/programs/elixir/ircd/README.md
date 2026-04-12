# ircd (Elixir)

IRC server program wiring the full Elixir IRC stack together.

## Overview

`ircd` is the top of the "Russian Nesting Doll" IRC stack. It connects the
pure IRC logic (`irc_server`) to the TCP transport layer (`irc_net_stdlib`)
via the `DriverHandler` bridge module. The result is a complete, runnable
IRC server.

## Running

### As a Mix escript

```bash
cd code/programs/elixir/ircd
mix escript.build
./coding_adventures_ircd --port 6667 --server-name irc.example.com
```

### Via Mix (development)

```bash
mix run -e 'CodingAdventures.Ircd.main(["--port", "6667"])'
```

## Command-line Options

| Flag                    | Default      | Description                         |
|-------------------------|--------------|-------------------------------------|
| `--host HOST`           | `0.0.0.0`    | Bind address                        |
| `--port PORT`           | `6667`       | TCP port                            |
| `--server-name NAME`    | `irc.local`  | Server hostname shown in messages   |
| `--motd LINE`           | `Welcome.`   | MOTD line (may be repeated)         |
| `--oper-password PASS`  | _(empty)_    | Password for the OPER command       |

## Architecture

```
TCP socket
   | raw bytes
EventLoop.on_data()           <- irc_net_stdlib
   | conn_id, raw bytes
DriverHandler.on_data()       <- ircd (this program)
   | feeds bytes into per-connection Framer
Framer.frames()               <- irc_framing
   | "NICK alice"
IrcProto.parse()              <- irc_proto
   | Message(command: "NICK")
IrcServer.on_message()        <- irc_server
   | [{conn_id, Message}]
IrcProto.serialize()
   | ":irc.local 001 alice :Welcome\r\n"
EventLoop.send_to()           <- irc_net_stdlib
   | bytes on the wire
```

## Key Modules

- `CodingAdventures.Ircd` — entry point, argument parsing, `Config` struct.
- `CodingAdventures.Ircd.DriverHandler` — GenServer bridging `irc_net_stdlib`
  callbacks to `irc_server` state machine calls.

## In the Stack

    ircd (this program)
      |
      +-- irc_net_stdlib   <-- TCP event loop
      |
      +-- irc_server       <-- pure IRC state machine
      |
      +-- irc_framing      <-- CRLF framing
      |
      +-- irc_proto        <-- message parsing
