# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Payload-free Matter command invocation and attribute-report batch summaries
  for runtime planning and read-model telemetry.

## [0.1.0] - 2026-05-09

### Added

- Initial Matter identifier and cluster constants for D23 Thread/Matter
  integration work.
- Cluster-to-`smart-home-core` capability mapping for lighting, sensing, locks,
  climate, scenes, and input devices.
- Attribute-report mapping helpers for on/off, level, temperature, humidity,
  occupancy, door-lock state, and thermostat setpoints.
- Canonical `DeviceCommand` projection into Matter on/off, level,
  color-temperature, and door-lock command invocations.
