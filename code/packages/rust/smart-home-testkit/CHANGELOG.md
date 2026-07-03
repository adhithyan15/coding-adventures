# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Scripted mDNS worker scan executor fixtures that can run runtime-produced
  scan plans into deterministic reports without opening network sockets.
- A Hue pairing fixture path that matches the application registration HTTP
  plan, parses the scripted Hue response, simulates Vault storage, and completes
  the runtime pairing session without raw secret values in audit metadata.
- A normalized Hue-style room scene fixture seeded into registry and runtime
  helpers for scene inventory/read tests.

## [0.1.0] - 2026-05-08

### Added

- Deterministic Hue mDNS scan-report fixtures, with discovery worker-run
  fixtures now flowing through `MdnsWorkerScanReport` before seeding runtime
  discovery catalogs.
- Deterministic Hue discovery worker schedule fixtures, with discovery runtime
  seeding now flowing through `SmartHomeRuntime::record_scheduled_discovery_worker_run`.
- Deterministic fixture clock for freshness and supervisor tests.
- Normalized Hue-like bridge, device, light entity, and sensor entity fixtures.
- Deterministic Hue discovery record and discovery-runtime fixtures built
  through `hue-core` mDNS normalization for testing unpaired bridge candidates
  without network I/O.
- Deterministic Hue discovery worker-run fixtures that seed discovery runtime
  candidates through scheduled runtime ingest.
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
- Non-consuming fake local HTTP server summaries for response status, method,
  body-size, metadata, and observation-window assertions.
- Fake MQTT broker publication helpers for retained-message and payload tests.
- Non-consuming fake MQTT broker summaries for retained/live publication,
  payload-size, metadata, and observation-window assertions.
- Scripted MQTT subscription helpers for deterministic topic-filter delivery
  matching and QoS assertions.
- Read-only fake MQTT broker publication queries by topic, prefix, retained flag,
  metadata, observation time, sort, and limit.
