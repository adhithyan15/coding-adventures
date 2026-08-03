# D18 Service Registry Reconciliation

## Status

This document specifies the dependency-light reconciliation kernel between the
D18 durable service registry and an authoritative host-process supervisor.

The implementation package is `chief-of-staff-service-reconciler`.

## Purpose

The service registry persists operator intent and the orchestrator's last
bounded observation. It is not process authority: cached PIDs, channel IDs, and
health timestamps must never be trusted after an orchestrator restart.

The reconciler repeatedly compares each registry entry with a fresh supervisor
observation and converges the host toward `DesiredState`. It does not spawn
processes itself. A caller supplies a supervisor implementation that can
inspect, start, and stop a host.

## Boundaries

The reconciler:

- reads and CAS-updates `chief-of-staff-service-registry` entries;
- treats the supervisor observation as the only live-process evidence;
- verifies that a live process belongs to the registered package hash;
- applies durable `Always`, `OnFailure`, and `Never` restart policy;
- rejects observations from the future and stale running heartbeats;
- performs at most one supervisor mutation per host per tick; and
- reports stable, host-name-ordered outcomes.

The reconciler does not:

- open files, sockets, or processes;
- read agent manifests or capabilities;
- verify package signatures;
- mint secure-channel identifiers;
- poll clocks or sleep; or
- own daemon scheduling, backoff, rate limiting, or circuit breakers.

Time, storage, and process authority remain injected. The runnable
orchestrator composes this kernel with package verification, the secure host
channel, the authenticated lifecycle messages defined in
[`host-control-protocol.md`](host-control-protocol.md), and a concrete OS-process
supervisor specified in
[`process-host-supervisor.md`](process-host-supervisor.md).

## Authoritative Supervisor Contract

One supervisor observation is either `Absent` or an `Instance` containing:

- the exact package hash used to launch it;
- lifecycle status: `Starting`, `Running`, `Stopping`, or `Exited`;
- process ID and start time while active;
- last heartbeat and UUID-v7 control-channel ID while running; and
- an optional exit code after exit (`0` means clean; non-zero or missing means
  failure).

Constructors enforce bounded structural validity. An active instance requires a
non-zero PID and start time. A running instance additionally requires a
heartbeat and control channel. An exited instance retains no PID or channel.
Heartbeats cannot precede process start. Reconciliation also rejects start or
heartbeat timestamps later than the caller-provided monotonic `now_ns`.

`HostSupervisor::inspect` must derive this evidence from an owned process
handle, authenticated control channel, or equivalent authoritative source. PID
existence alone is insufficient because PIDs can be reused.

For D18 host processes, `Running` begins only after a matching authenticated
`Ready(package_hash)` control message. Heartbeat time is the supervisor's trusted
monotonic receipt time for an authenticated `Heartbeat`; it is never a timestamp
asserted by the child.

## Reconciliation Tick

The caller loads registry entries in stable host-name order and invokes one
bounded tick with `now_ns` and a non-zero maximum heartbeat age.

Each host tick follows this sequence:

1. Inspect the supervisor. Ignore all cached liveness fields for authority.
2. Validate the observation and compare its package hash to the registration.
3. Derive one of `Observe`, `Start`, `Stop`, or `Defer`.
4. For `Start` or `Stop`, CAS a transitional registry observation before the
   external mutation.
5. Perform exactly one idempotent supervisor mutation.
6. Leave final liveness discovery to the next tick. If start fails, best-effort
   CAS an inactive failure observation. If stop fails, restore the authoritative
   live observation, except that a durable quarantine remains in force. A later
   tick retries convergence.

A concurrent registry edit can make either CAS fail. No supervisor mutation is
performed if the transition claim loses its CAS. If desired state changes after
a successful claim, a later tick observes the new revision and converges it.
This is bounded eventual convergence across storage and process systems, not an
impossible cross-system atomic transaction.

## Decision Matrix

### Desired `Stopped`

- `Absent` or `Exited`: record `Stopped` and preserve lifecycle counters.
- Active matching or mismatched instance: claim `Stopping`, then stop it.

### Desired `Running`

- Matching, fresh `Running`: record the authoritative live observation.
- Matching `Starting`: record `Starting`.
- Matching `Stopping`: defer until it becomes absent or exited.
- Mismatched active instance: claim `Restarting`, then stop it. A later tick
  starts only the registered package.
- Stale matching `Running`: restart only if policy permits; otherwise stop and
  record the failure.
- `Exited(0)`: `Always` restarts; `OnFailure` and `Never` record `Stopped`.
- `Exited(non-zero)` or `Exited(None)`: `Always` and `OnFailure` restart;
  `Never` records `Crashed`.
- `Absent` with no prior start evidence: start once for every policy, including
  `Never`.
- `Absent` after a clean completion: restart only for `Always`.
- `Absent` after failure: restart for `Always` and `OnFailure`.
- Cached `Starting` or `Restarting` with no authoritative instance: retry the
  interrupted launch.
- Unexpired `Quarantined`: defer while absent and drain any unexpectedly active
  instance without clearing the quarantine. At expiry, restart only if policy
  permits.

`restart_count` increments only when relaunching a previously started or failed
host, not on its first launch. Overflow fails closed into a permanent
quarantine. The caller-provided time becomes `last_restart_ns`.

## Heartbeat and Identity Safety

Age uses `now_ns - last_heartbeat_ns`; it never uses wall-clock time. A heartbeat
is stale only when its age is strictly greater than the configured maximum, so
the boundary itself remains healthy.

A package hash mismatch is never adopted into the registry. The reconciler
first stops that instance and records `Restarting`; it does not start a second
same-name instance in the same tick.

The reconciler copies PID and channel ID only from a validated current
supervisor observation. When an instance becomes inactive, both are cleared.

## Errors and Boundedness

Registry corruption, CAS conflicts, invalid supervisor observations, and
supervisor failures are explicit errors. Diagnostics contain host names and
operation names but no package bodies, secrets, channel payloads, or manifest
contents.

One full tick is bounded to 4096 registry entries by the registry itself and at
most one start or stop call per entry. No internal retry loop, sleep, or
unbounded allocation is allowed.

## Required Tests

The package must cover at least:

- first launch for all restart policies;
- clean and failed exits under every restart policy;
- stale cached PID ignored after orchestrator restart;
- fresh and boundary-age heartbeats retained;
- future and stale heartbeat handling;
- package-hash mismatch drained before replacement;
- stopped intent winning over any active state;
- quarantine deferral and expiry;
- restart counter preservation, increment, and overflow;
- claim CAS failure causing no supervisor mutation;
- supervisor start/stop failure recovery state;
- stable multi-host ordering and one mutation per host; and
- malformed supervisor observations and UUID-v7 channel validation.

Line coverage must be at least 95 percent. The crate must forbid unsafe code and
declare no direct filesystem, network, environment, process, clock, random, or
stream capability.
