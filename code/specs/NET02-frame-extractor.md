# NET02 — Frame Extractor

## Overview

TCP gives you a **byte stream** — a continuous flow of bytes with no message
boundaries. But protocols need **discrete messages**. HTTP needs to know where
headers end and the body begins. IRC needs to know where one command ends and the
next starts. RESP (the Redis protocol) needs to know how many bytes a bulk
string contains.

This package solves the fundamental problem: **how do you carve messages out of
a continuous byte stream?**

**Analogy:** TCP is like a garden hose — water flows continuously with no
natural divisions. But your protocol needs discrete cups of water. The frame
extractor is the **cup** — it collects bytes from the stream until a complete
message is ready, then hands you that message and starts collecting the next
one.

```
Garden Hose (TCP byte stream):
  ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~>

Frame Extractor (the cup):
  [cup 1: "GET / HTTP/1.0\r\n\r\n"]  [cup 2: "Hello, World!"]  [cup 3: ...]
```

Different protocols use different rules for where one message ends and the next
begins. This package provides **four strategies** for frame extraction, each
matching a common pattern found across network protocols:

| Strategy          | Rule                          | Used By                          |
|-------------------|-------------------------------|----------------------------------|
| Delimiter         | Read until a byte pattern     | IRC (`\r\n`), HTTP headers       |
| Length-Prefixed   | Read exactly N bytes          | HTTP body, WebSocket, RESP       |
| Read-to-End       | Read until EOF                | HTTP/1.0 body (no Content-Length)|
| Composite         | Chain strategies sequentially | HTTP response (headers + body)   |

The IRC framing package (`irc-framing`) already solves this problem for IRC
specifically — it uses CRLF delimiters with a 512-byte maximum. This package is
the **generic** version that works for any framing protocol. Think of
`irc-framing` as a specialized cup (exactly 512ml, CRLF-shaped) and this package
as a cup factory that can produce any size and shape.

---

## Where It Fits

```
tcp-client (NET01) → frame-extractor (NET02) → http1.0-lexer (NET03)
                          ↑
                      THIS PACKAGE

The flow for an HTTP/1.0 request:

  1. tcp-client opens a socket to the server
  2. Server sends back a stream of bytes
  3. frame-extractor carves that stream into:
     a. The header block (everything up to \r\n\r\n)
     b. The body (next Content-Length bytes, or read-to-end)
  4. http1.0-lexer parses the headers into structured data
```

**Depends on:** nothing (std only — operates on `impl BufRead`)
**Depended on by:** http1.0-client (NET05), Venture browser (BR01)
**Also reusable by:** RESP protocol (DT23), WebSocket framing, any line protocol

---

## Concepts

### What Is a "Frame"?

A **frame** is the protocol's unit of data. Different protocols call it
different things — a "message," a "packet," a "line," a "chunk" — but the idea
is the same: a contiguous sequence of bytes that carries one logical unit of
meaning.

```
Protocol        Frame                          Boundary Rule
──────────────────────────────────────────────────────────────
IRC             "PRIVMSG #ch :hello\r\n"       CRLF delimiter, max 512 bytes
HTTP headers    "HTTP/1.0 200 OK\r\n...\r\n"   Double-CRLF delimiter
HTTP body       "<html>...</html>"              Content-Length bytes, or EOF
RESP            "$5\r\nhello\r\n"               Length prefix (the "5")
```

### The Three Fundamental Framing Patterns

Every protocol in existence uses one of three patterns (or a combination):

#### Pattern 1: Delimiter-Based

Read bytes until you see a **sentinel value** — a special byte sequence that
means "end of message." The delimiter itself is never part of the message
content (or if it is, the protocol has an escaping mechanism).

```
Bytes on the wire:     H E L L O \r \n W O R L D \r \n
                       ─────────────── ───────────────
                        Frame 1          Frame 2
                              ↑ delimiter      ↑ delimiter
```

**Used by:** IRC, SMTP, HTTP status lines, HTTP headers, POP3, FTP control
channel, syslog, and virtually every "line protocol."

**Danger:** What if the delimiter never arrives? A malicious or buggy sender
could transmit gigabytes without ever sending `\r\n`. That is why delimiter
strategies need a **max_length** safety limit — if we have read more than N
bytes without finding the delimiter, we stop and return an error.

#### Pattern 2: Length-Prefixed

The sender tells you **in advance** how many bytes the message contains. You
read exactly that many bytes and you are done.

