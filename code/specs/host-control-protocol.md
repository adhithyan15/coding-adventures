# D18 Host Control Protocol

## Status

This document specifies the authenticated lifecycle and bounded host data-plane
protocol carried inside one D18 secure host channel. The implementation package is
`chief-of-staff-host-control-protocol`.

## Purpose

`chief-of-staff-secure-host-channel` authenticates, encrypts, and orders opaque
bytes. The service reconciler requires stronger process evidence: a host is
`Running` only after the expected signed package confirms readiness over that
authenticated channel, and later health evidence must come from authenticated
heartbeats rather than PID existence.

This protocol is the stable seam between those packages. It defines lifecycle
ordering plus the channel and provider-neutral completion operations a concrete
host needs without creating a second unauthenticated pipe. It does not spawn
processes, poll clocks, schedule heartbeats, interpret agent manifests, operate a
channel, or call a model provider.

## Roles and Authority

Every control session has exactly two roles inherited from the secure channel:

- the orchestrator sends `Terminate` and responses;
- the child host sends `Ready`, `Heartbeat`, and requests.

The orchestrator control endpoint is constructed with the immutable package hash
from the service-registry registration. A `Ready` record carries the exact package
hash independently reverified by the child. A mismatch is terminal and must never
produce `Running` evidence.

The timestamp attached to a received child event is not sent by the child. It is a
caller-supplied monotonic receipt time sampled by the supervising process after the
encrypted frame is received. This prevents a compromised child from forging a
fresh or future heartbeat. The later reconciliation tick still rejects receipt
times later than its own trusted `now_ns`.

After readiness, the child may have exactly one data-plane request in flight. It
mints non-zero `u64` request IDs beginning at one. The orchestrator requires each
ID to be the exact successor of the last request and returns the same ID. A
successful response kind must match the pending operation; a redacted `Failed`
response may complete any operation. Duplicate, skipped, unsolicited, or
cross-operation responses close the peer endpoint. This deliberately serial
contract matches the Level 1 runtime and avoids an unbounded correlation table.

## Lifecycle

The orchestrator endpoint begins in `AwaitingReady`:

1. The first authenticated child message must be `Ready(expected_package_hash)`.
2. A matching `Ready` transitions the endpoint to `Running` and yields authoritative
   package, session, and receipt-time evidence.
3. `Heartbeat` is accepted only in `Running` and refreshes the authoritative
   receipt time.
4. One data-plane request is accepted only in `Running`; no second request is
   accepted until the orchestrator sends its exact correlated response.
5. `Terminate` may be sent while awaiting readiness or running, and transitions the
   endpoint to `Terminating`.
6. No further application message is accepted or emitted after termination begins.

The child endpoint begins in `AwaitingReady`:

1. It must independently verify the package before sending `Ready(package_hash)`.
2. It may send `Heartbeat` only after readiness.
3. It may send one bounded data-plane request after readiness and must wait for the
   exactly correlated response before sending another.
4. It accepts only correlated responses or `Terminate` from the orchestrator and
   enters `Terminating` on the latter.

Duplicate readiness, heartbeat-before-ready, child-sent terminate,
orchestrator-sent ready/heartbeat/request, child-sent response/terminate, invalid
correlation, and messages after termination are protocol violations.
Authentication, decoding, package identity, role, correlation, or lifecycle
failure permanently closes the control endpoint. Callers terminate the underlying
process out of band after a terminal peer failure. Locally rejected construction
(for example an oversized publish) does not consume a request ID or close the
endpoint.

## Wire Format

Each plaintext passed to `SecureHostChannel` is one strict record:

```text
+----------+---------+------+--------------------------+
| magic    | version | kind | body                     |
| "D18C"  | u8 = 1  | u8   | exact bytes for the kind |
+----------+---------+------+--------------------------+
```

Kinds are:

| Tag | Direction | Name        | Body                         |
|-----|-----------|-------------|------------------------------|
| 1   | child -> orchestrator | `Ready`     | 32-byte package SHA-256 |
| 2   | child -> orchestrator | `Heartbeat` | empty                    |
| 3   | orchestrator -> child | `Terminate` | empty                    |
| 10  | child -> orchestrator | `Receive` | request ID, channel UUID-v7, limit |
| 11  | child -> orchestrator | `Publish` | request ID, channel UUID-v7, content type, payload |
| 12  | child -> orchestrator | `Acknowledge` | request ID, channel and message UUID-v7 |
| 13  | child -> orchestrator | `Complete` | request ID, provider-neutral completion call |
| 20  | orchestrator -> child | `Received` | request ID and verified message page |
| 21  | orchestrator -> child | `Published` | request ID, message UUID-v7, sequence, timestamp |
| 22  | orchestrator -> child | `Acknowledged` | request ID and monotonic sequence |
| 23  | orchestrator -> child | `Completed` | request ID and provider-neutral completion result |
| 24  | orchestrator -> child | `Failed` | request ID and redacted stable failure code |

