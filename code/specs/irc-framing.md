# irc-framing — Byte Stream to Line Frames

## Overview

`irc-framing` solves a fundamental TCP problem: TCP is a **byte stream**, not a message stream.
When you call `recv()` on a socket, you may receive half a message, one full message, or three
messages and the start of a fourth. There is no inherent concept of a "message boundary" at the
TCP layer.

IRC defines its own message boundary: every message ends with `\r\n` (carriage return + line
feed). The `Framer` holds an internal byte buffer, accepts raw bytes as they arrive, and yields
complete `\r\n`-terminated lines one at a time. The parser (`irc-proto`) never has to deal with
partial data.

This package is pure: no sockets, no threads, no I/O. It is a stateful buffer transformer.

---

## Layer Position

```
irc-proto   ← receives complete, \r\n-stripped lines; calls parse()
     ↑
irc-framing ← THIS PACKAGE: feed(bytes) / frames() → Iterator[bytes]
     ↑
irc-net     ← calls conn.read() and feeds raw bytes upward
```

`irc-framing` knows nothing about IRC message structure. It only knows about `\r\n` boundaries
and the 512-byte maximum line length defined in RFC 1459.

---

## Concepts

### The Partial Read Problem

Consider a client sending:

```
NICK alice\r\nUSER alice 0 * :Alice Smith\r\n
```

The OS may deliver this in any of these chunks:

```
recv 1: b"NICK al"
recv 2: b"ice\r\nUSER alice 0 * :Ali"
recv 3: b"ce Smith\r\n"
```

After each `recv`, we `feed()` the bytes to the framer. After recv 1 and 2, no complete line is
available. After recv 3, two complete lines can be yielded:
- `b"NICK alice"`
- `b"USER alice 0 * :Alice Smith"`

### The Multi-Frame Read

The opposite case: a small receive buffer returns multiple complete lines at once. The framer
must yield them all, in order, before the next `feed()`.

### The 512-Byte Limit

RFC 1459 specifies a maximum line length of 512 bytes **including the CRLF**. Lines longer than
510 bytes of content must be truncated. The framer enforces this: if the internal buffer grows
beyond 512 bytes without encountering a `\r\n`, the accumulated bytes are discarded and the
framer resets its internal state for that line. This prevents memory exhaustion from malformed
or malicious clients.

### Buffer Strategy

The framer maintains a `bytearray` as its internal accumulation buffer. Appending bytes to a
`bytearray` is O(1) amortized. Scanning for `\r\n` and slicing out the frame is O(n) in the
frame length, which is bounded by 512.

A ring buffer would be more memory-efficient for high-throughput scenarios, but a `bytearray` is
correct, simple, and fast enough for IRC traffic volumes.

---

## Public API

```python
from __future__ import annotations

from collections.abc import Iterator


class Framer:
    """Stateful byte-stream-to-line-frame converter.

    Feed raw bytes from the socket using feed(). Then iterate frames() to
    get each complete \\r\\n-terminated line, with the \\r\\n stripped.

    The Framer is NOT thread-safe. Each connection should have its own Framer.

    Example usage:
        framer = Framer()
        while True:
            data = conn.read()
            if not data:
                break
            framer.feed(data)
            for line in framer.frames():
                msg = parse(line.decode('utf-8', errors='replace'))
                handle(msg)
    """

    def feed(self, data: bytes) -> None:
        """Append raw bytes to the internal buffer.

        Should be called immediately after each socket read.
        data may be any length, including zero bytes (which is a no-op).
        """
        ...

    def frames(self) -> Iterator[bytes]:
        """Yield complete lines from the buffer, with \\r\\n stripped.

        Yields zero or more complete frames accumulated since the last call.
        Partial lines (no \\r\\n yet) are held in the buffer until the next feed().
        Lines exceeding 510 bytes of content are discarded (not yielded).

        This method is a generator; it is safe to call it after every feed().
        """
        ...

    def reset(self) -> None:
        """Discard all buffered data.

        Call this when a connection is closed or restarted, to ensure no
        stale bytes from one connection bleed into the next.
        """
        ...

    @property
    def buffer_size(self) -> int:
        """Number of bytes currently held in the internal buffer.

        Useful for monitoring and testing.
        """
        ...
```