```
HTTP Response:
  "Content-Length: 13\r\n\r\n"   ← header (extracted by delimiter strategy)
  "Hello, World!"                ← body: exactly 13 bytes
  ─────────────── 
   Frame (13 bytes)
```

This is the most efficient pattern — no scanning, no ambiguity. You know
exactly when the message ends before you start reading it.

**Used by:** HTTP body (Content-Length), WebSocket payload, RESP bulk strings,
binary protocols with fixed headers.

#### Pattern 3: Read-to-End (EOF)

The simplest possible strategy: read everything until the connection closes.
There is no delimiter, no length prefix — the message boundary IS the
connection boundary.

```
Server sends:    <html><body>Hello</body></html>
Server closes:   [TCP FIN]
Client:          "The message is everything I received."
```

This was common in HTTP/1.0 when servers did not send a Content-Length header.
The client simply read until the server closed the connection. The obvious
downside: **you cannot send multiple messages** on the same connection, because
the only way to signal "end of message" is to close the connection entirely.

**Used by:** HTTP/1.0 body (when no Content-Length), some file transfer
protocols.

### Composing Strategies: The HTTP Example

Real protocols often use **multiple framing strategies** in sequence. HTTP is
the perfect example:

```
HTTP/1.0 200 OK\r\n                    ┐
Content-Type: text/html\r\n            │ Phase 1: DelimiterStrategy("\r\n\r\n")
Content-Length: 45\r\n                  │   → extracts the header block
\r\n                                    ┘
<html><body>Hello, World!</body></html>   Phase 2: LengthPrefixedStrategy(45)
                                            → extracts the body (exactly 45 bytes)
```

The **CompositeStrategy** chains these two steps:

1. **Phase 1:** Use `DelimiterStrategy` with `\r\n\r\n` to extract the raw
   header block. This gives us a `Vec<u8>` containing all the headers.

2. **Parse headers:** The caller (not the frame extractor) parses the header
   block to find the `Content-Length` value. This is protocol-specific logic
   that does not belong in the generic framing library.

3. **Phase 2:** Use `LengthPrefixedStrategy(content_length)` to extract exactly
   that many bytes for the body. If there is no Content-Length header, fall back
   to `ReadToEndStrategy` — read until the server closes the connection.

```
fn extract_http_response(reader: &mut dyn BufRead) -> Result<(Vec<u8>, Vec<u8>), FrameError> {
    // Phase 1: Extract headers
    let mut header_strategy = DelimiterStrategy {
        delimiter: b"\r\n\r\n".to_vec(),
        max_length: Some(8192),   // 8 KB header limit (Apache default)
        include_delimiter: false,
    };
    let headers = header_strategy.extract(reader)?;

    // Phase 2: Parse Content-Length (protocol-specific, done by caller)
    let content_length = parse_content_length(&headers);

    // Phase 3: Extract body
    let body = match content_length {
        Some(len) => {
            let mut body_strategy = LengthPrefixedStrategy { length: len };
            body_strategy.extract(reader)?
        }
        None => {
            let mut eof_strategy = ReadToEndStrategy { max_length: Some(1_048_576) };
            eof_strategy.extract(reader)?
        }
    };

    Ok((headers, body))
}
```

This composition pattern is the reason the frame extractor exists as a generic
library rather than being hardcoded into each protocol implementation.

---

## Public API

