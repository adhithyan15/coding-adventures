# Host Protocol

## Overview

The Host Protocol is the wire contract between an **agent runtime** and its
**host process**. Every interaction an agent has with the outside world —
network calls, filesystem access, vault requests, channel reads and writes,
clock reads, log writes — passes over this protocol. The agent never calls
an OS API directly; it sends a request to its host, the host checks the
agent's manifest, and (if approved) the host performs the operation on the
agent's behalf and returns the result.

The contract is **runtime-agnostic** by design. The same protocol carries
requests from a native Rust agent (Tier 1, in-process call), a WebAssembly
agent (Tier 2, host-call import), a Deno agent (Tier 3, JSON over stdio),
or a BYO Python/Ruby/Elixir agent (Tier 3, JSON over stdio or Unix socket).
The orchestrator does not know — and does not need to know — which runtime
sits behind a particular host. The protocol is the only stable surface.

This is the same architectural pattern as Linux syscalls: there is one
ABI, and every language compiles to it. The "OS" here is the host
process; the "syscalls" are the methods defined in this spec.

**Why a wire protocol and not a Rust trait?** Because Tier 3 agents are
not in the host's address space. They live in a separate OS process, often
written in a different language, often with strong sandboxing between them
and the host. A trait would constrain Tier 1 only. JSON-RPC over a transport
covers all three tiers with one contract.

**Why JSON-RPC?** Because it is simple, ubiquitous, debuggable by hand,
language-agnostic, and supports the request/response and notification
patterns we need. Binary protocols are faster but cost a debuggability
budget we do not need to spend; the bottleneck for an agent is the model
call or the network round-trip, not the protocol parse.

---

## Where It Fits

```
Agent code (any tier)
    │
    │  speaks Host Protocol over a Transport
    ▼
Host Process (Rust)
    │
    ├── verifies the call against the manifest (capability-cage)
    ├── runs middleware (rate limit, audit log, trust boundary)
    ├── spawns an ephemeral sub-agent to do the work
    ├── returns the result over the same transport
    │
    ▼
OS (network, filesystem, vault, etc.)
```

**Depends on:** capability-cage (for manifest checks), supervisor (for
ephemeral sub-agent lifecycle), vault (for vault.* methods), actor (for
channel.* methods).

**Used by:** Tier 1 host-runtime-rust, Tier 2 host-runtime-wasm,
Tier 3 host-runtime-subprocess, every agent SDK in every language we
publish bindings for, the orchestrator (for service discovery), and any
external tool that wants to inspect an agent's request stream.

---

## Design Principles

1. **One contract, every tier.** A request shape that works for in-process
   calls also works for cross-process JSON. The SDK in each language adapts
   ergonomics, never semantics.
2. **Transport-agnostic.** The protocol does not care whether the bytes
   travel over stdio, a Unix domain socket, a Windows named pipe, an
   in-process channel, or a TCP loopback connection.
3. **Capability-namespaced.** Method names map directly to capability
   categories (`network.fetch` → `net:connect:*`, `fs.read` → `fs:read:*`).
   No method exists outside the capability taxonomy. To add a method, you
   add a capability category.
4. **Errors are structured, not freeform.** Every failure is one of a
   small enumerated set of codes with structured `data`. Agents can
   handle errors programmatically; humans can debug them quickly.
5. **Streaming is first-class.** Responses can be partial; a single
   request may produce many response messages before completion.
   Cancellation is symmetric: the agent can cancel mid-stream, and the
   host can abort with a reason.
6. **Versioned, additive.** v1 fields and methods are stable forever.
   Breaking changes mean v2. The transport carries the version on every
   handshake.
7. **Debuggable by hand.** A developer can `cat` a host's input and see
   what an agent is asking for. They can `echo` a JSON line into a host
   and see the response. No frame parsers, no bytecode, no required
   tooling.

---

## Transport

The protocol layers on top of any **bidirectional, message-framed,
ordered byte stream**. Two transports are normative for v1:

### Transport: stdio (line-delimited JSON)

The agent process is spawned by the host with stdin/stdout piped. Each
JSON message is a single line terminated by `\n`. The agent never writes
to stderr; stderr is reserved for the runtime to surface uncaught panics
to the host's logs.

