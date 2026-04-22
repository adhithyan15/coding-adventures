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
  -> TCP return mailbox
  -> Rust socket write
```

The current wire codec is deliberately boring: one versioned JSON object per
line. The important part is that the envelope comes from
`generic-job-protocol`, so TCP is only a consumer of the reusable job contract.
Application-specific data stays inside payload structs.

The package tests use a Python Mini Redis worker as one example application:
RESP bytes enter over TCP, Rust forwards opaque byte jobs, and the TCP callback
immediately returns to the reactor. Python queues and parses RESP, Python
assembles RESP replies, a Rust response task posts those replies to the TCP
return mailbox, and the reactor writes the returned opaque bytes. That proves
the embeddable server with a real language worker without making Python, Redis,
or RESP part of the crate's public identity.

## Current Limitations

- Mailbox mode can use a configurable stdio worker process pool through
  `generic-job-runtime`; the default remains one worker for deterministic local
  behavior.
- `worker_job_timeout` can bound mailbox-mode jobs so stuck workers return
  timeout responses instead of leaking capacity forever.
- The worker protocol uses the JSON-line `generic-job-protocol` codec over
  stdio for debuggability, not throughput.
- Worker responses are asynchronous with respect to TCP reads, but still flow
  through one response reader per worker process.
- The current payload structs are the first raw-byte transport shape; a later
  crate should make those stable for all language bridges.
- This validates the runtime seam; it still needs production cancellation,
  worker restart, and backpressure-to-read-pausing policies.

The next production step is to harden the process-pool path with worker restart
policy, cancellation, response size enforcement, and TCP read pausing when the
worker queue is saturated.

## Dependencies

- generic-job-protocol
- generic-job-runtime
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