```rust
use std::io::BufRead;

// ─────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────

/// Everything that can go wrong when extracting a frame.
///
/// These errors distinguish between "the data is wrong" (protocol violation)
/// and "the pipe is broken" (I/O failure). This matters because protocol
/// violations might be recoverable (skip and try the next frame) while I/O
/// failures usually are not.
pub enum FrameError {
    /// We read max_length bytes without finding the delimiter.
    ///
    /// This usually means either:
    /// - The sender is not following the protocol (missing CRLF)
    /// - The sender is malicious (trying to exhaust our memory)
    /// - max_length is set too low for the protocol
    DelimiterNotFound,

    /// The connection closed before we got a complete frame.
    ///
    /// For LengthPrefixedStrategy: we expected N bytes but got fewer.
    /// For DelimiterStrategy: the stream ended without the delimiter.
    UnexpectedEof,

    /// The frame exceeded the configured maximum length.
    ///
    /// Used by ReadToEndStrategy when a max_length limit is set.
    MaxLengthExceeded,

    /// An underlying I/O error occurred (disk failure, network error, etc.)
    IoError(std::io::Error),
}

// ─────────────────────────────────────────────
// Trait
// ─────────────────────────────────────────────

/// A strategy for extracting one complete frame from a byte stream.
///
/// Each call to `extract()` reads exactly one frame and returns it as a
/// `Vec<u8>`. The reader's position is advanced past the frame (and past
/// any delimiter), so the next call to `extract()` — on this or any other
/// strategy — picks up right where the last one left off.
///
/// This is the core abstraction. By programming against `dyn FrameStrategy`,
/// protocol implementations can swap framing strategies without changing
/// their parsing logic.
pub trait FrameStrategy {
    /// Read one complete frame from the stream.
    ///
    /// Blocks until a complete frame is available, an error occurs, or EOF
    /// is reached (which may or may not be an error depending on the
    /// strategy).
    fn extract(&mut self, reader: &mut dyn BufRead) -> Result<Vec<u8>, FrameError>;
}

// ─────────────────────────────────────────────
// Strategy 1: Delimiter
// ─────────────────────────────────────────────

/// Extract a frame by reading until a delimiter byte sequence is found.
///
/// This is the most common framing strategy. Line protocols use `\r\n`,
/// HTTP uses `\r\n\r\n` to separate headers from body, and some binary
/// protocols use null bytes.
///
/// ## Example
///
/// ```rust
/// use std::io::Cursor;
///
/// let data = b"HELLO\r\nWORLD\r\n";
/// let mut reader = Cursor::new(data.to_vec());
///
/// let mut strategy = DelimiterStrategy {
///     delimiter: b"\r\n".to_vec(),
///     max_length: Some(512),
///     include_delimiter: false,
/// };
///
/// let frame1 = strategy.extract(&mut reader).unwrap();
/// assert_eq!(frame1, b"HELLO");
///
/// let frame2 = strategy.extract(&mut reader).unwrap();
/// assert_eq!(frame2, b"WORLD");
/// ```
pub struct DelimiterStrategy {
    /// The byte sequence that marks the end of a frame.
    ///
    /// Common values:
    /// - `b"\r\n".to_vec()`       — line protocols (IRC, SMTP)
    /// - `b"\r\n\r\n".to_vec()`   — HTTP header/body boundary
    /// - `b"\0".to_vec()`          — null-terminated protocols
    pub delimiter: Vec<u8>,

    /// Maximum number of bytes to read before giving up.
    ///
    /// If `None`, there is no limit (dangerous for untrusted input!).
    /// If `Some(n)`, returns `FrameError::DelimiterNotFound` after reading
    /// `n` bytes without finding the delimiter.
    pub max_length: Option<usize>,

    /// Whether to include the delimiter in the returned frame.
    ///
    /// - `false` (typical): "HELLO\r\n" → returns b"HELLO"
    /// - `true`  (rare):    "HELLO\r\n" → returns b"HELLO\r\n"
    ///
    /// The delimiter is consumed from the stream in both cases — setting
    /// this to `false` just strips it from the output.
    pub include_delimiter: bool,
}

impl FrameStrategy for DelimiterStrategy { ... }

// ─────────────────────────────────────────────
// Strategy 2: Length-Prefixed
// ─────────────────────────────────────────────

/// Extract a frame by reading exactly `length` bytes.
///
/// The caller already knows how many bytes to read — typically from a
/// previously parsed header (like HTTP's Content-Length) or a protocol-
/// defined length field.
///
/// ## Example
///
/// ```rust
/// use std::io::Cursor;
///
/// let data = b"Hello, World!Extra bytes";
/// let mut reader = Cursor::new(data.to_vec());
///
/// let mut strategy = LengthPrefixedStrategy { length: 13 };
///
/// let frame = strategy.extract(&mut reader).unwrap();
/// assert_eq!(frame, b"Hello, World!");
/// // Reader is now positioned at "Extra bytes"
/// ```
pub struct LengthPrefixedStrategy {
    /// Exact number of bytes to read.
    ///
    /// Returns `FrameError::UnexpectedEof` if the stream ends before
    /// `length` bytes have been read.
    pub length: usize,
}

impl FrameStrategy for LengthPrefixedStrategy { ... }

// ─────────────────────────────────────────────
// Strategy 3: Read-to-End
// ─────────────────────────────────────────────