```
agent → host  (stdin)   {"jsonrpc":"2.0","id":1,"method":"system.now"}\n
host  → agent (stdout)  {"jsonrpc":"2.0","id":1,"result":{"unix_ms":...}}\n
```

This is the default for Tier 3 BYO agents and Deno agents. It works on
every OS, requires no IPC primitives, and is trivially loggable by
prepending `tee` between the agent and the host.

### Transport: Unix socket / Windows named pipe

A length-prefixed JSON frame protocol over a duplex socket. Each frame
is a 4-byte big-endian length followed by that many bytes of UTF-8 JSON.

```
┌────────────┬─────────────────────────────────────────┐
│ length: u32│ payload: utf-8 json (length bytes)       │
│ big-endian │                                          │
└────────────┴─────────────────────────────────────────┘
```

Used when the agent is a long-lived process that benefits from a single
connection per session, when the agent and host are in different security
contexts (sandbox boundary), or when the parent process needs the child's
stdio for other purposes (terminal, log forwarding).

### Transport: in-process channel

For Tier 1 and Tier 2 agents (Rust native or Wasm modules in the host's
address space), the same JSON messages flow through an in-process MPSC
channel. No serialization to bytes is required; the host can pass
`HostRequest` and `HostResponse` Rust structs directly. The wire format
is identical when serialized for debugging or recording.

### Choice does not affect semantics

A method that works over stdio works identically over a socket or an
in-process channel. The transport is invisible to the agent code and the
host's middleware chain.

---

## Message Shapes

Every message conforms to JSON-RPC 2.0 with a small, documented set of
extensions:

### Request (agent → host)

```json
{
  "jsonrpc": "2.0",
  "id":      1,
  "method":  "network.fetch",
  "params": {
    "url":    "https://api.weather.gov/points/47.6,-122.3",
    "method": "GET",
    "headers": { "User-Agent": "weather-agent/0.1" }
  }
}
```

| Field    | Required | Notes                                          |
|----------|----------|-----------------------------------------------|
| jsonrpc  | yes      | Must be the literal string `"2.0"`.           |
| id       | yes      | Integer, monotonically increasing per agent.  |
| method   | yes      | A method name from the namespace below.       |
| params   | yes      | An object whose shape is per-method.          |

### Response (host → agent), success

```json
{
  "jsonrpc": "2.0",
  "id":      1,
  "result": {
    "status":  200,
    "headers": { "Content-Type": "application/geo+json" },
    "body_b64": "eyJwcm9wZXJ0aWVzIjp7Li4ufX0="
  }
}
```

Binary bodies are base64-encoded in JSON to avoid escaping issues. SDKs
in each language decode into native byte buffers transparently.

### Response (host → agent), error

```json
{
  "jsonrpc": "2.0",
  "id":      1,
  "error": {
    "code":    -32001,
    "message": "CapabilityDenied",
    "data": {
      "requested": "net:connect:evil.com:443",
      "agent":     "weather-agent",
      "reason":    "host not in manifest"
    }
  }
}
```

### Notification (agent → host or host → agent)

A JSON-RPC notification has no `id` field and expects no response. Used
for fire-and-forget signals: log lines, cancellation, lifecycle events.

```json
{
  "jsonrpc": "2.0",
  "method":  "system.log",
  "params":  { "level": "info", "message": "fetched forecast" }
}
```

### Stream chunk (host → agent)

For methods that may produce many partial results before completion, the
host emits multiple **stream chunks** with the same `id` as the original
request, then a final `result` (or `error`) to terminate.

```json
{ "jsonrpc": "2.0", "id": 7, "stream": { "chunk": { "bytes_b64": "..." } } }
{ "jsonrpc": "2.0", "id": 7, "stream": { "chunk": { "bytes_b64": "..." } } }
{ "jsonrpc": "2.0", "id": 7, "result": { "status": 200, "complete": true } }
```

The `stream` field is the v1 extension to JSON-RPC. A receiver that does
not understand `stream` and reads only `result`/`error` will still
function correctly because the final terminator is a normal JSON-RPC
response.

---

## Method Namespace

Methods are organized by capability category. Every method name has the
form `category.verb` and maps unambiguously to one or more entries in the
capability manifest.

### `network.*` — corresponds to `net` capability

