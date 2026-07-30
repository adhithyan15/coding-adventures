# NET04 — HTTP/1 Head Parser

## Overview

Real HTTP/1 parsers are usually not built as a classic compiler pipeline with a
separate lexer and parser. They are usually **byte-oriented head parsers**:

1. find the end of the head
2. parse the start line
3. parse header lines
4. decide how the body should be read

That is the design this package follows.

`http1` reads raw bytes for a complete or partial HTTP/1 request/response head
and returns:

- a semantic `RequestHead` or `ResponseHead` from `http-core`
- the byte offset where the body begins
- a `BodyKind` describing how the caller should read the payload

This package does **not** parse HTML, JSON, or image content. It only parses
the HTTP/1 head and determines payload framing.

## Where It Fits

```text
tcp-client / buffered stream / frame source
  ↓ raw bytes
http1
  ↓ (head, body_offset, body_kind)
application code or http client
  ↓
HTML parser / JSON parser / image decoder / plain text renderer
```

## Concepts

### What the Parser Owns

HTTP/1 parsing owns:

- request line parsing
- status line parsing
- header parsing
- end-of-head detection
- body framing decisions

It does **not** own:

- HTML parsing
- gzip decoding
- image decoding
- application-specific semantics

### Head Parsing

The parser first finds the blank line that ends the head:

```text
GET /index.html HTTP/1.0\r\n
Host: example.com\r\n
User-Agent: Venture/0.1\r\n
\r\n
<body starts here>
```

Both `\r\n\r\n` and bare `\n\n` are accepted.

The parser returns the byte offset immediately after the blank line so the
caller knows where the body begins.

Head parsing is bounded before any byte-to-string conversion:

- at most 65,536 bytes through the terminating blank line
- at most 8,192 bytes in one start or field line
- at most 100 field lines
- at most 16 comma-separated transfer codings

Crossing a limit is a typed parse failure. Error values identify only the
failure class; they never retain raw request targets, reason phrases, or field
lines that a caller might later write to a log.

Request lines use exactly two single-space delimiters. Status lines use a
single space between version and the exactly three-digit status code, followed
by either no reason phrase or one space and the reason phrase. Tabs and
variable whitespace are not accepted as structural delimiters.

Field names must be non-empty RFC token bytes with no whitespace before the
colon. Field values trim only leading and trailing SP/HTAB and reject embedded
control bytes and obsolete line folding.

### Request Body Rules

For requests:

```text
final Transfer-Encoding: chunked  → BodyKind::Chunked
Content-Length: N                 → BodyKind::ContentLength(N) if N > 0
Content-Length: 0                 → BodyKind::None
Otherwise                         → BodyKind::None
```

A request with both `Transfer-Encoding` and `Content-Length` is rejected.
Transfer coding is rejected on HTTP/1.0, `chunked` must occur exactly once as
the final coding, and empty or malformed coding lists are rejected.

All `Content-Length` field values are parsed. Duplicate fields and
comma-coalesced values are accepted only when every bounded decimal value is
identical; conflicting or malformed values are rejected.

### Response Body Rules

For responses:

```text
response to HEAD                         → BodyKind::None
successful response to CONNECT           → tunnel after the head
1xx, 204, 304                            → BodyKind::None
final Transfer-Encoding: chunked         → BodyKind::Chunked
non-chunked Transfer-Encoding            → BodyKind::UntilEof
Content-Length: N                        → BodyKind::ContentLength(N) if N > 0
Content-Length: 0                        → BodyKind::None
Otherwise                                → BodyKind::UntilEof
```

Response parsing requires the corresponding request method so HEAD and CONNECT
semantics cannot be guessed. A successful CONNECT response reports the tunnel
transition separately from `BodyKind`. Outside the bodyless/tunnel cases,
`Transfer-Encoding` plus `Content-Length` is rejected. `chunked` must not be
repeated or followed by another transfer coding.

This is the core distinction between “header parsing” and “body parsing” in
HTTP/1: the parser usually does not consume the body, but it **does** decide
how the body must be consumed.

### Incremental-Friendly Shape

Even if the first implementation works on a complete in-memory buffer, the API
should already look like a real HTTP parser API:

```text
ParsedRequestHead {
  head,
  body_offset,
  body_kind,
}
```

That means later incremental parsers can preserve the same output shape while
changing only the internal mechanics.

## Public API

```rust
pub struct ParsedRequestHead {
    pub head: RequestHead,
    pub body_offset: usize,
    pub body_kind: BodyKind,
}

pub struct ParsedResponseHead {
    pub head: ResponseHead,
    pub body_offset: usize,
    pub body_kind: BodyKind,
    pub switches_protocol: bool,
}

pub enum Http1ParseError {
    IncompleteHead,
    HeadTooLarge,
    LineTooLong,
    TooManyHeaders,
    TooManyTransferCodings,
    InvalidStartLine,
    InvalidVersion,
    InvalidStatusCode,
    InvalidHeaderLine,
    InvalidContentLength,
    InvalidTransferEncoding,
    AmbiguousFraming,
}

pub fn parse_request_head(input: &[u8]) -> Result<ParsedRequestHead, Http1ParseError>;
pub fn parse_response_head(
    request_method: &str,
    input: &[u8],
) -> Result<ParsedResponseHead, Http1ParseError>;
```

Equivalent APIs should exist in all supported languages.

## Algorithm

### Request

1. Skip leading blank lines.
2. Find the first non-empty line.
3. Parse the strict request line as:
   - method
   - target
   - version
4. Parse headers line by line until an empty line.
5. Validate every framing field and determine `BodyKind`.
6. Return the parsed head plus `body_offset`.

### Response

1. Skip leading blank lines.
2. Parse the strict status line as:
   - version
   - status code
   - reason phrase
3. Parse headers line by line until an empty line.
4. Validate every framing field and determine `BodyKind` using the request
   method.
5. Return the parsed head plus `body_offset`.

## Testing Strategy

1. Parse a simple GET request.
2. Parse a POST request with `Content-Length`.
3. Parse a 200 OK response with `Content-Length`.
4. Parse a response without `Content-Length` and return `UntilEof`.
5. Parse `204 No Content` and return `BodyKind::None`.
6. Parse `304 Not Modified` and return `BodyKind::None`.
7. Preserve duplicate headers.
8. Accept bare LF line endings.
9. Trim optional whitespace after the colon.
10. Preserve colons inside header values.
11. Reject malformed start lines.
12. Reject malformed headers with no colon.
13. Reject malformed `Content-Length`.
14. Reject whitespace before a field colon and invalid field-name/control
    bytes.
15. Reject transfer-encoding/content-length ambiguity and non-final or repeated
    `chunked` request coding.
16. Accept repeated content lengths only when every value is identical.
17. Frame HEAD responses as bodyless and successful CONNECT responses as
    tunnels.
18. Enforce head, line, field-count, and transfer-coding bounds.
19. Prove typed errors cannot expose raw request targets or field values.

## Scope

**In scope:**

- request-line parsing
- status-line parsing
- header parsing
- body-offset detection
- HTTP/1 body-kind detection

**Out of scope:**

- incremental socket reader state machines
- chunk body decoding
- gzip/deflate decoding
- HTML parsing
- HTTP/2 and HTTP/3 parsing

## Implementation Languages

This package will be implemented in:

- C
- C#
- Python
- F#
- Go
- Haskell
- Ruby
- TypeScript
- Rust
- Elixir
- Perl
- Lua
- Swift
