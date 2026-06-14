# smart-home-discovery

Discovery-record and mDNS scan primitives for the D23 smart-home runtime.

Most of this crate is transport-neutral record shaping. Its LAN mDNS helpers use
`udp-client` for bounded datagram collection, but still keep vendor-specific
normalization, cloud APIs, credential storage, and actor supervision outside
this package. It gives discovery workers a shared shape for:

- mDNS/SSDP/BLE/USB/DHCP/MQTT/webhook/cloud/manual discovery sources
- bounded IPv4/IPv6 mDNS scans that send PTR questions and collect replies
- per-interface IPv4/IPv6 mDNS worker scan requests, plans, and aggregate
  reports for supervised discovery actors
- injectable mDNS worker scan executors that run request/report and grouped
  plan handoffs without coupling runtime mutation to socket I/O
- mDNS/DNS-SD PTR/SRV/TXT/A/AAAA response parsing into advertisements
- per-datagram scan failures for malformed mDNS responses
- bridge candidate records with stable integration/native identifiers
- stable discovery fingerprints and freshness signals for supervisor loops
- confidence, pairing requirement, interface, and expiry metadata for repeated
  observations
- manual bridge address normalization
- mDNS advertisement endpoint helpers
- deterministic candidate catalogs
- first-class discovery worker runs with per-source failures, durations, and
  catalog-ingest summaries
- catalog-backed scan and pairing hints derived from first-party integration
  metadata
- pairing plans that rank discovered bridges and identify the next pairing
  action before any worker opens a credential flow
- pairing-plan summaries for host/UI loops by actionability, required human
  work, freshness, source, requirement, action, and next actionable target
- bounded pairing-plan queries for integration, source, freshness, requirement,
  action, priority, human-action, sort, and limit selectors
- source/address/time preference scoring for duplicate bridge candidates
- catalog-level fresh/stale/expired signal summaries with next transition time
- catalog-level discovery record summaries by source, confidence, pairing
  requirement, address coverage, and freshness
- worker-run summaries that separate reported records from inserted, replaced,
  and ignored catalog outcomes
- freshness filtering for supervisor/discovery loops
- projection into unpaired `smart-home-core::Bridge` records

Hue-specific discovery, Vault credential storage, and actor supervision live in
later integration/runtime crates.

## Dependencies

- smart-home-core
- smart-home-integration-catalog
- udp-client

## Development

```bash
bash BUILD
```
