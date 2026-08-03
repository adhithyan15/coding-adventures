# D18 Host Control Protocol

## Status

This document specifies the authenticated lifecycle protocol carried inside one
D18 secure host channel. The implementation package is
`chief-of-staff-host-control-protocol`.

## Purpose

`chief-of-staff-secure-host-channel` authenticates, encrypts, and orders opaque
bytes. The service reconciler requires stronger process evidence: a host is
`Running` only after the expected signed package confirms readiness over that
authenticated channel, and later health evidence must come from authenticated
heartbeats rather than PID existence.

This protocol is the stable seam between those packages. It defines the minimum
control messages and lifecycle ordering needed by a concrete process supervisor.
It does not spawn processes, poll clocks, schedule heartbeats, or interpret agent
manifests.

## Roles and Authority

Every control session has exactly two roles inherited from the secure channel:

- the orchestrator sends `Terminate`;
- the child host sends `Ready` and `Heartbeat`.

The orchestrator control endpoint is constructed with the immutable package hash
from the service-registry registration. A `Ready` record carries the exact package
hash independently reverified by the child. A mismatch is terminal and must never
produce `Running` evidence.

The timestamp attached to a received child event is not sent by the child. It is a
caller-supplied monotonic receipt time sampled by the supervising process after the
encrypted frame is received. This prevents a compromised child from forging a
fresh or future heartbeat. The later reconciliation tick still rejects receipt
times later than its own trusted `now_ns`.

## Lifecycle

The orchestrator endpoint begins in `AwaitingReady`:

1. The first authenticated child message must be `Ready(expected_package_hash)`.
2. A matching `Ready` transitions the endpoint to `Running` and yields authoritative
   package, session, and receipt-time evidence.
3. `Heartbeat` is accepted only in `Running` and refreshes the authoritative
   receipt time.
4. `Terminate` may be sent while awaiting readiness or running, and transitions the
   endpoint to `Terminating`.
5. No further application message is accepted or emitted after termination begins.

The child endpoint begins in `AwaitingReady`:

1. It must independently verify the package before sending `Ready(package_hash)`.
2. It may send `Heartbeat` only after readiness.
3. It accepts only `Terminate` from the orchestrator and then enters `Terminating`.

Duplicate readiness, heartbeat-before-ready, child-sent terminate,
orchestrator-sent ready/heartbeat, and messages after termination are protocol
violations. Authentication, decoding, package identity, role, or lifecycle failure
permanently closes the control endpoint. Callers terminate the underlying process
out of band after a terminal peer failure.

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

Records are at most 38 bytes before secure-channel encryption. Unknown versions,
unknown tags, truncation, trailing bytes, and bodies with the wrong exact length
are rejected. Diagnostics identify only the failure class; they never include
package bytes, keys, ciphertext, or plaintext.

## Public API

```rust
pub enum ChildEvent {
    Ready { package_hash: [u8; 32], received_at_ns: u64 },
    Heartbeat { received_at_ns: u64 },
}

pub enum OrchestratorEvent {
    Terminate,
}

pub struct OrchestratorControl { /* channel, expected hash, lifecycle */ }
pub struct ChildControl { /* channel, lifecycle */ }

impl OrchestratorControl {
    pub fn new(channel: SecureHostChannel, expected_package_hash: [u8; 32])
        -> Result<Self, ControlError>;
    pub fn receive_child(&mut self, frame: &[u8], received_at_ns: u64)
        -> Result<ChildEvent, ControlError>;
    pub fn terminate(&mut self) -> Result<Vec<u8>, ControlError>;
    pub fn session_id(&self) -> SessionId;
    pub fn state(&self) -> ControlState;
}

impl ChildControl {
    pub fn new(channel: SecureHostChannel) -> Result<Self, ControlError>;
    pub fn ready(&mut self, package_hash: [u8; 32])
        -> Result<Vec<u8>, ControlError>;
    pub fn heartbeat(&mut self) -> Result<Vec<u8>, ControlError>;
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
or random capability. It receives complete encrypted frames and trusted receipt
times from its caller. Length-prefix framing, pipe ownership, process launch/reap,
heartbeat scheduling, time sampling, and hard-kill fallback belong to the concrete
process-supervisor adapter.

## Required Tests

The package must cover:

- matching readiness followed by multiple heartbeats;
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
