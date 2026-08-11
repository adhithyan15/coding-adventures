# Changelog

## Unreleased

- Route schedule registration and supervised mDNS ticks through the shared
  `smart-home-controller-runtime` authority with revision-guarded persistence.
- Import legacy service-owned schedule records without overwriting central
  state, while retaining service health and run-report journals separately.
- Add persisted `udp_multicast` discovery-source parsing.

## 0.1.0

- Add an actor-owned scheduled mDNS discovery lifecycle.
- Persist and restore worker cadence, retry pressure, run reports, and service
  health through `StorageBackend`.
- Add local-folder restart tests for successful and failed discovery runs.