| Method            | Manifest check                       | Purpose                  |
|-------------------|--------------------------------------|--------------------------|
| `network.fetch`   | `net:connect:host:port`              | HTTP/HTTPS request       |
| `network.connect` | `net:connect:host:port`              | Open a TCP socket        |
| `network.listen`  | `net:listen:host:port`               | Bind a server socket     |
| `network.dns`     | `net:dns:hostname`                   | Resolve a hostname       |

Params for `network.fetch`:
```
{
  "url":     string,
  "method":  "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD",
  "headers": map<string, string>,
  "body_b64": string  (optional)
}
```

Result:
```
{
  "status":   number,
  "headers":  map<string, string>,
  "body_b64": string
}
```

### `fs.*` — corresponds to `fs` capability

| Method      | Manifest check          | Purpose                |
|-------------|-------------------------|------------------------|
| `fs.read`   | `fs:read:path`          | Read file contents     |
| `fs.write`  | `fs:write:path`         | Write file contents    |
| `fs.create` | `fs:create:path`        | Create empty file      |
| `fs.delete` | `fs:delete:path`        | Remove a file          |
| `fs.list`   | `fs:list:path`          | List directory entries |
| `fs.stat`   | `fs:read:path`          | File metadata          |

Path globs in the manifest (`fs:read:./grammars/*.tokens`) are expanded
by the host's middleware; the agent only ever requests literal paths,
which the host validates against the glob.

### `vault.*` — corresponds to `vault` capability

| Method                | Manifest check                | Purpose                          |
|-----------------------|-------------------------------|----------------------------------|
| `vault.requestLease`  | `vault:read:secret-name`      | Get a TTL'd lease for a secret    |
| `vault.requestDirect` | `vault:direct:secret-name`    | Send secret directly to a peer    |
| `vault.releaseLease`  | (none — own lease)            | Surrender a lease early           |

`vault.requestDirect` does not return the secret to the calling agent;
the secret is delivered on a separate channel to a third party (e.g., the
browser host). The calling agent only learns "lease created" or "denied."

### `channel.*` — corresponds to `channel` capability

| Method            | Manifest check                       | Purpose                  |
|-------------------|--------------------------------------|--------------------------|
| `channel.read`    | (registered receiver on channel)     | Read next message        |
| `channel.write`   | (registered originator on channel)   | Append a message         |
| `channel.ack`     | (registered receiver)                | Advance read offset      |
| `channel.peek`    | (registered receiver)                | Read without advancing   |

Channel access is checked by the supervisor at wiring time, not at every
call. If an agent calls `channel.write` on a channel it is not the
registered originator of, the host returns `CapabilityDenied`.

### `proc.*` — corresponds to `proc` capability

| Method        | Manifest check          | Purpose                    |
|---------------|-------------------------|----------------------------|
| `proc.exec`   | `proc:exec:cmdname`     | Run an external command    |
| `proc.spawn`  | `proc:fork`             | Spawn a managed subprocess |
| `proc.signal` | `proc:signal:cmdname`   | Send a signal              |

Argv shapes can be constrained in the manifest (e.g.,
`proc:exec:git` may be paired with an arg-pattern). The host validates
the full argv against the manifest before exec.

### `system.*` — always available, no capability check

| Method               | Purpose                                   |
|----------------------|-------------------------------------------|
| `system.now`         | Monotonic clock in nanoseconds            |
| `system.unixTime`    | Wall-clock time in milliseconds           |
| `system.randomBytes` | CSPRNG bytes from the host                |
| `system.log`         | Structured log line                       |
| `system.identity`    | Return the agent's own ID and manifest    |

These do not require manifest entries because they are universally needed
and have no privacy implications. `system.log` writes to the host's log
stream; the host may add fields (agent id, timestamp).

### Reserved namespaces

- `host.*` — reserved for future host-to-agent server methods.
- `_internal.*` — reserved for protocol metadata (handshake, ping).

Agents must not implement methods in these namespaces.

---

## Error Model

All errors use JSON-RPC's `error` object with a numeric code, a short
machine-readable `message`, and a structured `data` payload.

### v1 Error Codes

