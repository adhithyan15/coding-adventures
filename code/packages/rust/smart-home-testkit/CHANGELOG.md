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
- Non-consuming scripted fake event stream summaries for supervision tests.
- Hue SSE event-stream spec/state helpers and a driver that advances both
  event-stream supervision state and seeded `smart-home-runtime` instances.
- Fake command bus helpers for deterministic command/result assertions.
- Fake local HTTP response helpers for request-plan matching and read-only
  response queries.
- Fake MQTT broker publication helpers for retained-message and payload tests.
- Scripted MQTT subscription helpers for deterministic topic-filter delivery
  matching and QoS assertions.
- Read-only fake MQTT broker publication queries by topic, prefix, retained flag,
  metadata, observation time, sort, and limit.
