# generic-job-runtime

Bounded job executors for `generic-job-protocol`.

This crate is the runtime layer above the portable `JobRequest<T>` /
`JobResponse<U>` envelope. It provides executors for both in-process Rust jobs
and stdio process-pool workers such as Python, Ruby, Perl, Lua, or other bridge
targets.

## Current Scope

- Bounded in-flight job submission with `queue_full` backpressure.
- A transport-neutral `RustThreadPool` executor for in-process Rust jobs.
- Stable affinity routing so related jobs, such as one TCP connection's bytes,
  stay on the same worker process.
- Async response collection from worker stdout.
- Thread-pool cancellation for queued jobs and logical cancellation for running
  jobs when the handler returns.
- Thread-pool panic containment that converts panics into portable job errors.
- Per-job deadlines and default job timeouts that emit portable timed-out
  responses and release queue capacity.
- Worker-exit detection that converts abandoned in-flight jobs into portable
  executor errors.
- Opt-in stdio worker restart policies that can revive a dead worker slot while
  preserving worker-index affinity.
- Capability and limit metadata that adapters can inspect.
- Compact response-summary draining for supervisor/read-side tools that only
  need terminal status, retryability, trace, and attempt facts.
- Aggregated response-summary drain batches for supervisor/read-side tools that
  need terminal counts without retaining response payloads.
- Drain outcome helpers for classifying empty, successful, failed, and
  retryable-failed response batches.
- Non-consuming executor snapshots for supervisor/read-side tools, including
  live workers, in-flight jobs, queued jobs, running jobs, and saturation.
- Queue-pressure bands and percent-threshold checks for D18C supervisors that
  need stable read-side backpressure signals.
- Snapshot health classification for D18C supervisors to distinguish idle,
  busy, saturated, draining, and offline executors.
- Snapshot supervision recommendations for backpressure, worker restart, and
  graceful draining decisions.

The crate does not know about TCP, RESP, Redis, IRC, or sockets. Those adapters
submit typed job payloads and decide how to apply responses.

## Development

```bash
bash BUILD
```
