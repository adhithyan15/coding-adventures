# coding_adventures_rpc

Codec-agnostic RPC primitive. The abstract layer that `json-rpc` and
future codec-specific packages (`msgpack-rpc`, `protobuf-rpc`, …) build on.

---

## Where it fits

```
┌─────────────────────────────────────────────────────────────────┐
│  Application  (LSP server, tool, test client, …)                │
├─────────────────────────────────────────────────────────────────┤
│  coding_adventures_rpc                                          │
│  Server / Client                                                │
│  (method dispatch, id correlation, error handling,              │
│   handler registry, panic recovery)                             │
├─────────────────────────────────────────────────────────────────┤
│  RpcCodec  (pluggable)                                          │  ← JSON, MessagePack, …
│  RpcMessage ↔ bytes                                             │
├─────────────────────────────────────────────────────────────────┤
│  RpcFramer  (pluggable)                                         │  ← Content-Length, newline, …
│  byte stream ↔ discrete byte chunks                             │
├─────────────────────────────────────────────────────────────────┤
│  Transport (IO stream — STDIN/STDOUT, TCPSocket, StringIO, …)   │
└─────────────────────────────────────────────────────────────────┘
```

`json-rpc` is the concrete instantiation:

```
json-rpc = coding_adventures_rpc + JsonCodec + ContentLengthFramer
```

---

## Installation

```ruby
# Gemfile
gem "coding_adventures_rpc"
```

No runtime dependencies — stdlib only (`stringio` is used in tests).

---

## Quick start (server)

```ruby
require "coding_adventures_rpc"
require "my_json_codec"          # your JsonCodec implementation
require "my_content_length_framer" # your ContentLengthFramer

codec  = MyJsonCodec.new
framer = MyContentLengthFramer.new(STDIN, STDOUT)

CodingAdventures::Rpc::Server.new(codec, framer)
  .on_request("initialize") { |_id, _params| { capabilities: {} } }
  .on_request("ping")       { |_id, _params| "pong" }
  .on_notification("textDocument/didOpen") { |params| process(params) }
  .serve
```

---

## Quick start (client)

```ruby
require "coding_adventures_rpc"
require "my_json_codec"
require "my_content_length_framer"

codec  = MyJsonCodec.new
framer = MyContentLengthFramer.new(socket, socket)

client = CodingAdventures::Rpc::Client.new(codec, framer)
  .on_notification("textDocument/publishDiagnostics") { |p| record(p) }

result = client.request("textDocument/hover", { "line" => 10, "character" => 5 })
client.notify("window/logMessage", { "type" => 3, "message" => "hi" })
```

---

## API

### Message types

All four are `Struct` classes with `keyword_init: true`:

| Class             | Fields                            | Direction        |
|-------------------|-----------------------------------|------------------|
| `RpcRequest`      | `id`, `method`, `params`          | client → server  |
| `RpcResponse`     | `id`, `result`                    | server → client  |
| `RpcErrorResponse`| `id`, `code`, `message`, `data`   | server → client  |
| `RpcNotification` | `method`, `params`                | either direction |

### ErrorCodes

```ruby
CodingAdventures::Rpc::ErrorCodes::PARSE_ERROR      # -32700
CodingAdventures::Rpc::ErrorCodes::INVALID_REQUEST  # -32600
CodingAdventures::Rpc::ErrorCodes::METHOD_NOT_FOUND # -32601
CodingAdventures::Rpc::ErrorCodes::INVALID_PARAMS   # -32602
CodingAdventures::Rpc::ErrorCodes::INTERNAL_ERROR   # -32603
```

### RpcCodec interface

Any object used as a codec must respond to:

```ruby
codec.encode(msg)    # RpcMessage → String (binary bytes)
codec.decode(bytes)  # String → RpcMessage OR raise RpcError
```

Include `CodingAdventures::Rpc::RpcCodec` for documentation and default
`NotImplementedError` guards.

### RpcFramer interface

Any object used as a framer must respond to:

```ruby
framer.read_frame    # → String | nil (nil = clean EOF)
framer.write_frame(bytes)  # → nil
```

Include `CodingAdventures::Rpc::RpcFramer` for documentation and default
`NotImplementedError` guards.

### Server

```ruby
Server.new(codec, framer)
  .on_request(method)      { |id, params| result_or_error_response }
  .on_notification(method) { |params| }
  .serve                   # blocks until EOF
```

Dispatch rules:
- Request, handler found → call handler, write `RpcResponse` or `RpcErrorResponse`
- Request, no handler → write `-32601 Method not found`
- Request, handler raises → write `-32603 Internal error` (panic recovery)
- Notification, handler found → call handler, write nothing
- Notification, no handler → silently drop
- Incoming responses → discarded (server-only mode)

### Client

```ruby
client = Client.new(codec, framer)
client.on_notification(method) { |params| }  # server-push handler
client.request(method, params)  # → result OR raise RpcError
client.notify(method, params)   # → nil (fire-and-forget)
```

---

## Implementing a codec

```ruby
class MyCodec
  include CodingAdventures::Rpc::RpcCodec

  def encode(msg)
    # 1. Convert msg to a native data structure
    # 2. Serialise to bytes
    # 3. Return bytes with Encoding::BINARY
  end

  def decode(bytes)
    # 1. Deserialise bytes → native object
    #    On failure → raise RpcError.new(ErrorCodes::PARSE_ERROR, ...)
    # 2. Discriminate on message shape
    #    On unknown shape → raise RpcError.new(ErrorCodes::INVALID_REQUEST, ...)
    # 3. Return the appropriate Struct
  end
end
```

## Implementing a framer

```ruby
class MyFramer
  include CodingAdventures::Rpc::RpcFramer

  def initialize(in_stream, out_stream)
    @in  = in_stream
    @out = out_stream
  end

  def read_frame
    # Read bytes up to the frame boundary.
    # Return nil on clean EOF.
    # Raise RpcError on framing errors.
  end

  def write_frame(bytes)
    # Wrap bytes in framing envelope and write to @out.
  end
end
```

---

## Test coverage

Run with:

```sh
bundle install
bundle exec rake test
```

Coverage is measured with SimpleCov. The minimum threshold is 80%;
actual coverage exceeds 95%.

---

## License

MIT
