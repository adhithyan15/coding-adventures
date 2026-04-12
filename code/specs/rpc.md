# RPC — Generic Remote Procedure Call Primitive

## Overview

JSON-RPC 2.0 is one concrete instantiation of a more general idea: a pattern
where one process calls named procedures on another process, passing parameters
and receiving results (or errors). The **serialization format** (JSON) and the
**framing scheme** (Content-Length headers) are separable concerns. The RPC
semantics — requests, responses, notifications, error codes, method dispatch,
id correlation — are the same regardless of how the bytes look on the wire.

This spec defines a codec-agnostic `rpc` package that captures those semantics.
The `json-rpc` package (already implemented) is a thin instantiation of `rpc`
with a JSON codec and a Content-Length framer. Future packages — `protobuf-rpc`,
`msgpack-rpc`, `xml-rpc` — will be different instantiations of the same `rpc`
layer.

```
┌─────────────────────────────────────────────────────────────┐
│  Application  (LSP server, custom tool, test client, …)     │
├─────────────────────────────────────────────────────────────┤
│  rpc                                                        │
│  RpcServer / RpcClient                                      │
│  (method dispatch, id correlation, error handling,          │
│   handler registry, panic recovery)                         │
├─────────────────────────────────────────────────────────────┤
│  RpcCodec                                                   │  ← JSON, Protobuf,
│  (RpcMessage ↔ bytes)                                       │     MessagePack, XML,
│                                                             │     custom binary
├─────────────────────────────────────────────────────────────┤
│  RpcFramer                                                  │  ← Content-Length,
│  (byte stream ↔ discrete byte chunks)                       │     WebSocket frames,
│                                                             │     length-prefix,
│                                                             │     newline-delimited
├─────────────────────────────────────────────────────────────┤
│  Transport                                                  │  ← stdin/stdout, TCP,
│  (raw byte stream)                                          │     Unix socket, pipe
└─────────────────────────────────────────────────────────────┘
```

Each layer knows only about the layer immediately below it. The `rpc` layer
never touches byte serialization or framing. The codec never knows about method
dispatch. The framer never knows about procedure names.

---

## Why Abstract Over JSON?

Think of RPC like a phone call. The information being exchanged (who's calling,
what they want, the answer they get back) is the RPC layer. The *language* they
speak — English, French, Spanish — is the codec. The *phone network* — landline,
mobile, VoIP — is the transport + framing.

