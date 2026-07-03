# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-06

### Added

- ZDO descriptor and active-endpoint response parsing.
- APS request builders for node descriptor, simple descriptor, and active
  endpoint discovery.
- APS request builders and status parsers for bind/unbind requests.
- Deterministic interview planning for node descriptor, active endpoint, and
  simple descriptor request sequencing.
- Compact interview-plan summaries for pending ZDO descriptor work.
- Compact node/simple descriptor and interview read summaries for discovery
  tooling.
- Descriptor-inventory summaries for payload-free node, endpoint, profile, and
  cluster coverage reads.
- Descriptor-inventory readiness summaries for identity, endpoint, profile,
  cluster, and lighting coverage handoff checks.
- Unique cluster coverage and profile-family endpoint counts for ZDO interview
  read summaries.
- Read-summary helper predicates for duplicate cluster references, lighting
  cluster coverage, and mixed profile-family interviews.
- Zigbee interview summary projection into `smart-home-core` device records.
