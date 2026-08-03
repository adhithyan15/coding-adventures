# D18 Chief Daemon Runtime

## Status

This document specifies the fail-closed scheduling and serving boundary for the
D18 Chief daemon.

The Rust implementation package is `chief-of-staff-daemon-runtime`.

## Purpose

The runtime binds the authenticated `chief-of-staff-daemon-api` WebSocket
listener to the bounded reconciliation operation owned by the same serialized
control plane. It turns the request-driven API into a continuously converging
daemon without moving durable state, process authority, channel keys, message
payloads, or trust decisions into the scheduler.

Configuration-file parsing, concrete identity and trusted-key loading, the final
executable, OS service installation, pipeline wiring, and streaming operations
remain later adapters.

## Construction

The caller supplies:

- a concrete `TransportPlatform`;
- an explicit listener `BindAddress` and WebSocket options;
- an owned `Arc<DaemonApi<C, A>>`; and
- a validated non-zero reconciliation interval.

Binding opens only the explicitly supplied listener. It does not select a
wildcard address, read environment variables, discover a home directory, or
load configuration.

## Startup Ordering

`serve` performs one synchronous reconciliation tick before accepting any
WebSocket request. If that tick fails, the listener never serves and the exact
failure category is returned to the caller.

This startup tick is mandatory. Persisted PIDs, statuses, heartbeats, and
control-channel identifiers are historical evidence, not live process
authority after a daemon restart.

## Background Scheduling

After successful startup reconciliation, one scheduler thread waits for the
configured interval and invokes the same serialized `DaemonApi::reconcile_once`
boundary. WebSocket requests and scheduled ticks cannot concurrently mutate the
control plane because both use its existing mutex.

Each tick remains bounded by the orchestrator core. The scheduler never loops
inside a reconciliation operation and does not inspect payloads or keys.

## Failure and Shutdown

A background reconciliation failure is fatal for the serving runtime. The
scheduler requests cooperative WebSocket shutdown, the serving thread joins the
scheduler, and `serve` returns a stable reconciliation error. Continuing to
accept lifecycle commands while convergence is unavailable would present a
misleadingly healthy control surface.

An external WebSocket stop request also wakes and joins the scheduler without
waiting for the full reconciliation interval. No detached scheduler thread may
outlive `serve`.

Transport failure, reconciliation failure, invalid configuration, and scheduler
panic remain distinct stable error categories. Diagnostics contain no request
contents, package bytes, credentials, key material, or message payloads.

## Capabilities

The runtime listens through the already-declared WebSocket transport and reads
or waits on process-local time for scheduling. Concrete storage, process,
randomness, identity, keyring, and authorization adapters retain their own
capability declarations.

## Required Tests

The package must cover:

- rejection of a zero reconciliation interval;
- successful startup reconciliation before serving;
- startup failure before any request is accepted;
- periodic reconciliation through the shared serialized control plane;
- fatal background failure stopping the listener and surfacing the failure; and
- cooperative external shutdown joining the scheduler promptly.

The package forbids unsafe code and targets at least 95 percent line coverage.
