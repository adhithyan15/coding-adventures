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
  state, describing entity capabilities, and inspecting bridge health through
  the registry without dispatching integration work.
- Supervisor primitives for bridge-worker heartbeat tracking and restart
  signaling.
- Desired-state reconciliation for missing, stale, or drifted entity state,
  producing deterministic corrective commands and supervision events.
- Deterministic supervision ticks that combine optimistic-state expiry,
  desired-state reconciliation, and worker restart checks into one report.
- Deterministic worker restart plans for inspecting overdue bridge workers
  before mutating supervisor state.
- Worker restart reconciliation marks registered bridges degraded and emits
  deterministic health events.
