# HTTPS Transport

## Overview

The HTTPS Transport is the durable seam between agent code that needs
to make outbound HTTPS requests and the underlying mechanism that
performs them. Today, that mechanism is **`curl`** invoked as a
subprocess; tomorrow it can be a native Rust TLS stack we write in
this repository. The agents and hosts that consume this transport see
exactly one thing — a `HttpsTransport` trait — and never know which
implementation is behind it.

This is the same pattern you used for the storage abstraction in
D18A: a single trait, multiple backends, the consumers depend on the
trait. Here the trait is `HttpsTransport`, and the two known backends
are `CurlBridge` (today) and `NativeTls` (future). Other reasonable
backends include `WindowsWinHttp` (using the Windows HTTP API
directly) and a `MockTransport` for tests. They all satisfy the same
contract.

The contract is durable; the implementation behind it is not. When
we eventually write native TLS in this repository, swapping the
backend is a one-line change at the consumer's wiring point. No
agent code, no host runtime code, no test changes.

This spec defines:

1. The `HttpsTransport` trait and the request/response/error vocabulary.
2. The `CurlBridge` implementation — what `curl` arguments it
   constructs, how it parses curl output, what failure modes it
   surfaces.
3. The capability-cage integration: every call goes through
   `secure_proc::command(manifest, "curl", ...)` so the manifest
   gates which hosts and ports the agent can reach.
4. The expected failure modes and their mappings to `HttpsError`.
5. The migration path to a native backend without disturbing
   callers.

---

## Where It Fits

```
   Agent code (e.g., weather-host)
        │
        │  uses HttpsTransport::get / post / ...
        ▼
   HttpsTransport trait    ← THIS SPEC
        │
        ▼
   CurlBridge impl  (today)         NativeTls impl (future)
        │                                  │
        │  spawns curl subprocess          │  uses our TLS stack
        ▼                                  ▼
   secure_proc::command (capability-cage)  network-stack + future TLS
        │
        ▼
   external HTTPS endpoint
```

**Depends on:**
- `capability-cage-rust` — `secure_proc::command` for the curl spawn.
- `process-manager` — for the underlying spawn / wait / capture.
- `json-parser`, `json-value` — for parsing curl's `--write-out` JSON.
- `time` — for timeouts.

**Used by:**
- `weather-host` — fetches forecasts from `api.weather.gov`.
- `email-host` — talks to Gmail SMTP via curl's `smtps://` mode (see
  `smtp-transport.md` — the same `CurlBridge` pattern).
- Any future agent that needs HTTPS without us shipping a native
  TLS implementation.

---

## Design Principles

1. **Trait is the durable artifact.** Implementations may come and
   go. Callers depend on the trait, never on `CurlBridge` or any
   other concrete backend.

2. **Capability-cage gated.** Every request goes through
   `secure_proc::command`, which checks the agent's manifest for
   `proc:exec:curl`. The manifest also constrains which network
   targets the agent can reach (the host runtime's middleware
   rejects requests to URLs whose host isn't in `net:connect:*`,
   even when the underlying mechanism is curl).

3. **No persistent connections.** Each request spawns a fresh
   subprocess. No connection pooling, no shared TLS sessions, no
   keep-alive across requests. This matches the "ephemeral
   sub-agent" pattern in `host-runtime-rust.md`: the spawn lives
   for one request and dies.

4. **Errors are structured.** Every failure is an
   `HttpsError` variant with enough information to handle
   programmatically. We do not propagate raw curl exit codes or
   stderr text — we map them.

