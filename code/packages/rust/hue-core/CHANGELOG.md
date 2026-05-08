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
- Mapping helpers from discovered Hue bridge/device/light records into
  `smart-home-core`.
- Hue light state update helpers that project partial Hue light changes into
  normalized `StateDelta` records.
