# NET04 — HTTP/1.0 Parser

## Overview

The HTTP/1.0 lexer (NET03) reads raw bytes and produces **tokens** — method,
URI, version, header names and values, status codes. The lexer knows syntax: it
knows where one token ends and the next begins. But it does not know what the
tokens **mean**.

That is the parser's job. The parser consumes a stream of `HttpToken` values
from the lexer and assembles them into **semantic structures**: `HttpRequest` and
`HttpResponse`. It understands that a `Content-Type` header describes the body
format, that status 301 means "go somewhere else," and that a HEAD response has
no body regardless of what the headers claim.

**Analogy:** The lexer is like a mail room clerk who opens envelopes and
separates the address, stamp, and letter. The parser is the reader who
understands what the letter says — "this is a redirect, go to this other
address" or "here is the HTML page you requested."

```
Mail Room Clerk (Lexer):
  Envelope → [Address] [Stamp] [Letter]
  "I found three pieces. I don't know what they mean."

Reader (Parser):
  [Address: 301 Moved] [Letter: Location: https://new-site.com]
  "This is a redirect. The content has moved to a new address."
```

The lexer/parser split follows the same architecture used throughout this repo
(see 02-lexer.md and 03-parser.md for the general pattern). The lexer handles
the messy byte-level details; the parser reasons about protocol semantics on
clean, typed tokens.

---

## Where It Fits

```
http1.0-lexer (NET03) → http1.0-parser (NET04) → http1.0-client (NET05)
                              ↑ THIS PACKAGE
```

The flow for a complete HTTP/1.0 exchange:

```
  1. tcp-client (NET01) opens a connection to the server
  2. frame-extractor (NET02) carves the byte stream into header block + body
  3. http1.0-lexer (NET03) tokenizes the header block into HttpToken values
  4. http1.0-parser (NET04) assembles tokens into HttpRequest / HttpResponse
     ↑ THIS STEP
  5. http1.0-client (NET05) uses the parsed response to follow redirects,
     render HTML, or report errors
```

**Depends on:** http1.0-lexer (NET03) — for `HttpToken` types
**Depended on by:** http1.0-client (NET05), Venture browser (BR01)

---

## Concepts

### Tokens In, Structs Out

The parser's job is straightforward: walk through an ordered list of tokens and
build a struct. Think of it like reading a form — you see "Name:" followed by
"Alice," so you fill in `name = "Alice"`. The tokens arrive in protocol-defined
order:

```
Response tokens (in order):
  [Version: HTTP/1.0] [Status: 200] [Reason: OK]
  [HeaderName: Content-Type] [HeaderValue: text/html]
  [HeaderName: Content-Length] [HeaderValue: 45]
  [Body: <html>...</html>]

Parsed into:
  HttpResponse {
      version: Http10,
      status: 200,
      reason: "OK",
      headers: [("Content-Type", "text/html"), ("Content-Length", "45")],
      body: b"<html>...</html>",
  }
```

### Header Semantics

HTTP headers are **case-insensitive** key-value pairs. The header name
`Content-Type`, `content-type`, and `CONTENT-TYPE` all refer to the same
header. The parser preserves the original casing in storage but provides
case-insensitive lookup methods.

A single response may contain **multiple headers with the same name**. Per
RFC 1945 (HTTP/1.0), this is valid — the values should be treated as if they
were a single header joined by commas. The parser stores all headers in
order and lets the caller decide how to handle duplicates.

```
Set-Cookie: session=abc123
Set-Cookie: theme=dark

Stored as: [("Set-Cookie", "session=abc123"), ("Set-Cookie", "theme=dark")]
Lookup "Set-Cookie" → returns "session=abc123" (first match)
```

### Content-Type Parsing

The `Content-Type` header packs two pieces of information into one value:
the **media type** (what kind of data) and optional **parameters** (how
it is encoded):

```
Content-Type: text/html; charset=utf-8
              ─────────  ──────────────
              media type   parameter

Content-Type: application/json
              ────────────────
              media type (no parameters)

Content-Type: text/html; charset=iso-8859-1; boundary=something
              ─────────  ─────────────────── ──────────────────
              media type  first parameter      second parameter
```

The parser extracts the media type and the `charset` parameter (if present).
Other parameters are ignored — they are rarely used in HTTP/1.0 practice.

### Status Code Categories

HTTP status codes follow a numbering convention where the first digit tells
you the **category** of response:

```
Category    Range    Meaning              HTTP/1.0 Examples
─────────────────────────────────────────────────────────────
1xx         100–199  Informational        (not used in practice)
2xx         200–299  Success              200 OK
3xx         300–399  Redirect             301 Moved Permanently
                                          302 Found
4xx         400–499  Client error         400 Bad Request
                                          403 Forbidden
                                          404 Not Found
5xx         500–599  Server error         500 Internal Server Error
                                          503 Service Unavailable
```

