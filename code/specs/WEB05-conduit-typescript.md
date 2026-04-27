# WEB05 — TypeScript/Node.js Conduit

## Overview

A TypeScript/Node.js port of the Conduit web framework, backed by the same
Rust `web-core` engine used by Ruby (WEB02), Python (WEB03), and Lua (WEB04)
ports. Handlers are plain TypeScript/JavaScript functions; routing, lifecycle
hooks, and HTTP I/O run in Rust via an N-API native addon.

---

## Architecture

```
TypeScript DSL (src/*.ts)
    — Application, Request, HandlerContext, HaltError, Server
    ↓  response protocol: undefined (no override) | [status, headers, body]
conduit_native_node (Rust cdylib, ext/conduit_native_node/)
    — napi_register_module_v1, route/hook registration, TSFN dispatch
    ↓  napi_threadsafe_function per handler
web-core (WebApp, WebServer, HookRegistry, Router)
    ↓
embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
```

### Threading model (critical)

Node.js runs JS on a single V8 main thread. N-API functions may only be called
from this main thread. `web-core` dispatches requests on background Rust I/O
threads.

**Solution: `napi_create_threadsafe_function` (TSFN)**

Every JS callback registered with the server is wrapped in a TSFN at `serve()`
time. When a request arrives on a Rust background thread:

1. Allocate a `RequestSlot` on the heap:
   `{ env_map, response: Arc<(Mutex<Option<WebResponse>>, Condvar)> }`
2. `napi_call_threadsafe_function(tsfn, Box::into_raw(slot), NAPI_TSFN_BLOCKING)` —
   queues a call on the JS event queue and **blocks the background thread**.
3. The TSFN's `call_js_cb` runs on the V8 main thread:
   - Builds a `ctx` JS object from `env_map`
   - Calls the user's JS handler: `result = js_handler(ctx)`
   - Parses the return value into `WebResponse`
   - Stores it in `slot.response` and signals the `Condvar`
4. Background thread wakes, reads the response, sends it.

**`serve()` vs `serve_background()`**:
Node.js is event-loop-based — there is no "block the main thread" mode. Both
`serve()` and `serveBackground()` start the HTTP server on a background Rust
thread and return immediately. The process stays alive because the TsFns are
ref-counted with `napi_ref_threadsafe_function`; calling `stop()` releases them
so the event loop can exit.

**Before/after filter dispatch**: Multiple TSFN calls execute sequentially,
each blocking the background thread in turn. This serialises JS execution
while allowing the Rust I/O layer to remain non-blocking.

**HaltError detection**: After `lua_pcall` equivalent (calling the JS handler),
check `napi_is_exception_pending`. If an exception is pending:
- `napi_get_and_clear_last_exception` extracts the thrown value
- `napi_get_named_property(ex, "__conduit_halt")` checks for HaltError marker
- If present, extract `{ status, body, headers }` → `WebResponse`
- If absent, pass message string to the error handler TSFN

---

## Package layout

```
code/packages/typescript/conduit/
├── BUILD
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── package.json               -- @coding-adventures/conduit
├── tsconfig.json
├── index.js                   -- ESM entry (loads .node, re-exports)
├── index.d.ts                 -- Public TypeScript type declarations
├── ext/conduit_native_node/
│   ├── Cargo.toml             -- cdylib, deps: node-bridge, web-core
│   └── src/lib.rs
└── src/
    ├── index.ts               -- re-exports all public API
    ├── application.ts         -- Application class
    ├── halt_error.ts          -- HaltError class
    ├── handler_context.ts     -- HandlerContext class
    ├── request.ts             -- Request class
    └── server.ts              -- Server class
tests/
    ├── test_halt_error.ts
    ├── test_request.ts
    ├── test_handler_context.ts
    ├── test_application.ts
    └── test_server.ts         -- E2E via real TCP (node:http)
```

---

## TypeScript DSL

