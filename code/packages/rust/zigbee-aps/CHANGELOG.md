# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- `ApsFrameBatchSummary` for payload-free APS frame-stream delivery, profile,
  cluster, security, ack-request, and payload-volume rollups.
- `ApsFrameSummary` for payload-free APS routing, delivery, profile, and
  cluster read models.
- `ApsCommandFrame`, `ApsCommandId`, and `ApsCommandSummary` for APS command
  identifiers, key-management classification, and command payload preservation.
- `BindingTableSummary` for counting group/device bindings, cluster families,
  unique APS binding endpoints, source endpoint shapes, and cluster coverage.
- `BindingTableReadinessSummary` for application-source, destination, cluster,
  and source-endpoint hygiene checks.

## [0.1.0] - 2026-05-06

### Added

- APS frame-control, endpoint, group, cluster, profile, counter, and delivery
  mode primitives.
- APS frame parser/encoder for unicast, broadcast, group, and indirect
  addressing with payload preservation.
- Endpoint/cluster/profile addressing helpers and an in-memory APS binding table
  for device and group destinations.
