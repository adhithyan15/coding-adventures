# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- `IntegrationDescriptor` builder/query helpers plus a canonical integration
  descriptor catalog for Hue, Zigbee, Z-Wave, Thread, Matter, and MQTT
  bootstrap families.
- `IntegrationCatalogSummary` and `canonical_integration_catalog_summary()` for
  compact read-side inspection of integration coverage.
- `SmartHomeToolCatalogSummary` and `smart_home_tool_catalog_summary()` for
  compact read-side inspection of the smart-home tool surface.
- Health and command-result status helpers for shared supervision/read-side
  classification of pairing, attention, acceptance, rejection, and timeout
  states.
- `CapabilitySurfaceSummary` and `Entity::capability_summary()` for compact
  describe-capabilities views over entity capability surfaces.

## [0.1.0] - 2026-05-06

### Added

- Normalized bridge, device, entity, capability, event, command, scene, and
  state snapshot types for D23.
- Protocol identifier records for Hue, Zigbee, Z-Wave, Thread, Matter, MQTT,
  and vendor adapters.
- D18D-style smart-home tool descriptors and command risk-tier helpers.
- Read-only `smart_home.observe_supervision` tool descriptor for status loops.
- Agent capability grant primitives for checking smart-home tool access before
  runtime dispatch.
- Authorization-decision records for capturing allowed or denied tool/command
  checks with matched and missing grants.
- Canonical capability catalog helpers for light, scene, lock, climate, sensor,
  and input integration families.
- MQTT topic names, filters, QoS levels, roles, and bindings for MQTT-backed
  device integrations.
