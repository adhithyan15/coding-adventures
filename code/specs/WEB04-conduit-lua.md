# WEB04 — Lua Conduit

## Overview

A Lua 5.4 port of the Conduit web framework backed by the same Rust `web-core`
engine used by the Ruby (WEB02) and Python (WEB03) ports. Handlers are plain
Lua functions; routing, lifecycle hooks, and HTTP I/O run in Rust.

---

## Architecture

```
Lua DSL (conduit/*.lua)
    — Application, Request, HandlerContext, HaltError, Server
    ↓  response protocol: nil (no override) | {status, headers, body}
conduit_native (Rust cdylib, ext/conduit_native/)
    — luaopen_conduit_native, route/hook registration, dispatch, lock mgmt
    ↓
web-core (WebApp, WebServer, HookRegistry, Router)
    ↓
embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
```

### Threading model

Lua 5.4 is single-threaded. `lua_State` is not thread-safe.
`web-core` dispatches requests on background Rust I/O threads.

Solution: every Rust thread that calls into Lua acquires an
`Arc<Mutex<()>>` (the "Lua lock") before calling `lua_pcall`. The lock
serialises all Lua re-entries; Lua is only ever executing on one OS
thread at a time.

For `serve()`, the calling Lua thread blocks inside `web_server.serve()`.
Since Lua is blocked, no other Lua code can run concurrently — but the
I/O threads still need the lock to protect the `lua_State` pointer
during the serve loop.

`serve_background()` (non-blocking) spawns a Rust `std::thread` that
runs the server loop. Used by tests to start the server without blocking
the Lua test runner.

---

## Package layout

```
code/packages/lua/conduit/
├── BUILD
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── coding-adventures-conduit-1.0.0-1.rockspec
├── ext/conduit_native/
│   ├── Cargo.toml
│   └── src/lib.rs
├── conduit/
│   ├── init.lua          -- require("conduit") entry point
│   ├── application.lua   -- Application class
│   ├── halt.lua          -- HaltError table + raise/check helpers
│   ├── handler_context.lua  -- HandlerContext (response helpers)
│   ├── request.lua       -- Request object
│   └── server.lua        -- Server wrapper
└── tests/
    ├── test_application.lua
    ├── test_halt.lua
    ├── test_handler_context.lua
    ├── test_request.lua
    └── test_server.lua
```

---

## Lua DSL

```lua
local conduit = require("conduit")
local html    = conduit.html
local json    = conduit.json
local halt    = conduit.halt

local app = conduit.Application.new()

app:before(function(ctx)
    if ctx:path() == "/down" then
        halt(503, "Under maintenance")
    end
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
    return html("<h1>Not Found: " .. ctx:path() .. "</h1>", 404)
end)

app:error_handler(function(ctx, err)
    return json({ error = "Internal Server Error" }, 500)
end)

app:set("app_name", "Conduit Hello")

local server = conduit.Server.new(app, { host = "127.0.0.1", port = 3000 })
server:serve()
```

### Response helpers (module-level, importable)

| Helper | Returns |
|--------|---------|
| `conduit.html(body [, status])` | `{status, {{"content-type","text/html"}}, body}` |
| `conduit.json(tbl [, status])` | `{status, {{"content-type","application/json"}}, json_str}` |
| `conduit.text(body [, status])` | `{status, {{"content-type","text/plain"}}, body}` |
| `conduit.redirect(location [, status])` | `{301‖302, {{"location",location}}, ""}` |
| `conduit.halt(status, body [, headers])` | raises `HaltError` |

---

## HandlerContext

Every handler receives a `ctx` (HandlerContext) that provides:

- `ctx:method()` — HTTP method string
- `ctx:path()` — request path
- `ctx:params()` — route params table (`{name = "Alice"}`)
- `ctx:query()` — parsed query string table
- `ctx:headers()` — request headers table (lowercase keys)
- `ctx:body()` — raw body string
- `ctx:json_body()` — parse body as JSON (raises HaltError 400 on invalid JSON)
- `ctx:content_type()` — Content-Type header value or nil

Response helpers on ctx mirror the module-level helpers:
- `ctx:html(body [, status])`, `ctx:json(tbl [, status])`, `ctx:text(body [, status])`
- `ctx:halt(status, body [, headers])`, `ctx:redirect(location [, status])`

---

## HaltError

`HaltError` is a Lua table raised with `error()`:

```lua
{ __conduit_halt = true, status = 503, body = "...", headers = {} }
```

`conduit.halt(status, body [, headers])` constructs and raises one.

Rust dispatch catches it via `lua_pcall` return code:
- Non-zero pcall return → check if top of stack is a table with `__conduit_halt = true`
- If yes: parse `{status, body, headers}` into a `WebResponse`
- If no: treat as an unhandled error, call the error handler

---

## Request → env table keys (mirrors Ruby/Python)

```
REQUEST_METHOD, PATH_INFO, QUERY_STRING
conduit.route_params   (table)
conduit.query_params   (table)
conduit.headers        (table, lowercase keys)
conduit.body           (string)
conduit.content_type   (string or nil)
conduit.content_length (integer or nil)
SERVER_PROTOCOL, rack.url_scheme, REMOTE_ADDR, REMOTE_PORT
SERVER_NAME, SERVER_PORT
```

