# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- `NeighborTableSummary`, `RouteTableSummary`, and `NwkTopologySummary` read
  models for supervising neighbor roles, relationships, identity coverage,
  link metrics, freshness, router candidates, and route health.

## [0.1.0] - 2026-05-06

### Added

- Zigbee NWK address and frame-control primitives.
- NWK frame parser/encoder for base headers, optional IEEE addresses,
  multicast control, radius, sequence, and payload bytes.
- Source-route relay subframe parsing, encoding, and next-relay helpers.
- Neighbor and route table primitives for router/end-device relationships,
  freshness expiry, router candidate ranking, and next-hop lookup.
- Typed NWK command payload primitives for route request, route reply, network
  status, and route record route-discovery messages.