```
JSON-RPC standard:
  -32700   ParseError             Invalid JSON received
  -32600   InvalidRequest         JSON-RPC envelope malformed
  -32601   MethodNotFound         No such method in this version
  -32602   InvalidParams          Params shape wrong for this method
  -32603   InternalError          Host-side bug; should not happen

Host Protocol extensions (range -32000 to -32099):
  -32000   ProtocolVersion        Agent speaks a version we do not support
  -32001   CapabilityDenied       Manifest does not permit this call
  -32002   TrustBoundaryDenied    Privilege tier challenge failed/denied
  -32003   RateLimited            Too many calls in a short window
  -32004   ResourceNotFound       Vault secret/file/channel not present
  -32005   ResourceExpired        Lease/handle no longer valid
  -32006   Cancelled              Request cancelled by agent or shutdown
  -32007   Timeout                Operation exceeded its deadline
  -32008   Conflict               Concurrent modification (e.g., channel sequence)
  -32009   Upstream               Underlying OS/network call failed
```

Every error code includes a `data` object with code-specific fields. For
`CapabilityDenied` it is `{requested, agent, reason}`. For `Upstream` it
is `{kind, os_code, message}`. The full per-code shapes are in the JSON
schema published alongside this spec.

### Error normalization

To prevent side-channel inference, certain errors are normalized before
being returned to the agent:

- `fs.read` on a path the agent has no permission to read returns
  `CapabilityDenied`, **not** `ResourceNotFound`. The agent cannot infer
  whether the file exists.
- `network.fetch` against a denied host returns `CapabilityDenied`
  before any DNS lookup. The agent cannot use the protocol to probe DNS.
- `vault.requestLease` for a non-existent secret returns
  `CapabilityDenied` (not `ResourceNotFound`) when the agent has no
  manifest entry; if the agent has an entry but the secret is missing
  from the vault, it returns `ResourceNotFound`.

---

## Capability ↔ Method Mapping

The capability cage taxonomy and the protocol method namespace are
**locked together by construction**. Each method maps to exactly one
capability category, and adding a method requires adding (or extending)
a capability category. There are no "side-door" methods.

```
Capability category    Methods
─────────────────────  ──────────────────────────────────────────────
fs                     fs.read, fs.write, fs.create, fs.delete,
                       fs.list, fs.stat
net                    network.fetch, network.connect, network.listen,
                       network.dns
proc                   proc.exec, proc.spawn, proc.signal
env                    (no method — env is scrubbed at exec time)
ffi                    (no method — host does not expose FFI)
time                   system.now, system.unixTime
stdin                  (transport-level; not a method)
stdout                 system.log

vault                  vault.requestLease, vault.requestDirect,
                       vault.releaseLease
channel                channel.read, channel.write, channel.ack,
                       channel.peek
```

`env`, `ffi`, and `stdin` deliberately have no protocol surface. Agents
do not get an env-read method (the host scrubs and re-injects only
allowed vars at exec). Agents do not get FFI (the host won't load native
libs on their behalf; if you need it, write a Tier 1 native host
instead). Agents do not read stdin as a stream from the host (the
transport itself uses stdin; conflating them creates parser
ambiguities).

---

## Streaming and Cancellation

### Streaming responses

Methods may produce a stream of partial results. The agent opts in by
the method itself (e.g., `network.fetch` with `stream: true` in params)
or implicitly when the method's contract specifies streaming.

```
agent → host: { id: 7, method: "network.fetch",
                params: { url: "...", stream: true } }

host → agent: { id: 7, stream: { chunk: { bytes_b64: "..." } } }
host → agent: { id: 7, stream: { chunk: { bytes_b64: "..." } } }
host → agent: { id: 7, stream: { chunk: { bytes_b64: "..." } } }
host → agent: { id: 7, result: { status: 200, complete: true } }
```

The `result` (or `error`) is always the final message for an `id`.

### Cancellation

The agent may cancel a pending request by sending a notification:

```json
{ "jsonrpc": "2.0", "method": "_internal.cancel", "params": { "id": 7 } }
```

The host responds with a final `error` for that `id`:

```json
{ "jsonrpc": "2.0", "id": 7, "error": { "code": -32006, "message": "Cancelled" } }
```

The host may cancel its own work (e.g., during shutdown) by emitting the
same `Cancelled` error unsolicited.

### Backpressure

