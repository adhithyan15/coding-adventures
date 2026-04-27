# coding_adventures_conduit

A Sinatra/Express-inspired web framework for Elixir, backed by the Rust
`web-core` engine via a custom Erlang NIF.

This is the Elixir port (WEB06) in the Conduit series — same DSL surface
as the Ruby (WEB02), Python (WEB03), Lua (WEB04), and TypeScript (WEB05)
ports.

## Where it sits in the stack

```
Elixir DSL (Conduit.Application, .Server, .HandlerContext, .Request)
    ↓  GenServer messages
Conduit.Dispatcher (single gen_server, handles all requests serially)
    ↓  Conduit.Native NIF calls
conduit_native (Rust cdylib)  ←——  enif_send  —┐
    ↓                                          │
web-core (WebApp, WebServer)                   │
    ↓                                          │
embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
                                               │
                       ↑ Rust I/O thread sends ┘
                         {:conduit_request, slot_id, handler_id, env_map}
                         then blocks on a Condvar until Native.respond/2
                         signals the slot.
```

## Quick example

```elixir
alias CodingAdventures.Conduit
alias CodingAdventures.Conduit.{Application, Server}
import CodingAdventures.Conduit.HandlerContext

app =
  Application.new()
  |> Application.before_filter(fn req ->
       if req.path == "/down", do: halt(503, "Maintenance")
     end)
  |> Application.get("/", fn _req ->
       html("<h1>Hello from Conduit!</h1>")
     end)
  |> Application.get("/hello/:name", fn req ->
       json(%{message: "Hello " <> req.params["name"]})
     end)
  |> Application.post("/echo", fn req ->
       json(CodingAdventures.Conduit.Request.json_body!(req))
     end)
  |> Application.not_found_handler(fn req ->
       html("<h1>Not Found: #{req.path}</h1>", 404)
     end)
  |> Application.error_handler(fn _req ->
       json(%{error: "Internal Server Error"}, 500)
     end)

{:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 3000)
Server.serve(server)
```

## DSL reference

| Function | Effect |
|----------|--------|
| `Application.new/0` | Empty app |
| `Application.get(app, "/path", fun)` (also `post`, `put`, `delete`, `patch`) | Route |
| `Application.before_filter(app, fun)` | Filter, registration order |
| `Application.after_filter(app, fun)` | Filter, runs on responses |
| `Application.not_found_handler(app, fun)` | Catch-all 404 handler |
| `Application.error_handler(app, fun)` | Runs on uncaught exceptions |
| `Application.put_setting(app, key, value)` | Settings storage |
| `html(body, status \\ 200)` | `text/html; charset=utf-8` response |
| `json(value, status \\ 200)` | JSON response (Elixir 1.18+ `JSON`) |
| `text(body, status \\ 200)` | `text/plain; charset=utf-8` response |
| `respond(status, body, headers \\ %{})` | Custom response |
| `halt(status, body \\ "", headers \\ %{})` | `throw` Sinatra-style |
| `redirect(location, status \\ 302)` | `Location:` header + status |

## Handler return shape

A handler must return:

- `{status_int, headers_map, body_binary}` — concrete response
- `nil` (or anything not matching the tuple shape) — pass through

Throw a `{:conduit_halt, ...}` to short-circuit before/after the route handler.

## Threading model in 30 seconds

BEAM has no global lock — multiple schedulers run on multiple OS threads.
Naively calling Elixir functions from a Rust I/O thread is impossible:
NIFs can only `enif_send` messages to processes. So Conduit:

1. Marks `server_serve/1` as a **dirty I/O NIF** so it can block on the
   accept loop without starving the BEAM scheduler.
2. Each Rust I/O thread `enif_send`s `{:conduit_request, slot_id, hid, env}`
   to a single `Conduit.Dispatcher` GenServer, then blocks on a Condvar.
3. The dispatcher runs the handler and calls `Native.respond(slot_id, resp)`
   (a regular fast NIF) which signals the Condvar.
4. The Rust I/O thread wakes up and writes the HTTP response.

There's a **30-second handler timeout** — if the dispatcher is wedged,
the request returns `500 — handler timeout` rather than parking the
Rust thread forever.

## Why a Single Dispatcher (not One Process per Request)?

Simplicity, mostly. A single dispatcher matches the user mental model
"filters run in registration order" and doesn't churn through process
creation per request. For high-throughput shards or per-request worker
processes, see WEB07 — Conduit OTP, the pure-OTP reimplementation.

## Building

```sh
# In this package directory:
sh BUILD     # cargo build, copy .so, mix test
```

The BUILD file calls `cargo build --release` in `native/conduit_native/`,
copies the resulting `.so`/`.dylib` into `priv/conduit_native.so`, then
runs `mix compile` and `mix test --cover`.

## Status

This is WEB06 of the Conduit series. WEB07 (Conduit OTP — pure OTP, no
Rust) is a follow-up PR for learning OTP supervision trees from scratch.
