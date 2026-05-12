# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- `RegistrySupervisionSummary` plus read-view and registry helpers for compact
  health, pairing, and refresh-work status loops.

## [0.1.0] - 2026-05-06

### Added

- `InMemorySmartHomeRegistry` for bridge, device, entity, scene, state, event,
  and protocol-native identifier indexes.
- Duplicate protocol identifier detection so Hue, Zigbee, Z-Wave, Thread, and
  future Matter resources cannot silently alias different normalized records.
- Event recording and state-cache updates from normalized `DeviceEvent`
  `StateDelta` values.
- Event selector queries for bridge/device/entity/type and timestamp-bounded
  replay windows.
- Device/entity selectors for bridge, health, kind, capability, and cached-state
  freshness queries.
- State refresh plans that enumerate missing or stale entity state with
  bridge/device identity for pollers and supervisors.
- State refresh result application with reports for refreshed and still-missing
  entities.
- Capability-grant storage, principal indexes, active-grant queries, and
  status updates for Chief of Staff agent authorization.
- Authorization-decision audit storage with principal and outcome lookup.
- Read-only and read-write registry views for D18D-style read/write separation.
