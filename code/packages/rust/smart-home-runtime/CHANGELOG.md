# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-06

### Added

- Runtime event bus with subscription filters for all, bridge, entity, command,
  and supervision events.
- Event replay checkpoints that let new subscribers catch up from a prior
  runtime event-log position before receiving live deliveries.
- Boxed registry-backed runtime errors to keep public `Result` error payloads
  small as the runtime API grows.
- `SmartHomeRuntime` facade over `smart-home-registry` for command validation,
  optimistic state caching, event replay, and bridge health updates.
- Grant-backed command authorization path for checking Chief of Staff agent
  capabilities before command acceptance.
- Registry-backed authorization decision auditing for accepted and rejected
  authorized commands.
- Registry-backed tool authorization decisions for Chief of Staff tool calls.
- D18D-style read tool execution for listing bridges/devices, reading entity
  state, describing entity capabilities, inspecting bridge health, and observing
  supervision status through the registry without dispatching integration work.
- D18D-style subscribe tool execution for authorized, filtered event-stream
  subscriptions with checkpointed replay metadata.
- D18D-style pair-bridge execution with short-lived pairing sessions, VaultRef
  completion, and credential-free bridge registry updates.
- D18D-style command tool execution for authorized `smart_home.command` calls,
  including tool-level audit decisions, command-level audit decisions, and
  deterministic runtime command/correlation ids.
- Supervisor primitives for bridge-worker heartbeat tracking and restart
  signaling.
- Worker heartbeat deadline schedules for deterministic supervisor wakeups.
- Desired-state reconciliation for missing, stale, or drifted entity state,
  producing deterministic corrective commands and supervision events.
- Non-mutating supervision plans that preview state refresh targets,
  pairing expiry, desired-state drift, and overdue worker restarts before a tick
  writes.
- Read-only supervision observations that combine due supervision work with
  worker heartbeat schedules for status tools.
- Deterministic supervision ticks that combine optimistic-state expiry,
  desired-state reconciliation, and worker restart checks into one report.
- Deterministic worker restart plans for inspecting overdue bridge workers
  before mutating supervisor state.
- Worker restart reconciliation marks registered bridges degraded and emits
  deterministic health events.
- Read-side queries for event-log entries, subscription backlogs, pairing
  sessions, desired-state targets, and supervised bridge workers.
- Bounded event-bus delivery peeking and draining for subscription polling.
- Event-bus unsubscribe lifecycle that returns undelivered events and clears
  subscription delivery state.
- Compact read-only runtime snapshots that summarize registry counts,
  event-bus backlog, supervisor restart pressure, pairing expiry, desired
  state, and stale cached state without mutating runtime state.
- Event-bus backlog status helpers that distinguish absent subscribers,
  caught-up streams, and backlogged streams.
