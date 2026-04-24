# coding-adventures-conduit

Flask-like Python web framework backed by the Rust `web-core` HTTP engine.
Route, TCP, and HTTP/1 processing all live in Rust; Python owns handler logic
and DSL sugar.

```python
from coding_adventures.conduit import Conduit

app = Conduit()

@app.before_request
def auth(ctx):
    if ctx.path == "/admin" and not ctx.header("x-api-key"):
        ctx.halt(401, "Unauthorized")

@app.get("/")
def index(ctx):
    ctx.html("<h1>Hello from Conduit!</h1>")

@app.get("/hello/<name>")
def hello(ctx):
    ctx.json({"message": f"Hello {ctx.params['name']}"})

@app.post("/echo")
def echo(ctx):
    ctx.json(ctx.request.json())

@app.not_found
def on_not_found(ctx):
    ctx.html(f"<h1>Not Found: {ctx.path}</h1>", 404)

@app.error_handler
def on_error(ctx, err):
    ctx.json({"error": "Internal Server Error"}, 500)

if __name__ == "__main__":
    app.serve(port=3000)
```

## How it fits in the stack

```
your_app.py  (Flask-like decorators — ~50 lines)
    ↓
coding_adventures.conduit  (Conduit DSL: decorators, HandlerContext, HaltException)
    ↓
conduit_native  (Rust cdylib — routing, hook dispatch, GIL management via web-core)
    ↓
web-core  (WebApp, WebServer, Router, HookRegistry, 12 lifecycle hooks)
    ↓
embeddable-http-server → tcp-runtime → kqueue / epoll / IOCP
```

Routing lives entirely in Rust. For each request, Rust dispatches the matched
route index back to Python with a pre-built `env` dict. Python executes the
handler and returns `[status, headers, body]` — or `None` to fall through.

## GIL threading model

`server.serve()` releases the Python GIL before blocking on the Rust event
loop (`PyEval_SaveThread`). Each inbound request re-acquires the GIL via
`PyGILState_Ensure` before calling back into Python, then releases it
afterwards. This means the Python signal handler for Ctrl-C works correctly.

## Route patterns

Routes use Flask-style `<param>` syntax, converted internally to the Rust
web-core `:param` format:

| Flask pattern            | Rust pattern           |
|--------------------------|------------------------|
| `/hello/<name>`          | `/hello/:name`         |
| `/users/<id>/posts/<n>`  | `/users/:id/posts/:n`  |

## Response helpers

All helpers raise `HaltException` internally to exit the handler immediately.
They do **not** return — use them as the last statement (no `return` needed).

| Helper                           | Content-Type            | Default status |
|----------------------------------|-------------------------|----------------|
| `ctx.json(data)`                 | application/json        | 200            |
| `ctx.html(content)`              | text/html               | 200            |
| `ctx.text(content)`              | text/plain              | 200            |
| `ctx.halt(status, body, headers)`| (caller sets headers)   | —              |
| `ctx.redirect(url, status=302)`  | (sets Location header)  | 302            |

## Request access

`HandlerContext` (`ctx`) delegates attribute access to the underlying
`Request` object, so both styles work:

```python
# Direct on ctx (delegate style):
ctx.path          # str — URL path
ctx.method        # str — "GET", "POST", …
ctx.params        # dict — route named params
ctx.query_params  # dict — parsed query string
ctx.headers       # dict — lowercase header names
ctx.body          # str — raw request body

# Explicit via ctx.request:
ctx.request.json()   # parse body as JSON (raises HaltException(400) on error)
ctx.request.form()   # parse body as URL-encoded form data
ctx.request.header("content-type")  # single header lookup
```

## Filters and hooks

### Before filters

```python
@app.before_request
def filter_fn(ctx):
    # Runs for EVERY request, including unmatched paths.
    # Call ctx.halt() or another helper to short-circuit.
    pass
```

### After filters

```python
@app.after_request
def log_fn(ctx):
    # Runs after every matched route handler (side effects only).
    # Any HaltException raised here is silently swallowed.
    print(f"[after] {ctx.method} {ctx.path}")
```

### Not-found handler

```python
@app.not_found
def missing_fn(ctx):
    ctx.html(f"<h1>Not Found: {ctx.path}</h1>", 404)
```

### Error handler

```python
@app.error_handler
def error_fn(ctx, err: str):
    ctx.json({"error": "Internal Server Error", "detail": err}, 500)
```

## Settings

```python
app.settings["app_name"] = "My App"
print(app.settings["app_name"])
```

## Security notes

- `ctx.redirect()` does **not** validate the URL. If the URL is derived from
  user input (e.g. a `return_to` parameter), validate it before calling to
  prevent open redirects (CWE-601).
- `ctx.request.json()` raises `HaltException(400)` on invalid JSON rather than
  leaking raw parser exceptions.

## Running

```sh
python hello.py
```

```sh
curl http://localhost:3000/
curl http://localhost:3000/hello/Alice
curl -X POST http://localhost:3000/echo -H 'Content-Type: application/json' -d '{"ping":"pong"}'
curl -i http://localhost:3000/redirect
curl http://localhost:3000/halt
curl http://localhost:3000/down
curl http://localhost:3000/error
curl http://localhost:3000/missing
```
