# NET03 — HTTP/1.0 Lexer

## Overview

HTTP/1.0 is a **text-based protocol** — requests and responses are plain ASCII
text with a rigid but simple structure. Before we can understand what an HTTP
message *means* (is this a redirect? does it have a JSON body?), we need to
break the raw bytes into meaningful pieces. That is the lexer's job.

**Analogy:** Imagine you receive a letter in a foreign language. Before you can
translate it, you need to identify the individual words, the punctuation marks,
the paragraph breaks. You don't need to understand the meaning yet — you just
need to know where one word ends and the next begins. The HTTP lexer does
exactly this: it takes a stream of bytes and identifies the *tokens* — method,
URI, version, status code, headers, body — without interpreting their meaning.

```
Raw bytes on the wire:              What the lexer sees:
                                    
GET /index.html HTTP/1.0\r\n       [Method: "GET"]
Host: www.example.com\r\n          [RequestUri: "/index.html"]
User-Agent: Venture/0.1\r\n        [Version: "HTTP/1.0"]
\r\n                                [HeaderName: "Host"]
                                    [HeaderValue: "www.example.com"]
                                    [HeaderName: "User-Agent"]
                                    [HeaderValue: "Venture/0.1"]
                                    [Body: (empty)]
                                    [Eof]
```

### Historical Context

HTTP was invented by **Tim Berners-Lee** at CERN in 1989, alongside HTML and
URLs — the three pillars of the World Wide Web. The first version, HTTP/0.9
(1991), was breathtakingly simple. The entire protocol was one line:

```
GET /path\r\n
```

The server responded with the raw HTML body and closed the connection. No
headers. No status codes. No content types. If you asked for a page and it
didn't exist, the server just closed the connection with no explanation.

**HTTP/1.0** (RFC 1945, May 1996) changed everything. It added:

- **Status codes** — the server tells you *what happened* (200 OK, 404 Not
  Found, 301 Moved Permanently)
- **Headers** — metadata about the request and response (Content-Type,
  Content-Length, User-Agent)
- **Methods beyond GET** — POST for sending data, HEAD for metadata-only
  requests
- **Version negotiation** — the client declares which HTTP version it speaks

This was the protocol that **Mosaic** and early **Netscape Navigator** used to
fetch web pages. Every page load was a separate TCP connection: connect, send
request, receive response, disconnect. Simple but expensive — a page with 10
images meant 11 TCP connections.

HTTP/1.0 is the sweet spot for learning: complex enough to be a real protocol,
simple enough to implement in a weekend. It has no chunked encoding, no
persistent connections, no trailers — just clean request-response pairs.

---

## Where It Fits

```
tcp-client (NET01) → frame-extractor (NET02) → http1.0-lexer (NET03) → http1.0-parser (NET04)
                                                     ↑
                                                 THIS PACKAGE

The flow for an HTTP/1.0 response:

  1. tcp-client opens a socket and sends a request
  2. Server sends back a stream of bytes
  3. frame-extractor carves the stream into a complete HTTP message
  4. http1.0-lexer tokenizes those bytes into structured tokens
  5. http1.0-parser (next layer) interprets the tokens into a typed
     Request/Response struct with semantic meaning
```

**Depends on:** Nothing. Operates on `&[u8]` — raw byte slices.

**Depended on by:** `http1.0-parser` (NET04), which consumes the token stream
and produces a typed HTTP request or response structure.

---

## Concepts

### The HTTP/1.0 Wire Format

HTTP messages are plain text (well, ASCII) with a very specific structure. Let's
trace through a full request and response to see every byte.

#### Request Format

```
Method SP Request-URI SP HTTP-Version CRLF
Header-Name: Header-Value CRLF
Header-Name: Header-Value CRLF
CRLF
[body]
```

Where `SP` is a single space (0x20) and `CRLF` is carriage-return + line-feed
(0x0D 0x0A). Here's a concrete example with every byte labeled:

```
G  E  T  SP /  i  n  d  e  x  .  h  t  m  l  SP H  T  T  P  /  1  .  0  CR LF
H  o  s  t  :  SP w  w  w  .  e  x  a  m  p  l  e  .  c  o  m  CR LF
U  s  e  r  -  A  g  e  n  t  :  SP V  e  n  t  u  r  e  /  0  .  1  CR LF
CR LF
```

