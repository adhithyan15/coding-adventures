# NET05 — HTTP/1.0 Client

## Overview

The HTTP/1.0 client is a thin orchestrator that wires together three
independent packages into a complete HTTP client. It does
almost nothing on its own. Instead, it sequences calls through a pipeline of
single-purpose packages, each handling one layer of the problem:

| Step | Package              | Responsibility                          |
|------|----------------------|-----------------------------------------|
| 1    | url-parser (NET00)   | Parse URL into scheme, host, port, path |
| 2    | tcp-client (NET01)   | Open a TCP socket to the server         |
| 3    | *(inline)*           | Write the HTTP request line + headers   |
| 4    | http1                    | Parse the response head and framing  |
| 5    | tcp-client               | Read the bounded response body       |

The current Rust implementation consolidates the original NET02–NET04 wire
pipeline behind the shared `http1` and `http-core` contracts. The older
component names below explain the educational decomposition; `http1-client`
uses the current package boundaries.

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
│        bounded synchronous orchestration                             │
│                                                                      │
│  │url-parser│ → │tcp-client│ → │ http1 + http-core │             │
│  │  NET00   │   │  NET01   │   │ response framing  │             │
│  └──────────┘   └──────────┘   └───────────────────┘             │
└──────────────────────────────────────────────────────────────────────┘
```

NET05 is the first package in the networking stack where a user can make an
actual HTTP request with a single function call. Everything below it is
plumbing; this is the faucet.

### Dependency Tree

```
http1-client (NET05)
├── url-parser (NET00)
├── tcp-client (NET01)
├── http1
└── http-core
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

Step 4: tcp-client + http1
        Read bounded header lines through the blank-line terminator
        parse_response_head(&raw_head) → ResponseHead + BodyKind

Step 5: tcp-client
        BodyKind::ContentLength(n) → read exactly n bounded bytes
        BodyKind::UntilEof         → stream bounded chunks through clean EOF
        BodyKind::None             → empty body

Step 6: Redirect following (if status is 301 or 302)
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

### 5. Why This Stays Small

Most complexity lives in the dependency packages:

- URL parsing? NET00 handles it.
- TCP sockets? NET01 handles it.
- HTTP syntax and semantic response heads? `http1` and `http-core` handle it.

The client sequences those components and owns only transport policy: bounds,
redirects, request metadata validation, and the HTTP/1.0 connection lifecycle.

## Public API

### Rust

```rust
/// An HTTP/1.0 client that orchestrates the current NET00–NET05 packages.
///
/// All configuration has sensible defaults. For most use cases, the
/// free function `get()` is sufficient — you only need `HttpClient`
/// if you want to customize timeouts, bounds, user-agent, or redirects.
pub struct HttpClient {
    /// TCP connect/read/write options.
    pub connect_options: tcp_client::ConnectOptions,

    /// Maximum number of redirects to follow before returning
    /// TooManyRedirects. Default: 5.
    pub max_redirects: usize,

    /// Hard response head and body bounds.
    pub max_head_bytes: usize,
    pub max_body_bytes: usize,

    /// The User-Agent header sent with every request.
    /// Default: "Venture/0.1".
    pub user_agent: String,
}

impl HttpClient {
    /// Create a new client with default settings.
    pub fn new() -> Self;

    /// Perform an HTTP/1.0 GET request.
    ///
    /// This runs the full pipeline: parse URL → connect → send request →
    /// parse a bounded response → follow redirects if needed.
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
    Url(url_parser::UrlError),

    /// TCP connection failed (NET01).
    Tcp(tcp_client::TcpError),

    /// HTTP response-head parsing failed.
    Http(http1::Http1ParseError),

    /// Followed too many redirects (default limit: 5).
    TooManyRedirects { limit: usize },

    /// The URL scheme is not "http". HTTPS is out of scope for NET05.
    UnsupportedScheme(String),

    /// A response exceeded the configured head or body bound.
    ResponseHeadTooLarge { limit: usize },
    ResponseBodyTooLarge { limit: usize },
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
`HttpClientError::Tcp`.

### 7. DNS Failure

Attempt to connect to `"nonexistent.invalid"`. Verify the client returns
`HttpClientError::Tcp` (DNS resolution is part of TCP connect).

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
- Pipeline orchestration (`url-parser` → `tcp-client` → `http1`)
- Redirect following (301, 302) with configurable limit
- Configurable timeouts, bounds, redirect limit, and user-agent
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