/// Extract a frame by reading all remaining bytes until EOF.
///
/// This is the simplest strategy — the message boundary is the connection
/// boundary. Used when the sender closes the connection to signal "end of
/// message."
///
/// ## Example
///
/// ```rust
/// use std::io::Cursor;
///
/// let data = b"<html><body>Hello</body></html>";
/// let mut reader = Cursor::new(data.to_vec());
///
/// let mut strategy = ReadToEndStrategy { max_length: Some(1_048_576) };
///
/// let frame = strategy.extract(&mut reader).unwrap();
/// assert_eq!(frame, b"<html><body>Hello</body></html>");
/// ```
pub struct ReadToEndStrategy {
    /// Maximum number of bytes to read before returning an error.
    ///
    /// If `None`, there is no limit (dangerous for untrusted input!).
    /// If `Some(n)`, returns `FrameError::MaxLengthExceeded` after reading
    /// `n` bytes without reaching EOF.
    ///
    /// A typical default: 1 MB (`1_048_576`) for HTTP bodies, or 10 MB for
    /// file downloads.
    pub max_length: Option<usize>,
}

impl FrameStrategy for ReadToEndStrategy { ... }

// ─────────────────────────────────────────────
// Strategy 4: Composite
// ─────────────────────────────────────────────

/// Chain multiple strategies sequentially, extracting one frame per strategy.
///
/// Each strategy in the list extracts one frame from the stream, in order.
/// The returned `Vec<Vec<u8>>` contains one entry per strategy.
///
/// This is how multi-phase protocols like HTTP work:
/// 1. DelimiterStrategy("\r\n\r\n") extracts the header block
/// 2. LengthPrefixedStrategy(content_length) extracts the body
///
/// ## Example
///
/// ```rust
/// let mut composite = CompositeStrategy {
///     strategies: vec![
///         Box::new(DelimiterStrategy {
///             delimiter: b"\r\n\r\n".to_vec(),
///             max_length: Some(8192),
///             include_delimiter: false,
///         }),
///         Box::new(LengthPrefixedStrategy { length: 13 }),
///     ],
/// };
///
/// let frames = composite.extract_all(&mut reader)?;
/// // frames[0] = header block
/// // frames[1] = body (13 bytes)
/// ```
pub struct CompositeStrategy {
    /// The strategies to apply in order.
    ///
    /// Each strategy is consumed once: the first extracts one frame, then
    /// the second extracts the next frame from wherever the first left off.
    strategies: Vec<Box<dyn FrameStrategy>>,
}

