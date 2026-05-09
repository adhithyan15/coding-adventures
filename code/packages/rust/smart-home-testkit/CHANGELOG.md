# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-08

### Added

- Deterministic fixture clock for freshness and supervisor tests.
- Normalized Hue-like bridge, device, light entity, and sensor entity fixtures.
- Registry seeding helpers for installing normalized fixture records into the
  in-memory smart-home registry.
- Helpers for confirmed, stale, and optimistic state snapshots.
- Helpers for update, unavailable, and error device events.
- Scripted fake event stream with event, disconnect, and gap markers.
- Hue SSE event-stream spec/state helpers and a driver that advances both
  event-stream supervision state and seeded `smart-home-runtime` instances.
- Fake command bus helpers for deterministic command/result assertions.
- Fake MQTT broker publication helpers for retained-message and payload tests.