You can change the language or the network without changing the information that
is exchanged. A call center can handle English and Spanish callers with the same
policies, the same escalation paths, the same error handling ("I'm sorry, I
can't help with that" is the same sentiment in any language).

The same is true here:

- A Rust LSP server speaks JSON over stdio today.
- Tomorrow it can speak MessagePack over a Unix socket — no changes to the
  server's handler logic, only the codec and framer are swapped.
- A Python test client that sends JSON requests can be retargeted to send
  Protobuf requests by swapping one dependency.

### Current Situation vs. Target

```
CURRENT (json-rpc packages)          TARGET
──────────────────────────           ──────────────────────────
Server                               RpcServer
  reader: MessageReader                codec:   RpcCodec      ← pluggable
  writer: MessageWriter                framer:  RpcFramer     ← pluggable
  handlers: Map<method, fn>            handlers: Map<method, fn>

  serve() {                            serve() {
    loop {                               loop {
      frame = reader.read_frame()          bytes = framer.read_frame()
      msg   = parse_json(frame)            msg = codec.decode(bytes)
      dispatch(msg)                        dispatch(msg)
      resp  = build_response(result)       resp = build_response(result)
      json  = serialize_json(resp)         bytes = codec.encode(resp)
      writer.write_frame(json)             framer.write_frame(bytes)
    }                                    }
  }                                    }
```

The only change in `serve()` is that the JSON-specific calls become
codec/framer interface calls. The dispatch logic, error handling, panic
recovery, and handler API are unchanged.

---

## Message Types

RPC messages are codec-agnostic. They carry a `Value` type parameter that
represents the codec's native dynamic value type — `serde_json::Value` for
JSON, `rmpv::Value` for MessagePack, a proto `Message` for Protobuf, etc.

In pseudocode (language-agnostic notation):

```
type RpcId = String | Integer

type RpcMessage<V> =
  | RpcRequest      { id: RpcId, method: String, params: Option<V> }
  | RpcResponse     { id: RpcId, result: Option<V> }
  | RpcErrorResponse{ id: RpcId, code: Integer, message: String, data: Option<V> }
  | RpcNotification { method: String, params: Option<V> }
```

Notes:
- `RpcId` is always a string or integer. `null` id is only used for
  `RpcErrorResponse` when the original request was so malformed that its id
  could not be extracted.
- `RpcRequest` and `RpcResponse` / `RpcErrorResponse` are correlated by id.
- `RpcNotification` has no id and generates no response.
- The codec is responsible for translating between `RpcMessage<V>` and bytes;
  the RPC layer never inspects `V`.

In statically-typed languages (Go, Rust, TypeScript) the `V` parameter is
explicit. In dynamically-typed languages (Python, Ruby, Elixir, Lua, Perl) it
is implicit — any value can be params.

---

## Error Codes

Error codes are codec-agnostic integers. The same table applies regardless of
whether the codec is JSON, MessagePack, or Protobuf.

| Code                    | Name              | When to use                                        |
|-------------------------|-------------------|----------------------------------------------------|
| `-32700`                | Parse error       | The framed bytes could not be decoded by the codec |
| `-32600`                | Invalid request   | Decoded successfully but not a valid RPC message   |
| `-32601`                | Method not found  | No handler registered for the method               |
| `-32602`                | Invalid params    | Handler rejected the params as malformed           |
| `-32603`                | Internal error    | Unexpected error inside the handler                |
| `-32000` to `-32099`    | Server errors     | Implementation-defined server errors               |

LSP reserves `-32899` to `-32800` for LSP-specific codes. The `rpc` layer must
not use that range — it belongs to the application layer above.

---

## Interfaces

### RpcCodec

The codec translates between `RpcMessage<V>` and a raw byte slice. It is
stateless — a single codec instance can be used concurrently.

```
interface RpcCodec<V> {
  // Encode an RpcMessage to bytes ready for the framer.
  encode(msg: RpcMessage<V>) → bytes

  // Decode a byte slice produced by the framer into an RpcMessage.
  // Returns Err with an RpcErrorResponse on failure (parse or schema error).
  decode(bytes) → Result<RpcMessage<V>, RpcErrorResponse<V>>
}
```

The codec does not touch framing. It receives exactly the payload bytes (no
Content-Length header, no WebSocket envelope) and returns exactly the payload
bytes.

Example instantiations:
- `JsonCodec` — uses the `json` package; `V = JsonValue`
- `MsgpackCodec` — uses a MessagePack library; `V = MsgpackValue`
- `ProtobufCodec` — uses proto reflection; `V = proto.Message`

### RpcFramer

The framer reads and writes discrete byte chunks from a raw byte stream. It
knows nothing about the content of those chunks — it only concerns itself with
boundaries.

```
interface RpcFramer {
  // Read the next frame (payload bytes) from the stream.
  // Returns None on clean EOF.
  // Returns Err on framing error (e.g., malformed Content-Length header).
  read_frame() → Option<Result<bytes, error>>

  // Write a frame (payload bytes) to the stream, applying whatever
  // envelope the framing scheme requires.
  write_frame(bytes) → Result<(), error>
}
```

Example instantiations:
- `ContentLengthFramer` — prepends `Content-Length: N\r\n\r\n`; used by LSP
- `LengthPrefixFramer` — prepends a 4-byte big-endian length; compact TCP variant
- `NewlineFramer` — appends `\n`; used by NDJSON streaming
- `WebSocketFramer` — wraps in WebSocket data frames
- `PassthroughFramer` — no framing; each `write_frame` is one complete stream
  (useful when HTTP handles framing externally)

### RpcTransport

The transport is the raw byte stream. In most languages this is already
captured by the language's standard I/O traits or interfaces (`io.ReadWriter`
in Go, `Read + Write` in Rust, `IO` in Ruby/Python, etc.). No new interface
is needed — `RpcFramer` is parameterised by whatever the language's standard
stream type is.

