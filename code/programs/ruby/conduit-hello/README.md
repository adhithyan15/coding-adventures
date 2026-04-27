# conduit-hello

A full Sinatra-style demo built on `conduit`. Exercises every Phase 3 DSL
feature: before/after filters, response helpers (`json`, `html`, `halt`,
`redirect`), custom not-found and error handlers, and settings.

## Routes

| Method | Path | What it shows |
|--------|------|---------------|
| `GET` | `/` | `html` helper — returns an HTML greeting page |
| `GET` | `/hello/:name` | `json` helper — returns `{ message: "Hello <name>" }` |
| `POST` | `/echo` | `request.json` — echoes the JSON body back |
| `GET` | `/redirect` | `redirect` helper — 301 to `/` |
| `GET` | `/halt` | `halt` helper — 403 Forbidden unconditionally |
| `GET` | `/down` | Blocked by `before` filter — returns 503 before routing |
| `GET` | `/error` | Raises intentionally — custom `error` handler returns 500 JSON |
| `GET` | `/*` | Custom `not_found` handler — 404 HTML with the path |

## Features demonstrated

**Before filter** — runs for every request before route lookup (including
unmatched paths). Blocks `/down` with 503 "Under maintenance".

**After filter** — runs after every matched route. Logs `[after] METHOD PATH`
to stdout as a side effect.

**Response helpers** — `html`, `json`, `halt`, `redirect` all raise `HaltError`
internally to short-circuit the handler pipeline. Ruby catches it; Rust never
sees it.

**Custom not-found handler** — overrides the default 404 with a styled HTML page.

**Custom error handler** — catches any Ruby exception raised in a route handler
and returns structured JSON with the error detail.

**Settings** — `set :app_name, "Conduit Hello"` stored in `app.settings`; read
at startup to print the server banner.

## How it fits in the stack

```
hello.rb  (you are here — ~95 lines of Ruby)
    ↓
coding_adventures_conduit  (Sinatra-style DSL: Application, HandlerContext, HaltError)
    ↓
conduit_native  (Rust extension — routing, hook dispatch, GVL management via web-core)
    ↓
web-core  (WebApp, WebServer, Router, HookRegistry, 12 lifecycle hooks)
    ↓
embeddable-http-server → tcp-runtime → kqueue / epoll / IOCP
```

Routing lives entirely in Rust. For each request, Rust dispatches the matched
route index back to Ruby with a pre-built env hash. Ruby executes the handler
block and returns `[status, headers, body]` — or `nil` to fall through.

## Running

```sh
ruby hello.rb
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
