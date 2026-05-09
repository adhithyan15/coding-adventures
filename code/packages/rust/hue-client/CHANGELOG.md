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
- Hue bridge resource decoding for paired bridge identity and time zone data.
- Hue device resource decoding with product metadata and CLIP v2 service refs.
- Hue grouped-light resource decoding for room/zone aggregate lights.
- Hue room, zone, and scene resource decoding for area-scoped scene recall.
- Hue motion and button resource decoding for normalized sensor/input support.
- Hue event-stream Server-Sent Events parsing into batches and raw resource
  records.
- Incremental Hue event-stream decoder for split Server-Sent Events chunks.
- Hue light, motion, and button state update extraction from resource snapshots
  and event-stream batches for normalized runtime state deltas.
