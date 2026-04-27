# WEB03 — Conduit Python (Flask-like DSL)

## Overview

`conduit` for Python is a Flask-like web framework backed by the same Rust
`web-core` HTTP engine used by the Ruby port (WEB02). Routing, TCP framing,
and the HTTP/1 pipeline all live in Rust. Python owns handler logic, decorators,
filter registration, and the DSL surface.

The goal is a DSL that feels like Flask — function decorators, explicit `ctx`
argument, `ctx.json(data)` to respond — but runs on a Rust event loop with
no WSGI layer in between.

```python
from coding_adventures.conduit import Conduit

app = Conduit()

@app.before_request
def maintenance(ctx):
    if ctx.path == "/down":
        ctx.halt(503, "Under maintenance")

@app.get("/")
def index(ctx):
    ctx.html("<h1>Hello from Conduit!</h1>")

@app.get("/hello/<name>")
def hello(ctx):
    ctx.json({"message": f"Hello {ctx.params['name']}"})

@app.post("/echo")
def echo(ctx):
    ctx.json(ctx.request.json())

@app.get("/redirect")
def do_redirect(ctx):
    ctx.redirect("/", 301)

@app.not_found
def on_not_found(ctx):
    ctx.html(f"<h1>Not Found: {ctx.path}</h1>", 404)

@app.error_handler
def on_error(ctx, err):
    ctx.json({"error": "Internal Server Error"}, 500)

app.settings["app_name"] = "Conduit Hello"

if __name__ == "__main__":
    print(f"Starting {app.settings['app_name']}...")
    server = app.serve(host="127.0.0.1", port=3000)
```

## Architecture: Rust vs. Python Split

The same split as WEB02 — Rust owns plumbing, Python owns logic.

```text
┌──────────────────────────────────────────────────────────────┐
│  Flask-like DSL layer (Python)                                 │
│  Conduit  HandlerContext  HaltException  Request  Server       │
│  before/after filters · halt/redirect/json/html helpers        │
│  not_found/error decorators · settings dict                    │
└──────────────────────────┬───────────────────────────────────┘
                           │  native_dispatch_route(i, env)  → None | [s,h,b]
                           │  native_run_before_filters(env) → None | [s,h,b]
                           │  native_run_after_filters(env, [s,h,b]) → [s,h,b]
                           │  native_run_not_found(env)      → None | [s,h,b]
                           │  native_run_error_handler(env, msg) → None | [s,h,b]
┌──────────────────────────▼───────────────────────────────────┐
│  conduit_native (Rust cdylib)                                  │
│  Wraps web-core WebApp · registers hooks at init time          │
│  Releases GIL (PyEval_SaveThread) for blocking serve()         │
│  Re-acquires GIL (PyGILState_Ensure) for every Python call     │
│  Detects HaltException signal: None vs. [s,h,b] from Python   │
└──────────────────────────┬───────────────────────────────────┘
                           │  WebApp::handle(HttpRequest) → HttpResponse
┌──────────────────────────▼───────────────────────────────────┐
│  web-core  (WEB00)                                             │
│  Router · HookRegistry · WebApp · WebServer                    │
│  All 12 lifecycle hook points                                  │
└──────────────────────────────────────────────────────────────┘
```

**Rust owns:**
- TCP accept loop (kqueue/epoll/IOCP)
- HTTP/1 request parsing and response serialization
- Route pattern matching (`:param`, `*wildcard`)
- Hook dispatch ordering and lifecycle
- GIL management (release for blocking I/O, acquire for Python calls)

**Python owns:**
- `@app.get(...)`, `@app.post(...)`, etc. — decorator DSL
- Handler functions: receive a `HandlerContext` as first argument
- Before/after filters, not_found, error_handler registration
- `ctx.json()`, `ctx.html()`, `ctx.text()`, `ctx.halt()`, `ctx.redirect()`
- `HaltException` — raised by response helpers to short-circuit; never seen by Rust
- Request body parsing: `ctx.request.json()`, `ctx.request.form()`
- `app.settings` dict

## GIL Threading Protocol

Python's GIL (Global Interpreter Lock) prevents parallel Python execution.
The Rust web-core `serve()` call blocks forever — if it held the GIL, Python
could not run any other threads (including signal handlers for Ctrl+C).

The solution mirrors Ruby's GVL protocol exactly:

