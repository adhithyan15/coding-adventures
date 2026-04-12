# irc-framing

Stateful byte-stream-to-line-frame converter for IRC connections.

## The TCP byte-stream problem

TCP is a byte stream, not a message stream.  When you call `recv()` on a
socket, the operating system hands back however many bytes happen to be
available — which may be half a message, one message, or three messages and
the start of a fourth.  IRC defines `\r\n` (carriage return + line feed) as
its message boundary.  The `Framer` class absorbs raw byte chunks into an
internal buffer and yields complete, `\r\n`-stripped lines so that the parser
layer (`irc-proto`) never has to deal with partial data.

## Usage

```python
from irc_framing import Framer

framer = Framer()

# Inside your receive loop:
while True:
    data = conn.read(4096)   # raw bytes from the socket
    if not data:
        break
    framer.feed(data)
    for line in framer.frames():
        # line is a complete bytes object with \r\n stripped
        handle(line.decode("utf-8", errors="replace"))
```

## API summary

| Method / Property | Description |
|---|---|
| `feed(data: bytes) -> None` | Append raw socket bytes to the buffer. |
| `frames() -> Iterator[bytes]` | Yield all complete lines currently in the buffer. |
| `reset() -> None` | Discard all buffered data (call on reconnect). |
| `buffer_size: int` | Number of bytes currently held in the buffer. |

Lines exceeding 510 bytes of content (RFC 1459 maximum of 512 including CRLF)
are silently discarded.  Bare `\n` (LF-only) terminators are also accepted for
compatibility with simple bots and clients.

## Specification

See [`../../../specs/irc-framing.md`](../../../specs/irc-framing.md) for the
full design rationale, layer diagram, and test strategy.

## Development

```bash
# Run tests (from this directory)
bash BUILD
```
