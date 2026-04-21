# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-21

### Added

- Added executor capability and limit types for generic job adapters.
- Added a bounded stdio process-pool executor for JSON-line
  `generic-job-protocol` workers.
- Added affinity-based worker routing so related jobs stay on the same process.
- Added tests for affinity routing and queue-full backpressure.
