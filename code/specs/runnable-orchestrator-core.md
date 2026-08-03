# D18 Runnable Orchestrator Core

## Status

This document specifies the smallest runnable, transport-independent D18
orchestrator composition.

The implementation package is `chief-of-staff-orchestrator-core`.

## Purpose

The core binds durable host intent, deterministic reconciliation, authoritative
process supervision, and durable channel topology behind one bounded API. It is
the stateful application layer used later by the WebSocket daemon and operator
CLI; it is not itself a socket server, scheduler, daemon installer, or command
parser.

The core is deliberately underpowered. It owns no channel master keys, receiver
private keys, vault secrets, message payloads, or in-process agent handlers.
Verified child processes integrate `chief-of-staff-host-runtime`; the parent
coordinates their lifecycle through `chief-of-staff-process-supervisor` and the
authenticated control channel.

## Composition

The generic core receives:

- one repository `StorageBackend` used by both `ServiceRegistry` and
  `ChannelDefinitionStore`;
- one `HostSupervisor` implementation;
- one injected monotonic clock;
- validated reconciliation configuration; and
- one channel-wiring authorizer.

A production constructor composes the core with `ProcessHostSupervisor`, its
trusted package keyring, long-lived X3DH identity, fixed absolute child program,
fresh UUID-v7 session source, and the same monotonic clock used for authenticated
receipt evidence.

The backend and cryptographic trust objects are borrowed from the runnable
daemon owner. The core does not create a self-referential filesystem backend or
copy zeroizing identity material.

## Host Intent API

The core exposes bounded operations to:

- register one immutable `HostRegistration` with initial desired state;
- load or list registered hosts in stable host-name order;
- CAS-update only `DesiredState` for an existing registration;
- inspect one host using the owned supervisor as live authority; and
- deregister one exact loaded revision only when desired state is `Stopped` and
  supervisor authority reports `Absent` or `Exited`.

Registration preserves the service registry's idempotency rules. A conflicting
package identity under the same host name is rejected. Desired-state updates do
not mutate package path, package hash, or restart policy. Deregistration never
deletes active intent first and then hopes the process stops; callers request
`Stopped`, reconcile, inspect, and only then delete.

## Bounded Reconciliation

`reconcile_once` samples the injected monotonic clock exactly once and invokes
one stable-order `ServiceReconciler::reconcile_all` tick. It does not sleep,
retry forever, or run a background thread. Each host still receives at most one
supervisor mutation per tick as defined by
[`service-registry-reconciliation.md`](service-registry-reconciliation.md).

The core rejects a clock sample earlier than its previous successful tick. It
updates the remembered sample only after reconciliation succeeds, so a failed
tick can be retried against the same time without creating artificial progress.
The runnable daemon chooses scheduling cadence and shutdown policy.

On startup, the daemon invokes the same `reconcile_once` operation. Cached PIDs,
statuses, heartbeats, and control-channel IDs never become live authority merely
because they were persisted before a restart.

## Health Evidence

`health_check` returns the durable loaded host and a fresh
`SupervisorObservation`. The two are intentionally distinct:

- durable state answers what was requested and last recorded; and
- supervisor state answers what this process currently owns and authenticated.

The core does not collapse disagreement into a misleading single status. The
next reconciliation tick performs convergence and durable CAS updates.

## Channel Topology and Authorization

The core may create, load, and irreversibly destroy durable channel definitions.
It never opens an originator or receiver endpoint, distributes a key, or reads a
message. Those operations remain in authorized hosts and the channel endpoint
packages.

Every create or destroy request is first presented to an injected
`ChannelWiringAuthorizer`. The authorizer can consult manifest privilege tiers,
the trust checker, and an approval side channel. An authorization denial or
authorizer failure performs no storage mutation. The core provides no implicit
allow-all production path.

Create remains idempotent only for the byte-identical active definition.
Destroy remains one-way and idempotent through the definition store's revision
CAS contract.

## Public API

The package exposes:

- `OrchestratorCore<S, A>` for injected supervisors and authorizers;
- `ProcessOrchestratorCore<A>` plus a production process-composition
  constructor;
- `ChannelWiringAuthorizer` and exact create/destroy request values;
- `HostHealth`, preserving durable and authoritative views;
- `OrchestratorCoreError`, preserving typed registry, reconciliation,
  supervisor, channel, and authorization failures; and
- registration, desired-state, health, reconciliation, channel-topology, and
  safe-deregistration operations.

Diagnostics must not contain package contents, message payloads, key bytes,
backend roots, executable arguments, or environment values.

## Capabilities

The generic core performs pure coordination over injected interfaces and
declares no direct filesystem, network, process, environment, clock, random, or
stream capability. Concrete storage and process dependencies retain their own
capability manifests. The future daemon binary will declare the union required
by the concrete adapters it constructs.

## Required Tests

The package must cover:

- idempotent registration and conflicting immutable identity;
- stable listing and CAS desired-state updates;
- one-sample bounded reconciliation with start, observe, stop, and restart
  behavior through an injected supervisor;
- clock-regression rejection and retry after a failed tick;
- health preserving separate durable and authoritative views;
- safe deregistration refusing desired-running and active hosts;
- channel create/load/destroy after authorization;
- authorization denial proving no channel mutation;
- stable payload-free error diagnostics; and
- production process-composition construction without spawning until a tick
  actually requests start.

The package forbids unsafe code and targets at least 95 percent line coverage.
