# embeddable-tcp-server

Language-neutral Rust TCP server for embedded application workers.

This crate is a prototype consumer of the transport stack we have been building.
Rust owns sockets, native event-loop integration, connection lifecycle, and
response writes. A worker process in Python, Ruby, Lua, Objective-C, or any
other host with a bridge/FFI story owns application semantics and responds
through `generic-job-protocol` frames.

## Why This Exists

The long-term goal is a safe, high-performance TCP server that languages with a
C FFI or native-extension story can consume. This crate proves the reusable
seam: an application worker can sit behind the Rust TCP runtime without opening
or polling sockets itself.

## Architecture

```text
client
  -> embeddable Rust TCP server
  -> generic JobRequest<TcpBytesPayload>
  -> embedded language/application worker
  -> generic JobResponse<TcpWriteFrame>
  -> Rust socket write
```

The current wire codec is deliberately boring: one versioned JSON object per
line. The important part is that the envelope comes from
`generic-job-protocol`, so TCP is only a consumer of the reusable job contract.
Application-specific data stays inside payload structs.

The package tests use a Python Mini Redis worker as one example application:
RESP bytes enter over TCP, Rust forwards opaque byte jobs, Python queues and
parses RESP, Python assembles RESP replies, and Rust writes the returned opaque
bytes. That proves the embeddable server with a real language worker without
making Python, Redis, or RESP part of the crate's public identity.

## Current Limitations

- The worker call is synchronous inside the TCP read callback.
- One worker process handles all delegated jobs.
- The worker protocol uses the JSON-line `generic-job-protocol` codec over
  stdio for debuggability, not throughput.
- The current payload structs are the first raw-byte transport shape; a later
  crate should make those stable for all language bridges.
- This validates the runtime seam; it is not the final high-performance
  process-pool architecture.

The next production step is to route these protocol frames through a real job
runtime so worker completion can happen asynchronously and language workers can
be backed by threads, processes, or another host-specific scheduler.

## Dependencies

- generic-job-protocol
- tcp-runtime
- transport-platform

`resp-protocol` is used only by Rust tests to generate and decode client-side
assertions. The embeddable TCP server does not parse or assemble RESP.

## Development

```bash
# Run tests
bash BUILD
```

The integration tests launch a real Rust TCP listener, start the Python Mini
Redis worker process, send RESP commands through a socket, and assert that the
Python-produced RESP replies come back through the generic embeddable server
seam.
