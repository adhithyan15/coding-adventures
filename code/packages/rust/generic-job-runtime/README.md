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

The crate does not know about TCP, RESP, Redis, IRC, or sockets. Those adapters
submit typed job payloads and decide how to apply responses.

## Development

```bash
bash BUILD
```
