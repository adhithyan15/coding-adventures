# Changelog

All notable changes to this package will be documented in this file.

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