The parser provides an `is_redirect()` helper that checks for status codes
301, 302, 303, and 307 — the four codes that mean "the resource you want
is at a different URL."

### Body Size Determination

In HTTP/1.0, figuring out how many bytes the body contains is not always
straightforward. The parser applies these rules in order:

```
Rule 1: Is this a HEAD response, or status 204/304?
  → Yes: body is empty, regardless of headers.

Rule 2: Is there a Content-Length header?
  → Yes: body is exactly Content-Length bytes.

Rule 3: Neither of the above?
  → Body extends until the server closes the connection.
     (This is HTTP/1.0's default behavior — no keep-alive.)
```

This is simpler than HTTP/1.1, which adds chunked transfer encoding. In
HTTP/1.0, the server simply closes the connection when done sending.

---

## Public API

```rust
// ─────────────────────────────────────────────
// Core Structs
// ─────────────────────────────────────────────

/// A parsed HTTP request (client → server).
///
/// Built from a sequence of HttpToken values produced by the lexer.
/// Contains all the information needed to understand what the client
/// is asking for.
///
/// ## Example
///
/// ```rust
/// let request = parse_request(&tokens)?;
/// println!("Client wants: {} {}", request.method, request.uri);
/// // "Client wants: GET /index.html"
/// ```
pub struct HttpRequest {
    /// The HTTP method: GET, POST, HEAD, etc.
    ///
    /// HTTP/1.0 defines three methods:
    /// - GET:  "Give me this resource"
    /// - POST: "Accept this data"
    /// - HEAD: "Give me just the headers for this resource"
    pub method: String,

    /// The resource being requested: "/index.html", "/api/users", etc.
    pub uri: String,

    /// The HTTP version. Always Http10 for this parser.
    pub version: HttpVersion,

    /// Request headers as ordered key-value pairs.
    ///
    /// Headers are stored in the order they appeared in the request.
    /// Multiple headers with the same name are stored as separate entries.
    pub headers: Vec<(String, String)>,

    /// The request body, if any.
    ///
    /// GET and HEAD requests typically have empty bodies.
    /// POST requests carry their payload here.
    pub body: Vec<u8>,
}

/// A parsed HTTP response (server → client).
///
/// Built from a sequence of HttpToken values produced by the lexer.
/// Contains the status code, headers, body, and convenience methods
/// for common operations like redirect detection and content-type
/// inspection.
///
/// ## Example
///
/// ```rust
/// let response = parse_response(&tokens)?;
/// if response.is_redirect() {
///     println!("Follow redirect to: {}", response.location().unwrap());
/// } else if response.is_html() {
///     println!("Got {} bytes of HTML", response.body.len());
/// }
/// ```
pub struct HttpResponse {
    /// The HTTP version. Always Http10 for this parser.
    pub version: HttpVersion,

    /// The three-digit status code: 200, 301, 404, 500, etc.
    pub status: u16,

    /// The human-readable reason phrase: "OK", "Not Found", etc.
    ///
    /// This exists for debugging and logging. Code should branch on
    /// the numeric status code, not the reason phrase — servers are
    /// free to send any text here.
    pub reason: String,

    /// Response headers as ordered key-value pairs.
    pub headers: Vec<(String, String)>,

    /// The response body as raw bytes.
    ///
    /// For text responses, decode using the charset from Content-Type.
    /// For binary responses (images, etc.), use the bytes directly.
    pub body: Vec<u8>,
}

/// The HTTP version. HTTP/1.0 is the only version this parser handles.
pub enum HttpVersion {
    Http10,
}

// ─────────────────────────────────────────────
// Convenience Methods
// ─────────────────────────────────────────────

impl HttpResponse {
    /// Case-insensitive header lookup. Returns the value of the first
    /// header whose name matches (ignoring ASCII case).
    ///
    /// ## Example
    ///
    /// ```rust
    /// // All of these find the same header:
    /// response.header("Content-Type");
    /// response.header("content-type");
    /// response.header("CONTENT-TYPE");
    /// ```
    pub fn header(&self, name: &str) -> Option<&str>

    /// Parse the Content-Type header into (media_type, charset).
    ///
    /// Extracts the media type and optional charset parameter from
    /// the Content-Type header value.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// // "text/html; charset=utf-8" → ("text/html", Some("utf-8"))
    /// // "application/json"         → ("application/json", None)
    /// // No Content-Type header     → None
    /// ```
    pub fn content_type(&self) -> Option<(String, Option<String>)>