5. **Sandbox compatibility.** The CurlBridge expects the agent's
   sandbox to allow exec of curl. Where the OS sandbox forbids it
   (e.g., Tier 2 WASM agents that can't spawn anything), the
   agent must use a different transport (e.g., a host-mediated
   `network.fetch` that runs the curl spawn in the host process,
   not in the agent).

6. **Streaming optional in v1.** The trait surface accepts a
   streaming response interface, but the v1 CurlBridge buffers the
   entire response before returning. Streaming arrives in v2 with
   curl's `--no-buffer` plus a worker thread reading stdout.

---

## The Trait

```rust
pub trait HttpsTransport: Send + Sync {
    /// Issue a single HTTPS request. The implementation handles
    /// connection setup, TLS negotiation, the request/response
    /// exchange, and connection teardown. Each call is independent.
    fn request(&self, req: HttpsRequest) -> Result<HttpsResponse, HttpsError>;
}

pub struct HttpsRequest {
    pub method:    HttpMethod,
    pub url:       String,
    pub headers:   Vec<(String, String)>,
    pub body:      Option<Vec<u8>>,
    pub timeout:   Option<std::time::Duration>,
    pub follow_redirects: bool,
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
    /// The URL was syntactically invalid before any network call.
    InvalidUrl       { url: String, reason: String },
    /// DNS resolution failed.
    DnsFailure       { host: String },
    /// The TCP connection could not be established.
    ConnectFailure   { host: String, port: u16, message: String },
    /// TLS handshake failed (cert mismatch, expired cert, protocol mismatch, etc.).
    TlsFailure       { host: String, message: String },
    /// The request timed out.
    Timeout          { elapsed_ms: u64 },
    /// The body exceeded a configured maximum.
    BodyTooLarge     { limit_bytes: u64 },
    /// The capability cage refused the call (proc:exec:curl missing,
    /// or the URL host is not in the manifest).
    CapabilityDenied { detail: String },
    /// The underlying spawn / read / wait failed.
    Spawn            { source: std::io::Error },
    /// The transport ran but the response could not be parsed.
    Parse            { message: String },
    /// Anything else from the backend (e.g., curl exit code we
    /// don't have a more specific mapping for).
    Backend          { code: i32, stderr_excerpt: String },
}
```

### Convenience methods

```rust
impl dyn HttpsTransport {
    pub fn get(&self, url: &str) -> Result<HttpsResponse, HttpsError> {
        self.request(HttpsRequest {
            method: HttpMethod::Get,
            url:    url.to_string(),
            headers: vec![],
            body:    None,
            timeout: None,
            follow_redirects: false,
        })
    }

    pub fn get_with_headers(
        &self, url: &str, headers: &[(&str, &str)],
    ) -> Result<HttpsResponse, HttpsError> { ... }

    pub fn post_json(
        &self, url: &str, body: &[u8],
    ) -> Result<HttpsResponse, HttpsError> { ... }
}
```

These exist purely for ergonomics; they construct a `HttpsRequest`
and delegate to `request()`. Implementations override only
`request()`.

---

## CurlBridge Implementation

`CurlBridge` is the v1 implementation. It spawns `curl` as a
subprocess for each call, reads its stdout, parses an exit code, and
returns the response or maps the failure.

### Construction

```rust
pub struct CurlBridge {
    manifest:    Arc<Manifest>,
    curl_path:   PathBuf,                 // default: "curl" (PATH lookup)
    user_agent:  String,                  // default: "coding-adventures/0.1"
    max_body:    u64,                     // default: 10 MiB
    default_timeout: std::time::Duration, // default: 30s
}

impl CurlBridge {
    pub fn new(manifest: Arc<Manifest>) -> Self;

    pub fn with_curl_path(mut self, path: PathBuf) -> Self;
    pub fn with_user_agent(mut self, ua: String) -> Self;
    pub fn with_max_body(mut self, bytes: u64) -> Self;
    pub fn with_default_timeout(mut self, d: std::time::Duration) -> Self;
}
```

The `manifest` is **the agent's manifest**. Every spawn calls
`secure_proc::command(&manifest, "curl", &args)`, so the manifest
must include `proc:exec:curl` for the bridge to function.

### Argv construction

For a `HttpsRequest`, `CurlBridge` builds:

```
curl
  --silent                            (no progress bar)
  --show-error                        (still print errors to stderr)
  --location? if follow_redirects     (otherwise no flag)
  --max-time <seconds>                (req.timeout or default_timeout)
  --max-filesize <bytes>              (max_body)
  --user-agent "<user_agent>"
  --request <METHOD>                  (GET/POST/etc.)
  --header "Header-Name: value"       (one per req.headers entry)
  --data-binary @<tmpfile>            (only if body present)
  --write-out '%{json}'               (curl emits a JSON metadata line)
  --output <body_tmpfile>             (response body goes here)
  --                                  (end of options, URL is positional)
  <req.url>
```

Notes on individual flags:

- **`--silent --show-error`** — quiet under normal operation,
  loud only on actual errors.
- **`--max-time`** — total timeout including TLS handshake. Caller
  can override per-request; otherwise the default applies.
- **`--max-filesize`** — abort if the response body would exceed
  the configured cap. Returns curl exit code 63 which we map to
  `BodyTooLarge`.
- **`--data-binary @<tmpfile>`** — request bodies are written to a
  temp file rather than passed via stdin. This avoids issues with
  binary data in shells and lets curl handle exact byte ordering.
  The temp file is created with `secure_file::create_file` against
  a small dedicated dir (`./.tmp/curl-bodies/<uuid>`), used once,
  and deleted via `secure_file::delete_file` after the call.
- **`--write-out '%{json}'`** — curl emits a single JSON document
  on stdout summarizing the request: HTTP status, header sizes,
  timing breakdown, redirect count, etc. We parse this for status,
  headers (extracted separately, see below), and timing.
- **`--output <body_tmpfile>`** — the response body goes to a
  temp file, not stdout. This separates the body bytes from the
  `--write-out` JSON. Read the file with `secure_file::read_file`.
- **`--header`** — request headers passed individually; we never
  concatenate user input.
- **`--`** — unambiguously separates URL from flags so a malicious
  URL beginning with `-` can't be mistaken for a flag.

### Response header capture

curl's `%{json}` output does not include the response headers
themselves (only their sizes). To capture them, the bridge also
passes:

```
--dump-header <hdr_tmpfile>
```

After curl exits, the bridge reads the header file (one header per
line, each `Name: value\r\n`), parses into a `Vec<(String, String)>`,
and includes them in `HttpsResponse.headers`.

### Exit-code mapping

Curl's exit codes (documented in `man curl`) map to `HttpsError` as
follows:

| curl exit | Meaning                                    | HttpsError           |
|-----------|--------------------------------------------|----------------------|
| 0         | Success                                    | (no error)           |
| 3         | URL malformed                              | `InvalidUrl`         |
| 6         | DNS resolution failed                      | `DnsFailure`         |
| 7         | Connect failure                            | `ConnectFailure`     |
| 28        | Timeout                                    | `Timeout`            |
| 35        | TLS handshake failure                      | `TlsFailure`         |
| 51 / 60   | Peer cert verify failed                    | `TlsFailure`         |
| 63        | `--max-filesize` exceeded                  | `BodyTooLarge`       |
| 22        | HTTP error (4xx/5xx) — only with `--fail`  | (we don't pass --fail; HTTP errors are returned in `status`) |
| any other | Generic backend error                      | `Backend`            |

We deliberately **do not** pass `--fail`. HTTP-level errors
(404, 500, etc.) are returned as `HttpsResponse` with a non-2xx
`status`. Only true transport-layer failures map to `HttpsError`.

### Timing budget

```
spawn curl  ──── ~ 5-15 ms     (process creation + binary load)
TLS handshake ── ~ 50-300 ms   (depends on RTT and cert chain)
HTTP exchange ── ~ 100-2000 ms (depends on remote responsiveness)
parse output ─── ~ 1-5 ms      (read tmpfiles, parse JSON, parse headers)
cleanup ──────── ~ 1-5 ms      (delete tmpfiles)
```

Totals are dominated by remote latency, exactly as a native
implementation would be. The spawn overhead is real but not
significant for the agent workloads we care about (a weather
agent making one HTTPS call per day will not notice).

### Concurrent calls

CurlBridge is `Send + Sync`. Multiple agent threads may call
`request()` concurrently; each spawns its own curl process. The
manifest check is read-only and contention-free. The temp-file
directory uses a UUID per call so there is no collision.

---

## Capability-Cage Integration

The bridge's spawn always goes through `secure_proc::command`:

```rust
let cmd = secure_proc::command(
    &self.manifest,
    self.curl_path.to_str().unwrap(),
    &argv,
)?;
```

On manifest violation (`proc:exec:curl` missing), the call returns
`HttpsError::CapabilityDenied { detail: "proc:exec:curl not in manifest" }`
without spawning anything.

Beyond `proc:exec:curl`, the host runtime's middleware additionally
checks the URL's host against the agent's `net:connect:*` entries
**before** the bridge is even invoked. The URL `https://api.weather.gov/...`
is parsed by the middleware; the host (`api.weather.gov`) and port
(443, default for HTTPS) are constructed; and `manifest.check(Net,
Connect, "api.weather.gov:443")` is called. A denial returns
`CapabilityDenied` to the agent before any subprocess is spawned.

This is the defense-in-depth principle in action: two independent
checks (`net:*` and `proc:exec:curl`), each capable of denying the
operation.

---

## Migration to NativeTls

When we eventually write a native TLS stack in this repository, the
migration is:

1. Implement `NativeTls` as `impl HttpsTransport for NativeTls`.
2. The wiring point — typically the constructor of `weather-host`
   or any other consumer — changes from
   ```rust
   let http: Arc<dyn HttpsTransport> =
       Arc::new(CurlBridge::new(manifest));
   ```
   to
   ```rust
   let http: Arc<dyn HttpsTransport> =
       Arc::new(NativeTls::new(manifest));
   ```
3. Remove `proc:exec:curl` from the agent's manifest; add the
   network capabilities the native stack needs (it should need the
   same `net:connect:*` entries the host already had).
4. No changes to agent business logic. No changes to tests that
   used the trait. Only the implementation switches.

The trait was designed to make this swap cheap.

---

## Test Strategy

### Unit Tests

1. **Argv construction.** For each method (GET/POST/PUT/etc.) and
   combinations of headers, body, timeout, and follow-redirects,
   verify the argv array exactly matches expectations.
2. **Header parsing.** Realistic header dumps (with continuations,
   duplicate headers, unusual whitespace) parse correctly.
3. **JSON metadata parsing.** A captured `%{json}` output parses
   into status, timing, etc.
4. **Exit-code mapping.** Every documented curl exit code maps to
   the expected `HttpsError` variant.
5. **Capability denial.** Manifest without `proc:exec:curl` →
   `CapabilityDenied`, no spawn attempted.
6. **Tempfile cleanup.** After every call, no body or header
   tempfiles remain on disk.
7. **URL safety.** A URL beginning with `-` is correctly delimited
   by `--` and not interpreted as a flag.

### Integration Tests

8. **End-to-end against a local HTTPS server.** Spin up a tiny test
   HTTPS server with a self-signed cert; verify curl is invoked
   with `--cacert` (a future ext) or `--insecure` (test only) and
   the response is parsed correctly.
9. **Real internet test (gated).** With explicit opt-in, fetch
   `https://api.weather.gov/points/47.6,-122.3` and verify a 200
   with non-empty JSON body.
10. **Timeout.** Issue a request with `timeout = 50ms` against a
    server that delays 200ms; verify `Timeout`.
11. **Body too large.** Issue a request against a server that
    streams more than `max_body`; verify `BodyTooLarge`.
12. **TLS failure.** Issue a request to a server with an invalid
    cert; verify `TlsFailure`.
13. **Concurrent calls.** Issue 10 simultaneous requests from
    different threads; verify all complete and return distinct
    bodies.

### Coverage Target

`>=90%` line coverage on the bridge logic. Curl's behavior is
external; we test argv construction and parsing exhaustively, and
real network calls in a gated integration suite.

---

## Trade-Offs

**curl as a subprocess is slow per-call.** Process spawn + binary
load adds 5-15ms per request. For an agent making one request per
day, this is invisible. For a high-throughput agent, this would
be a problem; such agents should wait for the native TLS backend.

**curl must be present on the system.** On Windows 10/11 it is in
`%SystemRoot%\System32\curl.exe`; on macOS it ships with the OS;
on most Linux distros it is preinstalled. For systems where it is
not, the agent fails at first request with `Spawn { NotFound }`.
We accept this; the user installs curl.

**No connection pooling.** Each request opens a new TLS connection.
For an agent calling the same endpoint many times in quick
succession, this is wasteful. We accept it for v1 because the
"ephemeral sub-agent" pattern means we'd have to plumb the pool
across host-runtime ephemeral spawns anyway, which is non-trivial.

**Body in tempfile.** Request bodies travel through a temp file
rather than the more elegant stdin pipe. This avoids shell escaping
issues and gives curl a precise byte count via the file size. The
cost is two extra fs syscalls per call.

**No streaming responses.** The full body is captured to a file,
read, and returned as a `Vec<u8>`. For a 10 MiB body that is
~100 ms of additional I/O on top of the network time. Acceptable
for v1; v2 streaming will use `--no-buffer` plus a reader thread.

**Cert validation uses curl's defaults.** That means curl's bundled
CA store is the trust anchor. On systems where curl uses the OS
trust store (Windows, modern macOS), this is fine. On minimal Linux
containers, the OS trust store may be missing and curl falls back
to its bundled bundle. We document this; if it becomes a problem,
the bridge can pass `--cacert` to a known-good bundle.

**Capability cage gates the spawn but not the URL.** The URL gating
happens in the host runtime middleware, not in the bridge itself.
The bridge trusts the host to have already validated the URL host
against `net:connect:*`. A bug in the host that lets a malicious
URL reach the bridge would still be caught by the orchestrator's
sandbox layer, which restricts where the host process itself can
talk.

---

## Future Extensions

- **NativeTls backend** — the obvious follow-on; replaces curl
  with our own TLS 1.3 implementation.
- **Connection pooling** — both for CurlBridge (via `--keepalive`
  in a long-lived helper process) and NativeTls (in-process).
- **Streaming responses** — a `request_stream` method that returns
  a `Reader` for incremental body consumption.
- **HTTP/2 and HTTP/3** — by passing `--http2` or `--http3` to curl
  in v1; native impl gets these later.
- **Upload streaming** — pass an in-memory stream to curl via
  `@-` (stdin) without the tempfile detour.
- **Custom CA bundle** — for environments that need pinned trust
  anchors.

These are deliberately out of scope for V1.
