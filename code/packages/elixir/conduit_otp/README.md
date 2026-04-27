# conduit_otp — WEB07

A pure-OTP Elixir reimplementation of the **Conduit** web framework. No Rust, no
NIFs, no external dependencies — every byte of the HTTP server lives in Elixir/Erlang.

## Why this exists

WEB06 (Conduit NIF) is the *practical* port: fast, delegating I/O to a Rust cdylib.
WEB07 is the *educational* port: slower on raw throughput, but every OTP concept
(`Application`, `Supervisor`, `DynamicSupervisor`, `gen_server`, `Agent`) is on display
with literate comments and worked examples.

Read the source top-to-bottom and you get a free OTP tutorial grounded in real
working code.

## Quick start

```elixir
alias CodingAdventures.ConduitOtp.{Application, Server}
import CodingAdventures.ConduitOtp.HandlerContext

app =
  Application.new()
  |> Application.before_filter(fn req ->
       if req.path == "/down", do: halt(503, "Maintenance")
     end)
  |> Application.get("/", fn _req ->
       html("<h1>Hello from Conduit OTP!</h1>")
     end)
  |> Application.get("/hello/:name", fn req ->
       json(%{message: "Hello " <> req.params["name"]})
     end)
  |> Application.post("/echo", fn req ->
       {200, %{"content-type" => req.content_type}, req.body}
     end)
  |> Application.not_found_handler(fn req ->
       html("<h1>Not Found: #{req.path}</h1>", 404)
     end)
  |> Application.error_handler(fn _req ->
       json(%{error: "Internal Server Error"}, 500)
     end)

{:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 3000)
Server.serve(server)   # blocks until Ctrl-C
```

To switch from WEB06 to WEB07, change one alias:

```elixir
# WEB06 (Rust NIF):
alias CodingAdventures.Conduit.{Application, Server}

# WEB07 (pure OTP):
alias CodingAdventures.ConduitOtp.{Application, Server}
```

## Architecture

```
OtpSupervisor  (:one_for_one, max_restarts 5 in 10s)
├─ RouteTable        (Agent — holds the Application struct)
├─ WorkerSupervisor  (DynamicSupervisor — spawns per-connection workers)
└─ Acceptor          (GenServer — owns the listen socket)
     └─ Worker × N   (:temporary — one per active HTTP connection)
```

Each module includes a "Teaching topic:" section at the top. Read them in order:

| # | File | OTP concept taught |
|---|------|--------------------|
| 1 | `otp_application.ex` | The `Application` behaviour; mix.exs hook |
| 2 | `otp_supervisor.ex` | Supervisor strategies; restart budget |
| 3 | `acceptor.ex` | gen_server; passive sockets; send-to-self loop |
| 4 | `worker_supervisor.ex` | DynamicSupervisor; `:temporary` restart |
| 5 | `worker.ex` | gen_server lifecycle; cooperative looping |
| 6 | `http_parser.ex` | `:erlang.decode_packet/3`; HTTP framing |
| 7 | `route_table.ex` | `Agent`; named processes; hot reload |
| 8 | `router.ex` | Pure functions; no process; named captures |
| 9 | `handler_context.ex` | `throw` for non-local control flow |
| 10 | `request.ex` | Plain struct; immutable view |
| 11 | `application.ex` | Functional combinators |
| 12 | `server.ex` | Public façade; ties everything together |

## Request / Response API

### Request struct fields

| Field | Type | Description |
|-------|------|-------------|
| `method` | `String.t()` | `"GET"`, `"POST"`, etc. |
| `path` | `String.t()` | `/hello/world` (no query string) |
| `query_string` | `String.t()` | `foo=bar` (no leading `?`) |
| `params` | `map` | Named route captures: `%{"name" => "Alice"}` |
| `query_params` | `map` | Parsed query string |
| `headers` | `map` | Lower-case header names |
| `body` | `binary` | Raw request body |
| `content_type` | `String.t()` | Value of `content-type` header |
| `content_length` | `integer` | Parsed `content-length` |

### Handler return values

Handlers return a `{status, headers, body}` 3-tuple:

```elixir
{200, %{"content-type" => "text/plain"}, "OK"}
```

Or use the helpers from `HandlerContext`:

```elixir
import CodingAdventures.ConduitOtp.HandlerContext

html("<h1>Hello</h1>")          # {200, %{"content-type" => "text/html..."}, ...}
json(%{ok: true})                # {200, %{"content-type" => "application/json"}, ...}
text("plain text")               # {200, %{"content-type" => "text/plain..."}, ...}
respond(204, "")                 # {204, %{}, ""}

halt(404, "Not found")           # throws — short-circuits dispatch
redirect("/login")               # throws — 302 redirect
```

## Running tests

```sh
cd code/packages/elixir/conduit_otp
mise exec -- mix deps.get
mise exec -- mix test --cover
```

148 tests, 80% coverage.

## Out of scope

- HTTP/2, HTTP/3 — HTTP/1.1 only.
- TLS — TCP only. Drop-in: swap `:gen_tcp` → `:ssl` in `Acceptor`.
- WebSockets — out of scope.
- Performance — use WEB06 for production workloads.
