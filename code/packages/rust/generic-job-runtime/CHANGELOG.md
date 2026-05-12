# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Added `JobExecutor::drain_response_summaries()` for consuming completed
  responses as compact protocol summaries for D18C supervisor/read-side tools.
- Added `JobResponseDrainSummary` and
  `JobExecutor::drain_response_summary_batch()` for aggregated D18C supervisor
  response-drain read models.
- Added `JobResponseDrainOutcome` and helper predicates for classifying drained
  response-summary batches without reinterpreting raw terminal counters.
- Added queue-pressure percentages and recommended supervision actions for
  executor snapshots so D18C supervisors can choose backpressure, worker
  restart, or graceful-drain behavior without reinterpreting raw counters.
- Added queue-pressure bands and percent-threshold helpers for stable D18C
  backpressure read models.
- Added executor admission-status helpers so D18C supervisors can explain why
  a runtime is or is not accepting jobs.

## [0.1.1] - 2026-04-22

### Added

- Added `RustThreadPool`, a generic in-process job executor that only depends
  on `JobRequest<T>` / `JobResponse<U>` and has no TCP/application awareness.
- Added bounded queueing, cancellation, timeout accounting, and panic
  containment for Rust thread-pool jobs.
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
- Added non-consuming executor snapshots for supervisor/read-side tools to
  inspect worker liveness, queue saturation, and in-flight job counts.
- Added executor snapshot health classification for supervisor/read-side tools
  to identify idle, busy, saturated, draining, and offline executors.

## [0.1.0] - 2026-04-21

### Added

- Added executor capability and limit types for generic job adapters.
- Added a bounded stdio process-pool executor for JSON-line
  `generic-job-protocol` workers.
- Added affinity-based worker routing so related jobs stay on the same process.
- Added tests for affinity routing and queue-full backpressure.