```typescript
import * as conduit from "@coding-adventures/conduit";

const app = new conduit.Application();

app.before((ctx) => {
    if (ctx.path() === "/down") conduit.halt(503, "Under maintenance");
});

app.get("/", (ctx) => conduit.html("<h1>Hello from Conduit!</h1>"));

app.get("/hello/:name", (ctx) =>
    conduit.json({ message: `Hello ${ctx.params()["name"]}` })
);

app.post("/echo", (ctx) => conduit.json(ctx.jsonBody()));

app.get("/redirect", (ctx) => conduit.redirect("/", 301));

app.notFound((ctx) =>
    conduit.json({ message: "Not Found", path: ctx.path() }, 404)
);

app.onError((ctx, err) =>
    conduit.json({ error: "Internal Server Error" }, 500)
);

app.set("app_name", "Conduit Hello");

const server = new conduit.Server(app, { host: "127.0.0.1", port: 3000 });
server.serve(); // non-blocking in Node.js; process stays alive via TSFN refs
```

### Response helpers (module-level)

| Helper | Return type | Status default |
|--------|-------------|----------------|
| `conduit.html(body, status?)` | `ConduitResponse` | 200 |
| `conduit.json(data, status?)` | `ConduitResponse` | 200 |
| `conduit.text(body, status?)` | `ConduitResponse` | 200 |
| `conduit.redirect(location, status?)` | `ConduitResponse` | 302 |
| `conduit.halt(status, body, headers?)` | `never` (throws) | — |

All helpers also available on `ctx`: `ctx.html(...)`, `ctx.json(...)`, etc.

### Response wire format (TS → Rust)

```typescript
type ConduitResponse = [number, [string, string][], string];
// [status, headers, body]
// headers: array of [name, value] pairs
```

---

## HandlerContext

Every handler and filter receives a `ctx: HandlerContext` that provides:

| Method | Description |
|--------|-------------|
| `ctx.method()` | HTTP method string ("GET", "POST", …) |
| `ctx.path()` | Request path ("/hello/world") |
| `ctx.params()` | Route params `{ name: "Alice" }` |
| `ctx.query()` | Parsed query-string `{ q: "foo" }` |
| `ctx.headers()` | Request headers (lowercase keys) |
| `ctx.body()` | Raw body string |
| `ctx.jsonBody()` | Parsed JSON body (throws HaltError 400 on invalid JSON) |
| `ctx.contentType()` | Content-Type header or null |

Response helpers on ctx mirror module-level:
- `ctx.html(body, status?)`, `ctx.json(data, status?)`, `ctx.text(body, status?)`
- `ctx.halt(status, body, headers?)`, `ctx.redirect(location, status?)`

---

## HaltError

```typescript
class HaltError extends Error {
    readonly __conduit_halt = true as const;
    constructor(
        public readonly status: number,
        public readonly body: string,
        public readonly haltHeaders: Record<string, string> = {}
    )
}
```

`conduit.halt(status, body, headers?)` constructs and throws one.

**Detection in Rust `call_js_cb`**:
1. `napi_is_exception_pending(env, &pending)` → true
2. `napi_get_and_clear_last_exception(env, &ex_val)`
3. `napi_get_named_property(env, ex_val, "__conduit_halt", &halt_marker)`
4. Check `halt_marker` is a boolean `true` via `napi_get_value_bool`
5. Extract `status` (number), `body` (string), `haltHeaders` (object) →
   build `WebResponse`

If `__conduit_halt` is absent or false: call the error handler TSFN with the
exception message string.

---

## Rust extension (conduit_native_node)

### Exposed JS API

```
conduit_native.newApp()                              → App (JS object + native wrap)
app.addRoute(method, pattern, fn)                   → undefined
app.addBefore(fn)                                   → undefined
app.addAfter(fn)                                    → undefined
app.setNotFound(fn)                                 → undefined
app.setErrorHandler(fn)                             → undefined
app.setSetting(key, value)                          → undefined
app.getSetting(key)                                 → string | undefined
conduit_native.newServer(app, host, port, maxConn)  → Server (JS object)
server.serve()                                      → undefined [non-blocking]
server.serveBackground()                            → undefined [same as serve()]
server.stop()                                       → undefined
server.localPort()                                  → number
server.running()                                    → boolean
```

### TSFN lifecycle

