# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- `UdpMulticast` source and catalog-mechanism projection for vendor LAN
  discovery over local UDP endpoints.
- `WsDiscovery` source and catalog-mechanism projection for ONVIF discovery
  records over local HTTP camera endpoints.
- Bounded IPv4/IPv6 mDNS scan helpers that build PTR queries, collect UDP
  replies through `udp-client`, and return deterministic `MdnsScanResult`
  envelopes.
- `MdnsWorkerScanRequest`, `MdnsWorkerScanPlan`, and `MdnsWorkerScanReport`
  for handing scheduled mDNS work to supervised per-interface scan actors and
  aggregating interface-level successes or failures.
- `MdnsWorkerScanExecutor`, default UDP execution helpers, and grouped plan
  runners for turning scheduled mDNS requests into worker scan reports while
  keeping socket I/O injectable for tests and supervision.
- mDNS/DNS-SD response parsing for PTR, SRV, TXT, A, and AAAA records,
  including compressed DNS names and per-datagram scan failures for malformed
  replies.
- `DiscoveryRecordSummary` and `DiscoveryCatalog::record_summary_at` for
  aggregate discovery planning by source, confidence, pairing requirement,
  address coverage, and freshness status.
- `DiscoveryWorkerRun`, worker failure records, worker run status, and
  `DiscoveryWorkerRunSummary` for deterministic discovery-worker handoff into
  catalog ingest.
- `DiscoveryPairingPlanSummary`, `DiscoveryPairingPlan::summary`, and
  `DiscoveryCatalog::pairing_plan_summary_at` for host-facing pairing queue
  rollups by actionability, human action, freshness, source, requirement,
  action, and next actionable target.

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
- Bounded pairing-plan query options for integration, source, freshness,
  requirement, action, priority, human-action, sorting, and limits.