```
Thread A (main Python thread)          Rust (web-core event loop)
─────────────────────────────          ──────────────────────────
server.serve()                         ...
  → conduit_native.server_serve()      ...
    → PyEval_SaveThread()   ────────── GIL released ──────────────
    → web_server.serve()               accept loop starts
                                       HTTP request arrives
                                       route matched (route_index = 2)
                                       dispatch_route_to_python(owner, 2, req)
                                         → PyGILState_Ensure()  ← acquire GIL
                                         → call owner.native_dispatch_route(2, env)
                                         → parse [s,h,b] or None
                                         → PyGILState_Release()
                                       return WebResponse
    → PyEval_RestoreThread() ───────── GIL restored ────────────
```

### Inline GIL declarations

`python-bridge` does not include GIL management. We declare them inline in
`ext/conduit_native/src/lib.rs`:

```rust
extern "C" {
    fn PyEval_SaveThread() -> *mut c_void;      // release GIL; returns ThreadState
    fn PyEval_RestoreThread(state: *mut c_void); // re-acquire GIL from state
    fn PyGILState_Ensure() -> i32;              // acquire GIL from any OS thread
    fn PyGILState_Release(state: i32);          // release GIL back
}
```

The `PyEval_SaveThread` / `PyEval_RestoreThread` pair is used exactly once —
around `web_server.serve()`. The `PyGILState_Ensure` / `PyGILState_Release`
pair wraps every Python callout (route, before, after, not_found, error).

## Python callable protocol

All route handlers, before/after filters, not_found, and error handlers are
stored as `PyObjectPtr` (Python callable objects). The Rust side holds an
`Arc<PyCallable>` wrapper around each pointer. `PyCallable` implements
`Send + Sync` (safe because we only call it while holding the GIL) and calls
`Py_DecRef` in its `Drop` implementation.

To call a Python dispatch method on the owner (NativeServer):
```rust
PyObject_CallMethodObjArgs(owner, method_name_obj, arg1, arg2, NULL)
```

Python dispatch methods on `NativeServer` (Python class, mirrors Ruby server.rb):
- `native_dispatch_route(index: int, env: dict) -> Optional[list]`
- `native_run_before_filters(env: dict) -> Optional[list]`
- `native_run_after_filters(env: dict, response: list) -> list`
- `native_run_not_found(env: dict) -> Optional[list]`
- `native_run_error_handler(env: dict, error: str) -> Optional[list]`

Each returns `None` (no override) or `[status, [[name,val],...], body]`.

## Response protocol (same for all languages)

```
Python handler returns None      → no short-circuit (fall through)
Python handler raises HaltException → caught by NativeServer dispatch methods
                                       → converted to [s, h, b] before returning to Rust
Python handler returns [s, h, b] → Rust parses and uses this response
```

`HaltException` is pure Python — Rust never sees it. This keeps the Rust layer
completely language-agnostic.

## env dict keys

The `env` dict passed to every Python dispatch method mirrors the Ruby hash:

| Key                      | Type     | Example                        |
|--------------------------|----------|--------------------------------|
| `"REQUEST_METHOD"`       | str      | `"GET"`                        |
| `"PATH_INFO"`            | str      | `"/hello/world"`               |
| `"QUERY_STRING"`         | str      | `"page=1&sort=asc"`            |
| `"SERVER_PROTOCOL"`      | str      | `"HTTP/1.1"`                   |
| `"REMOTE_ADDR"`          | str      | `"127.0.0.1"`                  |
| `"REMOTE_PORT"`          | int      | `54321`                        |
| `"SERVER_NAME"`          | str      | `"127.0.0.1"`                  |
| `"SERVER_PORT"`          | int      | `3000`                         |
| `"conduit.route_params"` | dict[str,str] | `{"name": "Alice"}`       |
| `"conduit.query_params"` | dict[str,str] | `{"page": "1"}`           |
| `"conduit.headers"`      | dict[str,str] | `{"content-type": "..."}` |
| `"conduit.body"`         | str      | `'{"ping":"pong"}'`            |
| `"conduit.content_type"` | str?     | `"application/json"`           |
| `"conduit.content_length"` | int?   | `15`                           |

## Package layout

```
code/packages/python/conduit/
├── BUILD
├── CHANGELOG.md
├── README.md
├── pyproject.toml
├── ext/conduit_native/
│   ├── Cargo.toml
│   └── src/lib.rs
└── src/coding_adventures/conduit/
    ├── __init__.py         # exports: Conduit, HaltException
    ├── application.py      # Conduit class (DSL decorators, routes, filters)
    ├── halt_exception.py   # HaltException(status, body, headers)
    ├── handler_context.py  # HandlerContext: json/html/text/halt/redirect, delegates to Request
    ├── request.py          # Request: params, query_params, headers, body, json(), form()
    └── server.py           # NativeServer: capsule wrapper + Python dispatch methods

code/programs/python/conduit-hello/
├── BUILD
├── CHANGELOG.md
├── README.md
├── hello.py               # 8-route demo
└── tests/
    └── test_conduit_hello.py
```

