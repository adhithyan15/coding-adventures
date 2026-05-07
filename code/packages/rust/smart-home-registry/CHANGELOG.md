# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-06

### Added

- `InMemorySmartHomeRegistry` for bridge, device, entity, scene, state, event,
  and protocol-native identifier indexes.
- Duplicate protocol identifier detection so Hue, Zigbee, Z-Wave, Thread, and
  future Matter resources cannot silently alias different normalized records.
- Event recording and state-cache updates from normalized `DeviceEvent`
  `StateDelta` values.
- Device/entity selectors for bridge, health, kind, capability, and cached-state
  freshness queries.