The blank line (CRLF immediately after the last header's CRLF) signals the end
of the header section. Everything after it is the body. For GET requests, the
body is typically empty.

#### Response Format

```
HTTP-Version SP Status-Code SP Reason-Phrase CRLF
Header-Name: Header-Value CRLF
Header-Name: Header-Value CRLF
CRLF
[body]
```

Example:

```
HTTP/1.0 200 OK\r\n
Content-Type: text/html\r\n
Content-Length: 45\r\n
\r\n
<html><body><h1>Hello, World!</h1></body></html>
```

### How Requests and Responses Differ

The first line is the giveaway:

| Message   | First line starts with | Structure                               |
|-----------|------------------------|-----------------------------------------|
| Request   | A method (GET, POST..) | Method SP URI SP Version CRLF           |
| Response  | "HTTP/"                | Version SP Status-Code SP Reason CRLF   |

The lexer detects which kind of message it's looking at by examining the first
bytes. If they spell out `HTTP/`, it's a response. Otherwise, it's a request
starting with a method name.

### Header Lines

After the first line, both requests and responses share the same header format:

```
Field-Name ":" OWS Field-Value OWS CRLF
```

Where `OWS` means "optional whitespace" — zero or more spaces or tabs. In
practice, most servers send exactly one space after the colon:

```
Content-Type: text/html
```

But the lexer must tolerate variations:

```
Content-Type:text/html           ← no space (valid)
Content-Type:   text/html        ← extra spaces (valid)
Content-Type:\ttext/html         ← tab (valid)
```

Header names are case-insensitive per the spec (`Content-Type` and
`content-type` mean the same thing), but the lexer preserves the original
casing as-is. Case normalization is the parser's job.

### The Body

Everything after the blank line (`\r\n\r\n`) is the body. The lexer captures it
as raw bytes without interpretation. The body might be HTML, an image, JSON, or
nothing at all. The lexer doesn't care — it just captures the bytes.

### CRLF Tolerance

The spec says lines end with `\r\n` (CRLF), but in the wild you will encounter
bare `\n` (LF) — especially from Unix-based servers or hand-crafted test data.
Robustness principle (Postel's Law): "Be conservative in what you send, be
liberal in what you accept." The lexer accepts both `\r\n` and bare `\n` as line
terminators.

---

## Public API

The public API is intentionally minimal. Two functions, one token type, one error
type.

### Token Type

```rust
/// A single token extracted from an HTTP/1.0 message.
///
/// Tokens represent the syntactic atoms of the HTTP wire format.
/// They carry no semantic meaning — the parser layer interprets
/// what the tokens mean.
///
/// # Example
///
/// Lexing the request `GET / HTTP/1.0\r\nHost: localhost\r\n\r\n` produces:
///
/// ```text
/// [Method("GET"), RequestUri("/"), Version("HTTP/1.0"),
///  HeaderName("Host"), HeaderValue("localhost"), Body([]), Eof]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum HttpToken {
    /// HTTP method: GET, POST, HEAD, PUT, DELETE, LINK, UNLINK.
    ///
    /// RFC 1945 defines GET, HEAD, and POST. Extension methods
    /// (PUT, DELETE, LINK, UNLINK) are also recognized.
    Method(String),

    /// The Request-URI: the resource being requested.
    ///
    /// Examples: "/", "/index.html", "/images/photo.gif"
    /// Can be an absolute URI or an absolute path.
    RequestUri(String),

    /// The HTTP version string, e.g. "HTTP/1.0".
    ///
    /// Appears in both request lines (third field) and
    /// response status lines (first field).
    Version(String),

    /// Three-digit integer status code from a response.
    ///
    /// Examples: 200, 301, 404, 500
    StatusCode(u16),

    /// Human-readable reason phrase from a response status line.
    ///
    /// Examples: "OK", "Moved Permanently", "Not Found"
    ReasonPhrase(String),

    /// Header field name (left side of the colon).
    ///
    /// Preserved exactly as received — no case normalization.
    /// Examples: "Content-Type", "Host", "User-Agent"
    HeaderName(String),

    /// Header field value (right side of the colon).
    ///
    /// Leading and trailing whitespace is trimmed.
    /// Examples: "text/html", "www.example.com", "1234"
    HeaderValue(String),

    /// The message body as raw bytes.
    ///
    /// Everything after the blank line (\r\n\r\n) that ends the
    /// header section. May be empty (zero-length Vec).
    Body(Vec<u8>),

    /// Signals the end of the token stream.
    Eof,
}
```

### Error Type

```rust
/// Errors that can occur during HTTP/1.0 lexing.
///
/// Each variant carries a human-readable message describing
/// what went wrong, suitable for diagnostic output.
#[derive(Debug, Clone, PartialEq)]
pub enum HttpLexError {
    /// The method token is not a valid HTTP method.
    /// e.g., "G3T /index.html HTTP/1.0"
    InvalidMethod(String),

    /// The status code is not a valid 3-digit integer in [100, 599].
    /// e.g., "HTTP/1.0 999 Oops" or "HTTP/1.0 abc OK"
    InvalidStatusCode(String),

    /// The version string doesn't match "HTTP/x.y" format.
    /// e.g., "GET / HTZP/1.0"
    InvalidVersion(String),

    /// A header line doesn't contain a colon separator.
    /// e.g., "Content-Type text/html" (missing colon)
    MalformedHeaderLine(String),

    /// The input ended unexpectedly before a complete message
    /// could be lexed.
    /// e.g., "GET /index.html" (no version, no CRLF)
    UnexpectedEof,
}
```

### Lexing Functions

```rust
/// Tokenize an HTTP/1.0 request from raw bytes.
///
/// Expects a byte slice containing a complete HTTP request:
/// request line, headers, blank line, and optional body.
///
/// # Example
///
/// ```rust
/// let input = b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n";
/// let tokens = lex_request(input)?;
/// assert_eq!(tokens[0], HttpToken::Method("GET".into()));
/// assert_eq!(tokens[1], HttpToken::RequestUri("/".into()));
/// assert_eq!(tokens[2], HttpToken::Version("HTTP/1.0".into()));
/// ```
///
/// # Errors
///
/// Returns `HttpLexError` if the input is malformed:
/// - `InvalidMethod` if the method is not a recognized token
/// - `InvalidVersion` if the version string is malformed
/// - `MalformedHeaderLine` if a header lacks a colon
/// - `UnexpectedEof` if the input is truncated
pub fn lex_request(input: &[u8]) -> Result<Vec<HttpToken>, HttpLexError>

/// Tokenize an HTTP/1.0 response from raw bytes.
///
/// Expects a byte slice containing a complete HTTP response:
/// status line, headers, blank line, and optional body.
///
/// # Example
///
/// ```rust
/// let input = b"HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nHello";
/// let tokens = lex_response(input)?;
/// assert_eq!(tokens[0], HttpToken::Version("HTTP/1.0".into()));
/// assert_eq!(tokens[1], HttpToken::StatusCode(200));
/// assert_eq!(tokens[2], HttpToken::ReasonPhrase("OK".into()));
/// ```
///
/// # Errors
///
/// Returns `HttpLexError` if the input is malformed:
/// - `InvalidVersion` if the version string is malformed
/// - `InvalidStatusCode` if the status code isn't a valid integer
/// - `MalformedHeaderLine` if a header lacks a colon
/// - `UnexpectedEof` if the input is truncated
pub fn lex_response(input: &[u8]) -> Result<Vec<HttpToken>, HttpLexError>
```

---

## Lexing Algorithm

The lexer works in sequential **phases**, consuming bytes from front to back.
No backtracking is needed — HTTP/1.0's wire format is designed to be parsed in
a single pass.

### Phase 1: First Line Detection

Look at the first non-whitespace bytes of the input:

```
Does it start with "HTTP/"?
  ├─ Yes → This is a response. Parse a status line.
  └─ No  → This is a request. Parse a request line.
```

Since the two public functions (`lex_request` and `lex_response`) already know
which type they expect, this detection is implicit — but the internal machinery
is the same.

### Phase 2a: Request Line (for requests)

```
GET /index.html HTTP/1.0\r\n
^^^                          → Method: scan until SP
    ^^^^^^^^^^^              → Request-URI: scan until SP
                ^^^^^^^^     → Version: scan until CRLF
```

1. Scan forward from the start, collecting bytes until the first space → Method
2. Skip the space, collect bytes until the next space → Request-URI
3. Skip the space, collect bytes until CRLF → Version
4. Advance past the CRLF

### Phase 2b: Status Line (for responses)

```
HTTP/1.0 200 OK\r\n
^^^^^^^^             → Version: scan until SP
         ^^^         → Status-Code: scan until SP, parse as u16
             ^^      → Reason-Phrase: scan until CRLF
```

1. Collect bytes until the first space → Version
2. Skip the space, collect bytes until the next space → Status-Code (parse as
   integer, must be 100-599)
3. Skip the space, collect bytes until CRLF → Reason-Phrase
4. Advance past the CRLF

### Phase 3: Headers

Repeatedly read lines until we hit an empty line (just CRLF):

```
Content-Type: text/html\r\n     → HeaderName("Content-Type"), HeaderValue("text/html")
Content-Length: 1234\r\n         → HeaderName("Content-Length"), HeaderValue("1234")
\r\n                             → (empty line — stop reading headers)
```

For each header line:

1. Scan until `:` → HeaderName
2. Skip the colon and any optional whitespace (spaces, tabs)
3. Collect remaining bytes until CRLF, trim trailing whitespace → HeaderValue
4. Emit HeaderName and HeaderValue tokens

If a line has no colon, emit `MalformedHeaderLine` error.

### Phase 4: Body

Everything after the blank line is the body:

1. Collect all remaining bytes into a `Vec<u8>`
2. Emit `Body(bytes)` — even if empty (zero bytes)
3. Emit `Eof`

### Tolerance Rules

Following Postel's Law ("be liberal in what you accept"):

| Situation                      | Behavior                              |
|--------------------------------|---------------------------------------|
| `\n` instead of `\r\n`        | Accept bare LF as a line terminator   |
| Leading whitespace before line | Strip and continue                    |
| Extra spaces around header `:` | Trim whitespace from header values    |
| Trailing whitespace in values  | Trim it                               |

---

## Testing Strategy

### Unit Tests by Phase

**Request line parsing:**
- Standard `GET / HTTP/1.0\r\n` request
- All RFC 1945 methods: GET, HEAD, POST
- Extension methods: PUT, DELETE
- Long URIs with query strings and fragments
- Missing version → `UnexpectedEof`
- Invalid method characters → `InvalidMethod`

**Status line parsing:**
- Common status codes: 200 OK, 301 Moved, 404 Not Found, 500 Server Error
- Three-digit boundary: 100, 599
- Non-numeric status code → `InvalidStatusCode`
- Out-of-range status code (e.g., 600) → `InvalidStatusCode`
- Empty reason phrase (just status code + CRLF)
- Multi-word reason phrase: "Moved Permanently"

**Header parsing:**
- Single header
- Multiple headers
- Header with no space after colon: `Host:localhost`
- Header with extra spaces: `Host:   localhost`
- Header with tab after colon
- Empty header value: `X-Empty:`
- Missing colon → `MalformedHeaderLine`
- Header value with colons (e.g., timestamps): `Date: Mon, 01 Jan 1996 00:00:00 GMT`

**Body parsing:**
- Non-empty body (HTML, plain text)
- Empty body (CRLF CRLF followed by nothing)
- Binary body (bytes > 127)

**CRLF tolerance:**
- Request with `\n` only (bare LF)
- Mixed `\r\n` and `\n` in the same message
- Leading whitespace before the first line

### Integration Tests

- Full request: method + URI + version + multiple headers + body
- Full response: version + status + reason + headers + body
- Real-world captures: lex actual HTTP/1.0 messages captured from
  well-known servers

### Edge Cases

- Empty input → `UnexpectedEof`
- Input that is only whitespace → `UnexpectedEof`
- Header with value containing a colon (split on first colon only)
- Very long header values (> 8KB)
- Reason phrase with special characters

---

## Scope

### In Scope

- Tokenization of HTTP/1.0 requests (RFC 1945 Section 5)
- Tokenization of HTTP/1.0 responses (RFC 1945 Section 6)
- CRLF and bare-LF tolerance
- Whitespace tolerance around header values
- Leading whitespace tolerance before first line
- All standard HTTP/1.0 methods (GET, HEAD, POST)
- Extension methods (PUT, DELETE, LINK, UNLINK)

### Out of Scope

- **HTTP/1.1 features:** chunked transfer encoding, persistent connections
  (keep-alive), the `Host` header requirement, trailers, 100-continue
- **HTTP/2 and HTTP/3:** binary framing, multiplexing, HPACK, QPACK
- **HTTPS/TLS:** encryption is a transport concern, not a lexing concern
- **Header value interpretation:** parsing `Content-Type: text/html; charset=utf-8`
  into type/subtype/parameters is the parser's job
- **Body decoding:** decompression (gzip, deflate), charset conversion
- **Request routing:** mapping URIs to handlers
- **Connection management:** opening/closing TCP sockets
