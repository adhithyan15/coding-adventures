# coding-adventures-json-rpc

JSON-RPC 2.0 over stdin/stdout with Content-Length framing — the wire
protocol beneath the Language Server Protocol (LSP).

## What is this?

This package implements the JSON-RPC 2.0 specification. It is the transport
layer that every LSP server in coding-adventures builds on. The LSP server for
Brainfuck (and all future language servers) registers method handlers here and
lets this library handle framing, dispatch, and error responses.

## Why JSON-RPC before LSP?

The LSP spec sits on top of JSON-RPC. Building JSON-RPC first:

1. Keeps the LSP layer thin — it only knows about LSP-specific methods, not framing.
2. Lets us test the transport independently of any language server logic.
3. Gives us a reusable library for any future RPC-based protocol (DAP, custom tools).

## Installation

```bash
uv pip install coding-adventures-json-rpc
```

## Quick start

```python
import sys
from json_rpc import Server, ResponseError

server = (
    Server(sys.stdin.buffer, sys.stdout.buffer)
    .on_request("initialize", lambda id, params: {"capabilities": {}})
    .on_request("shutdown", lambda id, params: None)
    .on_notification("exit", lambda params: None)
)
server.serve()
```

## Message types

All messages carry `"jsonrpc": "2.0"` on the wire.

| Type           | Has `id`? | Has `method`? | Has `result`/`error`? | Direction   |
|----------------|-----------|---------------|-----------------------|-------------|
| `Request`      | Yes       | Yes           | No                    | Client→Server |
| `Response`     | Yes       | No            | Yes                   | Server→Client |
| `Notification` | No        | Yes           | No                    | Client→Server |

## Wire format

Each message is preceded by an HTTP-inspired header:

```
Content-Length: <n>\r\n
\r\n
<UTF-8 JSON payload, exactly n bytes>
```

`Content-Length` is a byte count (not character count). For Unicode-heavy
payloads this distinction matters.

## Error codes

```python
from json_rpc import PARSE_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND
from json_rpc import INVALID_PARAMS, INTERNAL_ERROR
```

| Constant          | Code    | Meaning                                    |
|-------------------|---------|--------------------------------------------|
| `PARSE_ERROR`     | -32700  | Payload is not valid JSON                  |
| `INVALID_REQUEST` | -32600  | Valid JSON but not a valid Request object  |
| `METHOD_NOT_FOUND`| -32601  | Method has no registered handler           |
| `INVALID_PARAMS`  | -32602  | Wrong parameter shape for a method         |
| `INTERNAL_ERROR`  | -32603  | Unhandled exception inside a handler       |

## API reference

### `MessageReader(stream: IO[bytes])`

```python
reader = MessageReader(sys.stdin.buffer)
msg = reader.read_message()  # Returns Message | None (None = EOF)
raw = reader.read_raw()      # Returns str | None (raw JSON string)
```

### `MessageWriter(stream: IO[bytes])`

```python
writer = MessageWriter(sys.stdout.buffer)
writer.write_message(response)    # Typed message
writer.write_raw('{"jsonrpc":"2.0",...}')  # Raw JSON string
```

### `Server(in_stream, out_stream)`

```python
server = Server(sys.stdin.buffer, sys.stdout.buffer)
server.on_request("method_name", handler)       # handler(id, params) → result | ResponseError
server.on_notification("event_name", handler)   # handler(params) → None
server.serve()  # Blocks until EOF
```

## How it fits in the stack

```
code editor (client)
    │
    │  stdin/stdout (Content-Length framed JSON-RPC)
    │
    ▼
json-rpc (this package)
    │
    │  typed Message objects
    │
    ▼
lsp-server (future package)
    │
    │  AST, diagnostics, hover info
    │
    ▼
brainfuck (lexer + parser + interpreter)
```

## Dependencies

None. Standard library only (`json`, `io`, `dataclasses`).