The transport is responsible for backpressure. For stdio the OS pipe
buffer naturally backpressures the writer. For sockets the SDK should
not buffer unbounded outgoing messages. Agents that produce data faster
than the host can consume it should be killed by the host (intent
violation); the protocol does not define rate limits beyond the
`RateLimited` error code, which is per-method per-agent.

---

## Handshake and Versioning

The first message in every session is a handshake:

```
agent  → host: { jsonrpc: "2.0", id: 0, method: "_internal.hello",
                 params: { agent_id: "weather-agent",
                           protocol_versions: ["1.0"],
                           sdk: "rust-sdk@0.1.0" } }

host   → agent: { jsonrpc: "2.0", id: 0,
                  result: { protocol_version: "1.0",
                            host: "host-runtime-rust@0.1.0",
                            session_id: "uuid",
                            capabilities_summary: ["fs:read:./pkg/code/*",
                                                   "net:connect:api.weather.gov:443",
                                                   "vault:read:gmail-app-pw"] } }
```

If the host cannot serve any version the agent supports, it returns
`ProtocolVersion` and closes the transport.

### Version evolution

- New methods added → minor version bump, agent need not change.
- New params on existing methods → must default to backward-compatible
  behavior; minor version bump.
- Removed methods or changed param shapes → major version bump (v2).
- Error codes are append-only; codes never change meaning.

The protocol_version on a session is fixed at handshake. Version
negotiation happens once.

---

## Tier Compatibility

| Tier              | Transport                           | SDK style                                    |
|-------------------|-------------------------------------|----------------------------------------------|
| 1 (Rust native)   | in-process channel                  | sync `host_call(req) -> resp` + async wrapper|
| 2 (WASM)          | host-call import (no serialization) | wit-bindgen-style imports                    |
| 2 (WASM, JSON)    | message-passing if Component Model is unused | JSON over a single buffer            |
| 3 (Deno)          | stdio, line-delimited JSON          | TypeScript SDK with typed methods            |
| 3 (BYO Python)    | stdio, line-delimited JSON          | Python SDK auto-generated from spec          |
| 3 (BYO Ruby)      | stdio, line-delimited JSON          | Ruby SDK auto-generated from spec            |
| 3 (BYO Elixir)    | stdio or Unix socket                | Elixir SDK over `Port` or socket             |

All tiers see the same method names, same params, same result shapes,
same error codes. SDKs differ only in how they marshal calls into the
transport.

---

## Examples

### Tier 3 weather agent (Python over stdio)

```python
import json, sys, base64

def call(method, params):
    req = {"jsonrpc": "2.0", "id": call.next_id, "method": method, "params": params}
    call.next_id += 1
    sys.stdout.write(json.dumps(req) + "\n"); sys.stdout.flush()
    resp = json.loads(sys.stdin.readline())
    if "error" in resp: raise RuntimeError(resp["error"])
    return resp["result"]
call.next_id = 1

# Handshake
hello = call("_internal.hello",
             {"agent_id": "weather-agent",
              "protocol_versions": ["1.0"],
              "sdk":  "python@0.1"})

# Fetch weather
forecast = call("network.fetch",
                {"url": "https://api.weather.gov/points/47.6,-122.3",
                 "method": "GET",
                 "headers": {"User-Agent": "weather-agent/0.1"}})
body = json.loads(base64.b64decode(forecast["body_b64"]))

# Get email password from vault
lease = call("vault.requestLease",
             {"name": "gmail-app-password", "ttl_ms": 60_000})
password = base64.b64decode(lease["secret_b64"]).decode()
```

### Tier 1 native (Rust)

```rust
let host = HostHandle::connect_in_process()?;

host.call(&Hello {
    agent_id: "weather-agent",
    protocol_versions: vec!["1.0"],
    sdk: "rust-sdk@0.1",
})?;

let forecast = host.call(&NetworkFetch {
    url:    "https://api.weather.gov/points/47.6,-122.3",
    method: HttpMethod::Get,
    headers: hashmap! { "User-Agent" => "weather-agent/0.1" },
})?;

let lease = host.call(&VaultRequestLease {
    name:   "gmail-app-password",
    ttl_ms: 60_000,
})?;
```

