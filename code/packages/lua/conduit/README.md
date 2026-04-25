# Conduit for Lua 5.4 (WEB04)

A Lua 5.4 port of the Conduit web framework, backed by the same Rust `web-core`
engine used by the Ruby (WEB02) and Python (WEB03) ports. Handlers are plain
Lua functions; routing, lifecycle hooks, and HTTP I/O run in Rust via a cdylib
C extension.

## Architecture

```
Lua DSL (conduit/*.lua)
    — Application, Server, HandlerContext, HaltError
    ↓  response protocol: nil | {status, headers, body}
conduit_native (Rust cdylib)
    — luaopen_conduit_native, route/hook registration, dispatch, lock
    ↓
web-core (WebApp, WebServer, Router, HookRegistry)
    ↓
embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
```

## Quick start

```lua
local conduit = require("conduit")
local html    = conduit.html
local json    = conduit.json
local halt    = conduit.halt

local app = conduit.Application.new()

app:before(function(ctx)
    if ctx:path() == "/down" then halt(503, "Under maintenance") end
end)

app:get("/", function(ctx)
    return html("<h1>Hello from Conduit!</h1>")
end)

app:get("/hello/:name", function(ctx)
    return json({ message = "Hello " .. ctx:params()["name"] })
end)

app:post("/echo", function(ctx)
    return json(ctx:json_body())
end)

app:not_found(function(ctx)
    return html("<h1>Not Found</h1>", 404)
end)

app:error_handler(function(ctx, err)
    return json({ error = "Internal Server Error" }, 500)
end)

app:set("app_name", "Hello World")

local server = conduit.Server.new(app, { host = "127.0.0.1", port = 3000 })
server:serve()
```

## Response helpers

| Helper | Returns |
|--------|---------|
| `conduit.html(body [, status])` | `{status, {{"content-type","text/html"}}, body}` |
| `conduit.json(tbl [, status])` | `{status, {{"content-type","application/json"}}, json_str}` |
| `conduit.text(body [, status])` | `{status, {{"content-type","text/plain"}}, body}` |
| `conduit.redirect(location [, status])` | `{301‖302, {{"location",location}}, ""}` |
| `conduit.halt(status, body [, headers])` | raises `HaltError` |

All helpers are also available as `ctx:html(...)`, `ctx:json(...)`, etc.

## HandlerContext (ctx)

| Method | Description |
|--------|-------------|
| `ctx:method()` | HTTP method string ("GET", "POST", …) |
| `ctx:path()` | Request path ("/hello/world") |
| `ctx:params()` | Route params table (`{name = "Alice"}`) |
| `ctx:query()` | Parsed query-string table |
| `ctx:headers()` | Request headers (lowercase keys) |
| `ctx:body()` | Raw body string |
| `ctx:json_body()` | Parsed JSON body (raises 400 on invalid JSON) |
| `ctx:content_type()` | Content-Type header or nil |

## Building

```bash
cd ext/conduit_native && cargo build --release && cd ../..
# Copy the native extension where Lua can find it
python3 -c "import glob,shutil,sys,os; ..."
# Run tests
busted tests/ --verbose
```

Or just run the `BUILD` script via the repo build tool.

## Tests

60+ tests across five files using `busted`:

- `tests/test_halt.lua` — HaltError construction and raise
- `tests/test_request.lua` — Request accessor methods
- `tests/test_handler_context.lua` — Response helpers
- `tests/test_application.lua` — Route registration and settings
- `tests/test_server.lua` — Full E2E via real TCP (requires luasocket)

## Threading model

Lua 5.4 is single-threaded. `web-core` dispatches requests on background Rust
I/O threads. Every dispatch closure acquires an `Arc<Mutex<()>>` (the "Lua lock")
before calling `lua_pcall`, serialising all Lua re-entries to a single OS thread.

## Dependencies

- Lua 5.4
- Rust stable toolchain (for building the C extension)
- `busted` + `luasocket` (test-only)

## Related packages

- `code/packages/rust/web-core/` — Core HTTP engine
- `code/packages/rust/lua-bridge/` — Lua C API declarations
- `code/packages/ruby/conduit/` — Ruby reference implementation (WEB02)
- `code/packages/python/conduit/` — Python port (WEB03)
