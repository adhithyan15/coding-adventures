# Python Mini Redis over Rust TCP Runtime

## Status

Prototype specification for proving that a language runtime can consume the
Rust TCP runtime without owning sockets directly.

## Goal

Mini Redis should be able to run with Rust owning the transport plane and
Python owning application command semantics. The prototype intentionally keeps
the seam simple and inspectable before optimizing the worker protocol.

## Architecture

```text
client socket
    |
Rust tcp-runtime
    |
RESP request framing
    |
JSON-line worker request
    |
Python Mini Redis worker
    |
JSON-line worker response containing RESP bytes
    |
Rust tcp-runtime writes response
```

Rust is responsible for:

- Accepting TCP connections through `tcp-runtime`.
- Using the native platform transport selected by the workspace.
- Buffering partial RESP requests per connection.
- Converting complete RESP arrays into worker jobs.
- Writing RESP bytes back to the correct socket.

Python is responsible for:

- Executing Mini Redis commands.
- Owning key/value and hash state.
- Preserving connection-local state such as `SELECT`.
- Returning byte-exact RESP replies.

## Worker Protocol

Each request is one JSON object followed by a newline:

```json
{"id":"job-1","connection_id":"7","sequence":1,"argv_hex":["50494e47"]}
```

Each successful response is one JSON object followed by a newline:

```json
{"id":"job-1","connection_id":"7","sequence":1,"ok":true,"resp_hex":"2b504f4e470d0a"}
```

Arguments and responses are hex encoded so arbitrary RESP bytes can cross
standard streams without depending on terminal encodings.

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
- Worker communication uses JSON over standard streams, not a binary protocol.
- The worker stores data in memory and does not implement persistence.

These limits are intentional for the first prototype. The next production seam
should route parsed jobs through the generic job runtime so language workers can
run in a thread pool or process pool without blocking the reactor.

## Acceptance Criteria

- Python worker unit tests cover command correctness, error responses, and the
  JSON-line protocol.
- Rust tests launch the Python worker process.
- Rust tests launch a real TCP listener, send RESP commands, and receive
  Python-generated Redis replies.
- Documentation states that this is a prototype and not the final async worker
  architecture.