All integers are big-endian. Variable bytes and UTF-8 strings use a `u32` length;
vectors use their specified `u8` or `u16` count. UUID fields must be canonical
UUID-v7 values. Data-plane bodies are capped at 768 KiB, a single channel payload
or completion text at 512 KiB, a receive page and completion prompt at 64 items,
and completion metadata at 32 unique canonically ordered keys. Completion calls
are provider-neutral, text-only in v1, and bound model, prompt, stop, temperature,
token, seed, and metadata fields. Responses retain provider identity, usage,
finish reason, and latency for audit.

Unknown versions/tags/enum values, invalid UTF-8 or UUIDs, non-finite or out-of-range
temperatures, duplicate metadata keys, truncation, trailing bytes, oversized
fields, and bodies with the wrong exact length are rejected. Diagnostics identify
only the failure class; they never include package bytes, keys, ciphertext,
provider details, or plaintext.

## Public API

```rust
pub enum ChildEvent {
    Ready { package_hash: [u8; 32], received_at_ns: u64 },
    Heartbeat { received_at_ns: u64 },
    Request(DataPlaneRequest),
}

pub enum OrchestratorEvent {
    Terminate,
    Response(DataPlaneResponse),
}

pub struct OrchestratorControl { /* channel, expected hash, lifecycle */ }
pub struct ChildControl { /* channel, lifecycle */ }

impl OrchestratorControl {
    pub fn new(channel: SecureHostChannel, expected_package_hash: [u8; 32])
        -> Result<Self, ControlError>;
    pub fn receive_child(&mut self, frame: &[u8], received_at_ns: u64)
        -> Result<ChildEvent, ControlError>;
    pub fn terminate(&mut self) -> Result<Vec<u8>, ControlError>;
    pub fn respond(&mut self, response: DataPlaneResponse)
        -> Result<Vec<u8>, ControlError>;
    pub fn pending_request(&self) -> Option<(RequestId, DataPlaneOperation)>;
    pub fn session_id(&self) -> SessionId;
    pub fn state(&self) -> ControlState;
}

impl ChildControl {
    pub fn new(channel: SecureHostChannel) -> Result<Self, ControlError>;
    pub fn ready(&mut self, package_hash: [u8; 32])
        -> Result<Vec<u8>, ControlError>;
    pub fn heartbeat(&mut self) -> Result<Vec<u8>, ControlError>;
    pub fn request_receive(&mut self, channel_id: [u8; 16], limit: u16)
        -> Result<(RequestId, Vec<u8>), ControlError>;
    pub fn request_publish(&mut self, channel_id: [u8; 16],
        content_type: String, payload: Vec<u8>)
        -> Result<(RequestId, Vec<u8>), ControlError>;
    pub fn request_acknowledge(&mut self, channel_id: [u8; 16],
        message_id: [u8; 16])
        -> Result<(RequestId, Vec<u8>), ControlError>;
    pub fn request_completion(&mut self, call: CompletionCall)
        -> Result<(RequestId, Vec<u8>), ControlError>;
    pub fn receive_orchestrator(&mut self, frame: &[u8])
        -> Result<OrchestratorEvent, ControlError>;
    pub fn session_id(&self) -> SessionId;
    pub fn state(&self) -> ControlState;
}
```

Constructors reject a secure channel with the wrong role. The wrapper exposes the
secure session ID so a process supervisor can report it as the registry control
channel ID without minting a second identity.

## Boundaries

The package has no direct filesystem, network, stream, environment, process, clock,
random, channel-storage, key-custody, or model-provider capability. It receives
complete encrypted frames and trusted receipt times from its caller. It validates
and authenticates requests but does not authorize or execute them. Length-prefix
framing, pipe ownership, process launch/reap, heartbeat scheduling, time sampling,
service dispatch, and hard-kill fallback belong to adapters specified in
[`process-host-supervisor.md`](process-host-supervisor.md).

## Required Tests

The package must cover:

- matching readiness followed by multiple heartbeats;
- authenticated round trips for receive, publish, acknowledge, completion, and
  redacted failure;
- one-in-flight ordering, monotonic request IDs, exact response correlation, and
  successful-response operation matching;
- every data-plane variant, bound, invalid UTF-8/UUID/enum, duplicate metadata,
  truncation, and trailing bytes;
- package-hash mismatch failing closed;
- role-mismatched secure-channel constructors;
- heartbeat-before-ready and duplicate-ready rejection;
- graceful termination before and after readiness;
- wrong-direction message kinds;
- authenticated-frame tampering and replay failure;
- every truncated record prefix, unknown versions/tags, and trailing bytes;
- terminal endpoint behavior after every peer or channel failure; and
- session identity preservation.

Line coverage must be at least 95 percent. The crate must forbid unsafe code.
