# NET05 — HTTP/1.0 Client

## Overview

The HTTP/1.0 client is a thin orchestrator — roughly 100 lines of glue code that
wires together five independent packages into a complete HTTP client. It does
almost nothing on its own. Instead, it sequences calls through a pipeline of
single-purpose packages, each handling one layer of the problem:

| Step | Package              | Responsibility                          |
|------|----------------------|-----------------------------------------|
| 1    | url-parser (NET00)   | Parse URL into scheme, host, port, path |
| 2    | tcp-client (NET01)   | Open a TCP socket to the server         |
| 3    | *(inline)*           | Write the HTTP request line + headers   |
| 4    | frame-extractor (NET02) | Extract header frame, then body      |
| 5    | http1.0-lexer (NET03)   | Tokenize the raw response bytes      |
| 6    | http1.0-parser (NET04)  | Build a structured HttpResponse      |

This is the unix-pipe philosophy made concrete: each package does one thing
well, and the client simply connects them in sequence.

## Where It Fits

```
┌──────────────────────────────────────────────────────────────────────┐
│                      Application Code                                │
│                  calls http1_client::get(url)                        │
└──────────────────────────┬───────────────────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────────────────┐
│                  HTTP/1.0 Client (NET05)                             │
│          ~100 lines of orchestration glue                            │
│                                                                      │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐  ┌────────┐  ┌─────┐│
│  │url-parser│→ │tcp-client│→ │frame-extractor│→ │ lexer  │→ │parse││
│  │  NET00   │  │  NET01   │  │    NET02      │  │ NET03  │  │NET04││
│  └──────────┘  └──────────┘  └───────────────┘  └────────┘  └─────┘│
└──────────────────────────────────────────────────────────────────────┘
```

NET05 is the first package in the networking stack where a user can make an
actual HTTP request with a single function call. Everything below it is
plumbing; this is the faucet.

### Dependency Tree

```
http1.0-client (NET05)
├── url-parser (NET00)
├── tcp-client (NET01)
├── frame-extractor (NET02)
├── http1.0-lexer (NET03)
└── http1.0-parser (NET04)
```

## Concepts

### 1. The Pipeline — Step by Step

This is the full journey of an HTTP/1.0 GET request. Every step maps to a
package boundary, so you can test and reason about each one independently.

```
User calls: http1_client::get("http://info.cern.ch/hypertext/WWW/TheProject.html")

Step 1: url-parser (NET00)
        "http://info.cern.ch/hypertext/WWW/TheProject.html"
        → Url { scheme: "http", host: "info.cern.ch", port: 80, path: "/hypertext/WWW/TheProject.html" }

Step 2: tcp-client (NET01)
        connect("info.cern.ch", 80, default_options)
        → TcpConnection (live socket)

Step 3: Write request
        TcpConnection.write_all(b"GET /hypertext/WWW/TheProject.html HTTP/1.0\r\nHost: info.cern.ch\r\nUser-Agent: Venture/0.1\r\n\r\n")
        TcpConnection.shutdown_write() — signal we're done sending

Step 4: frame-extractor (NET02)
        Phase A: DelimiterStrategy("\r\n\r\n").extract(conn) → header bytes
        Phase B: Parse Content-Length from raw headers, or fall back to ReadToEnd
        Phase C: LengthPrefixedStrategy(len).extract(conn) → body bytes
                 OR ReadToEndStrategy.extract(conn) → body bytes

Step 5: http1.0-lexer (NET03)
        lex_response(&raw_bytes) → Vec<HttpToken>

Step 6: http1.0-parser (NET04)
        parse_response(&tokens) → HttpResponse { status: 200, headers: [...], body: b"<html>..." }

Step 7: Redirect following (if status is 301 or 302)
        Extract Location header → resolve against base URL → go to Step 1
        Max 5 redirects to prevent infinite loops

Return: HttpResponse to caller
```

### 2. Request Construction

HTTP/1.0 requests are simple text. The client constructs them by string
formatting — no serialization library needed:

```
GET <path> HTTP/1.0\r\n
Host: <host>\r\n
User-Agent: Venture/0.1\r\n
Accept: */*\r\n
\r\n
```

The blank line (`\r\n\r\n`) terminates the headers and signals "no request
body." For GET requests, there is never a body.

### 3. Connection Lifecycle

HTTP/1.0 is one-request-per-connection. The full lifecycle is:

1. Open TCP connection
2. Send request
3. Call `shutdown_write()` to signal we are done sending
4. Read the complete response
5. Connection closes (server closes its end after responding)

There is no keep-alive, no pipelining, no multiplexing. One socket, one
request, one response, done. This simplicity is why we start with HTTP/1.0
rather than 1.1.

### 4. Redirect Following

Some URLs respond with a redirect instead of content. The HTTP/1.0 status
codes that trigger redirect following:

- **301 Moved Permanently** — the resource has a new canonical URL
- **302 Found** — the resource is temporarily at a different URL

Both include a `Location` header with the new URL. The client:

1. Extracts the `Location` header value
2. Resolves it against the base URL (the `Location` may be relative, e.g.,
   `/other-page` instead of `http://example.com/other-page`)
3. Starts the pipeline over from Step 1 with the new URL
4. Caps at 5 total redirects to prevent infinite loops

### 5. Why This Is ~100 Lines

All the complexity lives in the five dependency packages:

