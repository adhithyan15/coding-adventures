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

- the orchestrator sends one pre-ready `PackageTrust`, one pre-ready
  `LaunchBindings`, `Terminate`, and responses;
- the child host sends `Ready`, `Heartbeat`, and requests.

The orchestrator control endpoint is constructed with the immutable package hash
from the service-registry registration. A `Ready` record carries the exact package
hash independently reverified by the child. A mismatch is terminal and must never
produce `Running` evidence.

After the secure-channel handshake and before `Ready`, the orchestrator sends the
exact relevant public package key, trust class, and maximum privilege tier selected
during its own package verification. The child must authenticate this record, build
its own one-key verification keyring, and independently re-read and verify the package.
The record contains no private key and is bound to the fresh encrypted session. A
missing, duplicate, malformed, or post-ready trust record is rejected.

After package trust and before readiness, the orchestrator sends the exact
manifest-blind output of authorized pipeline wiring. Each binding maps one
signed channel name and read/write direction to one canonical UUID-v7 channel.
An optional Level 1 model binding carries only a bounded model selector,
temperature, and output-token cap. Names and UUIDs are unique. The independently
verified child must require the exact signed name and direction sets and must
reject missing, extra, or wrong-direction bindings before readiness. The parent
never learns the signed manifest from this comparison.

The production provider reads a bounded versioned host-binding record tied to
the exact durable registration. Wiring creates immutable channel-to-pipeline
claims and verifies current one-way membership before storing the record.
Launch resolution repeats registration, claim, active-lifecycle, and directional
membership checks, so package replacement, channel destruction, topology drift,
or cross-pipeline reuse fails before process creation.

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
Correlation is not authorization. Before service execution, the parent-side
dispatcher reloads the complete current durable pipeline binding. Receive and
acknowledge require the requested UUID to have read access, publish requires write
access, and completion requires the exact authorized model selector, temperature,
and token cap. A service response must preserve the request ID and successful
operation kind; drift is returned only as a redacted stable failure.
Injected services validate complete responses against the same public codec bounds
before returning channel- or provider-supplied fields, so malformed output cannot
defer failure into authenticated framing.

## Lifecycle

The orchestrator endpoint begins in `AwaitingReady`:

1. It sends exactly one `PackageTrust`.
2. It sends exactly one `LaunchBindings` after trust and before accepting readiness.
3. The first authenticated child message must be `Ready(expected_package_hash)`.
4. A matching `Ready` transitions the endpoint to `Running` and yields authoritative
   package, session, and receipt-time evidence.
5. `Heartbeat` is accepted only in `Running` and refreshes the authoritative
   receipt time.
6. One data-plane request is accepted only in `Running`; no second request is
   accepted until the orchestrator sends its exact correlated response.
7. `Terminate` may be sent while awaiting readiness or running, and transitions the
   endpoint to `Terminating`.
8. No further application message is accepted or emitted after termination begins.

The child endpoint begins in `AwaitingReady`:

1. It accepts exactly one authenticated `PackageTrust` record.
2. It accepts exactly one authenticated `LaunchBindings` record after trust.
3. It must independently verify the package and require the exact signed channel
   names/directions plus runtime-compatible model presence before sending
   `Ready(package_hash)`.
4. It may send `Heartbeat` only after readiness.
5. It may send one bounded data-plane request after readiness and must wait for the
   exactly correlated response before sending another.
6. It accepts only correlated responses or `Terminate` from the orchestrator and
   enters `Terminating` on the latter.

A stream-owning child adapter must preserve that distinction while blocked on an
exchange: `Terminate` is a normal shutdown outcome, whereas an unexpected trust or
launch record, malformed frame, or wrong response shape remains terminal failure.

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
| 4   | orchestrator -> child | `PackageTrust` | key ID, key type, maximum tier, Ed25519 public key |
| 5   | orchestrator -> child | `LaunchBindings` | named channel UUIDs and optional Level 1 model settings |
| 10  | child -> orchestrator | `Receive` | request ID, channel UUID-v7, limit |
| 11  | child -> orchestrator | `Publish` | request ID, channel UUID-v7, content type, payload |
| 12  | child -> orchestrator | `Acknowledge` | request ID, channel and message UUID-v7 |
| 13  | child -> orchestrator | `Complete` | request ID, provider-neutral completion call |
| 14  | child -> orchestrator | `CompleteWithTools` | request ID, completion controls, tool catalog/choice, and prior calls/results |
| 15  | child -> orchestrator | `ExecuteTool` | request ID and exact model-returned structured call |
| 16  | child -> orchestrator | `ListModelTools` | request ID |
| 20  | orchestrator -> child | `Received` | request ID and verified message page |
| 21  | orchestrator -> child | `Published` | request ID, message UUID-v7, sequence, timestamp |
| 22  | orchestrator -> child | `Acknowledged` | request ID and monotonic sequence |
| 23  | orchestrator -> child | `Completed` | request ID and provider-neutral completion result |
| 24  | orchestrator -> child | `Failed` | request ID and redacted stable failure code |
| 25  | orchestrator -> child | `ToolCompleted` | request ID, final text or one structured call, and provider audit result |
| 26  | orchestrator -> child | `ToolExecuted` | request ID and exact structured D18D result |
| 27  | orchestrator -> child | `ModelToolsListed` | request ID and exact installed model-tool catalog |

All multi-byte integers are big-endian. The package key ID and channel names use bounded `u8`
length; data-plane variable bytes and UTF-8 strings use a `u32` length; vectors
use their specified `u8` or `u16` count. UUID fields must be canonical
UUID-v7 values. Data-plane bodies are capped at 768 KiB, a single channel payload
or completion text at 512 KiB, a receive page and completion prompt at 64 items,
and completion metadata at 32 unique canonically ordered keys. Text-completion calls
remain byte-compatible with v1. Separate tool-aware records add at most 128 unique
bounded declarations, auto/required/named choice, and at most 128 replayable prior
calls/results. The catalog-list response reuses the same non-empty, unique,
bounded declaration encoding. Schemas and arguments must be JSON objects, each JSON value is capped
at 64 KiB, and a response contains exactly final text or one structured call.
Responses retain provider identity, usage, finish reason, latency, and tool-polyfill
evidence for audit.

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
    PackageTrust(PackageTrust),
    LaunchBindings(LaunchBindings),
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
    pub fn provide_package_trust(&mut self, trust: PackageTrust)
        -> Result<Vec<u8>, ControlError>;
    pub fn provide_launch_bindings(&mut self, bindings: LaunchBindings)
        -> Result<Vec<u8>, ControlError>;
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
    pub fn request_tool_completion(&mut self, call: ToolCompletionCall)
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
The production Level 1 consumer is specified in
[`level-one-host.md`](level-one-host.md).

## Required Tests

The package must cover:

- matching readiness followed by multiple heartbeats;
- authenticated bounded package trust required exactly once before readiness;
- authenticated bounded launch bindings required exactly once after trust and
  before readiness, including canonicalization and malformed/duplicate rejection;
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