```
newServer():
  For each handler ref in app:
    napi_create_threadsafe_function(env, js_fn, ..., call_js_cb, &tsfn)
    napi_ref_threadsafe_function(env, tsfn)   // keep event loop alive

serve():
  Spawn background Rust thread → web_server.serve()

stop():
  web_server.stop_handle().stop()
  Join background thread
  For each tsfn:
    napi_release_threadsafe_function(tsfn, NAPI_TSFN_RELEASE)
    napi_unref_threadsafe_function(env, tsfn)
```

### Rust state structs

```rust
struct NativeApp {
    routes: Vec<RouteEntry>,   // { method, pattern, js_fn_ref }
    before_refs: Vec<napi_ref>,
    after_refs: Vec<napi_ref>,
    not_found_ref: Option<napi_ref>,
    error_handler_ref: Option<napi_ref>,
    settings: HashMap<String, String>,
}

struct NativeServer {
    env: napi_env,              // V8 env — only touched on V8 thread (TSFN finalize)
    route_tsfns: Vec<RouteTsfn>, // { method, pattern, tsfn }
    before_tsfns: Vec<napi_threadsafe_function>,
    after_tsfns: Vec<napi_threadsafe_function>,
    not_found_tsfn: Option<napi_threadsafe_function>,
    error_handler_tsfn: Option<napi_threadsafe_function>,
    server: Option<PlatformWebServer>,
    running: Arc<AtomicBool>,
    bg_thread: Option<JoinHandle<()>>,
}
```

### RequestSlot (per-request heap-allocated context)

```rust
struct RequestSlot {
    env_map: HashMap<String, EnvValue>,
    response: Arc<(Mutex<Option<WebResponse>>, Condvar)>,
}
```

### Dispatch flow (per request)

```
web-core I/O thread:
  → build_env_map(request) → HashMap<String, EnvValue>
  → create Arc<(Mutex<Option<WebResponse>>, Condvar)>
  → run_before_filters:
      for each before_tsfn:
        alloc RequestSlot; call_threadsafe_function(tsfn, slot_ptr, BLOCKING)
        wait on condvar
        if response is Some → return it (halt)
  → dispatch route handler:
        alloc RequestSlot; call_threadsafe_function(route_tsfn, slot_ptr, BLOCKING)
        wait on condvar
        slot.response → WebResponse
  → run_after_filters (same pattern)
  → return final WebResponse

V8 main thread (call_js_cb):
  → reconstruct RequestSlot from data ptr
  → build ctx JS object from env_map (using NativeCtx class)
  → call js_handler(ctx) via napi_call_function
  → if exception pending:
      check __conduit_halt → WebResponse from HaltError
      else → call error_handler_tsfn (recursive, same pattern)
  → parse return value [status, headers, body] → WebResponse
  → store in slot.response; signal condvar
```

---

## node-bridge additions

The following N-API functions need to be added to
`code/packages/rust/node-bridge/src/lib.rs`:

```rust
// Threadsafe functions
pub type napi_threadsafe_function = *mut c_void;
pub type napi_threadsafe_function_release_mode = i32;
pub const NAPI_TSFN_RELEASE: i32 = 0;
pub const NAPI_TSFN_ABORT: i32 = 1;
pub type napi_threadsafe_function_call_mode = i32;
pub const NAPI_TSFN_NONBLOCKING: i32 = 0;
pub const NAPI_TSFN_BLOCKING: i32 = 1;
pub type napi_threadsafe_function_call_js =
    Option<unsafe extern "C" fn(napi_env, napi_value, *mut c_void, *mut c_void)>;

napi_create_threadsafe_function(env, func, async_resource,
    async_resource_name, max_queue_size, initial_thread_count,
    thread_finalize_data, thread_finalize_cb, context,
    call_js_cb, result) → napi_status
napi_acquire_threadsafe_function(tsfn) → napi_status
napi_call_threadsafe_function(tsfn, data, is_blocking) → napi_status
napi_release_threadsafe_function(tsfn, mode) → napi_status
napi_ref_threadsafe_function(env, tsfn) → napi_status
napi_unref_threadsafe_function(env, tsfn) → napi_status

// JS function calls and object inspection
napi_call_function(env, recv, func, argc, argv, result) → napi_status
napi_create_object(env, result) → napi_status
napi_get_named_property(env, object, utf8name, result) → napi_status
napi_typeof(env, value, result) → napi_status
napi_is_array(env, value, result) → napi_status
napi_get_value_int32(env, value, result) → napi_status
napi_get_value_bool(env, value, result) → napi_status

// Exception handling
napi_is_exception_pending(env, result) → napi_status
napi_get_and_clear_last_exception(env, result) → napi_status

// JS references (stable handles that survive GC moves)
pub type napi_ref = *mut c_void;
napi_create_reference(env, value, initial_refcount, result) → napi_status
napi_get_reference_value(env, ref_, result) → napi_status
napi_delete_reference(env, ref_) → napi_status
napi_new_instance(env, constructor, argc, argv, result) → napi_status

// Type enum constants
pub type napi_valuetype = i32;
pub const NAPI_UNDEFINED: i32 = 0;
pub const NAPI_NULL: i32 = 1;
pub const NAPI_BOOLEAN: i32 = 2;
pub const NAPI_NUMBER: i32 = 3;
pub const NAPI_STRING: i32 = 4;
pub const NAPI_OBJECT: i32 = 6;
pub const NAPI_FUNCTION: i32 = 7;
```

