# coding_adventures_json_rpc

JSON-RPC 2.0 over stdin/stdout with Content-Length framing.

This gem implements the transport layer beneath the Language Server Protocol
(LSP). Any LSP server in this repository delegates all message framing and
dispatch to this gem, keeping the LSP layer thin.

## What is JSON-RPC 2.0?

JSON-RPC is a stateless, lightweight remote procedure call protocol. A client
sends a **Request** (a JSON object with an `id` and a `method` name), and the
server replies with a **Response** (matching `id`, plus `result` or `error`).

For one-way events that need no reply, the client sends a **Notification** — a
JSON object with a `method` but no `id`. The server must not respond.

## Wire Format

Every message is preceded by an HTTP-inspired header block:

```
Content-Length: 97\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{...}}
```

The `Content-Length` is the **byte** length of the UTF-8-encoded JSON payload.

## Architecture

```
stdin  →  MessageReader  →  Server (dispatch)  →  MessageWriter  →  stdout
                                   ↓
                           on_request / on_notification blocks
```

## Quick Start

```ruby
require "coding_adventures_json_rpc"
CA = CodingAdventures::JsonRpc

STDIN.binmode
STDOUT.binmode

CA::Server.new(STDIN, STDOUT)
  .on_request("initialize") { |_id, _params| { capabilities: { hoverProvider: true } } }
  .on_notification("textDocument/didOpen") { |params|
    uri = params["textDocument"]["uri"]
    $stderr.puts "opened: #{uri}"
  }
  .serve
```

## API

### `MessageReader`

Reads one framed message from an IO stream.

```ruby
reader = CA::MessageReader.new(STDIN)
while (msg = reader.read_message)
  puts msg.inspect
end
```

- `read_message` — returns a typed message or `nil` on EOF
- `read_raw` — returns the raw JSON string without parsing

### `MessageWriter`

Writes one message to an IO stream with Content-Length framing.

```ruby
writer = CA::MessageWriter.new(STDOUT)
writer.write_message(CA::Response.new(id: 1, result: { ok: true }))
writer.write_raw('{"jsonrpc":"2.0","id":2,"result":null}')
```

### `Server`

Combines reader + writer with a method dispatch table.

```ruby
server = CA::Server.new(in_io, out_io)

server
  .on_request("method/name") { |id, params|
    # return result value, or a ResponseError
    { data: 42 }
  }
  .on_notification("event/name") { |params|
    # no return value; no response is sent
  }

server.serve
```

Dispatch rules:

| Situation | Action |
|-----------|--------|
| Request, handler found | Call block; send `result` or `error` |
| Request, no handler | Send `-32601 Method not found` |
| Request, handler raises | Send `-32603 Internal error` |
| Notification, handler found | Call block; send nothing |
| Notification, no handler | Silently ignore |

### Error Codes

```ruby
CA::ErrorCodes::PARSE_ERROR      # -32700
CA::ErrorCodes::INVALID_REQUEST  # -32600
CA::ErrorCodes::METHOD_NOT_FOUND # -32601
CA::ErrorCodes::INVALID_PARAMS   # -32602
CA::ErrorCodes::INTERNAL_ERROR   # -32603
```

### Message Types

All message types are immutable `Data.define` value objects:

```ruby
CA::Request.new(id:, method:, params: nil)
CA::Notification.new(method:, params: nil)
CA::Response.new(id:, result: nil, error: nil)
CA::ResponseError.new(code:, message:, data: nil)
```

## Production Notes

Always set binary mode on stdin/stdout before creating reader/writer:

```ruby
STDIN.binmode
STDOUT.binmode
```

Without this, Ruby's text-mode I/O on Windows may corrupt the `\r\n` framing
delimiter by translating it to `\n`.

## No Dependencies

This gem depends only on Ruby's standard library (`json`, `stringio`). It has
no runtime dependencies on any other coding-adventures gem.