---

## Rust extension (conduit_native)

### Exposed Lua API

```
conduit_native.new_app()                             → app userdata
conduit_native.app_add_route(app, method, pat, fn)  → nil
conduit_native.app_add_before(app, fn)               → nil
conduit_native.app_add_after(app, fn)                → nil
conduit_native.app_set_not_found(app, fn)            → nil
conduit_native.app_set_error_handler(app, fn)        → nil
conduit_native.new_server(app, host, port, max_conn) → server userdata
conduit_native.server_serve(server)                  → nil  [blocks]
conduit_native.server_serve_background(server)       → nil  [non-blocking]
conduit_native.server_stop(server)                   → nil
conduit_native.server_local_port(server)             → integer
conduit_native.server_running(server)                → boolean
conduit_native.server_dispose(server)                → nil
```

### Function reference storage

Lua callbacks are stored as integer refs via `luaL_ref(L, LUA_REGISTRYINDEX)`.
On dispatch: `lua_rawgeti(L, LUA_REGISTRYINDEX, ref)` pushes the function.
`lua_pcall(L, nargs, nresults, 0)` calls it with error capture.
On cleanup: `luaL_unref(L, LUA_REGISTRYINDEX, ref)` releases the slot.

### Rust state structs

```rust
struct LuaConduitApp {
    routes: Vec<RouteEntry>,     // {method, pattern, handler_ref}
    before_refs: Vec<i32>,       // luaL_ref ints for before filters
    after_refs: Vec<i32>,
    not_found_ref: i32,          // LUA_NOREF if not set
    error_handler_ref: i32,
}

struct LuaConduitServer {
    lua: *mut lua_State,         // NOT Send — only accessed under lua_lock
    lua_lock: Arc<Mutex<()>>,
    server: Option<PlatformWebServer>,
    running: Arc<AtomicBool>,
}
```

### Dispatch flow (per request)

```
web-core I/O thread receives request
  → closure captures (lua: *mut lua_State, lua_lock: Arc<Mutex<()>>, handler_ref: i32)
  → acquire lua_lock
  → lua_rawgeti(L, LUA_REGISTRYINDEX, handler_ref)  -- push function
  → push env table (build_env)
  → lua_pcall(L, 1, 1, 0)
  → if pcall failed: check HaltError or fall through to error handler
  → parse return value: nil → None, table → WebResponse
  → release lua_lock
```

---

## JSON encoding (Lua-side)

A minimal `conduit/json.lua` module handles JSON serialisation for handler
responses. It supports: strings, numbers, booleans, nil, arrays, objects.
No third-party JSON library dependency — the repo avoids external LuaRocks
packages beyond test tooling.

---

## BUILD

```shell
set -e
cd ext/conduit_native && cargo build --release && cd ../..
python3 -c "import glob,shutil,sys,os; libs=glob.glob('ext/conduit_native/target/release/libconduit_native.*')+glob.glob('ext/conduit_native/target/release/conduit_native.*'); src=next((l for l in libs if l.endswith(('.so','.dylib','.dll'))),None); sys.exit('no lib') if not src else shutil.copy(src,'conduit/conduit_native.dll' if os.name=='nt' else 'conduit/conduit_native.so')"
luarocks install busted --local 2>/dev/null || true
busted tests/ --coverage
```

---

## Tests (target: 40+)

### test_halt.lua
- HaltError table has correct fields
- `conduit.halt()` raises the error
- `is_halt_error()` returns true for HaltError, false otherwise

### test_handler_context.lua
- `html()` returns correct status/content-type/body
- `json()` serialises table to JSON
- `text()` returns text/plain
- `redirect()` returns 301 with Location header
- `halt()` raises HaltError with correct status/body
- `params()`, `query()`, `headers()`, `body()` return correct values
- `json_body()` parses valid JSON, raises 400 on invalid

### test_application.lua
- `app:get/post/put/delete/patch` register routes
- `app:before/after` register filter lists
- `app:not_found/error_handler` set single handlers
- `app:set/get` manage settings
- Routes are accessible via `app:routes()`

### test_server.lua (E2E via real TCP)
- GET / → 200 HTML
- GET /hello/Alice → 200 JSON `{message="Hello Alice"}`
- POST /echo → 200 JSON echo
- GET /redirect → 301 with Location: /
- GET /halt → 403 Forbidden
- GET /down → 503 maintenance (before filter)
- GET /error → 500 JSON `{error="Internal Server Error"}`
- GET /missing → 404 HTML with path
- Settings: `app:get("app_name")` returns "Conduit Hello"

---

## Demo program

`code/programs/lua/conduit-hello/hello.lua` — same 8-route demo as Ruby/Python:
`/`, `/hello/:name`, `/echo`, `/redirect`, `/halt`, `/down`, `/error`, custom not_found.

---

## Deviations from plan

None at this time. Implementation may note deviations if they arise.