The three-layer composition is therefore:

```
RpcServer(
  codec:   RpcCodec,    // how to interpret bytes as messages
  framer:  RpcFramer,   // how to split the stream into byte chunks
  // framer already holds a reference to the transport (stream)
)
```

---

## RpcServer

The server owns a codec, a framer, and two dispatch tables (one for requests,
one for notifications). Its `serve()` method drives the read-dispatch-write
loop until EOF or an unrecoverable error.

```
RpcServer<V>
  .new(codec: RpcCodec<V>, framer: RpcFramer)  → server

  // Register a handler for a named request method.
  // Calling on_request with the same method twice replaces the earlier handler.
  // Handler signature: fn(id: RpcId, params: Option<V>) → Result<V, RpcErrorResponse<V>>
  .on_request(method: String, handler: fn) → server   (chainable)

  // Register a handler for a named notification method.
  // Handler signature: fn(params: Option<V>) → void
  // Unknown notifications are silently dropped per the spec.
  .on_notification(method: String, handler: fn) → server   (chainable)

  // Start the blocking read-dispatch-write loop.
  // Returns on clean EOF. Panics or returns error on unrecoverable I/O failure.
  .serve() → void
```

### serve() behaviour

```
loop:
  bytes = framer.read_frame()
  if bytes == None: break   // clean EOF

  msg = codec.decode(bytes)
  if msg == Err(e):
    // Framing or decode error. Send error response with null id.
    error_response = RpcErrorResponse { id: null, ... }
    framer.write_frame(codec.encode(error_response))
    continue

  match msg:
    RpcRequest(req):
      handler = dispatch_table[req.method]
      if handler == None:
        resp = RpcErrorResponse { id: req.id, code: -32601, ... }
      else:
        result = catch_panic(|| handler(req.id, req.params))
        resp = result_to_response(req.id, result)
      framer.write_frame(codec.encode(resp))

    RpcNotification(notif):
      handler = dispatch_table[notif.method]
      if handler != None:
        catch_panic(|| handler(notif.params))
      // Unknown notifications silently dropped. Never write a response.

    RpcResponse | RpcErrorResponse:
      // Servers that only respond ignore incoming responses.
      // (Bidirectional peers route these to the pending-request table.)
      pass
```

### Panic safety

Handler panics must not kill the server process. The `serve()` loop wraps each
handler call in the language's panic/exception recovery mechanism:

- Go: `defer recover()`
- Rust: `std::panic::catch_unwind`
- Python: `try/except BaseException`
- Ruby: `rescue Exception`
- Elixir: `try/rescue` or spawn isolated process
- TypeScript: `try/catch`
- Lua: `pcall`
- Perl: `eval { ... }`

A recovered panic returns a `-32603 Internal error` response with
`data: "handler panicked"` (or the panic message if recoverable).

---

## RpcClient

The client sends requests to a remote server and receives responses. It also
sends fire-and-forget notifications.

```
RpcClient<V>
  .new(codec: RpcCodec<V>, framer: RpcFramer)  → client

  // Send a request and wait (blocking) for the matching response.
  // Generates and manages the request id internally.
  // Returns Ok(result_value) on success, Err(RpcErrorResponse) on error.
  .request(method: String, params: Option<V>) → Result<V, RpcErrorResponse<V>>

  // Send a notification. No response is expected or waited for.
  .notify(method: String, params: Option<V>) → void

  // Optional: receive any server-initiated notifications (server push).
  // The client calls this to register handlers for methods the server sends
  // unprompted (e.g., "textDocument/publishDiagnostics" in LSP).
  .on_notification(method: String, handler: fn) → client   (chainable)
```

