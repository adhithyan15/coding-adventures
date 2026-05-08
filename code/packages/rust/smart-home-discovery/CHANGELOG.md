# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-08

### Added

- Initial pure discovery package for D23 smart-home bridge candidates.
- Discovery source taxonomy for mDNS, SSDP, manual, cloud fallback, and
  simulator/test records.
- Manual bridge input normalization into discovery records.
- mDNS advertisement endpoint helpers.
- Deterministic in-memory discovery catalog with replacement and query helpers.
- Preferred-record upserts and freshness filtering for repeated discovery
  loops.
- Projection from discovery candidates into unpaired `smart-home-core::Bridge`
  records.
