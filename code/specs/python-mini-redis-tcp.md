# Python Mini Redis over Rust TCP Runtime

## Status

Prototype specification for proving that a language runtime can consume the
Rust TCP runtime without owning sockets directly.

## Goal

Mini Redis should be able to run with Rust owning the transport plane and
Python owning application command semantics. The prototype now uses the shared
`generic-job-protocol` envelope so TCP is one consumer of a reusable
cross-language job contract, not the owner of a Python-specific protocol.

## Architecture

```text
client socket
    |
Rust tcp-runtime
    |
RESP request framing
    |
generic JobRequest<RedisCommandPayload> JSON line
    |
Python Mini Redis worker
    |
generic JobResponse<RedisResponsePayload> JSON line
    |
Rust tcp-runtime writes response
```

Rust is responsible for:

- Accepting TCP connections through `tcp-runtime`.
- Using the native platform transport selected by the workspace.
- Buffering partial RESP requests per connection.
- Converting complete RESP arrays into `JobRequest<RedisCommandPayload>`.
- Writing RESP bytes back to the correct socket.

Python is responsible for:

- Executing Mini Redis commands.
- Owning key/value and hash state.
- Preserving connection-local state such as `SELECT`.
- Returning byte-exact RESP replies.

## Worker Protocol

The worker protocol is a generic job-protocol frame. Redis-specific fields live
inside the payload, while connection routing uses generic metadata.

Each request is one JSON object followed by a newline:

```json
{"version":1,"kind":"request","body":{"id":"job-1","payload":{"argv_hex":["50494e47"]},"metadata":{"affinity_key":"7","sequence":1}}}
```

Each successful response is one JSON object followed by a newline:

```json
{"version":1,"kind":"response","body":{"id":"job-1","result":{"status":"ok","payload":{"resp_hex":"2b504f4e470d0a"}},"metadata":{"affinity_key":"7","sequence":1}}}
```

Arguments and responses are hex encoded so arbitrary RESP bytes can cross
standard streams without depending on terminal encodings. The JSON-line codec is
the phase-one wire encoding from `generic-job-protocol`; the protocol crate can
add a binary codec later without making TCP or Python redefine the envelope.

## Supported Commands

- `PING`
- `SET`
- `GET`
- `EXISTS`
- `DEL`
- `INCRBY`
- `HSET`
- `HGET`
- `HEXISTS`
- `SELECT`

## Current Limitations

- The Rust TCP callback waits synchronously for the Python worker response.
- The prototype uses one Python worker process.
- Worker communication uses the JSON-line `generic-job-protocol` codec over
  standard streams, not a binary protocol.
- The worker stores data in memory and does not implement persistence.

These limits are intentional for the first prototype. The next production seam
should route parsed jobs through the generic job runtime so language workers can
run in a thread pool or process pool without blocking the reactor.

## Acceptance Criteria

- Python worker unit tests cover command correctness, error responses, and the
  generic job-protocol JSON-line shape.
- Rust `generic-job-protocol` tests cover the reusable request/response envelope
  independently from TCP.
- Rust tests launch the Python worker process.
- Rust tests launch a real TCP listener, send RESP commands, and receive
  Python-generated Redis replies.
- Documentation states that this is a prototype and not the final async worker
  architecture.
