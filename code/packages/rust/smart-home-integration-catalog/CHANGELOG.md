# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-08

### Added

- Integration catalog enums for category, connectivity class, discovery,
  authentication, and implementation status.
- Primitive-family metadata for discovery, transport, auth/pairing, command
  mapping, capability policy, Vault leases, supervision, camera/media,
  telemetry, and test simulators.
- First-party seed catalog entries for Hue, Zigbee, Z-Wave, Thread, MQTT,
  Matter, HomeKit Controller, ESPHome, Tasmota, Shelly, TP-Link/Tapo, WLED,
  LIFX, cameras/media, energy/climate, and cloud hubs.
- Virtual alias entries for product lines supported by another integration or
  standard.
- Read-only D18D tool descriptors for listing/describing integrations and
  primitive families.
- Typed ecosystem-survey source rows for the cross-platform references used to
  plan primitive families across Home Assistant, Hubitat, Homey Pro,
  SmartThings, openHAB, Homebridge, ioBroker, Domoticz, Jeedom, HomeSeer, Apple
  Home, Google Home, Alexa, Z-Wave Alliance, and Thread Group.
- Query helpers for integration id, category, connectivity, capability,
  primitive family, implementation status, and rollout priority.
- Primitive backlog planning helpers for ranking the shared primitive families
  needed by priority-bounded rollout waves.
- Computed policy-surface helpers so Chief of Staff tools can identify camera,
  entry-access, climate, energy, cloud, credential, radio-network, and local
  actuation review boundaries before activating integrations.
