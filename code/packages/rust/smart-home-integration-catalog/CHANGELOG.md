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
- Ecosystem primitive coverage reports that map surveyed platforms onto rollout
  backlog primitive families.
- Primitive coverage summary helpers for counting uncovered, single-source, and
  multi-platform primitive-backlog rows.
- Ecosystem platform coverage item and summary helpers that show which surveyed
  platform lessons overlap a priority-bounded reusable primitive backlog.
- Query helpers for integration id, category, connectivity, capability,
  primitive family, implementation status, and rollout priority.
- Primitive backlog planning helpers for ranking the shared primitive families
  needed by priority-bounded rollout waves.
- Integration activation planning helpers for resolving virtual aliases,
  standard-backed products, required primitives, capabilities, auth modes,
  dependencies, and review tiers before enabling an integration.
- `IntegrationActivationPlanSummary` for compact direct/delegated target,
  review, local/cloud, dependency, primitive, and capability rollups over
  activation-plan sets.
- `IntegrationActivationCandidate` and `IntegrationActivationCandidateSummary`
  for ranking ready, human-review, and blocked activation work after applying
  host-specific readiness context.
- `IntegrationActivationAction` and `IntegrationActivationActionSummary` for
  converting activation candidates into concrete activate, policy-review,
  primitive, capability, and dependency work items.
- `IntegrationActivationAgendaStage` and `IntegrationActivationAgendaSummary`
  for grouping activation candidates and concrete action work by rollout
  priority wave.
- `IntegrationActivationRunwayStage` and `IntegrationActivationRunwaySummary`
  for grouping activation candidates by rollout priority wave and identifying
  actionable, review, and blocked stages.
- `IntegrationActivationDependencyGraph` plus node, edge, and summary types for
  exposing satisfied and blocking integration prerequisites in rollout plans.
- Integration readiness reports that expose missing primitive families, missing
  capability grants, and missing delegated integrations before activation.
- Integration readiness summaries for compact activation-ready, blocker,
  review, cloud, local, and delegated-target rollups.
- Integration readiness gap inventories that group missing primitive families,
  capability grants, and delegated integration dependencies by affected
  integrations.
- Computed policy-surface helpers so Chief of Staff tools can identify camera,
  entry-access, climate, energy, cloud, credential, radio-network, and local
  actuation review boundaries before activating integrations.
- `IntegrationPolicySurfaceInventoryItem`, `IntegrationPolicySurfaceSummary`,
  and policy-surface inventory helpers for compact review, cloud, local, and
  privilege-tier planning rollups.
- Composable bounded integration catalog queries for combining priority,
  primitive, capability, policy, protocol, local/cloud, and virtual alias
  selectors in read-only D18D tools.
