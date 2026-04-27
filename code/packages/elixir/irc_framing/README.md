# irc_framing (Elixir)

TCP byte-stream framing for the IRC protocol.

## Overview

`irc_framing` is the second layer of the IRC stack. IRC messages are
CRLF-terminated text lines. TCP is a byte-stream protocol with no concept of
message boundaries. This package bridges that gap: it buffers incoming bytes
and emits complete lines.

## Usage

```elixir
alias CodingAdventures.IrcFraming.Framer

# Create a new framer with an empty buffer.
framer = Framer.new()

# Feed raw bytes from the TCP socket.
framer = Framer.feed(framer, "NICK alice\r\nUSER alice 0 *")

# Extract complete lines (CRLF-terminated).
{framer, lines} = Framer.frames(framer)
# lines => ["NICK alice"]
# framer now holds "USER alice 0 *" as the incomplete tail

# More bytes arrive to complete the message.
framer = Framer.feed(framer, " :Alice\r\n")
{_framer, lines} = Framer.frames(framer)
# lines => ["USER alice 0 * :Alice"]
```

## Facade

The top-level `CodingAdventures.IrcFraming` module delegates to `Framer`:

```elixir
alias CodingAdventures.IrcFraming

framer = IrcFraming.new()
framer = IrcFraming.feed(framer, "PING :server\r\n")
{framer, lines} = IrcFraming.frames(framer)
```

## Design

- Lines may end with `\r\n` (CRLF) or bare `\n` (LF). Both are accepted.
- Lines longer than 510 bytes (RFC 1459 limit) are silently discarded.
- The framer struct is immutable and purely functional — no processes,
  no I/O, no side effects.
- `feed/2` returns a new framer with the buffered bytes appended.
- `frames/1` returns `{new_framer, [line]}` — the framer with the remaining
  partial data, and a list of complete line strings (without the CRLF suffix).

## In the Stack

    ircd (program)
      |
      +-- irc_net_stdlib   <-- TCP event loop
      |
      +-- irc_server       <-- pure IRC state machine
      |
      +-- irc_framing      <-- this package (CRLF framing)
      |
      +-- irc_proto        <-- message parsing

## Dependencies

- `irc_proto` — for the `Message` struct (transitively required).
