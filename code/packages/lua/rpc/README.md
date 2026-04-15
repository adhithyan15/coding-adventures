# coding-adventures-rpc (Lua)

Codec-agnostic RPC primitive for Lua. Implements the abstract server and client
that `json-rpc` and future codec-specific packages build on top of.

## What it does

This package captures the **rules** of remote procedure calls without caring about
the wire format. Think of it like a phone call:

- The *information exchanged* (request, response, notification) is this package.
- The *language spoken* (JSON, MessagePack, Protobuf) is the **codec** — pluggable.
- The *phone network* (Content-Length stdio, TCP, WebSocket) is the **framer** — pluggable.

## Where it fits in the stack

```
┌───────────────────────────────────────────────┐
│  Application (handlers, business logic)        │
├───────────────────────────────────────────────┤
│  coding_adventures.rpc  ← THIS PACKAGE         │
│  RpcServer / RpcClient                         │
├───────────────────────────────────────────────┤
│  RpcCodec (pluggable — JSON, msgpack, …)       │
├───────────────────────────────────────────────┤
│  RpcFramer (pluggable — Content-Length, …)     │
├───────────────────────────────────────────────┤
│  Transport (stdin/stdout, TCP, …)              │
└───────────────────────────────────────────────┘
```

See `code/specs/rpc.md` for the full architecture specification.

## Interface contracts

Since Lua has no formal interface keyword, contracts are documented:

**RpcCodec** — any table with these methods:
```lua
codec:encode(msg)   → bytes (string)
codec:decode(bytes) → msg, err
```

**RpcFramer** — any table with these methods:
```lua
framer:read_frame()        → bytes|nil   -- nil = clean EOF
framer:write_frame(bytes)  → ok, err
```

**Message tables** (plain Lua tables with a `kind` field):
```lua
{ kind="request",      id=id,  method=method, params=params }
{ kind="response",     id=id,  result=result }
{ kind="error",        id=id,  code=code, message=msg, data=data }
{ kind="notification", method=method, params=params }
```

## Usage

### Server

```lua
local rpc = require("coding_adventures.rpc")

-- Build a server with your codec and framer (e.g., from coding_adventures.json_rpc).
local server = rpc.Server.new(my_codec, my_framer)

-- Register handlers (chainable).
server
    :on_request("add", function(id, params)
        -- Return result, nil on success.
        return params.a + params.b, nil
    end)
    :on_request("fail_me", function(id, params)
        -- Return nil, error_table on error.
        return nil, { code = rpc.errors.INVALID_PARAMS, message = "bad input" }
    end)
    :on_notification("log", function(params)
        io.stderr:write("[log] " .. tostring(params) .. "\n")
    end)

-- Blocking serve loop. Returns on clean EOF.
server:serve()
```

### Client

```lua
local rpc = require("coding_adventures.rpc")

local client = rpc.Client.new(my_codec, my_framer)

-- Register a handler for server-push notifications (chainable).
client:on_notification("progress", function(params)
    print("progress:", params)
end)

-- Blocking request. Returns result, nil on success; nil, err on error.
local result, err = client:request("add", { a = 3, b = 4 })
if err then
    print("error:", err.code, err.message)
else
    print("result:", result)   -- 7
end

-- Fire-and-forget notification.
client:notify("log", "something happened")
```

## Error codes

| Constant          | Value   | Meaning                                  |
|-------------------|---------|------------------------------------------|
| `PARSE_ERROR`     | -32700  | Codec could not decode the frame bytes   |
| `INVALID_REQUEST` | -32600  | Decoded but not a valid RPC message      |
| `METHOD_NOT_FOUND`| -32601  | No handler registered for the method     |
| `INVALID_PARAMS`  | -32602  | Handler rejected the params              |
| `INTERNAL_ERROR`  | -32603  | Unexpected error thrown by the handler   |

## Running tests

```sh
luarocks make --local coding-adventures-rpc-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
```

## Dependencies

None. Only Lua's standard library is used.
