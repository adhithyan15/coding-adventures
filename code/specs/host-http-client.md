# Host HTTP Client

## Overview

`host-http-client` is the defensive HTTP/HTTPS client that lives
**inside the host process** between agent code (which calls
`host.network.fetch`) and the underlying transport
(`https-transport`). It exists to add a layer of protection
against **attacker-controlled remote sites** that try to do
something malicious with the agent's network request: SSRF
attempts, oversized responses, decompression bombs, smuggling
attacks, weird headers, redirect loops to private addresses,
and dozens of similar tricks.

The structure is:

```
   Agent code
        │
        │  host.network.fetch(url, opts)
        ▼
   host-http-client (this spec)            ← defense layer
        │  - manifest check                 (R1: capability cage)
        │  - URL hardening
        │  - request hardening
        │  - response hardening
        │  - decompression-bomb defense
        │  - response classification
        │
        ▼
   https-transport (Http1OverTls)         ← protocol layer
        │
        ▼
   tls-platform-windows / etc.            ← TLS layer
        │
        ▼
   the network
```

`https-transport` already does some defensive parsing: head size
limit, body size limit, timeouts, redirect handling, refused
non-HTTPS redirects. The host-http-client is a **second ring**
that:

- Adds checks `https-transport` is too low-level to know about
  (e.g., "is this redirect target a private IP?").
