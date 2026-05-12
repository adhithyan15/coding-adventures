# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- MQTT publication specs for outbound command topics, including QoS, retain,
  payload format, command/correlation context, deterministic publication keys,
  and audit metadata projection.
- Home Assistant MQTT discovery specs for deterministic config topics, state
  and availability subscriptions, command publications, and discovery metadata.
- `EventStreamFleetSummary` and `event_stream_fleet_summary_at()` for compact
  supervisor health, cursor coverage, gap, heartbeat, and restart-readiness
  rollups across stream workers.

## [0.1.0] - 2026-05-08

### Added

- Event stream transport, status, cursor, and checkpoint primitives.
- Deterministic stream specifications for Hue-style SSE, WebSocket, MQTT,
  cloud push, serial, and radio report workers.
- MQTT topic filter, subscription, QoS, retain-policy, and publication cursor
  primitives.
- Bounded reconnect policy with deterministic exponential backoff.
- Event stream runtime state helpers for connection, heartbeat, event, gap,
  disconnect, stale-state, and restart-plan decisions.
- Checkpoint resume helpers that rebuild stream state from durable cursors while
  rejecting mismatched stream ids.
- Heartbeat deadline and restart schedules for batching supervisor wakeups
  across multiple stream workers.
- Event stream state query options for filtering by integration, bridge,
  status, transport, cursor needs, heartbeat deadlines, stale state, reconnect
  readiness, restart plans, pending gaps, sort order, and bounded result count.
