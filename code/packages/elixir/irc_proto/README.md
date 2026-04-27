# irc_proto (Elixir)

Pure IRC message parsing and serialization for Elixir.

## Overview

`irc_proto` is the lowest layer of the IRC stack. It converts between raw IRC
text lines (as defined by RFC 1459) and structured `Message` structs — with no
I/O, no processes, and no side effects.

## Usage

```elixir
alias CodingAdventures.IrcProto

{:ok, msg} = IrcProto.parse("NICK alice")
# => %Message{prefix: nil, command: "NICK", params: ["alice"]}

{:ok, msg2} = IrcProto.parse(":irc.local 001 alice :Welcome to the server!")
# => %Message{prefix: "irc.local", command: "001", params: ["alice", "Welcome to the server!"]}

wire = IrcProto.serialize(msg)
# => "NICK alice
"
```

## API

- `IrcProto.parse(line)` — `{:ok, Message.t()}` or `{:error, String.t()}`
- `IrcProto.serialize(msg)` — `String.t()` with CRLF terminator

## Message format (RFC 1459)

```
[:prefix] command [param1 param2 ... [:trailing]]

```

- Prefix is optional, identified by a leading `:`.
- Command is normalised to uppercase.
- The last param may start with `:` to include spaces.
- Maximum 15 params (RFC 1459 §2.3).