- URL parsing? NET00 handles it.
- TCP sockets? NET01 handles it.
- Framing (knowing where headers end and body begins)? NET02 handles it.
- Tokenizing HTTP? NET03 handles it.
- Building structured responses? NET04 handles it.

The client just calls them in order. This is the payoff of the unix-pipe
architecture: the integration layer is trivial because the components are
well-defined.

## Public API

### Rust

```rust
use std::time::Duration;

/// An HTTP/1.0 client that orchestrates the NET00–NET04 pipeline.
///
/// All configuration has sensible defaults. For most use cases, the
/// free function `get()` is sufficient — you only need `HttpClient`
/// if you want to customize timeouts, user-agent, or redirect limits.
pub struct HttpClient {
    /// Maximum number of redirects to follow before returning
    /// TooManyRedirects. Default: 5.
    max_redirects: usize,

    /// The User-Agent header sent with every request.
    /// Default: "Venture/0.1".
    user_agent: String,

    /// How long to wait for the TCP connection to establish.
    /// Default: 30 seconds.
    connect_timeout: Duration,

    /// How long to wait for data during response reading.
    /// Default: 30 seconds.
    read_timeout: Duration,
}

impl HttpClient {
    /// Create a new client with default settings.
    pub fn new() -> Self;

    /// Perform an HTTP/1.0 GET request.
    ///
    /// This runs the full pipeline: parse URL → connect → send request →
    /// extract frames → lex → parse → follow redirects if needed.
    pub fn get(&self, url: &str) -> Result<HttpResponse, HttpClientError>;
}

/// Convenience function: perform a GET with default options.
///
/// Equivalent to `HttpClient::new().get(url)`.
pub fn get(url: &str) -> Result<HttpResponse, HttpClientError>;
```

### Error Types

Every error wraps the underlying package error so the caller can inspect
exactly what went wrong and at which pipeline stage:

```rust
pub enum HttpClientError {
    /// URL parsing failed (NET00).
    UrlError(url_parser::UrlError),

    /// TCP connection failed (NET01).
    ConnectionError(tcp_client::ConnectionError),

    /// Frame extraction failed (NET02).
    FrameError(frame_extractor::FrameError),

    /// Lexing failed (NET03).
    LexError(http1_lexer::LexError),

    /// Parsing failed (NET04).
    ParseError(http1_parser::ParseError),

    /// Followed too many redirects (default limit: 5).
    TooManyRedirects { limit: usize },

    /// The URL scheme is not "http". HTTPS is out of scope for NET05.
    UnsupportedScheme(String),
}
```

## Testing Strategy

### 1. Integration — Full Pipeline

Spin up a localhost `TcpListener` that serves a canned HTTP/1.0 response:
`"HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello"`. Call
`http1_client::get("http://127.0.0.1:<port>/")` and verify the returned
`HttpResponse` has status 200 and body `b"hello"`.

### 2. Redirect Following

Server returns `302 Found` with `Location: http://127.0.0.1:<port>/target`.
The `/target` endpoint returns `200 OK`. Verify the client follows the redirect
and returns the final 200 response.

### 3. Redirect Loop Detection

Server always returns `302 Found` pointing back to itself. Verify the client
returns `HttpClientError::TooManyRedirects` after 5 attempts.

### 4. Relative Redirect Resolution

Server returns `302 Found` with `Location: /other` (a relative path). Verify
the client resolves it against the original URL and follows it correctly.

### 5. No Content-Length (Read to End)

Server sends response headers without `Content-Length`, writes body bytes, then
closes the connection. Verify the client reads the complete body using the
read-to-end fallback strategy.

### 6. Connection Refused

Attempt to connect to a port with no listener. Verify the client returns
`HttpClientError::ConnectionError`.

### 7. DNS Failure

Attempt to connect to `"nonexistent.invalid"`. Verify the client returns
`HttpClientError::ConnectionError` (DNS resolution is part of TCP connect).

### 8. Large Response

Server sends a 1 MB body. Verify the client receives the complete body without
truncation or corruption.

### 9. Real-World Smoke Test

`GET http://info.cern.ch/` — the first website ever published. Verify a 200
response with an HTML body. This test is `#[ignore]` by default (requires
network access) but runnable via `cargo test -- --ignored`.

## Scope

### In Scope

- HTTP/1.0 GET requests
- Request line and header construction
- Pipeline orchestration (NET00 → NET01 → NET02 → NET03 → NET04)
- Redirect following (301, 302) with configurable limit
- Configurable timeouts and user-agent
- Error propagation from all pipeline stages

### Out of Scope

- POST, PUT, DELETE, or any method with a request body
- HTTP/1.1 (chunked encoding, persistent connections, pipelining)
- HTTPS / TLS — a future NET06 TLS package would slot between NET01 and NET02
- Cookies, authentication, caching
- Proxy support
- Connection pooling
- Async I/O
- Request body sending

### Future: HTTP/1.1 Client

A future `http1.1-client` would swap in `http1.1-lexer` / `http1.1-parser`
and add:

- **Chunked transfer encoding** — body arrives in length-prefixed chunks
  rather than a single Content-Length block
- **Persistent connections** — reuse the same TCP socket for multiple requests
  (keep-alive is the default in 1.1)
- **Host header requirement** — mandatory in 1.1 (we already send it, but 1.0
  servers may ignore it)
- **`Connection: close` signaling** — explicitly request one-shot behavior
  when keep-alive is not desired
