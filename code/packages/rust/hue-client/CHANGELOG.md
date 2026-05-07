# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-06

### Added

- Transport-neutral CLIP v2 request builders for registration, resource reads,
  commands, and event-stream connections.
- Injectable `HueTransport` trait plus `HueClient` facade for tests and later
  runtime adapters.
- Hue v2 envelope/error parsing, registration response parsing, and light
  resource decoding.
- Hue event-stream Server-Sent Events parsing into batches and raw resource
  records.
- Hue light state update extraction from resource snapshots and event-stream
  batches for normalized runtime state deltas.
