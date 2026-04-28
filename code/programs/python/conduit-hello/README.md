# conduit-hello (Python)

A full Flask-like demo built on `coding-adventures-conduit`. Exercises every
WEB03 DSL feature: before/after filters, response helpers (`json`, `html`,
`halt`, `redirect`), custom not-found and error handlers, and settings.

## Routes

| Method | Path           | What it shows                                     |
|--------|----------------|---------------------------------------------------|
| `GET`  | `/`            | `ctx.html` — HTML greeting page                   |
| `GET`  | `/hello/<name>`| `ctx.json` — `{ message: "Hello <name>" }`        |
| `POST` | `/echo`        | `ctx.request.json()` — echoes the JSON body back  |
| `GET`  | `/redirect`    | `ctx.redirect` — 301 to `/`                       |
| `GET`  | `/halt`        | `ctx.halt` — 403 Forbidden unconditionally        |
| `GET`  | `/down`        | Blocked by `@before_request` filter — 503         |
| `GET`  | `/error`       | Raises intentionally — `@error_handler` → 500     |
| `GET`  | `/*`           | `@not_found` handler — 404 HTML with the path     |

## Features demonstrated

**`@app.before_request`** — runs for every request before route lookup
(including unmatched paths). Blocks `/down` with 503 "Under maintenance".

**`@app.after_request`** — runs after every matched route. Logs
`[after] METHOD PATH` to stdout as a side effect.

**Response helpers** — `ctx.html`, `ctx.json`, `ctx.halt`, `ctx.redirect`
all raise `HaltException` internally to short-circuit the handler. Python
catches it; Rust never sees it.

**`@app.not_found`** — overrides the default 404 with a styled HTML page.

**`@app.error_handler`** — catches any Python exception raised in a route
handler and returns structured JSON with the error detail.

**`app.settings`** — `app.settings["app_name"] = "Conduit Hello"` stored in
a dict; read at startup to print the server banner.

## How it fits in the stack

```
hello.py  (you are here — ~80 lines of Python)
    ↓
coding_adventures.conduit  (Flask-like DSL: Conduit, HandlerContext, HaltException)
    ↓
conduit_native  (Rust extension — routing, hook dispatch, GIL management via web-core)
    ↓
web-core  (WebApp, WebServer, Router, HookRegistry, 12 lifecycle hooks)
    ↓
embeddable-http-server → tcp-runtime → kqueue / epoll / IOCP
```

## Running

```sh
python hello.py
```

Then try:

```sh
curl http://localhost:3000/
curl http://localhost:3000/hello/Adhithya
curl -X POST http://localhost:3000/echo -H 'Content-Type: application/json' -d '{"ping":"pong"}'
curl -i http://localhost:3000/redirect
curl http://localhost:3000/halt
curl http://localhost:3000/down
curl http://localhost:3000/error
curl http://localhost:3000/missing
```

Press `Ctrl-C` to stop.