    /// Content-Length as usize, if the header is present and contains
    /// a valid non-negative integer.
    ///
    /// Returns None if the header is missing or contains a non-numeric
    /// value (which is a server bug, but not our problem to fix).
    pub fn content_length(&self) -> Option<usize>

    /// Is this response a redirect?
    ///
    /// Returns true for status codes 301, 302, 303, and 307 — the
    /// four codes that mean "look somewhere else."
    ///
    /// ```
    /// 301 Moved Permanently  — "This page has moved forever"
    /// 302 Found              — "This page has moved temporarily"
    /// 303 See Other          — "Go GET this other URL instead"
    /// 307 Temporary Redirect — "Same as 302 but keep the method"
    /// ```
    pub fn is_redirect(&self) -> bool

    /// The Location header value, used with redirects.
    ///
    /// When is_redirect() is true, this tells you where to go next.
    /// Returns None if the header is missing (a buggy redirect).
    pub fn location(&self) -> Option<&str>

    /// Is the response body HTML?
    ///
    /// Returns true if the Content-Type media type starts with
    /// "text/html". This is a convenience for browser-like clients
    /// that need to decide whether to render or download.
    pub fn is_html(&self) -> bool
}

// ─────────────────────────────────────────────
// Parser Functions
// ─────────────────────────────────────────────

/// Parse a sequence of tokens into an HttpRequest.
///
/// Expects tokens in this order:
///   [Method] [Uri] [Version] ([HeaderName] [HeaderValue])* [Body]?
///
/// Returns an error if required tokens are missing or malformed.
///
/// ## Example
///
/// ```rust
/// let tokens = lexer::tokenize_request(b"GET / HTTP/1.0\r\nHost: example.com\r\n\r\n")?;
/// let request = parse_request(&tokens)?;
/// assert_eq!(request.method, "GET");
/// assert_eq!(request.uri, "/");
/// ```
pub fn parse_request(tokens: &[HttpToken]) -> Result<HttpRequest, HttpParseError>

/// Parse a sequence of tokens into an HttpResponse.
///
/// Expects tokens in this order:
///   [Version] [Status] [Reason] ([HeaderName] [HeaderValue])* [Body]?
///
/// Returns an error if required tokens are missing or malformed.
///
/// ## Example
///
/// ```rust
/// let tokens = lexer::tokenize_response(raw_bytes)?;
/// let response = parse_response(&tokens)?;
/// assert_eq!(response.status, 200);
/// assert_eq!(response.reason, "OK");
/// ```
pub fn parse_response(tokens: &[HttpToken]) -> Result<HttpResponse, HttpParseError>

// ─────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────

/// Everything that can go wrong when parsing HTTP tokens.
///
/// These errors indicate that the token stream does not match the
/// expected structure of an HTTP message. The lexer already validated
/// syntax (well-formed tokens), so these are semantic errors — the
/// tokens are individually valid but do not form a coherent message.
pub enum HttpParseError {
    /// The token stream has no method token (e.g., empty input).
    MissingMethod,

    /// A method was found but no URI followed it.
    MissingUri,

    /// Method and URI were found but no version followed.
    MissingVersion,

    /// The status code is not a valid three-digit integer.
    ///
    /// HTTP status codes must be in the range 100–599.
    InvalidStatusCode,