The native SDK presents a typed API, but the wire format on the
in-process channel is the same JSON shape — and a debugger configured to
log the channel sees identical messages to the Python example.

### Capability-denied error

```json
agent → host: { "jsonrpc": "2.0", "id": 14, "method": "network.fetch",
                "params": { "url": "https://evil.com/steal", "method": "GET" } }

host  → agent: { "jsonrpc": "2.0", "id": 14,
                 "error": {
                   "code": -32001,
                   "message": "CapabilityDenied",
                   "data": {
                     "requested": "net:connect:evil.com:443",
                     "agent":     "weather-agent",
                     "reason":    "host not in manifest" }}}
```

The host's middleware rejects the call before any DNS lookup occurs.
The agent receives a structured error it can handle programmatically.

---

## Test Strategy

### Conformance suite (every SDK runs the same suite)

1. **Handshake.** Hello with a supported version → success. Hello with
   no overlap → ProtocolVersion error and transport close.
2. **Method dispatch.** Each method in the namespace, with valid params
   and a granted manifest, returns the expected result shape.
3. **Capability denial.** Each method, with a manifest that denies it,
   returns `CapabilityDenied` with the correct `data.requested`.
4. **Param validation.** Malformed params return `InvalidParams` with
   the offending field path in `data`.
5. **Error normalization.** `fs.read` on a non-existent path that the
   agent could read if it existed returns `ResourceNotFound`. The same
   call without the manifest entry returns `CapabilityDenied`. Both
   responses are independent of file existence.
6. **Streaming.** A streamed `network.fetch` produces ≥1 chunk and
   exactly one terminal `result` with the same `id`. No `result` after
   `result`. No chunks after `result`.
7. **Cancellation.** A pending request cancelled mid-flight returns
   `Cancelled`. No further messages with that `id` arrive.
8. **Out-of-order.** The host MAY interleave responses for different
   `id`s. The SDK MUST tolerate this.

### Cross-tier interop

The same agent logic, expressed in three SDKs (Rust, Python, TypeScript),
must produce byte-identical request streams when run against a recording
host. This proves the SDKs are not adding semantics.

### Coverage target

`>=95%` of the protocol surface covered by the conformance suite.
Method dispatch, error codes, streaming, cancellation, and handshake
are all individually exercised.

---

## Dependencies

- **capability-cage** — manifest checks before every method dispatch.
- **vault-***  — backs `vault.*` methods.
- **actor** — backs `channel.*` methods.
- **JSON** — parsing and serializing protocol messages.

---

## Trade-Offs

**JSON parsing on every message.** A binary protocol (CBOR, MessagePack,
Cap'n Proto) would be 2-5× faster to parse. We accept the cost: an agent
makes O(10) calls per second in steady state; protocol parsing is not
the bottleneck. Debuggability is.

**Base64 for binary bodies.** A 33% size overhead for response bodies.
Acceptable for agents that fetch JSON APIs. For agents that move large
binary blobs (image processing, model weights), consider a separate
streaming-bytes transport once a use case demands it.

**One id per agent, not per host.** Request IDs are scoped to a single
agent's session. The host distinguishes by transport identity, not by
some global ID. Simpler to implement, harder to misuse, but means logs
need to record which session each id came from.

**No bidirectional RPC.** The agent never serves methods that the host
calls. Only the agent calls into the host. The `host.*` namespace is
reserved for future use but not implemented in v1; if we need
host-initiated calls, they will be notifications (one-way) until proven
otherwise.

**Stream chunks are unstructured payloads.** A chunk is just bytes; it
has no schema. Higher-level streaming (e.g., LLM token streaming with
metadata per token) builds on top in a method-specific way.

---

## Future Extensions

- **Compression.** Optional content-encoding negotiated at handshake
  for bandwidth-constrained transports.
- **Bidirectional methods.** A `host.*` namespace for the host to
  request things from the agent (e.g., "summarize your state for a
  checkpoint").
- **Binary side-channel.** A second transport for large payloads with
  zero-copy semantics, referenced by handle from JSON messages.
- **Span-based tracing.** Per-request trace IDs that propagate through
  ephemeral sub-agents and downstream calls.

These are deliberately deferred. v1 ships with the minimum surface that
covers Tier 1 / 2 / 3 with one contract.
