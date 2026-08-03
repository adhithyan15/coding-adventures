# D18 Process Host Supervisor

## Status

This document specifies the concrete OS-process adapter that connects the D18
service reconciler to verified agent packages, secure host channels, and the
authenticated host-control protocol.

The implementation package is `chief-of-staff-process-supervisor`.

## Purpose

The durable service registry records intent but is not process authority. The
process host supervisor owns every child it reports, independently verifies the
registered package immediately before each launch, bootstraps one fresh secure
session, and derives readiness and heartbeat evidence only from authenticated
child messages.

The adapter implements `chief_of_staff_service_reconciler::HostSupervisor`. It
does not implement scheduling, restart policy, backoff, or durable state; those
remain responsibilities of the reconciler and runnable orchestrator.

## Process Contract

One configured host program contains a non-empty absolute executable path and
at most 128 fixed arguments. The adapter never invokes a shell or performs
ambient `PATH` resolution. It launches the program
with piped standard input and output and with the verified package directory as
its working directory. Standard error is inherited so diagnostics do not share
the authenticated protocol stream.

Standard input and output carry binary records:

```text
u32 big-endian payload length | payload bytes
```

Payloads are between 1 byte and 1 MiB inclusive. At most 64 complete records may
wait between the reader and supervisor, so a flooding child receives pipe
backpressure rather than consuming unbounded parent memory. Writers flush each complete
record. Readers reject zero length, oversized length, truncation, or I/O failure
before handing a payload to the secure-channel or control protocol. An invalid
stream is terminal for that child.

After spawning, the parent writes one `BootstrapOffer` and waits for one
`ClientHello`. The wait has a non-zero configured real-time bound. The accepted
secure channel is wrapped in `OrchestratorControl`. Subsequent framed records
are encrypted host-control frames. The child-side helper performs the inverse
bootstrap over any `Read` and `Write`, then exposes only `ready`, `heartbeat`,
and `receive_terminate` operations.

## Package and Identity Safety

Before every actual spawn, including a restart, the adapter:

1. verifies the package signature and contents with the injected trusted
   `PackageKeyring`;
2. compares the verified SHA-256 identity to the exact hash stored in the
   `HostRegistration`; and
3. refuses to launch on any verification or identity mismatch.

The child independently verifies its package before sending
`Ready(package_hash)`. A mismatched authenticated readiness record closes the
control endpoint and terminates the child.

Every launch receives a fresh non-zero UUID-v7 `SessionId` from an injected
source. The session ID becomes the registry `ChannelId`; the adapter never mints
a second channel identity. The orchestrator X3DH identity is borrowed rather
than copied, preserving its zeroizing key ownership.

## Process Authority

The adapter retains the `std::process::Child` handle, framed pipe endpoints,
reader thread, authenticated control state, exact launch hash, and lifecycle
timestamps for each host name. It never adopts a PID from the registry or treats
PID existence as ownership.

`inspect` first drains all complete reader events and authenticates them with
the orchestrator control endpoint, then samples child exit state through the
owned handle. It reports:

- `Starting` after spawn and secure bootstrap, before matching readiness;
- `Running` only after matching authenticated readiness, with the trusted
  receipt time of the latest authenticated readiness or heartbeat;
- `Stopping` after graceful termination or forced-kill initiation; and
- `Exited` after reaping, retaining the exit code when available.

Reader receipt times come from the injected monotonic clock after a complete
frame is received. Child-supplied timestamps are never accepted. EOF, framing
failure, authentication failure, illegal control ordering, package mismatch,
or premature exit fails closed and triggers process cleanup.

## Start and Stop Idempotency

`start` is idempotent for an active owned child launched from the same package
hash. It refuses a second active launch with a different registered hash. A new
launch may replace an absent or reaped instance.

`stop` is idempotent for absent, already-stopping, and exited hosts. For an
active secure session it writes one authenticated `Terminate`, waits until the
configured non-zero graceful deadline, and hard-kills then reaps the process if
it has not exited. A control write failure causes immediate hard-kill cleanup.

Dropping the adapter hard-kills and reaps all remaining owned children and
joins their reader threads. No process is intentionally orphaned.

## Injected Sources

Production adapters are provided for:

- UUID-v7 session generation; and
- monotonic nanosecond sampling from one `Instant` origin.

Tests inject deterministic session and clock sources. Time values in supervisor
observations are opaque monotonic values and must not be compared with wall
clock time.

## Public API

The package exposes:

- validated `ProcessSupervisorConfig` and `HostProgram` values;
- `MonotonicClock` and `SessionIdSource` interfaces plus production adapters;
- `ProcessHostSupervisor`, implementing the reconciler `HostSupervisor` trait;
- `ChildProcessControl<R, W>` for host-runtime integration; and
- bounded, input-independent `ProcessSupervisorError` diagnostics.

## Capabilities

The concrete adapter requires direct filesystem/package-read, process
spawn/kill/reap, pipe I/O, clock, randomness, and thread capabilities. It opens
no network sockets, reads no ambient environment variables, invokes no shell,
and persists no state.

## Required Tests

The package must cover:

- zero-length, oversized, truncated, and valid framed records;
- invalid configuration and stable redacted diagnostics;
- signature or registered-hash failure before process spawn;
- real cross-platform child bootstrap, matching readiness, heartbeat, and
  graceful termination;
- idempotent same-hash start and refusal of an active different-hash launch;
- bootstrap timeout, malformed bootstrap, wrong readiness, and exit-before-ready
  cleanup;
- graceful-stop timeout with hard-kill fallback;
- exited observations retaining exit status without PID authority; and
- drop-time cleanup of every owned child.

The package forbids unsafe code and targets at least 90 percent line coverage.