- Tightens the defaults `https-transport` permits (e.g., we cap
  bodies at the host level even tighter, depending on the
  agent's per-method budget).
- Strips response data the agent should not see (cookies,
  Set-Cookie, Server-Timing, anything that could leak via the
  agent's logs).
- Bounds resource consumption per request and per agent.
- Is the **single point** where audit records about network
  requests are emitted to the orchestrator's audit channel.

This is the defense-in-depth principle from `host-runtime-rust.md`
applied specifically to outbound HTTP. The agent is in a Rust
cage with a manifest. The manifest is enforced by
`capability-cage-rust`. The TCP+TLS+HTTP exchange goes through
`https-transport` which has its own defensive parsing. And on top
of all that, the host-http-client adds the **request-level
hardening that only the host knows enough to do** — because the
host is the broker that sees every request and can reason about
the cumulative pattern.

If `https-transport` is "the safe HTTP library," the
host-http-client is "the safe HTTP library wrapped in a hostile-
internet-aware reverse proxy that we run inside our own host."

---

## Where It Fits

```
   Agent (Tier 1 native, or Tier 2 WASM, or Tier 3 subprocess)
        │
        │  host.network.fetch(url, opts) → response
        ▼
   host-http-client (this spec)
        ├── manifest check (cage)
        ├── URL canonicalization + reject-unsafe-targets
        ├── request hardening (header allowlist, body cap, timeout cap)
        ├── per-agent rate limiter integration
        ├── delegate to https-transport
        ├── response hardening (decompress safely, header strip)
        ├── audit emit
        └── return to agent
            │
            ▼
   https-transport (Http1OverTls)
            │
            ▼
   tls-platform-windows (Schannel)
            │
            ▼
   transport-platform (TCP)
            │
            ▼
   the internet
```

**Depends on:**
- `https-transport` — the actual HTTP/1.1+TLS exchange.
- `capability-cage-rust` — `secure_net::dial` (transitively via
  https-transport) and the manifest check directly.
- `secure-host-channel` — the rate-limiter is shared with the
  channel layer's token bucket.
- `time` — timeouts and per-agent budget windows.
- `json-parser`, `json-value` — for parsing canonicalized URL
  components and response Content-Type headers.

**Used by:**
- `host-runtime-rust` instantiates a `HostHttpClient` per agent
  process. Every `host.network.fetch` call from the agent runs
  through it.
- The host-http-client is **not** exposed publicly to agents as a
  separate crate; agents see only the `host::network::fetch`
  SDK surface, which is implemented in terms of this client.

---

## Design Principles

1. **Defense in depth.** `https-transport` is one ring. This is
   another. A bug in either does not produce a vulnerability if
   the other still holds.

2. **Reject early, fail loud.** Defensive rules are enforced
   before any network activity. A URL that fails canonical
   validation never reaches `https-transport`. A response that
   fails post-fetch validation is not returned to the agent;
   instead the agent gets `HttpResponseRejected`.

3. **Treat the response as adversarial.** Even from a host the
   agent's manifest permits (`api.weather.gov`), the response
   bytes are still attacker-influenceable (compromise, MITM
   despite TLS, or the host itself acting badly). Headers are
   stripped to a tiny allowlist; body is size-bounded; encoding
   is validated; embedded sub-resource references are not
   followed automatically.

4. **Per-agent resource bounds.** The client tracks per-agent
   request count, total bytes downloaded, and total wall-clock
   spent inside `fetch`. Agents that exceed budgets get
   `Budgeted` errors before the next request even tries.

5. **Audit every request, every response, every rejection.** The
   orchestrator's audit log is the source of truth for "what did
   this agent do on the network." Every `fetch` call produces
   one audit record with the request shape, the response status,
   the byte counts, and any rejection reason.

6. **Single configuration knob set, declared per-agent.** The
   tunables (max body, max redirects, header allowlist, etc.)
   live in the agent's manifest under a `network` section that
   the cage validates. Defaults are conservative; agents
   override with justifications.

7. **Decompression is a privileged operation.** Compressed
   responses (gzip, brotli, deflate) are decompressed by the
   client only after a per-request budget check. Decompression
   bombs (small compressed bytes that expand to gigabytes) are
   the most common DOS vector against HTTP clients; we cap the
   decompressed size and the compression ratio.

---

## URL Hardening (pre-request)

Every URL is canonicalized and validated before any network
call. The validations:

### 1. Scheme is HTTPS only.

`http://` URLs are rejected with `Invalid::SchemeNotHttps`.
There is no plaintext-HTTP path through this client.
(Use a different host method if you genuinely need plaintext.)

### 2. Userinfo rejected.

`https://user:pass@example.com/...` is rejected with
`Invalid::UserinfoNotAllowed`. Credentials belong in headers.

### 3. Host is a public DNS name or a public IP, not a private one.

The client maintains a small hard-coded list of forbidden
address ranges and refuses any URL whose host (after DNS
resolution if it is a hostname) lies in one of them. The list:

- 10.0.0.0/8
- 172.16.0.0/12
- 192.168.0.0/16
- 127.0.0.0/8 (localhost)
- 169.254.0.0/16 (link-local, including the AWS/Azure metadata service)
- 224.0.0.0/4 (multicast)
- 0.0.0.0/8
- ::1/128, fc00::/7, fe80::/10, ::/128 (IPv6 equivalents)

Any URL targeting these returns
`Invalid::PrivateAddressForbidden`. Agents that genuinely need
to reach a private host (a development tool, a local test
server) declare so via an explicit manifest annotation
(`net:connect:127.0.0.1:8080` with `private_address: true`).
The annotation requires Tier 1 challenge at registration so the
user sees it.

This is the **SSRF defense**. The most common server-side
request forgery exploit is "make the agent fetch
http://169.254.169.254/latest/meta-data/iam/security-credentials/"
to get cloud credentials. The host-http-client refuses, before
any DNS lookup happens.

### 4. Host is in the manifest's `net:connect` allowlist.

This duplicates `https-transport`'s check but happens **earlier**
(before DNS resolution that could leak which host we are about
to talk to even if the connection then fails). The check is on
the literal host as written in the URL.

### 5. Path is normalized.

`..`, repeated `/`, percent-encoded path traversal — all
canonicalized. The agent sees the canonicalized URL in the
audit record.

### 6. Query string is bounded.

Total URL length capped at 2048 chars by default. Per-agent
override available in the manifest.

### 7. No credentials in query.

A query string containing parameter names like `token`, `key`,
`password`, `secret`, `api_key`, `apikey`, `access_token`,
`auth`, `bearer` is rejected with `Invalid::CredentialsInQuery`.
The client's reasoning: secrets in URLs land in server access
logs, browser history, referer headers, and audit records.
Agents that genuinely need to send a token use an
Authorization header. The list of forbidden parameter names is
configurable per-agent.

---

## Request Hardening (pre-send)

### Header allowlist

Only headers in a per-method allowlist are permitted. Default
allowlist for GET:

- `Accept`
- `Accept-Encoding`
- `Accept-Language`
- `Authorization` (Bearer / Basic / Digest only — never raw
  username:password format)
- `Cache-Control`
- `If-Modified-Since`
- `If-None-Match`
- `Range`
- `Referer` (only with explicit per-request opt-in; default off)
- `User-Agent`

For POST/PUT/PATCH:
- everything in the GET set, plus:
- `Content-Type`
- `Content-Length` (set by the client, not the caller — supplied
  values rejected)
- `Content-Encoding` (gzip/deflate/br only)

Headers outside the allowlist are rejected at request time. The
agent gets `Invalid::HeaderForbidden { name: "..." }`. The
allowlist is overridable per-agent via the manifest, with each
extra header requiring justification.

### Method allowlist

GET/HEAD/POST/PUT/DELETE/PATCH only. CONNECT/TRACE/OPTIONS
rejected. CONNECT can tunnel arbitrary traffic; TRACE leaks
request data; OPTIONS is rarely needed and adds attack surface
for poorly-configured CORS endpoints.

### Body size cap

Default 100 KiB (typical request bodies are way smaller). Agent
manifest can raise it with justification.

### Total request size cap

URL + headers + body, default 200 KiB.

### Caller cannot set Cookie or Cookie2 headers

The agent cannot send cookies. We are not a browser; we are an
API client. Each request is independent. Cookies that would
otherwise be persistent are explicitly forbidden.

### Authorization header validation

If `Authorization` is supplied, it must match one of:
- `Bearer <opaque-token>`
- `Basic <base64-credentials>`
- `Digest <params>`
- `Token <opaque-token>` (some legacy APIs use this)

Anything else is rejected. This catches misconfigurations that
would put credentials in unexpected formats and helps the audit
record classify them properly (without logging the secret).

---

## Response Hardening (post-fetch)

### Body size cap

The default per-fetch is 5 MiB; per-agent override allowed but
capped at 50 MiB. `https-transport` enforces its own cap; ours
is tighter and per-call.

### Decompression-bomb defense

If `Content-Encoding` is `gzip`, `br`, or `deflate`, the client
decompresses internally. During decompression:

- After every 64 KiB of decompressed output, check:
  - Total decompressed > body size cap → abort with
    `DecompressionBomb`.
  - Compression ratio (decompressed_so_far / compressed_input_so_far)
    > 100 → abort with `DecompressionBomb`. Real-world ratios
    for natural text/JSON are 3-10× on gzip; 100 is well above
    pathological.
- Maximum total decompressed always capped at 5x the body size
  cap (so 5 MiB cap → 25 MiB max decompressed even if ratio
  stays low until the end).

The agent receives the decompressed bytes. The
`Content-Encoding` header is removed before returning to the
agent (the agent is reading plain bytes; lying about encoding
would be confusing).

### Header allowlist (response side)

The agent sees only a small allowlist of response headers:

- `Content-Type`
- `Content-Length` (computed from the actual body, not what the
  server sent)
- `Last-Modified`
- `ETag`
- `Cache-Control` (informational; we don't act on it)
- `Date`

Everything else is **stripped** before return:

- `Set-Cookie`, `Set-Cookie2`: stripped. We are not a browser.
- `Server`, `X-Powered-By`: stripped. Identifies vendor; agent
  does not need it.
- `WWW-Authenticate`: stripped (the agent does not negotiate
  auth challenges; explicit Authorization at request time).
- `X-*`, `Server-Timing`, `Link`, `Strict-Transport-Security`,
  `Content-Security-Policy`, etc.: all stripped.
- Any custom header: stripped unless explicitly allowed in the
  agent's per-host header allowlist.

The audit record retains the full response header set for
forensics; only what is returned to the agent is stripped.

### Content-Type discipline

The client refuses to return responses with a `Content-Type`
not in the agent's per-request expected list. By default for
JSON-fetching agents, the expected list is:

- `application/json`
- `application/geo+json`
- `application/ld+json`
- `text/json`
- `text/plain` (for endpoints that lazily return plaintext)

A response with `Content-Type: text/html` to a JSON-expecting
agent gets `Invalid::UnexpectedContentType`. This catches
common attacker tricks where a compromised endpoint serves
HTML/JS/binary in place of expected JSON to confuse a
poorly-validated downstream parser.

The expected list is overridable per request via the agent's
opts.

### Redirect re-checking

`https-transport` already re-checks the manifest on each
redirect. The host-http-client adds:

- The redirect URL goes through the same URL-hardening pipeline
  (no private addresses, no userinfo, no credentials in query,
  etc.) before being followed.
- A redirect cap that is **tighter than https-transport's** by
  default (3 instead of 5).
- A redirect that changes hostname is logged at higher
  verbosity in the audit log.

### Timing budget

Total wall clock for a single `fetch` (including all redirects,
TLS handshake, body read, decompression) is capped at the
agent's per-call budget (default 30s). Exceeding it returns
`Timeout`.

---

## Per-Agent Budgets

Beyond per-request limits, the client tracks per-agent
cumulative usage:

```rust
struct AgentBudget {
    /// Number of fetch calls in the current window.
    requests_per_minute:    u32,
    requests_per_hour:      u32,

    /// Total bytes downloaded in the current window.
    bytes_per_minute:       u64,
    bytes_per_hour:         u64,

    /// Total wall-clock spent inside fetch.
    fetch_seconds_per_min:  u32,
    fetch_seconds_per_hour: u32,
}
```

Defaults (per agent, all configurable in the manifest's
`network` section):

```
requests_per_minute       60
requests_per_hour         600
bytes_per_minute          10 MiB
bytes_per_hour            100 MiB
fetch_seconds_per_minute  30  (cumulative)
fetch_seconds_per_hour    300
```

Exceeding any returns `Budgeted { which: bytes_per_hour }`
without making the request. Budgets reset on a sliding window
like the `secure-host-channel`'s token bucket.

The orchestrator can also set global per-process caps that
override per-agent settings (for an emergency-throttle scenario).

---

## Audit Records

Every `fetch` call emits one audit record on the
`_internal.audit` channel:

```json
{
  "kind":             "host.network.fetch",
  "ts_ms":            1747382400123,
  "agent_id":         "weather-fetcher",
  "method":           "GET",
  "url_canonical":    "https://api.weather.gov/points/47.6062,-122.3321",
  "url_redacted":     "https://api.weather.gov/points/<lat>,<lon>",
  "status":           200,
  "bytes_received":   12453,
  "bytes_decompressed": 12453,
  "content_type":     "application/geo+json",
  "elapsed_ms":       217,
  "redirects":        0,
  "rejected":         null,
  "request_headers_count": 2,
  "response_headers_full": { ...full headers, post-strip... }
}
```

If rejected:

```json
{
  ...,
  "status":   null,
  "rejected": {
    "stage":  "url_hardening",
    "reason": "PrivateAddressForbidden",
    "detail": { "host": "169.254.169.254", "matched_range": "169.254.0.0/16" }
  }
}
```

The orchestrator forwards these to its persistent audit log.
A `--audit-tail` CLI command on the orchestrator shows them
live for debugging.

---

## Manifest Section

Every agent that uses `host.network.fetch` has a `network`
section in its `required_capabilities.json`:

```json
{
  "version": 1,
  "package": "rust/weather-fetcher-host",
  "capabilities": [
    {
      "category": "net",
      "action":   "connect",
      "target":   "api.weather.gov:443",
      "flavor":   "ingestion",
      "trust":    "untrusted",
      "justification": "Fetch Seattle forecast"
    },
    {
      "category": "channel",
      "action":   "write",
      "target":   "weather-snapshots",
      "flavor":   "internal"
    }
  ],
  "network": {
    "max_body_bytes":          102400,
    "max_redirects":           1,
    "max_request_seconds":     20,
    "expected_content_types":  ["application/geo+json"],
    "extra_request_headers":   ["Accept"],
    "requests_per_minute":     2,
    "bytes_per_minute":        262144
  },
  "justification": "..."
}
```

Defaults are conservative; the manifest tightens further for an
agent that knows it has narrow needs (the weather fetcher only
ever makes 1 request per tick = 12 per hour, never POSTs, never
reads more than 30 KiB).

The cage rejects manifests where:
- A `network` setting is below 0 or implausibly small (fail-shut
  intent: the value 0 in `requests_per_minute` would mean "never
  make a request" — interpreted as "this agent is broken").
- A `network` setting exceeds the global hard cap (e.g.,
  `max_body_bytes` > 50 MiB).

---

## Public API

The host-http-client lives **inside** the host runtime; agents
see it only through the existing `host::network::fetch` SDK
surface defined in `host-protocol.md`. There is no separate
public crate for agents to depend on.

For host-runtime-rust authors:

```rust
pub struct HostHttpClient {
    manifest:    Arc<Manifest>,
    transport:   Arc<dyn HttpsTransport>,
    audit_sink:  Arc<dyn AuditSink>,
    budgets:     Arc<RwLock<AgentBudget>>,
    /* opaque internals */
}

impl HostHttpClient {
    pub fn new(
        manifest:   Arc<Manifest>,
        transport:  Arc<dyn HttpsTransport>,
        audit_sink: Arc<dyn AuditSink>,
    ) -> Self;

    /// Service a host.network.fetch RPC from the agent.
    pub fn fetch(
        &self,
        request:  HostNetworkFetchRequest,
    ) -> Result<HostNetworkFetchResponse, HostError>;
}

/// What the host SDK passes to the client; mirrors the
/// host-protocol HostNetworkFetch params after deserialization.
pub struct HostNetworkFetchRequest {
    pub url:      String,
    pub method:   HttpMethod,
    pub headers:  Vec<(String, String)>,
    pub body:     Option<Vec<u8>>,
    pub opts:     FetchOpts,
}

pub struct HostNetworkFetchResponse {
    pub status:   u16,
    pub headers:  Vec<(String, String)>,  // already stripped
    pub body:     Vec<u8>,                // already decompressed
}
```

Errors are mapped to the existing `HostError` enum from
`host-runtime-rust.md`; specifically:

- URL hardening rejection → `HostError::CapabilityDenied { reason }`
  (the agent does not need to distinguish URL/header/budget
  rejections; all are "you can't do this here").
- Response hardening rejection → `HostError::Upstream { source }`
  with a structured inner error.
- Budget exceeded → `HostError::RateLimited { retry_after }`
  with the retry hint based on which window opened up first.

---

## Test Strategy

### URL Hardening

1. http URL → `SchemeNotHttps`.
2. URL with userinfo → `UserinfoNotAllowed`.
3. URL whose host is `169.254.169.254` → `PrivateAddressForbidden`.
4. URL whose host is `127.0.0.1` → `PrivateAddressForbidden`
   (unless manifest opts in).
5. URL whose host resolves at runtime to a private IP via DNS →
   `PrivateAddressForbidden` (DNS pinning at validation time).
6. URL with `?token=abc` → `CredentialsInQuery`.
7. URL longer than 2048 chars → `UrlTooLong`.
8. URL with `..` traversal → canonicalized.

### Request Hardening

9. Caller-supplied `Cookie` header → `HeaderForbidden`.
10. Caller-supplied `Content-Length` → `HeaderForbidden`.
11. Method `CONNECT` → `MethodForbidden`.
12. Body of 200 KiB with default 100 KiB cap → `BodyTooLarge`.

### Response Hardening

13. Server returns `text/html` to a JSON-expecting agent →
    `UnexpectedContentType`.
14. Server returns 100 MiB body → `BodyTooLarge`.
15. Server returns gzip that decompresses to 50 MiB →
    `DecompressionBomb`.
16. Server returns gzip with ratio 200:1 → `DecompressionBomb`.
17. Server returns `Set-Cookie` → header stripped from response;
    audit record retains it.
18. Server returns 4 redirects (cap is 3) → `RedirectLimit`.
19. Redirect to a private IP → `PrivateAddressForbidden`
    even if the original URL was public.

### Per-Agent Budgets

20. 61st request in a minute (cap 60) → `Budgeted`.
21. Cumulative bytes across requests exceeds hourly cap →
    `Budgeted`.
22. Budget windows roll over correctly.

### Audit

23. Every fetch (success and rejection) produces exactly one
    audit record with the right shape.
24. Rejected requests show `status: null` and the rejection
    reason.
25. Successful requests show `status`, `bytes_received`,
    `bytes_decompressed`, `elapsed_ms`.

### Coverage Target

`>=90%` line coverage on the client. The most security-critical
paths (URL hardening, decompression-bomb, response stripping)
deserve thorough fuzzing.

---

## Trade-Offs

**Strict by default; manifest opts looser.** The client refuses
many requests on conservative defaults. Agents that need looser
behavior (a higher body cap, a private host, more redirects)
must declare so explicitly. The friction is the point.

**Header stripping breaks some APIs.** Some servers include
information in non-standard headers that an agent might want
(rate-limit headers, pagination links). The agent declares
those headers in its manifest's `expected_response_headers`
allowlist with justification. The default-deny posture catches
servers that try to leak data via unusual headers.

**Decompression has CPU cost.** Decompressing on the host (in
the host runtime's process) means we burn CPU there instead of
in the agent process. We accept this; the alternative
(passing the compressed bytes through to the agent) means the
agent has to decompress, which means it needs the cage to allow
decompression code, which means a third defense-in-depth ring
that adds nothing. Decompressing once at the boundary is right.

**Per-agent budgets can starve a legitimate burst.** An agent
that has a sudden need for more than its hourly bytes budget
hits a hard wall. The user can reconfigure (Tier 1 challenge
to update the manifest), but it interrupts work. We accept this;
the alternative (no caps) means a compromised agent can
exfiltrate gigabytes before anyone notices.

**Forbidden private-address list is hard-coded.** A site running
on a non-standard private subnet (10.123.0.0 inside a custom
VPN, e.g.) won't be blocked by name but will be blocked by IP
range. Agents that need to talk to such a host must declare it
explicitly with `private_address: true`.

**No HTTP/2 or HTTP/3.** v1 covers only what `https-transport`
supports — HTTP/1.1. When `https-transport` adds HTTP/2,
this client gets the same defenses applied (the additional
attack surface — server push, settings frames — gets defended
at a higher layer).

**No request signing.** AWS-style request signing (SigV4) is not
implemented here. Agents that need it sign in their own code
before passing to fetch. This client just transports.

**Audit records contain headers (post-strip).** A reviewer who
looks at the audit log will see what the agent saw. Servers'
unusual headers will be visible in the audit but not in the
agent's call return. This is intentional: forensics needs full
data; the agent gets only what it needs.

---

## Future Extensions

- **Connection pooling** — when `https-transport` adds it, the
  client can opt agents in based on per-host trust.
- **HTTP/2 + HTTP/3** — additional defense for server push and
  settings frames.
- **Request signing helpers** — SigV4, JWT-bearer, OAuth-bearer
  shortcuts that integrate with the OAuth broker.
- **Per-host header allowlists** that ship with the platform for
  well-known APIs (a default allowlist for Gmail's API, etc.).
- **Response body validators** — pre-defined JSON schema checks
  the client runs before returning to the agent.
- **Compression of outgoing requests** — currently we don't
  compress; for upload-heavy agents, opt-in compression would
  help.

These are deliberately out of scope for v1.
