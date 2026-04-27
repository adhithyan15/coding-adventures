# coding-adventures-conduit

A Sinatra/Express-inspired web framework for Node.js, powered by the
**web-core** Rust engine.  Part of the Conduit series — ports of the same
framework to Ruby, Lua, TypeScript, Elixir, and Perl.

## What it is

Conduit gives you an Express-like DSL for defining routes and middleware,
but the HTTP engine running underneath is the same high-performance Rust
core used by the Ruby and Lua ports.  TypeScript route handlers run on the
V8 main thread; the Rust side handles I/O on a background thread and
communicates with JS via N-API threadsafe functions.

```
TypeScript route handlers (V8 main thread)
         ↕  napi_threadsafe_function
  Rust background thread  (web-core WebServer)
         ↕
  kqueue / epoll / IOCP  (tcp-runtime)
```

## Quick start

```typescript
import { Application, Server, html, json, halt, redirect } from "coding-adventures-conduit";

const app = new Application();

// Before filter — runs on every request before routing.
app.before((req) => {
  if (req.path === "/down") halt(503, "Under maintenance");
});

// Routes
app.get("/", () => html("<h1>Hello from Conduit!</h1>"));

app.get("/hello/:name", (req) =>
  json({ message: `Hello ${req.params.name}!` })
);

app.post("/echo", (req) => {
  const body = req.json<{ ping: string }>();
  return json({ pong: body.ping });
});

app.get("/redirect", () => redirect("/"));

// Catch-all handlers
app.notFound((req) => html(`Not Found: ${req.path}`, 404));
app.onError((_req, err) => json({ error: err }, 500));

// Bind and serve
const server = new Server(app, { host: "127.0.0.1", port: 3000 });
server.serve();
```

## Installation

This package is not yet published to npm.  Install from the monorepo by
running the BUILD script:

```bash
cd code/packages/typescript/conduit
./BUILD   # builds Rust cdylib, copies .node, npm ci, tsc, vitest
```

## API

### `Application`

| Method | Description |
|--------|-------------|
| `get(pattern, handler)` | Register a GET route |
| `post(pattern, handler)` | Register a POST route |
| `put(pattern, handler)` | Register a PUT route |
| `delete(pattern, handler)` | Register a DELETE route |
| `patch(pattern, handler)` | Register a PATCH route |
| `before(handler)` | Register a before-request filter |
| `after(handler)` | Register an after-response filter |
| `notFound(handler)` | Custom 404 handler |
| `onError(handler)` | Custom error handler |
| `set(key, value)` | Store a setting |
| `getSetting(key)` | Read a setting |

All methods return `this` for chaining.

### `Request`

| Property | Type | Description |
|----------|------|-------------|
| `method` | `string` | HTTP method (`"GET"`, `"POST"`, …) |
| `path` | `string` | Request path (`"/hello/world"`) |
| `queryString` | `string` | Raw query string (without `?`) |
| `params` | `Record<string,string>` | Named route captures |
| `queryParams` | `Record<string,string>` | Parsed query parameters |
| `headers` | `Record<string,string>` | Lowercased request headers |
| `bodyText` | `string` | Raw request body |
| `contentType` | `string` | `Content-Type` header value |
| `contentLength` | `number` | `Content-Length` as a number |

| Method | Description |
|--------|-------------|
| `json<T>()` | Parse body as JSON (cached) |
| `form()` | Parse body as `application/x-www-form-urlencoded` (cached) |

### Response helpers

| Function | Description |
|----------|-------------|
| `html(body, status?)` | `text/html` response |
| `json(value, status?)` | `application/json` response (serialises with `JSON.stringify`) |
| `text(body, status?)` | `text/plain` response |
| `respond(status, body, headers?)` | Full control |
| `halt(status, body?, headers?)` | Throw a `HaltError` to short-circuit |
| `redirect(location, status?)` | Throw a `HaltError` with `Location` header (default 302) |

### `Server`

```typescript
const server = new Server(app, { host: "127.0.0.1", port: 3000 });

server.serve();           // blocking — keeps event loop alive
server.serveBackground(); // non-blocking — for tests
server.stop();            // signal shutdown
server.localPort;         // OS-assigned port (useful when port: 0)
server.running;           // true while background thread is active
```

## Route patterns

Pattern syntax is the same as Sinatra and the Rust `web-core` router:

| Pattern | Matches | `req.params` |
|---------|---------|--------------|
| `/hello/:name` | `/hello/Alice` | `{ name: "Alice" }` |
| `/files/*` | `/files/a/b/c` | (unnamed wildcard) |
| `/health` | `/health` | `{}` |

## HaltError protocol

`halt()` and `redirect()` throw a `HaltError`.  The Rust cdylib catches it
(via `napi_is_exception_pending` + `napi_get_and_clear_last_exception`) and
converts it to a `WebResponse` using the `__conduit_halt`, `status`, `body`,
and `haltHeaderPairs` fields.

Unhandled exceptions (not `HaltError`) are caught and routed to `onError()`.

## Threading model

```
Request arrives on OS thread (kqueue/epoll/IOCP)
    → Rust background thread calls napi_call_threadsafe_function
    → JS callback executes on V8 main thread
    → JS returns [status, headers, body] (or throws HaltError)
    → Condvar signals the background thread
    → Rust sends HTTP response
```

`napi_ref_threadsafe_function` keeps the Node.js event loop alive while the
server is running.  `server.stop()` triggers `napi_unref_threadsafe_function`
on all TsFns and signals the background thread to exit, allowing the event
loop to drain naturally.

## Architecture overview

```
coding-adventures-conduit  (this package)
├── src/
│   ├── index.ts          ← public API surface
│   ├── application.ts    ← route + filter registry
│   ├── halt_error.ts     ← HaltError, halt(), redirect()
│   ├── handler_context.ts← html(), json(), text(), respond()
│   ├── request.ts        ← Request wrapper
│   └── server.ts         ← Server (loads native module, drives web-core)
└── ext/conduit_native_node/  ← Rust N-API cdylib
    ├── Cargo.toml
    ├── build.rs
    └── src/lib.rs        ← napi_register_module_v1, TSFN dispatch, etc.

Rust deps (monorepo):
  node-bridge → N-API bindings
  web-core    → WebApp, Router, HookRegistry
  embeddable-http-server → WebServer
  tcp-runtime → kqueue/epoll/IOCP event loop
```

## Tests

```bash
npx vitest run               # all tests
npx vitest run --reporter=verbose
```

94+ unit tests across five files:
- `halt_error.test.ts` — HaltError, halt(), redirect()
- `request.test.ts` — Request parsing, json(), form()
- `handler_context.test.ts` — response builders
- `application.test.ts` — DSL registration
- `server.test.ts` — 20 E2E tests via real TCP (requires compiled .node)

## Related packages

| Package | Language | Status |
|---------|----------|--------|
| `coding_adventures/conduit` (Ruby gem) | Ruby | ✅ merged |
| `coding_adventures.conduit` (Python) | Python | ✅ merged |
| `coding_adventures.conduit` (Lua) | Lua | ✅ in review |
| `coding-adventures-conduit` (this) | TypeScript/Node.js | 🚧 WEB05 |
| `CodingAdventures.Conduit` (Elixir) | Elixir | planned WEB06 |
| `CodingAdventures::Conduit` (Perl) | Perl | planned WEB07 |