---

## BUILD

```shell
set -e
cd ext/conduit_native_node && cargo build --release && cd ../..
python3 -c "
import glob, shutil, sys, os
libs = glob.glob('ext/conduit_native_node/target/release/libconduit_native_node.*') + \
       glob.glob('ext/conduit_native_node/target/release/conduit_native_node.*')
src = next((l for l in libs if l.endswith(('.so', '.dylib', '.dll'))), None)
if src is None: sys.exit('ERROR: no conduit_native_node lib found')
shutil.copy(src, 'conduit_native_node.node')
print('Copied', src)
"
npm ci --quiet
npx tsc --noEmit
npx vitest run --coverage
```

---

## Tests (target: 40+)

### test_halt_error.ts
- HaltError has correct `__conduit_halt = true` marker
- HaltError stores status, body, headers
- `conduit.halt()` throws HaltError
- HaltError is instanceof Error

### test_request.ts
- `method()`, `path()`, `params()`, `query()`, `headers()`, `body()`,
  `contentType()` return correct values from env map
- `jsonBody()` parses valid JSON
- `jsonBody()` throws HaltError 400 on invalid JSON

### test_handler_context.ts
- `html()` returns `[200, [["content-type","text/html"]], body]`
- `json()` serialises object to JSON string with correct content-type
- `text()` returns `[200, [["content-type","text/plain"]], body]`
- `redirect()` returns `[302, [["location","/"]], ""]`
- `halt()` throws HaltError
- Custom status overrides default 200
- Response helpers available both on ctx and as module-level functions

### test_application.ts
- `app.get/post/put/delete/patch` register routes
- `app.before/after` register filter arrays
- `app.notFound/onError` set single handlers
- `app.set/getSetting` manage settings
- Routes accessible via `app.routes()`

### test_server.ts (E2E via real TCP)
- GET / → 200 HTML
- GET / has text/html content-type
- GET /hello/Alice → 200 JSON `{ message: "Hello Alice" }`
- GET /hello/World → body contains "World"
- POST /echo → 200 JSON echo
- GET /redirect → 301 with Location: /
- GET /halt → 403 Forbidden
- GET /down → 503 via before filter
- GET /error → 500 JSON via error handler
- GET /missing → 404 via notFound handler
- Custom 404 includes path in response
- `server.localPort()` returns valid port number
- `server.running()` returns true while serving
- `app.getSetting("app_name")` returns configured value
- GET /hello/:name handles special chars in name

---

## Demo program

`code/programs/typescript/conduit-hello/hello.ts` — same 8-route demo:
`/`, `/hello/:name`, `/echo`, `/redirect`, `/halt`, `/down`, `/error`,
custom `notFound`.

---

## Deviations from plan

- `serve()` and `serveBackground()` are equivalent in Node.js (both
  non-blocking). The process stays alive via `napi_ref_threadsafe_function`.
  This is the idiomatic Node.js server lifecycle, not a limitation.
- `HaltError.headers` renamed `haltHeaders` to avoid collision with the
  native `Error.prototype` (no built-in `headers` property, but avoids
  any future confusion). The `__conduit_halt` wire protocol is unchanged.
