# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-06

### Added

- Zigbee NWK address and frame-control primitives.
- NWK frame parser/encoder for base headers, optional IEEE addresses,
  multicast control, radius, sequence, and payload bytes.
- Source-route relay subframe parsing, encoding, and next-relay helpers.
- Neighbor and route table primitives for router/end-device relationships,
  freshness expiry, router candidate ranking, and next-hop lookup.
