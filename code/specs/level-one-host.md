# D18 Level 1 Host

## Status

This document specifies the first concrete child executable for D18 Level 1
`SKILL.md` packages. The implementation package and binary are both named
`chief-of-staff-host`.

## Purpose

The process supervisor owns package registration, process authority, and the
orchestrator half of the encrypted control channel. The skill runtime owns prompt
construction and turn ordering but has no process or transport capability. The
Level 1 host composes those existing boundaries into the executable selected by
the daemon's explicit host path.

The host is not a second orchestrator. It receives only one launch's authenticated
public package trust and pipeline bindings, independently verifies the sealed
package in its working directory, interprets the signed skill, and sends data-plane
requests over the existing authenticated session.

## Launch and Verification

The executable accepts exactly `--package-runtime skill`. Missing, extra, non-UTF-8,
or different runtime arguments fail before bootstrap. Deno packages are rejected
until a separately reviewed adapter is composed. No environment variable selects
the package, runtime, channels, model, or trust.

After secure bootstrap, the host:

1. receives the exact authenticated public package trust;
2. constructs a one-key `PackageKeyring`;
3. independently verifies the sealed package rooted at its current directory;
4. requires the verified runtime to be `Skill`;
5. receives authenticated launch bindings;
6. derives `LevelOneLaunchPlan` and requires exact signed names and directions;
7. requires exactly one read binding and one write binding; and
8. sends `Ready` with the independently verified package digest.

No data-plane request or readiness record is emitted before all checks pass.

## V1 Topology

The signed package and launch protocol permit multiple read and write bindings,
but the current Level 1 turn has one input and one output. Choosing the first name,
merging channels, or broadcasting responses would silently invent routing policy.
V1 therefore fails closed unless there is exactly one read channel and one write
channel. Multi-channel packages remain valid inputs to discovery and wiring; a
future host revision must define deterministic routing before executing them.

## Turn and Recovery Ordering

Each iteration requests at most one verified message. An empty page sleeps for a
bounded interval before the next poll. A non-empty page executes these operations
in order:

1. require UTF-8 input;
2. construct the provider-neutral completion from authenticated skill instructions
   and model settings;
3. request completion through the parent-side authorized service;
4. publish the non-empty text result to the sole write channel; and
5. acknowledge the input on the sole read channel.

A redacted `Unavailable` response to the read-only receive operation follows the
same bounded delay as an empty page. This lets the daemon's fail-closed placeholder
service remain supervised without a restart loop. Other receive failures and every
completion, publication, acknowledgement, response-shape, or control failure are
terminal for that child. Because acknowledgement is last, failures before it leave
the durable receiver cursor unchanged for supervised restart and replay. The host
never retries a possibly completed provider or publication operation inside the
same process without a durable idempotency contract.

## Health and Shutdown

The host emits authenticated heartbeats after readiness at a bounded interval. It
does not busy-spin while its input channel is empty. The serialized protocol still
permits only one request in flight.

The orchestrator may send authenticated `Terminate` instead of a data-plane
response. The child helper surfaces this as a distinct graceful condition, and the
host exits successfully whether termination arrives during receive, completion,
publish, or acknowledgement. Authentication, framing, correlation, and unexpected
message-kind failures remain unsuccessful exits.

## Boundaries

The host directly uses only its standard streams, process arguments, current
package directory, monotonic process-local time, and bounded sleep. It opens no
network socket, reads no model credential, holds no channel key, and accesses no
durable channel store. Channel cryptography, key custody, authorization, and model
execution remain behind the parent-side `HostDataPlaneService`.

## Required Tests

The package must cover:

- exact process argument selection and unsupported runtime rejection;
- one-read/one-write topology acceptance and multi-channel rejection;
- real cross-platform process bootstrap with a signed Level 1 package;
- independent child verification and exact launch-plan matching before readiness;
- receive, completion, publish, acknowledge, then idle receive over the encrypted
  process pipes;
- exact channel UUID, model, prompt, payload, and acknowledgement mapping;
- authenticated termination during an exchange producing exit code zero; and
- strict Clippy and rustdoc gates with unsafe code forbidden.