### Id management

The client maintains a monotonically increasing integer counter. Each call to
`request()` increments the counter and uses the new value as the request id.
The counter starts at 1.

```
state:
  next_id:  Integer = 1
  pending:  Map<RpcId, WaitHandle<Result<V, RpcErrorResponse<V>>>>
```

### Blocking request flow

```
request(method, params):
  id = next_id++
  msg = RpcRequest { id, method, params }
  framer.write_frame(codec.encode(msg))

  // Wait for the matching response.
  loop:
    bytes = framer.read_frame()
    if bytes == None: return Err("connection closed")

    msg = codec.decode(bytes)
    match msg:
      RpcResponse(resp) if resp.id == id:
        return Ok(resp.result)
      RpcErrorResponse(resp) if resp.id == id:
        return Err(resp)
      RpcNotification(notif):
        handler = notification_handlers[notif.method]
        if handler != None: handler(notif.params)
        // Continue waiting — this was a server-push, not our response.
      _:
        // Response for a different id, or unexpected message — ignore.
        continue
```

This synchronous model is appropriate for our use case: the LSP client (editor
plugin) sends one request at a time and waits. For concurrent use, a
thread-safe pending-request map and a dedicated reader goroutine/thread can be
added as an extension without changing the handler API.

### Notification-only client

When only notifications are needed (e.g., a log sink, a metrics emitter),
construct with a `NullFramer` that discards incoming bytes. `request()` is not
called; only `notify()` is used.

---

## Concrete Instantiations

### json-rpc (current)

```
json-rpc = rpc + JsonCodec + ContentLengthFramer
```

The existing `json-rpc` package is refactored to delegate to `rpc` internally.
The public API (MessageReader, MessageWriter, Server) remains identical — they
become thin wrappers that construct the appropriate codec and framer.

```
// existing public API, unchanged:
Server.new(stdin, stdout)
  // internally: RpcServer.new(JsonCodec, ContentLengthFramer(stdin, stdout))
```

### Future: msgpack-rpc

```
msgpack-rpc = rpc + MsgpackCodec + LengthPrefixFramer
```

MessagePack is a compact binary serialization format. Combined with a 4-byte
length prefix, this gives a low-overhead RPC suitable for inter-process
communication on the same machine where wire bandwidth is cheap but CPU cycles
for JSON parsing matter.

### Future: protobuf-rpc

```
protobuf-rpc = rpc + ProtobufCodec + LengthPrefixFramer
```

Protobuf-encoded RPC messages with a type-URL discriminator. Used where
strongly-typed schemas and backwards-compatible evolution matter more than
flexibility (e.g., gRPC-like internal APIs).

### Future: json-ws-rpc (JSON over WebSocket)

```
json-ws-rpc = rpc + JsonCodec + WebSocketFramer
```

JSON-RPC 2.0 over WebSocket. The WebSocket protocol handles framing; the codec
stays the same as `json-rpc`. Used for browser-based clients where TCP sockets
are not available.

---

## Package Structure

Package name: `rpc` in all languages.

```
go/rpc/
  codec.go          — RpcCodec interface
  framer.go         — RpcFramer interface
  message.go        — RpcMessage, RpcId, RpcErrorResponse types
  server.go         — RpcServer
  client.go         — RpcClient
  errors.go         — Standard error code constants
  *_test.go

typescript/rpc/src/
  codec.ts
  framer.ts
  message.ts
  server.ts
  client.ts
  errors.ts
  index.ts

python/rpc/src/rpc/
  codec.py
  framer.py
  message.py
  server.py
  client.py
  errors.py
  __init__.py

ruby/rpc/lib/coding_adventures/rpc/
  codec.rb
  framer.rb
  message.rb
  server.rb
  client.rb
  errors.rb
  version.rb

elixir/rpc/lib/rpc/
  codec.ex
  framer.ex
  message.ex
  server.ex
  client.ex
  errors.ex

rust/rpc/src/
  codec.rs
  framer.rs
  message.rs
  server.rs
  client.rs
  errors.rs
  lib.rs

lua/rpc/src/coding_adventures/rpc/
  codec.lua
  framer.lua
  message.lua
  server.lua
  client.lua
  errors.lua

perl/rpc/lib/CodingAdventures/Rpc/
  Codec.pm
  Framer.pm
  Message.pm
  Server.pm
  Client.pm
  Errors.pm
```

