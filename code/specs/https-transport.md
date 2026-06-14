# HTTPS Transport

## Overview

`https-transport` is the in-process HTTPS client for the agent
system. It composes the existing `http1` crate (HTTP/1.1 head
parsing and body framing) with the `tls-platform` crate
(OS-provided TLS) to give callers a single trait — `HttpsTransport`
— for issuing HTTPS requests. There is no subprocess, no curl, no
shell-out: every byte of the request and response flows through
in-process code, with TLS handled by the OS's own implementation.

The trait is the durable artifact. Today the only implementation
is `Http1OverTls` (HTTP/1.1 over a `TlsStream`). Tomorrow we may
add `Http2OverTls` for HTTP/2, or an experimental `Http3OverQuic`
for HTTP/3. Callers depend on the trait, not on the version they
are talking to.

The spec deliberately scopes itself to **request/response HTTPS**:
issue a request, get a response, close the stream. WebSocket
upgrades, server-sent events, and long-lived streaming responses
will live in sibling specs (`wss-transport`, `sse-transport`)
that compose the same primitives differently. We do one thing
here and do it well.

This is the rewrite of the previously-closed
`https-transport.md` (PR #2240) that proposed a curl-bridge as
the v1 implementation. The current direction — wrap the OS-provided
TLS via `tls-platform` and use the existing `http1` crate above it
— gives us in-process semantics, OS-managed certificate trust,
and zero new TLS code in the repository, all under the same public
trait the curl version would have offered.

---

## Where It Fits

```
   Agent code (e.g., weather-host, github-host)
        │
        │  uses HttpsTransport::request → HttpsResponse
        ▼
   HttpsTransport trait    ← THIS SPEC
        │
        ▼
   Http1OverTls implementation
        │
        ├── tls-platform   →  TlsStream (Schannel / Network.framework / OpenSSL)
        ├── http1          →  serialize request head, parse response head, frame body
        └── transport-platform  → underlying TCP socket via secure_net::dial
```

**Depends on:**
- `tls-platform` — the `TlsConnector` and `TlsStream` we read/write through.
- `http1` — request-line + header serialization, response-head parsing, body framing.
- `http-core` — shared HTTP method/status/header types.
- `transport-platform` — the TCP socket abstraction (transitively via tls-platform).
- `capability-cage-rust` — `secure_net::dial` for the TCP connect; the manifest gates `net:connect:host:port`.
- `time` — request/handshake timeouts.
- `url` parsing helpers (lightweight; we just need scheme/host/port/path).

**Used by:**
- `weather-host` — fetches forecasts from `api.weather.gov`.
- `oauth` — token endpoint POSTs and userinfo GETs.
- A future `github-host`, `calendar-host`, etc.
- Any agent that needs to call an HTTPS API.

---

## Design Principles

1. **One trait, swap the implementation.** The trait surface is
   the durable artifact. Today: HTTP/1.1 over OS-TLS. Tomorrow:
   HTTP/2, HTTP/3, alternative TLS backends, native Rust TLS.
   Callers don't change.

2. **In-process only.** No subprocess, no shell-out. Every byte
   passes through code we wrote and can step through with a
   debugger.

3. **Build on what we already have.** `http1` parses heads;
   `tls-platform` does TLS; `transport-platform` does TCP. We
   compose, we don't duplicate.

4. **Capability-cage gated.** The TCP connect goes through
   `secure_net::dial(manifest, "tcp", "host:port")`. The TLS
   handshake then runs on top. Without the manifest's
   `net:connect:host:port`, no socket opens; without a socket, no
   TLS; without TLS, no request.

5. **Defensive parsing.** Response heads are parsed by `http1`,
   which already validates malformed input. We size-limit the head
   (default 64 KiB) and the body (default 10 MiB, configurable per
   request) to bound memory.

6. **No persistent connections in v1.** Each request opens a fresh
   TCP + TLS, sends, reads, closes. Connection pooling is a future
   enhancement; in the meantime we mirror the "ephemeral
   sub-agent" pattern.

7. **No automatic retries.** A failed request returns an error.
   Higher layers (the agent or a retry middleware) decide whether
   and how to retry.

8. **HTTP errors are responses, not errors.** A 4xx or 5xx response
   comes back as `HttpsResponse { status: 4xx, ... }`. Only true
   transport-layer failures (DNS, connect, TLS handshake, malformed
   response) map to `HttpsError`.

---

## Trait Surface

```rust
pub trait HttpsTransport: Send + Sync {
    /// Issue a single HTTPS request and return the full response.
    /// The implementation handles TCP connect, TLS handshake, the
    /// HTTP exchange, and connection teardown. Each call is
    /// independent; no connection reuse in v1.
    fn request(&self, req: HttpsRequest) -> Result<HttpsResponse, HttpsError>;
}

pub struct HttpsRequest {
    pub method:           HttpMethod,
    pub url:              String,
    pub headers:          Vec<(String, String)>,
    pub body:             Option<Vec<u8>>,

    pub connect_timeout:  Option<Duration>,   // default 10s
    pub handshake_timeout: Option<Duration>,  // default 10s
    pub read_timeout:     Option<Duration>,   // default 30s
    pub max_body_size:    Option<u64>,        // default 10 MiB

    pub follow_redirects: bool,               // default false
    pub max_redirects:    u8,                 // default 5 (if enabled)
}

pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

pub struct HttpsResponse {
    pub status:    u16,
    pub headers:   Vec<(String, String)>,
    pub body:      Vec<u8>,
}

pub enum HttpsError {
    /// URL did not parse as a valid HTTPS URL.
    InvalidUrl       { url: String, reason: String },
    /// TCP connect failed (DNS, refused, unreachable, etc.).
    TcpConnect       { source: io::Error },
    /// TLS handshake failed.
    Tls              { source: TlsError },
    /// Response head could not be parsed.
    HeadParse        { source: Http1Error },
    /// Response body framing could not be applied (e.g., bad chunked encoding).
    BodyFraming      { source: Http1Error },
    /// Response head exceeded the configured max (default 64 KiB).
    HeadTooLarge     { limit: usize },
    /// Response body exceeded `max_body_size`.
    BodyTooLarge     { limit: u64 },
    /// A timeout fired (connect, handshake, read).
    Timeout          { kind: TimeoutKind, elapsed_ms: u64 },
    /// Connection closed before the response was complete.
    ConnectionClosedEarly,
    /// Capability cage refused the underlying TCP connect.
    CapabilityDenied { detail: String },
    /// I/O error during read or write.
    Io               { source: io::Error },
    /// Followed a redirect to a non-HTTPS URL (we refuse) or to a
    /// host the manifest doesn't permit.
    RedirectRefused  { from: String, to: String, reason: String },
    /// Hit the redirect cap.
    RedirectLimit    { limit: u8 },
}

pub enum TimeoutKind {
    Connect,
    Handshake,
    Read,
}
```

### Convenience methods

```rust
impl dyn HttpsTransport {
    pub fn get(&self, url: &str) -> Result<HttpsResponse, HttpsError> {
        self.request(HttpsRequest::get(url))
    }
    pub fn get_with_headers(&self, url: &str, headers: &[(&str, &str)])
        -> Result<HttpsResponse, HttpsError> { ... }
    pub fn post_json(&self, url: &str, body: &[u8])
        -> Result<HttpsResponse, HttpsError> { ... }
    pub fn post_form(&self, url: &str, fields: &[(&str, &str)])
        -> Result<HttpsResponse, HttpsError> { ... }
}

impl HttpsRequest {
    pub fn get(url: &str) -> Self {
        Self { method: HttpMethod::Get, url: url.into(), ..Default::default() }
    }
    pub fn post(url: &str) -> Self { ... }
    pub fn header(mut self, name: &str, value: &str) -> Self { ... }
    pub fn body(mut self, body: Vec<u8>) -> Self { ... }
    pub fn timeout(mut self, d: Duration) -> Self { ... }
    // builder pattern for ergonomics
}
```

---

## `Http1OverTls` Implementation

The v1 implementation is a single struct that composes the
dependencies. There is no inheritance; just function calls.

```rust
pub struct Http1OverTls {
    manifest:   Arc<Manifest>,
    tls:        Arc<dyn TlsConnector>,
    user_agent: String,
    max_head:   usize,         // default 64 KiB
}

impl Http1OverTls {
    pub fn new(
        manifest: Arc<Manifest>,
        tls:      Arc<dyn TlsConnector>,
    ) -> Self;

    pub fn with_user_agent(mut self, ua: String) -> Self;
    pub fn with_max_head(mut self, bytes: usize) -> Self;
}

impl HttpsTransport for Http1OverTls {
    fn request(&self, req: HttpsRequest) -> Result<HttpsResponse, HttpsError> {
        // 1. Parse URL.
        // 2. TLS connect (which calls secure_net::dial for the TCP).
        // 3. Serialize request head + body via http1.
        // 4. Write to TLS stream.
        // 5. Read response head incrementally; parse via http1.
        // 6. Read response body per http1's framing instructions.
        // 7. Apply size limits and timeouts.
        // 8. Optionally follow redirects (with same manifest checks).
        // 9. close_notify on the TLS stream.
        // 10. Return HttpsResponse.
    }
}
```

### Per-step detail

#### 1. URL parsing

We accept only `https://` URLs. Anything else returns `InvalidUrl`
immediately, before any network activity. From the URL we extract:

- host (the part the manifest gates against; will also be the SNI
  hostname and the cert hostname check)
- port (default 443 if absent)
- path (with query string)
- userinfo segment is rejected (`https://user:pass@host/...`) —
  credentials belong in the Authorization header, not in URLs.

#### 2. TLS connect

```rust
let stream = self.tls.connect(
    host,
    port,
    &TlsConfig {
        min_version:        TlsVersion::Tls12,
        max_version:        TlsVersion::Tls13,
        alpn_protocols:     vec!["http/1.1".into()],
        root_store:         RootStore::SystemDefault,
        server_name:        Some(host.into()),
        verify_mode:        VerifyMode::Strict,
        handshake_timeout:  req.handshake_timeout.unwrap_or(default),
    },
)?;
```

Inside `tls.connect`, the implementation calls
`secure_net::dial(&self.manifest, "tcp", "host:port")` for the TCP
connect. If the manifest doesn't include
`net:connect:host:port`, `dial` returns
`io::Error::other(CapabilityViolationError)` and the TLS connector
maps that to `TlsError::TcpConnect`. We then map that to
`HttpsError::CapabilityDenied`.

ALPN advertises only `http/1.1`. When we add HTTP/2, we'll add
`h2` to the list and switch the upper-half implementation based on
`stream.negotiated_alpn()`.

#### 3. Request head + body serialization

We build the wire bytes:

```
<METHOD> <PATH> HTTP/1.1\r\n
Host: <host>\r\n
User-Agent: <user_agent>\r\n
<additional headers from req.headers, one per line>\r\n
Content-Length: <body.len()>\r\n     (if body present)
Connection: close\r\n
\r\n
<body bytes>
```

Headers are validated:
- Names match `token` per RFC 7230 (no CR/LF, no control chars).
- Values are stripped of leading/trailing whitespace.
- We refuse to send a `Host` header (we always set it from the URL).
- We refuse to send `Content-Length` (we set it from `body.len()`).
- We refuse to send `Connection` (we always set `Connection: close`).
- We refuse to send `Transfer-Encoding` (we don't chunk requests in v1).

Any of the refusal cases would return `InvalidUrl { reason: "header X
is reserved" }` — we re-use the variant for input-shape failures.

#### 4. Write request

Single `write_all` call to the TLS stream. Errors map to
`HttpsError::Io`.

#### 5. Read response head

We read into a growing buffer (start 4 KiB, double on need, up to
`max_head`). After every read we attempt to parse the head with
`http1`. When `http1` reports the head is complete, we extract:

- status code
- headers (preserving order)
- body offset within the buffer (where body bytes start)
- body framing instructions (`none`, `content-length: N`,
  `until-eof`, `chunked`)

If the buffer hits `max_head` without the head completing, we
return `HeadTooLarge`.

If the read returns 0 bytes before any data arrives,
`ConnectionClosedEarly`. If a partial head is followed by EOF,
same.

#### 6. Read response body

Per the framing instructions:

- `none`: response has no body (typically HEAD requests, 204, 304).
- `content-length: N`: read exactly `N` bytes after the head. If
  fewer arrive before EOF, `ConnectionClosedEarly`.
- `until-eof`: read until the server closes the connection (only
  for HTTP/1.0 responses we reply to or `Connection: close` on
  HTTP/1.1 with no Content-Length).
- `chunked`: parse chunked transfer encoding via `http1`'s body
  decoder; concatenate chunks into a single `Vec<u8>`.

In every mode we enforce `max_body_size`. Exceeding it returns
`BodyTooLarge`.

The portion of the head buffer beyond `body_offset` is the start
of the body — we use it before reading more from the stream.

#### 7. Timeouts

Three independent timeouts:
- **Connect timeout** — applied around the TCP connect. The
  underlying `secure_net::dial` (and the `transport-platform`
  socket) supports a deadline; we pass it through.
- **Handshake timeout** — passed to `tls.connect` via
  `TlsConfig.handshake_timeout`.
- **Read timeout** — applied around each read on the TLS stream
  during head and body assembly. If the timer fires mid-read, we
  abort the stream and return `Timeout { kind: Read, ... }`.

Defaults are 10/10/30 seconds. Callers override via
`HttpsRequest`.

#### 8. Redirects (optional)

If `follow_redirects` is true, we examine the response status:

- `301`, `302`, `303`, `307`, `308` with a `Location` header.

For 301/302/303, the next request becomes a `GET` and the body is
dropped (per RFC 7231).

For 307/308, the method and body are preserved.

The new URL must:
- Be HTTPS (no `http://` redirects — we refuse).
- Have a host that the manifest permits (re-check `net:connect`).
- Not exceed `max_redirects` (default 5).

A refused redirect returns `RedirectRefused`. Hitting the cap
returns `RedirectLimit`.

#### 9. Close

We send TLS `close_notify` and let the TLS stream drop. The TCP
socket closes with it.

#### 10. Return

`HttpsResponse { status, headers, body }`.

---

## Capability Cage Integration

Two things, once each:

1. **TCP connect.** `secure_net::dial(&manifest, "tcp",
   "<host>:<port>")` runs inside `tls.connect`. The manifest must
   include `net:connect:<host>:<port>` (with optional glob, e.g.,
   `net:connect:*.weather.gov:443`).

2. **Redirect target re-check.** When following a redirect, the
   new host is re-checked against `net:connect:*` before the new
   TCP connect. A redirect to a host the manifest doesn't permit
   becomes `RedirectRefused`.

That's it. The HTTP layer itself does not perform any new
sandboxed operations beyond the TCP connect; no proc, no fs, no
vault. Headers and body are all in-memory bytes.

---

## Comparison with the Closed Curl-Bridge Spec (#2240)

Same trait surface; entirely different implementation. The trait
was deliberately specified so that a curl-bridge or an in-process
implementation would both be valid. We chose the in-process route
because:

- **No subprocess overhead.** Per-request spawn of curl was
  ~5-15 ms. In-process is ~microseconds.
- **OS-managed trust without a binary dependency.** Both
  approaches use the system trust store; the in-process path does
  it via the OS TLS API directly rather than via curl's wrappers.
- **Debuggable end to end.** Stepping through a request stays in
  Rust and a single TLS implementation; no shell-out boundary.
- **Composes with our own primitives.** We get to test
  `http1`-on-real-bytes and `tls-platform`-on-real-bytes via a
  user-facing trait, exercising both daily.

The previous spec's CurlBridge code path is gone. If someone needs
a curl-bridge for some reason (debugging a backend mismatch,
running against a proxy that requires curl-specific flags), they
can implement `HttpsTransport for CurlBridge` as a separate
package and wire it in at the consumer's constructor — the trait
permits it.

---

## Test Strategy

### Unit Tests

1. **URL parsing.**
   - `https://api.weather.gov/points/47.6,-122.3` → host, port=443, path.
   - `https://example.com:8443/x?q=1` → custom port, path with query.
   - `http://example.com/` → `InvalidUrl`.
   - `https://user:pass@host/` → `InvalidUrl` (userinfo not allowed).
   - Missing host → `InvalidUrl`.
2. **Request serialization.**
   - GET with no body produces the exact wire bytes expected
     (compared to a captured fixture).
   - POST with body adds `Content-Length` correctly.
   - Reserved headers (`Host`, `Content-Length`, `Connection`,
     `Transfer-Encoding`) supplied by the caller are rejected.
3. **Response head parsing.**
   - Standard 200/headers/empty-body case.
   - 304 with no body.
   - Chunked response with multiple chunks.
   - Response with extra trailing whitespace in headers.
4. **Body framing.**
   - `Content-Length: N` reads exactly N.
   - `Transfer-Encoding: chunked` reassembles correctly.
   - Connection close with no Content-Length reads `until-eof`.
5. **Size limits.**
   - Response with > `max_head` head returns `HeadTooLarge`.
   - Response with > `max_body_size` body returns `BodyTooLarge`.
6. **Timeouts.** Each timeout kind fires correctly given a slow
   mock TLS stream.
7. **Redirect handling.**
   - 301 Location → fetched.
   - 302 to `http://` → `RedirectRefused`.
   - Redirect to a manifest-denied host → `RedirectRefused`.
   - 6 redirects with `max_redirects = 5` → `RedirectLimit`.
8. **Capability denial.** Manifest without `net:connect:host:443`
   → `CapabilityDenied`, no TCP connection opened.

### Integration Tests

9. **End-to-end against a local HTTPS server.** Spin up a tiny
   test HTTPS server with a fixture cert (loaded via
   `RootStore::Custom`); issue GET, POST, redirected GET; verify
   round-trip.
10. **Real internet test (gated).** With explicit opt-in, fetch
    `https://api.weather.gov/points/47.6,-122.3` and verify a 200
    with non-empty JSON body.
11. **TLS failure.** Connect to a server with an invalid cert →
    `Tls { CertVerifyFailed }`.
12. **Hostname mismatch.** Connect to a server whose cert doesn't
    cover the requested host → `Tls { HostnameMismatch }`.
13. **Concurrent requests.** Issue 10 simultaneous requests from
    different threads; verify all complete and return distinct
    bodies.
14. **Cross-backend.** The same suite runs against each
    `tls-platform` backend (Schannel, Network.framework, OpenSSL)
    on its respective OS — confirms behavior parity at the HTTPS
    layer.

### Coverage Target

`>=90%` line coverage for the implementation. Defensive parsing
and timeout handling deserve thorough tests; redirect handling is
small but error-prone.

---

## Trade-Offs

**No connection pooling in v1.** Each request opens a fresh TCP +
TLS. For an agent making one request per day (the weather agent),
this is invisible. For a high-rate agent it costs the TLS
handshake (~50-300 ms) per request. A future revision adds
pooling with proper connection lifecycle.

**HTTP/1.1 only in v1.** Many real-world endpoints (api.weather.gov,
api.openai.com, api.github.com) negotiate HTTP/2 by default in
modern clients. ALPN advertises only `http/1.1` for now; servers
must accept that. Adding HTTP/2 means an `Http2OverTls`
implementation behind the same trait — not a breaking change.

**Body fits in memory.** The full response body lands in a
`Vec<u8>` before returning. For 10 MiB JSON or HTML, fine. For
multi-GB downloads, wrong. Future streaming `request_stream` API
returns a `Reader` for incremental consumption.

**No request streaming.** Requests with bodies are serialized
fully before sending. For uploading a large file, wrong; future
work.

**No automatic retries.** A connection reset by peer returns
`Io`. The agent decides whether to retry. We considered an
opt-in `retry_policy` field but defer it to a higher-layer
middleware so it doesn't entangle with the core protocol.

**`Connection: close` always.** We do not negotiate keep-alive in
v1 because we don't pool connections. When pooling lands, this
flips to `Connection: keep-alive` for poolable sessions.

**ALPN only `http/1.1`.** We don't negotiate `h2` because we
can't speak it yet. A 2.0 implementation could be added under
the same trait by reading `stream.negotiated_alpn()` and
dispatching to a different upper half.

**Reserved-header rejection is strict.** Callers cannot supply
their own `Host`, `Content-Length`, `Connection`, or
`Transfer-Encoding` headers. This avoids ambiguity (which one
wins if the caller sets one and we set another?). The cost is
minor inflexibility; the benefit is unambiguous wire bytes.

**Userinfo URLs rejected.** `https://user:pass@host/` is
disallowed at parse time. Credentials belong in the Authorization
header (which the caller can set freely). This avoids leaking
credentials through logs that include URLs.

---

## Future Extensions

- **Connection pooling** — keyed by (host, port, ALPN). Configurable
  max idle and max-per-key.
- **HTTP/2 and HTTP/3** — sibling implementations behind the same
  trait, dispatch by ALPN.
- **Streaming requests / responses** — `request_stream` returning
  a `Reader`; body is consumed incrementally.
- **Per-request retry policy** — opt-in middleware-style retry on
  idempotent methods.
- **Cookie jar** — opt-in stateful cookie handling for agents that
  need it (most don't).
- **Compression** — accept `gzip`, `br`, `deflate` and decompress
  transparently; advertise via `Accept-Encoding` when enabled.
- **mTLS** — client certificates passed through `TlsConfig`.

These are deliberately out of scope for v1.
