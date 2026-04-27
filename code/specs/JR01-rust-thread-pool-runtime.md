# JR01: Rust Thread-Pool Runtime

## Status

Initial implementation landed in `generic-job-runtime`.

## Goal

Provide a reusable in-process Rust executor for generic jobs. The thread pool
must not know whether a job came from TCP, a UI event loop, a benchmark driver,
an FFI bridge, or a compiler pipeline. It only schedules `JobRequest<T>` values
and emits `JobResponse<U>` values.

## Non-Goals

- No socket ownership.
- No TCP, Redis, IRC, WebSocket, or UI protocol awareness.
- No language-VM callback policy.
- No reactor sharding.

## Contract

```text
RustThreadPool<T, U>
  try_submit(JobRequest<T>) -> Result<(), SubmitError>
  cancel(JobId) -> CancelResult
  drain_responses(max) -> Vec<JobResponse<U>>
  shutdown()
  capabilities() -> ExecutorCapabilities
```

The handler receives the full `JobRequest<T>` and returns a `JobResult<U>`.
The executor wraps that result with the original job id and metadata.

## Semantics

- Queue depth is bounded by `ExecutorLimits.max_queue_depth`.
- `queue_full` is backpressure, not a fatal error.
- Queued cancellation removes the queued job, releases capacity, and emits
  `JobResult::Cancelled`.
- Running cancellation is logical: the handler is allowed to finish, but its
  stale result is suppressed and replaced with `JobResult::Cancelled`.
- Panics are caught and converted into `JobError` with
  `origin = panic_or_exception`.
- Default job timeout and per-job deadlines use the same portable
  `JobResult::TimedOut` shape as other executors.

## Why This Stays Generic

This pool is the future execution primitive for Rust-native work. TCP can use
it later by producing byte-oriented jobs, but the pool must remain equally
usable for UI event handling, compiler passes, benchmark workloads, or any
other CPU-bound task that fits the generic job envelope.

## Future Work

- Affinity-aware lanes for per-connection or per-document ordering.
- Cooperative cancellation tokens for long-running CPU handlers.
- Metrics for queue depth, in-flight jobs, cancellations, timeouts, and panics.
- Optional work stealing once correctness and ordering semantics are stable.
