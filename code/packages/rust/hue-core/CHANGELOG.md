# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Hue command-plan projection summaries for generated-command and ignored-delta
  reconciliation telemetry.
- Hue discovery worker-run projection from generic `MdnsScanResult` envelopes,
  preserving scan parse failures as per-source D23 worker failures.
- Hue discovery worker-run projection from aggregate `MdnsWorkerScanReport`
  envelopes, preserving per-interface scan failures and report metadata.
- Hue mDNS and cloud-fallback discovery normalization into D23
  `DiscoveryRecord` candidates, including bridge-candidate batches and
  discovery-record pairing-plan handoff.
- Hue discovery worker-run projection that keeps mDNS/cloud records and
  per-observation failures in a generic D23 discovery-worker envelope.
- Hue application registration request and discovered-bridge pairing plan
  helpers for local physical-presence pairing flows.
- Hue pairing exchange helpers that build local HTTP registration plans, parse
  Hue success and link-button error responses, produce Vault secret payloads,
  and hand off only non-secret completion metadata.
- Hue command summaries and command-plan rollups for payload-free write-surface
  telemetry, including light-surface, surface-breadth, and capability-breadth
  helper predicates.
- Hue scene-set summaries for payload-free recall/read-model telemetry across
  scene batches.
- Grouped-light color-temperature command projection from normalized D23 state
  deltas.
- Hue command planning from normalized D23 `StateDelta` records for direct and
  grouped light writes.
- `HueCommandPlan` for bundling generated Hue commands, request projections,
  and ignored D23 capabilities from reconciliation passes.
- Hue scene summaries for recall planning and read-model telemetry, including
  room/zone scope and projected action coverage.
- Hue state update summaries for read models and event-stream telemetry across
  light, grouped-light, motion, and button updates.
- `HueStateUpdate` and `HueStateUpdateSetSummary` for unified state update
  streams across light, grouped-light, motion, and button resources, including
  mixed-surface, owner-coverage, and partial-state helper predicates.

## [0.1.0] - 2026-05-06

### Added

- Hue CLIP v2 resource type/id/path primitives.
- Structured Hue command intents for light, grouped-light, color-temperature,
  and scene requests.
- Typed Hue bridge resources for paired bridge identity/health refresh.
- Typed Hue device resources with product metadata and service references.
- Typed Hue grouped-light resources for room/zone aggregate lights.
- Typed Hue room, zone, and scene resources with scene recall/core projection
  helpers.
- Typed Hue motion and button resources with normalized sensor/input entity
  projection helpers.
- Mapping helpers from discovered Hue bridge/device/light records into
  `smart-home-core`.
- Hue light, motion, and button state update helpers that project partial Hue
  changes into normalized `StateDelta` records.
- Hue grouped-light state update helpers for room/zone aggregate light state.
- Hue light snapshots and scene desired states now use canonical D23 capability
  ids as object keys, matching smart-home runtime reconciliation.
- Direct Hue light brightness and color-temperature command helpers.
