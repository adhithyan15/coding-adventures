# coding-adventures-json-rpc (Lua)

JSON-RPC 2.0 transport library — Content-Length-framed messages over stdin/stdout.

## Overview

JSON-RPC 2.0 is the wire protocol underlying the Language Server Protocol (LSP).
This package implements the transport layer: reading and writing framed messages,
and dispatching them to registered handlers.  It has **no** dependency on any
other coding-adventures package — only Lua's standard library is required.

### Why JSON-RPC?

LSP servers (e.g., for Brainfuck, Algol, Starlark) must speak JSON-RPC.  By
implementing the transport layer once and reusing it across all language servers,
each server only needs to register handlers — it never touches framing or dispatch.

## Framing

Each message is preceded by an HTTP-style header:

```
Content-Length: <n>\r\n
\r\n
<UTF-8 JSON payload, exactly n bytes>
```

`Content-Length` is the byte length of the UTF-8-encoded JSON body.  The reader
reads the header, extracts `n`, then reads exactly `n` bytes — no more, no less.

## Quick Start

```lua
local JsonRpc = require("coding_adventures.json_rpc")

local server = JsonRpc.Server:new(io.stdin, io.stdout)

-- Register a request handler — must return a result table or ResponseError.
server:on_request("initialize", function(id, params)
  return { capabilities = { hoverProvider = true } }
end)

-- Register a notification handler — return value is ignored.
server:on_notification("textDocument/didOpen", function(params)
  -- parse params.textDocument.text ...
end)

-- Block until stdin closes.
server:serve()
```

## API Reference

### Message constructors

```lua
JsonRpc.Request(id, method, params)      -- params optional
JsonRpc.Response(id, result)
JsonRpc.ErrorResponse(id, error_obj)     -- error_obj = { code, message, data? }
JsonRpc.Notification(method, params)     -- params optional
```

### MessageReader

```lua
local reader = JsonRpc.MessageReader:new(stream)
local msg, err = reader:read_message()   -- returns message, nil  OR  nil, error  OR  nil, nil (EOF)
local raw, err = reader:read_raw()       -- returns raw JSON string
```

### MessageWriter

```lua
local writer = JsonRpc.MessageWriter:new(stream)
writer:write_message(message)
writer:write_raw(json_string)
```

### Server

```lua
local server = JsonRpc.Server:new(in_stream, out_stream)
server:on_request("method/name", function(id, params) return result end)
server:on_notification("method/name", function(params) end)
server:serve()
```

### Error constants

```lua
JsonRpc.errors.PARSE_ERROR      -- -32700
JsonRpc.errors.INVALID_REQUEST  -- -32600
JsonRpc.errors.METHOD_NOT_FOUND -- -32601
JsonRpc.errors.INVALID_PARAMS   -- -32602
JsonRpc.errors.INTERNAL_ERROR   -- -32603
```

## Running Tests

```bash
cd tests
busted . --verbose --pattern=test_
```

## Relationship to LSP

The Language Server Protocol spec (`code/specs/lsp-server.md`) builds on top of
this package.  The JSON-RPC layer is protocol-agnostic — it knows nothing about
LSP method names or parameter shapes.