    /// The Content-Type header value could not be parsed.
    ///
    /// This means the media type portion is empty or malformed,
    /// e.g., "; charset=utf-8" (no media type before the semicolon).
    MalformedContentType,
}
```

### Design Decisions

**Why `Vec<(String, String)>` for headers instead of `HashMap`?** Three
reasons:

1. **Order preservation.** Some protocols care about header order (proxies
   may reorder, but we should not lose information).
2. **Duplicate headers.** A `HashMap` would silently drop duplicates.
   `Set-Cookie` appears multiple times in practice.
3. **Simplicity.** Linear scan over a few dozen headers is faster than
   hashing for realistic HTTP/1.0 response sizes.

**Why `Vec<u8>` for the body instead of `String`?** HTTP bodies can be
binary (images, compressed content). Using `Vec<u8>` is correct for all
content types. Text bodies can be decoded by the caller using the charset
from `content_type()`.

**Why separate `parse_request` and `parse_response`?** Requests and
responses have different token structures (method/uri/version vs.
version/status/reason). A single `parse()` function would need a flag or
enum to distinguish them, adding complexity for no benefit.

---

## Testing Strategy

All tests operate on pre-constructed token slices — no network I/O, no
lexer dependency at test time. This isolates parser logic from lexer
behavior.

### Happy Path

1. **200 OK with body** — Parse a standard response with Content-Type,
   Content-Length, and an HTML body. Verify all fields are populated
   correctly.
2. **301 redirect with Location** — Parse a redirect response. Verify
   `is_redirect()` returns true and `location()` returns the target URL.
3. **404 Not Found** — Parse an error response. Verify status is 404 and
   `is_redirect()` returns false.
4. **200 with no Content-Length** — Body determined by EOF (the `body`
   field contains everything the lexer provided after headers). Verify
   the body is captured correctly.
5. **GET request** — Parse a simple GET request with Host header.
6. **POST request with body** — Parse a POST request carrying form data.

### Header Handling

7. **Case-insensitive lookup** — Set header as "Content-Type", look up as
   "content-type", "CONTENT-TYPE", and "Content-type". All must return the
   same value.
8. **Multiple headers with same name** — Two "Set-Cookie" headers. Verify
   both are stored. Verify `header()` returns the first one.
9. **No headers** — Response with status line and body but zero headers.
   Verify `header()` returns None for any name.
10. **Header with empty value** — `X-Empty:` (value is empty string).
    Verify it is stored and retrievable.

### Content-Type Parsing

11. **With charset** — `text/html; charset=utf-8` →
    `("text/html", Some("utf-8"))`.
12. **Without charset** — `application/json` →
    `("application/json", None)`.
13. **With extra parameters** — `text/html; charset=iso-8859-1; boundary=x`
    → `("text/html", Some("iso-8859-1"))`. The boundary parameter is
    ignored.
14. **Case variations** — `Text/HTML; Charset=UTF-8` — media type and
    charset should be case-preserved but correctly extracted.
15. **No Content-Type header** — `content_type()` returns `None`.

### Redirect Detection

16. **301 is redirect** — `is_redirect()` returns true.
17. **302 is redirect** — `is_redirect()` returns true.
18. **303 is redirect** — `is_redirect()` returns true.
19. **307 is redirect** — `is_redirect()` returns true.
20. **200 is not redirect** — `is_redirect()` returns false.
21. **Location on non-redirect** — A 200 response with a Location header.
    `location()` still returns the value (it is just a header lookup), but
    `is_redirect()` returns false.

### Body Rules

22. **HEAD response has no body** — Even if Content-Length is present, a
    HEAD response should have an empty body.
23. **204 No Content** — Body is empty regardless of headers.
24. **304 Not Modified** — Body is empty regardless of headers.

### Error Cases

25. **Empty token list** — `parse_request` returns `MissingMethod`.
26. **Method only** — `parse_request` returns `MissingUri`.
27. **Method and URI only** — `parse_request` returns `MissingVersion`.
28. **Invalid status code** — Status token of "abc" returns
    `InvalidStatusCode`.
29. **Status code out of range** — Status token of "999" returns
    `InvalidStatusCode`.

### Real-World Responses

30. **Captured 200 from archive.org** — A real HTTP/1.0 response captured
    from a live server, tokenized and parsed. Verify headers and body
    match expected values.
31. **Captured 301 from archive.org** — A real redirect response. Verify
    `is_redirect()` and `location()` work correctly on real-world data.

---

## Scope

**In scope:**
- Parsing `HttpToken` streams into `HttpRequest` and `HttpResponse` structs
- Case-insensitive header lookup
- Content-Type parsing (media type and charset extraction)
- Status code category helpers (redirect detection, HTML detection)
- Content-Length extraction
- Body size rules for HEAD, 204, and 304 responses
- Descriptive error types for malformed token streams

**Out of scope:**
- **HTTP/1.1 features** — Chunked transfer encoding, keep-alive connections,
  trailers. HTTP/1.1 is a substantially more complex protocol; this parser
  handles only HTTP/1.0.
- **Cookie parsing** — The `Set-Cookie` header has its own complex grammar
  (expires, path, domain, secure, httponly). A dedicated cookie parser
  belongs in a separate package.
- **Authentication headers** — `WWW-Authenticate` and `Authorization` have
  scheme-specific formats (Basic, Digest, Bearer). Parsing these is out of
  scope.
- **Caching headers** — `Cache-Control`, `ETag`, `If-Modified-Since`, etc.
  These are HTTP/1.1 concepts with limited relevance to HTTP/1.0.
- **Multipart bodies** — `multipart/form-data` has its own boundary-based
  framing. This parser treats the body as an opaque `Vec<u8>`.
- **Content decoding** — gzip, deflate, or charset conversion. The parser
  gives you raw bytes; decoding is the caller's responsibility.

---

## Implementation Languages

This package will be implemented in:
- **Rust** (primary, for the Venture browser networking stack)
- Future: all 9 languages following the standard pattern
