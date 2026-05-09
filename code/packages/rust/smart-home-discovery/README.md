# smart-home-discovery

Pure discovery-record primitives for the D23 smart-home runtime.

This crate does not open sockets, send mDNS packets, call vendor cloud APIs, or
write credentials. It gives discovery workers a shared shape for:

- mDNS/SSDP/BLE/USB/DHCP/MQTT/webhook/cloud/manual discovery sources
- bridge candidate records with stable integration/native identifiers
- stable discovery fingerprints and freshness signals for supervisor loops
- confidence, pairing requirement, interface, and expiry metadata for repeated
  observations
- manual bridge address normalization
- mDNS advertisement endpoint helpers
- deterministic candidate catalogs
- catalog-backed scan and pairing hints derived from first-party integration
  metadata
- pairing plans that rank discovered bridges and identify the next pairing
  action before any worker opens a credential flow
- bounded pairing-plan queries for integration, source, freshness, requirement,
  action, priority, human-action, sort, and limit selectors
- source/address/time preference scoring for duplicate bridge candidates
- catalog-level fresh/stale/expired signal summaries with next transition time
- freshness filtering for supervisor/discovery loops
- projection into unpaired `smart-home-core::Bridge` records

Network transports, Hue-specific discovery, Vault credential storage, and actor
supervision live in later integration/runtime crates.

## Dependencies

- smart-home-core
- smart-home-integration-catalog

## Development

```bash
bash BUILD
```
