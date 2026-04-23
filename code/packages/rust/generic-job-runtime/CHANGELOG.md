# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-04-22

### Added

- Added pending-job tracking for stdio process-pool workers.
- Added per-job deadline/default-timeout handling that emits timed-out
  `JobResponse` values and releases in-flight capacity.
- Added worker-exit detection that reports executor-origin
  `worker_unavailable` responses for abandoned jobs.
- Added tests proving timeouts and worker exits do not leak queue capacity.
- Added `StdioWorkerRestartPolicy` with `Never`, `Always`, and bounded restart
  modes for reviving dead stdio worker slots.
- Added tests proving restarted workers can accept new jobs and bounded restart
  policies stop crash loops.

## [0.1.0] - 2026-04-21

### Added

- Added executor capability and limit types for generic job adapters.
- Added a bounded stdio process-pool executor for JSON-line
  `generic-job-protocol` workers.
- Added affinity-based worker routing so related jobs stay on the same process.
- Added tests for affinity routing and queue-full backpressure.
