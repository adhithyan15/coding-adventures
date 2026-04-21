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
application framing adapter
    |
generic JobRequest<ApplicationRequestPayload> JSON line
    |
embedded language/application worker
    |
generic JobResponse<ApplicationResponsePayload> JSON line
    |
Rust TCP server writes response bytes
```

Rust is responsible for:

- Accepting TCP connections through `tcp-runtime`.
- Using the native platform transport selected by the workspace.
- Owning connection lifecycle, routing metadata, and socket writes.
- Providing per-connection state to application framing adapters.
- Keeping protocol/session state that belongs to the transport connection, such
  as Mini Redis `SELECT` database state.
- Sending typed `JobRequest<T>` frames to the configured worker command.
- Validating response id before routing worker results back to sockets.

The embedded worker is responsible for:

- Executing application semantics.
- Owning application state.
- Returning typed `JobResponse<U>` frames.
- Avoiding socket ownership, socket identifiers, and transport framing details.

## Worker Protocol

The worker protocol is a generic job-protocol frame. Application-specific fields
live inside the payload. Rust correlates responses by job id; metadata remains
available for generic scheduling hints but should not be required for socket
identity.

Each request is one JSON object followed by a newline:

```json
{"version":1,"kind":"request","body":{"id":"job-1","payload":{"selected_db":0,"command":"PING","args_hex":[]},"metadata":{}}}
```

Each successful response is one JSON object followed by a newline:

```json
{"version":1,"kind":"response","body":{"id":"job-1","result":{"status":"ok","payload":{"selected_db":0,"response":{"kind":"simple_string","value":"PONG"}}},"metadata":{}}}
```

The example payloads above are from the Python Mini Redis integration test.
They are not part of the `embeddable-tcp-server` API. Other consumers can
define payloads for HTTP, IRC, WebSocket frames, custom binary protocols, UI
event dispatch, or CPU-bound jobs.

## Python Mini Redis Example

The current integration test uses Python Mini Redis as the first consumer:

- RESP bytes enter over a real TCP socket.
- The Rust test adapter parses complete RESP arrays into Redis command payloads
  and stores selected database state per TCP connection.
- `embeddable-tcp-server` sends those payloads to the Python worker as
  `JobRequest<T>`.
- Python executes the command and returns an engine-response payload inside
  `JobResponse<U>`.
- Rust encodes the engine response as RESP and writes bytes to the original
  socket.

This proves the cross-language seam without making Python or Redis part of the
transport package identity.

## Current Limitations

- The TCP callback waits synchronously for the worker response.
- The prototype uses one worker process.
- Worker communication uses the JSON-line `generic-job-protocol` codec over
  standard streams, not a binary protocol.
- Application framing is supplied by the embedding caller; the crate does not
  yet ship reusable RESP/HTTP/WebSocket adapters.

These limits are intentional for the first prototype. The next production seam
should route parsed jobs through the generic job runtime so language workers can
run in a thread pool or process pool without blocking the reactor.

## Acceptance Criteria

- Rust `generic-job-protocol` tests cover the reusable request/response envelope
  independently from TCP.
- `embeddable-tcp-server` exposes generic `WorkerCommand`, `StdioJobWorker`, and
  `EmbeddableTcpServer` types.
- No public Rust package or type is named after Python Mini Redis.
- Python worker unit tests cover command correctness, error responses, and the
  generic job-protocol JSON-line shape.
- Rust tests launch the Python worker process as one consumer example.
- Rust tests launch a real TCP listener, send RESP commands, and receive Redis
  replies generated from Python engine responses.
- Documentation states that this is a prototype and not the final async worker
  architecture.