impl CompositeStrategy {
    /// Extract one frame per strategy, returning all frames.
    ///
    /// If any strategy fails, the error is returned immediately and no
    /// further strategies are attempted.
    pub fn extract_all(&mut self, reader: &mut dyn BufRead) -> Result<Vec<Vec<u8>>, FrameError> {
        ...
    }
}
```

### Design Decisions

**Why `BufRead` and not `Read`?** Delimiter-based framing requires peeking at
bytes without consuming them. `BufRead` provides `fill_buf()` for exactly this.
If we took a raw `Read`, we would need our own internal buffer — duplicating
what `BufReader` already provides. By accepting `BufRead`, we let the caller
choose the buffer size and we stay simple.

**Why `Vec<u8>` and not `&[u8]`?** The extracted frame must outlive the read
buffer. When using a `BufReader`, the internal buffer may be reused for the next
read, so we cannot return a borrow into it. Owned `Vec<u8>` is the safe choice.

**Why mutable strategies?** A `CompositeStrategy` consumes its inner strategies
as it runs. Even simple strategies might want internal scratch buffers in the
future. `&mut self` keeps that door open.

---

## Testing Strategy

All tests use in-memory `Cursor<Vec<u8>>` buffers to simulate TCP streams.
No actual network I/O is needed.

### DelimiterStrategy Tests

1. **Single frame, CRLF:** `b"HELLO\r\n"` → extracts `b"HELLO"`
2. **Multiple frames:** `b"A\r\nB\r\nC\r\n"` → three calls yield A, B, C
3. **Include delimiter:** same input with `include_delimiter: true` → `b"HELLO\r\n"`
4. **Multi-byte delimiter:** `b"headers\r\n\r\nbody"` with `\r\n\r\n` → `b"headers"`
5. **Null delimiter:** `b"hello\0world\0"` with `\0` → `b"hello"`, `b"world"`
6. **Max length exceeded:** 1000 bytes with no delimiter, `max_length: 512` → `DelimiterNotFound`
7. **Max length exact fit:** delimiter appears at exactly max_length → success
8. **Empty frame:** `b"\r\n"` → extracts `b""` (empty vec)
9. **Delimiter at start:** `b"\r\nHELLO\r\n"` → first frame is empty, second is "HELLO"
10. **EOF without delimiter:** `b"HELLO"` (no trailing CRLF) → `UnexpectedEof`
11. **Empty input:** empty buffer → `UnexpectedEof`

### LengthPrefixedStrategy Tests

12. **Exact read:** 13 bytes from a 13-byte buffer → success
13. **Read from longer buffer:** 5 bytes from a 100-byte buffer → only first 5 bytes returned, reader advances by 5
14. **Unexpected EOF:** request 100 bytes from 50-byte buffer → `UnexpectedEof`
15. **Zero-length:** `length: 0` → returns empty `Vec<u8>`, reader does not advance
16. **Sequential reads:** two `LengthPrefixedStrategy` calls (5 bytes, then 3 bytes) from an 8-byte buffer

### ReadToEndStrategy Tests

17. **Read all:** 100-byte buffer, no max → returns all 100 bytes
18. **Max length enforced:** 100-byte buffer, `max_length: 50` → `MaxLengthExceeded`
19. **Empty input:** empty buffer → returns empty `Vec<u8>` (EOF is not an error for this strategy)
20. **Max length with exact fit:** 50-byte buffer, `max_length: 50` → success (boundary condition)

### CompositeStrategy Tests

21. **HTTP-style:** delimiter + length-prefixed → two frames (headers, body)
22. **Triple chain:** three delimiter strategies on three-line input → three frames
23. **First strategy fails:** first strategy errors → returns error, second never runs
24. **Second strategy fails:** first succeeds, second encounters unexpected EOF → error

### Partial Feed Tests (Simulating Chunked TCP)

Real TCP streams do not deliver all bytes at once. Data arrives in unpredictable
chunks. These tests use a custom `SlowReader` that yields a few bytes per
`read()` call to ensure strategies handle partial reads correctly.

25. **Delimiter across chunks:** delimiter `\r\n` split across two reads (`"HEL"` then `"LO\r\nWORLD"`)
26. **Length-prefixed across chunks:** 10 bytes requested, delivered in 3 + 4 + 3 byte reads
27. **Read-to-end across chunks:** multiple small reads before EOF

### Edge Cases

28. **Very long delimiter:** 16-byte delimiter (unusual but valid)
29. **Binary data:** frames containing null bytes, high bytes (0xFF), all byte values
30. **Large frame:** 1 MB frame to verify no artificial internal limits beyond max_length
31. **Overlapping delimiter patterns:** data containing partial delimiter matches (e.g., `\r` without `\n` when delimiter is `\r\n`)

---

## Scope

**In scope:**
- `DelimiterStrategy` — read until a byte pattern appears
- `LengthPrefixedStrategy` — read exactly N bytes
- `ReadToEndStrategy` — read all bytes until EOF
- `CompositeStrategy` — chain strategies sequentially
- `FrameError` error types with descriptive variants
- Safety limits (max_length) to prevent unbounded memory use
- Operates on `impl BufRead` for composability

**Out of scope:**
- **Stateful buffering** — The `irc-framing` package uses a `feed(bytes)` /
  `frames()` pattern with an internal buffer that accumulates bytes across
  multiple feeds. That is needed for event-driven / async architectures. This
  package uses synchronous blocking reads from `BufRead` — simpler, and
  sufficient for our HTTP/1.0 client.
- **Async I/O** — No `tokio`, no `async-std`, no futures. This package is
  purely synchronous. An async version could wrap these strategies behind
  `AsyncBufRead`, but that is a separate package.
- **Protocol-specific parsing** — This package extracts raw byte frames. It
  does NOT parse HTTP headers, decode WebSocket opcodes, or interpret RESP type
  prefixes. That is the job of the protocol layer above (NET03, etc.).
- **WebSocket frame decoding** — WebSocket has its own binary framing format
  (opcode, mask bit, payload length in 1/2/8 bytes). Decoding that format is
  protocol-specific parsing, not generic framing.
- **Chunked transfer encoding** — HTTP/1.1's chunked encoding is a complex
  stateful protocol on top of framing. Out of scope for this package and for
  our HTTP/1.0 client.

---

## Implementation Languages

This package will be implemented in:
- **Rust** (primary, for the Venture browser networking stack)
- Future: all 9 languages following the standard pattern
