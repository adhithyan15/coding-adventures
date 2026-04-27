"""
conduit-hello — Flask-like demo for the Python Conduit framework.

Exercises every Phase 3 (WEB03) DSL feature:
  - Before/after filters
  - Response helpers: json, html, halt, redirect
  - Custom not_found handler
  - Custom error_handler
  - Settings dict

## Routes

| Method | Path           | What it shows                                  |
|--------|----------------|------------------------------------------------|
| GET    | /              | html helper — greeting page                    |
| GET    | /hello/<name>  | json helper — {"message": "Hello <name>"}      |
| POST   | /echo          | ctx.request.json() — echoes JSON body back     |
| GET    | /redirect      | redirect helper — 301 to /                     |
| GET    | /halt          | halt helper — 403 Forbidden                    |
| GET    | /down          | Blocked by before filter — 503 maintenance     |
| GET    | /error         | Raises intentionally — custom error handler    |
| ANY    | /*             | Custom not_found handler — 404 HTML            |

## Stack

    hello.py  (this file — ~80 lines of Python)
        ↓
    coding_adventures.conduit  (Flask-like DSL)
        ↓
    conduit_native  (Rust extension — routing, GIL management, web-core)
        ↓
    web-core  (WebApp, WebServer, Router, HookRegistry, 12 lifecycle hooks)
        ↓
    embeddable-http-server → tcp-runtime → kqueue / epoll / IOCP
"""

import html as _html

from coding_adventures.conduit import Conduit

app = Conduit()
app.settings["app_name"] = "Conduit Hello"


# ── Before filter ────────────────────────────────────────────────────────────
# Runs for EVERY request before route lookup — including paths with no matching
# route. This means /down returns 503 even though it has no registered route.


@app.before_request
def maintenance(ctx):
    if ctx.path == "/down":
        ctx.halt(503, "Under maintenance")


# ── After filter ─────────────────────────────────────────────────────────────
# Runs after every matched route. Used here as a request logger.


@app.after_request
def logger(ctx):
    print(f"[after] {ctx.method} {ctx.path}")


# ── Routes ───────────────────────────────────────────────────────────────────


@app.get("/")
def index(ctx):
    # html.escape prevents XSS if app_name ever comes from external config.
    safe_name = _html.escape(str(app.settings["app_name"]))
    ctx.html(
        f"<h1>Hello from {safe_name}!</h1>"
        "<p>Try <a href='/hello/World'>/hello/World</a></p>"
    )


@app.get("/hello/<name>")
def hello(ctx):
    ctx.json({"message": f"Hello {ctx.params['name']}", "app": app.settings["app_name"]})


@app.post("/echo")
def echo(ctx):
    ctx.json(ctx.request.json())


@app.get("/redirect")
def do_redirect(ctx):
    ctx.redirect("/", 301)


@app.get("/halt")
def do_halt(ctx):
    ctx.halt(403, "Forbidden")


@app.get("/error")
def do_error(ctx):
    # Raises intentionally to exercise the error handler.
    raise RuntimeError("intentional error")


# ── Custom not_found handler ──────────────────────────────────────────────────


@app.not_found
def on_not_found(ctx):
    # html.escape prevents reflected XSS from a crafted URL path.
    safe_path = _html.escape(ctx.path)
    ctx.html(f"<h1>Not Found: {safe_path}</h1>", 404)


# ── Custom error handler ──────────────────────────────────────────────────────


@app.error_handler
def on_error(ctx, err):
    # Log full error detail server-side; never expose internals to the client.
    print(f"[error] {ctx.method} {ctx.path}: {err}", flush=True)
    ctx.json({"error": "Internal Server Error"}, 500)


# ── Entry point ───────────────────────────────────────────────────────────────

if __name__ == "__main__":
    print(f"Starting {app.settings['app_name']}...")
    app.serve(host="127.0.0.1", port=3000)
