# generic-job-runtime

Bounded job executors for `generic-job-protocol`.

This crate is the runtime layer above the portable `JobRequest<T>` /
`JobResponse<U>` envelope. The first executor is a stdio process pool for
language workers such as Python, Ruby, Perl, Lua, or other bridge targets.

## Current Scope

- Bounded in-flight job submission with `queue_full` backpressure.
- Stable affinity routing so related jobs, such as one TCP connection's bytes,
  stay on the same worker process.
- Async response collection from worker stdout.
- Per-job deadlines and default job timeouts that emit portable timed-out
  responses and release queue capacity.
- Worker-exit detection that converts abandoned in-flight jobs into portable
  executor errors.
- Capability and limit metadata that adapters can inspect.

The crate does not know about TCP, RESP, Redis, IRC, or sockets. Those adapters
submit typed job payloads and decide how to apply responses.

## Development

```bash
bash BUILD
```
