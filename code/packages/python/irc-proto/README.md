# irc-proto

`irc-proto` is a pure IRC message parsing and serialization library for Python. It converts
between the raw text lines of the IRC wire protocol and structured `Message` values — with no
I/O, no threads, and no side effects. Given a CRLF-stripped line of text, `parse()` returns a
`Message`; given a `Message`, `serialize()` returns CRLF-terminated bytes ready for the socket.

## Where it fits

`irc-proto` sits at the base of the IRC stack in coding-adventures. Every other IRC package
depends on its `Message` type, but `irc-proto` itself depends on nothing:

```
irc-server   ← applies IRC logic; emits/consumes Messages
     ↑
irc-client   ← user-facing session layer
     ↑
irc-proto    ← THIS PACKAGE: parse(line) → Message, serialize(msg) → bytes
     ↑
irc-framing  ← reads raw bytes; hands CRLF-stripped lines upward
```

See the full architecture spec at [`../../../specs/irc-proto.md`](../../../specs/irc-proto.md).

## Usage

```python
from irc_proto import Message, ParseError, parse, serialize

# Parse an incoming line (CRLF already stripped by the framer)
msg = parse(":alice!alice@host PRIVMSG #general :hello world")
print(msg.prefix)   # "alice!alice@host"
print(msg.command)  # "PRIVMSG"
print(msg.params)   # ["#general", "hello world"]

# Build and serialize an outgoing message
reply = Message(prefix=None, command="PRIVMSG", params=["#general", "Hi there!"])
wire = serialize(reply)   # b"PRIVMSG #general :Hi there!\r\n"

# Handle malformed input
try:
    parse("")
except ParseError as exc:
    print(f"Bad line: {exc}")
```

## Development

```bash
# Run tests (from the package directory)
bash BUILD
```

All tests live in `tests/test_irc_proto.py`. Coverage target: 95%+.
