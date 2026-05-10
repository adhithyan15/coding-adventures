# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- `ApsFrameSummary` for payload-free APS routing, delivery, profile, and
  cluster read models.
- `BindingTableSummary` for counting group/device bindings, cluster families,
  and unique APS binding endpoints.

## [0.1.0] - 2026-05-06

### Added

- APS frame-control, endpoint, group, cluster, profile, counter, and delivery
  mode primitives.
- APS frame parser/encoder for unicast, broadcast, group, and indirect
  addressing with payload preservation.
- Endpoint/cluster/profile addressing helpers and an in-memory APS binding table
  for device and group destinations.
