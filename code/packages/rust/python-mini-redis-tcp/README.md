# python-mini-redis-tcp

Rust TCP runtime bridge that delegates Mini Redis commands to a Python worker
process.

This crate is a prototype consumer of the transport stack we have been building:
Rust owns sockets, native event-loop integration, per-connection RESP request
buffers, and response writes. Python owns Mini Redis command execution and
application state.

## Why This Exists

The long-term goal is a safe, high-performance TCP runtime that languages with a
C FFI or native-extension story can consume. This crate proves the first seam:
an application written in Python can sit behind the Rust TCP runtime without
opening or polling sockets itself.

## Architecture

```text
client
  -> Rust tcp-runtime listener
  -> RESP parser
  -> generic JobRequest<RedisCommandPayload>
  -> Python Mini Redis worker
  -> generic JobResponse<RedisResponsePayload>
  -> Rust socket write
```

The current wire codec is deliberately boring: one versioned JSON object per
line. The important part is that the envelope comes from
`generic-job-protocol`, so TCP is only a consumer of the reusable job contract.
Redis-specific data stays inside payload structs.

## Current Limitations

- The Python worker call is synchronous inside the TCP read callback.
- One Python worker process handles all delegated commands.
- The worker protocol uses the JSON-line `generic-job-protocol` codec over
  stdio for debuggability, not throughput.
- This validates the runtime seam; it is not the final high-performance
  process-pool architecture.

The next production step is to route these protocol frames through a real job
runtime so worker completion can happen asynchronously and language workers can
be backed by threads, processes, or another host-specific scheduler.

## Dependencies

- generic-job-protocol
- resp-protocol
- tcp-runtime
- transport-platform

## Development

```bash
# Run tests
bash BUILD
```

The integration tests launch a real Rust TCP listener, start the Python worker
process, send RESP commands through a socket, and assert that the replies come
back from Python-owned Mini Redis state.
