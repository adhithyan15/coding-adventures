# Embeddable TCP Server

## Status

Prototype specification for the reusable Rust TCP server layer that can embed
application workers written in Python, Ruby, Lua, Objective-C, Perl, or any
other language with a bridge/FFI/process boundary.

## Goal

`embeddable-tcp-server` should keep transport ownership in Rust while allowing
an application worker in another language to own business logic. The crate must
not name one consumer, such as Python Mini Redis, in its public API. TCP is only
one consumer of the reusable `generic-job-protocol` contract.

## Architecture

```text
client socket
    |
embeddable Rust TCP server
    |
generic JobRequest<TcpBytesPayload> JSON line
    |
embedded language/application worker
    |
generic JobResponse<TcpWriteFrame> JSON line
    |
Rust TCP server writes response bytes
```

Rust is responsible for:

- Accepting TCP connections through `tcp-runtime`.
- Using the native platform transport selected by the workspace.
- Owning connection lifecycle, routing metadata, and socket writes.
- Sending opaque TCP byte jobs to the configured worker command.
- Writing opaque byte frames returned by the worker.
- Validating response id before routing worker results back to sockets.

The embedded worker is responsible for:

- Queueing inbound TCP byte jobs for application processing.
- Owning application protocol framing and frame assembly.
- Executing application semantics.
- Owning application and protocol session state keyed by an opaque stream id.
- Returning typed `JobResponse<U>` frames.
- Avoiding socket ownership and native event-loop concerns.

## Worker Protocol

The worker protocol is a generic job-protocol frame. The TCP payload is
deliberately generic: an opaque stream id and hex-encoded bytes. Rust
correlates responses by job id; the application worker uses `stream_id` for
protocol buffers and session state.

Each request is one JSON object followed by a newline:

```json
{"version":1,"kind":"request","body":{"id":"job-1","payload":{"stream_id":"7","bytes_hex":"2a310d0a24340d0a50494e470d0a"},"metadata":{}}}
```

Each successful response is one JSON object followed by a newline:

```json
{"version":1,"kind":"response","body":{"id":"job-1","result":{"status":"ok","payload":{"writes_hex":["2b504f4e470d0a"],"close":false}},"metadata":{}}}
```

The example payloads above are from the Python Mini Redis integration test.
They are not part of the `embeddable-tcp-server` API. Other consumers can
define payloads for HTTP, IRC, WebSocket frames, custom binary protocols, UI
event dispatch, or CPU-bound jobs.

## Python Mini Redis Example

The current integration test uses Python Mini Redis as the first consumer:

- RESP bytes enter over a real TCP socket.
- `embeddable-tcp-server` sends raw TCP bytes to the Python worker as
  `JobRequest<TcpBytesPayload>`.
- Python queues the job, buffers bytes by stream id, parses RESP frames,
  executes commands, assembles RESP responses, and returns write frames inside
  `JobResponse<TcpWriteFrame>`.
- Rust writes the returned opaque bytes to the original socket.

This proves the cross-language seam without making Python or Redis part of the
transport package identity.

## Current Limitations

- The TCP callback waits synchronously for the worker response.
- The prototype uses one worker process.
- Worker communication uses the JSON-line `generic-job-protocol` codec over
  standard streams, not a binary protocol.
- The worker can queue jobs internally, but this prototype still waits for one
  response frame before the TCP callback returns.

These limits are intentional for the first prototype. The next production seam
should route byte jobs through the generic job runtime so language workers can
run in a thread pool or process pool without blocking the reactor.

## Acceptance Criteria

- Rust `generic-job-protocol` tests cover the reusable request/response envelope
  independently from TCP.
- `embeddable-tcp-server` exposes generic `WorkerCommand`, `StdioJobWorker`, and
  `EmbeddableTcpServer` types.
- No public Rust package or type is named after Python Mini Redis.
- Python worker unit tests cover RESP framing, command correctness, error
  responses, per-stream session state, job queueing, and the generic
  job-protocol JSON-line shape.
- Rust tests launch the Python worker process as one consumer example.
- Rust tests launch a real TCP listener, send RESP commands, and receive Redis
  replies generated entirely by the Python worker.
- Documentation states that this is a prototype and not the final async worker
  architecture.