## Python DSL details

### Conduit class (`application.py`)

```python
class Conduit:
    def __init__(self): ...
    def get(self, pattern): ...      # decorator
    def post(self, pattern): ...     # decorator
    def put(self, pattern): ...      # decorator
    def patch(self, pattern): ...    # decorator
    def delete(self, pattern): ...   # decorator
    def head(self, pattern): ...     # decorator
    def options(self, pattern): ...  # decorator
    def before_request(self, fn): ...  # decorator
    def after_request(self, fn): ...   # decorator
    def not_found(self, fn): ...       # decorator
    def error_handler(self, fn): ...   # decorator
    def serve(self, host, port, max_connections=1024): ...  # start server
```

`routes` is a list of `Route(method, pattern, handler)` namedtuples.

### HandlerContext (`handler_context.py`)

The context object passed to every handler as the first (and only) argument.
Delegates unknown attribute lookups to the wrapped `Request` via `__getattr__`,
so `ctx.path` and `ctx.params` work directly.

```python
class HandlerContext:
    def __init__(self, request): ...
    def json(self, data, status=200): raise HaltException(status, json.dumps(data), ...)
    def html(self, content, status=200): raise HaltException(status, content, ...)
    def text(self, content, status=200): raise HaltException(status, content, ...)
    def halt(self, status, body="", headers=None): raise HaltException(status, body, headers)
    def redirect(self, url, status=302): raise HaltException(status, "", {"location": url})
    def __getattr__(self, name): return getattr(self.request, name)
```

### Request (`request.py`)

```python
class Request:
    def __init__(self, env): ...
    @property
    def method(self): ...       # str: "GET", "POST", ...
    @property
    def path(self): ...         # str: "/hello/world"
    @property
    def query_string(self): ... # str: "page=1"
    @property
    def params(self): ...       # dict[str, str]: route named params
    @property
    def query_params(self): ... # dict[str, str]: query string params
    @property
    def headers(self): ...      # dict[str, str]: lowercase header names
    @property
    def body(self): ...         # str: raw request body
    def json(self): ...         # parse body as JSON, raises HaltException(400) on bad JSON
    def form(self): ...         # parse body as URL-encoded form data → dict[str, str]
    def header(self, name): ... # get a header by name (case-insensitive)
```

### HaltException (`halt_exception.py`)

```python
class HaltException(Exception):
    def __init__(self, status: int, body: str = "", headers: dict | None = None):
        self.status = int(status)
        self.body = str(body)
        self.halt_headers = list(headers.items() if isinstance(headers, dict) else (headers or []))
```

### NativeServer (`server.py`)

Wraps the PyCapsule from `conduit_native.server_new(...)`. Implements the Python-side
dispatch methods that Rust calls back into. This is the mirror of Ruby's `server.rb`.

```python
class NativeServer:
    def __init__(self, app, host, port, max_connections=1024):
        self._app = app
        self._capsule = conduit_native.server_new(self, app, host, port, max_connections)

    def serve(self): conduit_native.server_serve(self._capsule)
    def stop(self): conduit_native.server_stop(self._capsule)
    def running(self): return conduit_native.server_running(self._capsule)
    def local_host(self): return conduit_native.server_local_host(self._capsule)
    def local_port(self): return conduit_native.server_local_port(self._capsule)

    def native_dispatch_route(self, index: int, env: dict): ...
    def native_run_before_filters(self, env: dict): ...
    def native_run_after_filters(self, env: dict, response: list): ...
    def native_run_not_found(self, env: dict): ...
    def native_run_error_handler(self, env: dict, error: str): ...
```

## Divergences from the plan

None currently — this spec is the plan.

## Hook-firing order

Identical to Ruby (WEB02):
1. `before_routing` — fires before route lookup for ALL requests (including 404)
2. Route handler — fires only if a route matched
3. `after_handler` — fires after the route handler for matched routes only
4. `on_not_found` — fires when no route matched (overrides default 404)

## Security notes

- **Open redirect**: `ctx.redirect()` does not validate the URL. If derived from
  user input, callers must validate it is a relative path or trusted origin to
  prevent CWE-601 (open redirect).
- **JSON body parsing**: Returns `HaltException(400)` on invalid JSON rather than
  leaking a raw exception message.
- **Content-type header**: Handlers should always set an explicit content-type.
  The helpers (`json`, `html`, `text`) do this automatically.
