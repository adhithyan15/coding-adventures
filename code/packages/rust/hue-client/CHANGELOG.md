# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Redacted debug output and drop-time application-key zeroization for Hue
  client configuration and credential-bearing HTTP requests.
- Hue snapshot summaries for compact resource-family, relationship-ref, scene
  action, and state-projection coverage over typed CLIP v2 snapshots, plus
  relationship, scene-state, projectable-surface, and partial-lighting
  predicates.
- Hue snapshot readiness summaries for bridge identity, lighting, area
  relationship, scene, sensor/input, and state projection handoff checks.
- Hue event-stream summaries for compact retry-hint, record, resource-item, and
  resource-type coverage over parsed Server-Sent Events batches, plus typed-item
  and multi-type predicates.
- `HueClient::get_grouped_light_state_updates` for room/zone aggregate-light
  state reads through the facade.
- Unified Hue state update extraction from full snapshots and event-stream
  batches, plus `HueClient::get_state_updates` for a single aggregate read.
- `HueClient::send_command_plan` and `send_command_plan()` for serially
  executing planned Hue commands and collecting per-command envelopes.

## [0.1.0] - 2026-05-06

### Added

- Transport-neutral CLIP v2 request builders for registration, resource reads,
  commands, and event-stream connections.
- Injectable `HueTransport` trait plus `HueClient` facade for tests and later
  runtime adapters.
- Typed aggregate `HueSnapshot` parsing from a single CLIP v2 resource snapshot
  envelope.
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
- Hue grouped-light state update extraction from resource snapshots and
  event-stream batches for room/zone aggregate state.