---

## Internal State Machine

The framer can be thought of as a simple state machine with two states:

```
        feed(data)
            │
            ▼
    ┌───────────────┐   found \r\n     ┌──────────────────────┐
    │  ACCUMULATING │ ──────────────→  │  YIELD frame, reset  │
    │  (waiting for │                  │  position to 0, loop │
    │   \r\n)       │ ←────────────────└──────────────────────┘
    └───────────────┘   more data in buffer
            │
            │ buffer_size >= 512, no \r\n found
            ▼
    ┌───────────────┐
    │  DISCARD line │  (reset buffer, continue)
    └───────────────┘
```

Implementation note: a single linear scan of the buffer looking for `b'\r\n'` (or just `b'\n'`
for leniency with clients that send only LF) is sufficient. RFC 1459 requires `\r\n` but many
clients only send `\n`. Accept both; strip both.

---

## Handling LF-only Clients

Some IRC clients (particularly simple bots) terminate lines with `\n` only rather than `\r\n`.
The framer should accept both:
- `\r\n` → strip both characters, yield frame
- `\n` (no preceding `\r`) → strip `\n`, yield frame
- A lone `\r` is not a frame boundary; it is treated as part of the data

---

## Test Strategy

Tests live in `tests/`. Coverage target: 98%+.

### Core frame extraction

- **Single complete frame**: `feed(b"NICK alice\r\n")` → `frames()` yields `b"NICK alice"`
- **Multiple frames in one feed**: `feed(b"NICK a\r\nUSER a 0 * :A\r\n")` → yields both lines
- **Split across feeds**: `feed(b"NICK al")` → no frames; `feed(b"ice\r\n")` → yields `b"NICK alice"`
- **Split CRLF across feeds**: `feed(b"NICK alice\r")` → no frames; `feed(b"\n")` → yields frame
- **Three feeds one frame**: `feed(b"NI")`, `feed(b"CK")`, `feed(b" a\r\n")` → `b"NICK a"`
- **Empty feed**: `feed(b"")` → `frames()` yields nothing
- **Multiple frames then partial**: `feed(b"A\r\nB\r\nC")` → yields `b"A"`, `b"B"`; `b"C"` held

### Max length enforcement

- **Exactly 510 bytes of content + CRLF**: yields the frame normally
- **511 bytes of content + CRLF**: frame is discarded, not yielded
- **512+ bytes no CRLF**: buffer grows to limit, discarded; next valid frame after yields normally
- **Two short frames after an overlong line**: overlong discarded; both short frames yielded

### LF-only handling

- `feed(b"NICK alice\n")` → yields `b"NICK alice"`
- `feed(b"NICK alice\r\n")` → yields `b"NICK alice"` (CRLF stripped, not just LF)

### Reset

- `feed(b"NICK al")` → `reset()` → `feed(b"USER a\r\n")` → only `b"USER a"` is yielded

### buffer_size property

- After `feed(b"hello")`, `buffer_size == 5`
- After `frames()` extracts a complete frame, `buffer_size` reflects only remaining bytes

---

## Future Extensions

- **Tagged message support (IRCv3)**: IRC tags can precede the `:prefix` and can be large. The
  512-byte limit applies only to the non-tag portion. A future `TaggedFramer` would parse the
  tag section separately and apply the 8192-byte tag limit defined by IRCv3.
- **Zero-copy variant**: for very high throughput, a ring buffer implementation would avoid
  copying bytes when yielding frames. The API would yield `memoryview` slices rather than `bytes`.
- **Metrics hook**: an optional callback `on_overlong(size: int) -> None` for monitoring.