---

## Refactoring json-rpc

Once `rpc` is implemented, the `json-rpc` packages are refactored as follows:

1. **Add dependency** on `rpc` in each language's `BUILD` / `Cargo.toml` /
   `pyproject.toml` etc.

2. **`JsonCodec`** — new struct that implements `RpcCodec<JsonValue>`:
   - `encode`: calls `json.Marshal` / `JSON.stringify` / `json.dumps` etc.
   - `decode`: calls `json.Unmarshal` / `JSON.parse` / `json.loads` etc. then
     discriminates on the key presence rules from the existing `parse_message`
     implementations.

3. **`ContentLengthFramer`** — new struct that implements `RpcFramer`:
   - `read_frame`: the existing `MessageReader.read_raw()` logic.
   - `write_frame`: the existing `MessageWriter.write_message()` logic.

4. **`Server` and `MessageReader` / `MessageWriter`** — thin wrappers:
   ```
   Server.new(reader, writer) {
     return RpcServer.new(
       JsonCodec.new(),
       ContentLengthFramer.new(reader, writer),
     )
   }
   ```

5. The **public API is unchanged**. `on_request`, `on_notification`, `serve`,
   `MessageReader.read_message`, `MessageWriter.write_message` all remain.
   Existing code that imports `json-rpc` needs no changes.

The refactoring is additive — no existing tests need to change.

---

## Relationship to Other Specs

| Spec         | Depends on  | Description                                |
|--------------|-------------|---------------------------------------------|
| `rpc.md`     | —           | Abstract RPC primitive (this spec)          |
| `json-rpc.md`| `rpc.md`    | JSON codec + Content-Length framer + rpc    |
| `LS00`       | `json-rpc`  | Generic LSP server framework                |
| `LS01`       | `LS00`      | Language-specific LSP bridge (lexer+parser) |

The `rpc` package sits below `json-rpc` in the dependency graph. It has no
dependencies on any other coding-adventures package — only the language's
standard library is needed.

---

## Test Coverage Targets

### RpcServer (tested via a mock codec + framer)

- Dispatches a request to its handler and writes the result response.
- Returns `-32601` Method not found for unregistered method.
- Returns the handler's `RpcErrorResponse` when the handler returns an error.
- Dispatches a notification to its handler without writing a response.
- Silently drops an unknown notification (no response, no error).
- Recovers from a panicking handler and sends `-32603 Internal error`.
- Sends error response with null id when the codec fails to decode a frame.

### RpcClient (tested via a mock codec + framer)

- `request()` encodes and sends the message, then returns the decoded result.
- `request()` returns an error when the server responds with `RpcErrorResponse`.
- `request()` returns an error when the connection is closed before response.
- `notify()` encodes and sends the message without waiting for a response.
- `on_notification()` handler is called when the server sends a push notification
  while the client is blocked in `request()`.
- Request ids are auto-generated and monotonically increasing.

### RpcCodec (tested independently for each concrete codec)

- `encode` followed by `decode` round-trips all four message types.
- `decode` returns parse error for malformed bytes.
- `decode` returns invalid request for well-formed bytes that are not an RPC message.

### RpcFramer (tested independently for each concrete framer)

- `write_frame` followed by `read_frame` round-trips a payload.
- `read_frame` returns `None` on clean EOF.
- `read_frame` returns an error for a malformed frame envelope.
- Multiple back-to-back frames are read correctly without cross-contamination.
