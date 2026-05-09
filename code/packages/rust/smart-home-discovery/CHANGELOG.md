# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-08

### Added

- Initial pure discovery package for D23 smart-home bridge candidates.
- Discovery source taxonomy for mDNS, SSDP, manual, cloud fallback, and
  simulator/test records.
- Discovery source taxonomy now covers Bluetooth, USB, DHCP, MQTT, and webhook
  observations for the broader primitive roadmap.
- Discovery records can carry confidence, pairing requirement, network
  interface, and explicit expiry metadata.
- Discovery records can project stable fingerprints and freshness signals for
  supervisor loops.
- Manual bridge input normalization into discovery records.
- mDNS advertisement endpoint helpers.
- Deterministic in-memory discovery catalog with replacement and query helpers.
- Preferred-record upserts and freshness filtering for repeated discovery
  loops.
- Catalog-level signal summaries count fresh, stale, and expired records and
  report the next freshness transition.
- Projection from discovery candidates into unpaired `smart-home-core::Bridge`
  records.
- Catalog-backed discovery hints that translate first-party integration entries
  into source, transport, protocol, priority, and pairing requirements.
- Deterministic pairing plans that rank discovered bridge candidates and map
  pairing requirements into next actions without opening credential flows.
