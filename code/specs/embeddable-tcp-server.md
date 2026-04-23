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
generic JobResponse<TcpWriteFrame> JSON line, read by worker response task
    |
TCP return mailbox writes response bytes
```

Rust is responsible for:

- Accepting TCP connections through `tcp-runtime`.
- Using the native platform transport selected by the workspace.
- Owning connection lifecycle, routing metadata, and socket writes.
- Sending opaque TCP byte jobs to the configured worker command.
- Returning to the TCP event loop immediately after sending a job.
- Providing a per-connection return mailbox for worker responses.
- Routing opaque byte frames from the worker response task into that mailbox.

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
- The TCP callback returns immediately after putting that job in the worker's
  inbox.
- Python queues the job, buffers bytes by stream id, parses RESP frames,
  executes commands, assembles RESP responses, and returns write frames inside
  `JobResponse<TcpWriteFrame>`.
- A Rust response task reads those frames and posts them to the TCP mailbox for
  the original stream.

This proves the cross-language seam without making Python or Redis part of the
transport package identity.

## Current Limitations

- Mailbox mode can use a configurable stdio worker process pool through
  `generic-job-runtime`; the default remains one process.
- Mailbox mode can bound worker queue depth and treats `queue_full` as
  backpressure: the TCP layer defers the already-read bytes, pauses reads for
  that stream, and resumes paused streams when worker completions release
  capacity.
- Mailbox mode can pass a default worker job timeout into the generic runtime
  so stuck jobs produce timed-out responses instead of leaking capacity.
- Mailbox mode can opt into generic job-runtime worker restart policy so a
  crashed process can be replaced without changing the worker slot index.
- Worker communication uses the JSON-line `generic-job-protocol` codec over
  standard streams, not a binary protocol.
- Mailbox delivery is bounded by the stream reactor's cooperative poll timeout
  until the transport layer exposes a thread-safe wake handle.
- Process-pool hardening is still incomplete: cancellation, ordered response
  buffering, startup timeouts, and richer backpressure telemetry need dedicated
  follow-up work.

These limits are intentional for the first process-pool slice. The TCP package
now consumes the generic job runtime, but the runtime still needs cancellation,
ordering policy, startup timeout policy, and production metrics before it should
be treated as the final high-performance architecture.

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
- Rust tests prove the TCP callback can accept later reads while an earlier
  worker response is still pending.
- Rust tests prove worker queue saturation pauses and replays TCP reads instead
  of closing connections or dropping bytes.
- Documentation states that this is a prototype and not the final async worker
  architecture.
