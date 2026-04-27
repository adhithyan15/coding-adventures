# irc-proto (Go)

Pure IRC message parsing and serialisation (RFC 1459).

## Overview

`irc-proto` is the lowest layer of the IRC stack. It converts between raw IRC
wire-format strings and structured `Message` values. It has no dependencies and
no side effects.

## API

```go
// Parse a CRLF-stripped IRC line into a Message.
msg, err := irc_proto.Parse(":irc.local 001 alice :Welcome!")

// Serialise a Message back to wire format (CRLF-terminated bytes).
data := irc_proto.Serialize(&irc_proto.Message{
    Prefix:  "irc.local",
    Command: "PING",
    Params:  []string{"token"},
})
```

## Message structure

```
:prefix COMMAND param1 param2 :trailing param with spaces
```

- `Prefix` — server or nick!user@host (omitted for client-originated messages).
- `Command` — always uppercased (e.g. `NICK`, `001`).
- `Params` — up to 15 parameters; the trailing param (prefixed with `:`) may
  contain spaces.

## Coverage

97.7% statement coverage.
