# Changelog

## Unreleased

- Add persisted `udp_multicast` discovery-source parsing.

## 0.1.0

- Add an actor-owned scheduled mDNS discovery lifecycle.
- Persist and restore worker cadence, retry pressure, run reports, and service
  health through `StorageBackend`.
- Add local-folder restart tests for successful and failed discovery runs.
