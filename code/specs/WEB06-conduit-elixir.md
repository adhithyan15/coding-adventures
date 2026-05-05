# WEB06 — Elixir Conduit (Rust NIF port)

## Overview

An Elixir port of the Conduit web framework backed by the same Rust `web-core`
engine used by the Ruby (WEB02), Python (WEB03), Lua (WEB04), and TypeScript
(WEB05) ports. Handlers are plain Elixir functions; routing, lifecycle hooks,
and HTTP I/O run in Rust. The Erlang VM (BEAM) loads the Rust NIF library at
boot time and the cdylib resolves the BEAM's `enif_*` symbols dynamically.

This is the "straight port" — same DSL surface, same protocol, same engine.
A separate spec (WEB07 — Elixir Conduit OTP) will reimplement the framework
on pure OTP primitives for learning purposes.

---

## Architecture

```
Elixir DSL (lib/coding_adventures/conduit/*.ex)
    — Application, Request, HandlerContext, HaltError, Server, Native
    ↓  response protocol: nil (no override) | {status, headers, body}
conduit_native (Rust cdylib, native/conduit_native/)
    — nif_init exporting NIFs, route/hook registration, dispatcher slot table
    ↓  enif_send across BEAM threads, enif_alloc_env per request
web-core (WebApp, WebServer, HookRegistry, Router)
    ↓
embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
```

### The threading puzzle

BEAM is fundamentally different from CPython/Lua/Ruby. It runs **multiple
schedulers on multiple OS threads**, with **isolated process heaps** and
**no global lock**. NIFs run on whichever scheduler thread happened to call
them. Two constraints:

1. **Long-running NIFs starve the scheduler.** A NIF must complete in
   ≤ 1 ms or use `ERL_NIF_DIRTY_JOB_IO_BOUND` to run on the dirty I/O
   thread pool instead. Our `serve/1` function blocks the calling thread
   for the lifetime of the server, so it is mandatory to mark it dirty.

2. **You cannot call Elixir functions directly from a NIF.** Unlike
   Lua's `lua_pcall` or Python's `PyObject_Call`, BEAM does not expose a
   "synchronously call this fun" entry point from C. The only way to
   trigger Elixir code execution is to **send a message** to an Elixir
   process and let the BEAM scheduler pick it up.

### Solution: send + slot-table dispatch

We use a hybrid pattern, conceptually identical to Node.js's
`napi_threadsafe_function` from WEB05 but built from BEAM primitives:

```
                     ┌─────────────────────────────────────────────┐
                     │         BEAM main scheduler thread          │
                     │  Conduit.Dispatcher (gen_server)            │
                     │  ──────────────────────────────────         │
                     │  receives {:request, slot_id, env_map}      │
                     │  looks up handler, runs it                  │
                     │  calls Native.respond(slot_id, result)      │
                     └─────────────────────────▲───────────────────┘
                                               │ message via enif_send
                                               │
                     ┌─────────────────────────┴───────────────────┐
                     │   Rust I/O thread (web-core)                │
                     │  ─────────────────────────────────────────  │
                     │  request arrives                            │
                     │  allocate slot { mutex, condvar, response } │
                     │  enif_alloc_env, build env_map term         │
                     │  enif_send(NULL, dispatcher_pid, env, msg)  │
                     │  block on condvar with 30s timeout          │
                     │  read response from slot, send HTTP reply   │
                     └─────────────────────────────────────────────┘
```

The Rust side maintains a `RwLock<HashMap<u64, Arc<Slot>>>` keyed by
monotonically increasing slot IDs. When the Elixir dispatcher finishes
processing a request, it calls `Conduit.Native.respond(slot_id, response)`
(a regular fast NIF) which:

1. Looks up the slot by ID
2. Locks the slot's `Mutex<Option<WebResponse>>`
3. Writes the response
4. Signals the slot's `Condvar`
5. Removes the entry from the table

The Rust I/O thread, blocked on the condvar, wakes up, takes the response,
and returns it to web-core which writes the HTTP response on the wire.

This pattern is intentionally identical in structure to WEB05's TSFN
dispatch, just using BEAM's `enif_send` instead of N-API's threadsafe
function queue.

### Why a single dispatcher process and not one-per-request?

We use a single `gen_server` (`Conduit.Dispatcher`) that holds all the
registered routes/filters/handlers and processes incoming `{:request, …}`
messages serially. This is simple, safe, and matches the user's mental
model of Sinatra/Express where handlers run "after each other".

For higher throughput, an upgrade path is documented in the spec:
multiple dispatcher processes running under a `PartitionSupervisor`,
sharded by slot ID. We do not implement this in WEB06 — Elixir Conduit OTP
(WEB07) will explore this design space.

### Why not a `Task.async`/`spawn` per request?

`Task.async` works, but each spawn allocates a new BEAM process (~2 KiB)
and adds GC pressure under load. The single-dispatcher model also makes
it trivial to demonstrate `:before_routing` filters running in registration
order, since everything is serialized through one mailbox. WEB07 will use
true per-request worker processes and contrast the two models.

---

## Package layout

```
code/packages/elixir/conduit/
├── BUILD
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── mix.exs
├── required_capabilities.json
├── native/conduit_native/
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/lib.rs                         # Rust NIF cdylib
├── lib/coding_adventures/conduit/
│   ├── application.ex                     # routes/filters/settings
│   ├── dispatcher.ex                      # gen_server: route lookup + handler invocation
│   ├── halt_error.ex                      # exception struct + halt/redirect helpers
│   ├── handler_context.ex                 # html/json/text/respond helpers
│   ├── native.ex                          # NIF wrapper (load_nif + stubs)
│   ├── request.ex                         # Request struct from env map
│   └── server.ex                          # Server orchestration (start/stop)
├── lib/coding_adventures/conduit.ex       # umbrella module re-exporting public API
└── test/
    ├── application_test.exs
    ├── halt_error_test.exs
    ├── handler_context_test.exs
    ├── request_test.exs
    └── server_test.exs                    # E2E via real TCP + httpc
```

---

## Elixir DSL

```elixir
alias CodingAdventures.Conduit
alias CodingAdventures.Conduit.{Application, Server}
import CodingAdventures.Conduit.HandlerContext, only: [html: 1, json: 1, json: 2, halt: 2, redirect: 1]

app =
  Application.new()
  |> Application.before_filter(fn req ->
       if req.path == "/down", do: halt(503, "Under maintenance")
     end)
  |> Application.get("/", fn _req ->
       html("<h1>Hello from Conduit!</h1>")
     end)
  |> Application.get("/hello/:name", fn req ->
       json(%{message: "Hello #{req.params["name"]}"})
     end)
  |> Application.post("/echo", fn req ->
       json(Request.json_body!(req))
     end)
  |> Application.not_found_handler(fn req ->
       html("<h1>Not Found: #{req.path}</h1>", 404)
     end)
  |> Application.error_handler(fn _req, _err ->
       json(%{error: "Internal Server Error"}, 500)
     end)
  |> Application.put_setting(:app_name, "Conduit Hello")

{:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 3000)
Server.serve(server)   # blocks until Server.stop(server)
```

### Why an immutable `Application` struct (not macros)?

Other Elixir web frameworks (Plug, Phoenix) use compile-time macros (`get
"/" do ... end`). Macros are powerful but obscure for someone learning the
framework — the DSL becomes a separate language to debug. Conduit Elixir
uses **runtime data structures**: `Application.new() |> Application.get(…)`
returns a struct containing route lists. This makes the framework's
behavior trivially inspectable (`IO.inspect(app)`) and matches the Ruby /
Python / Lua / TypeScript ports.

### Response helpers

| Helper | Returns |
|--------|---------|
| `html(body, status \\ 200)` | `{status, %{"content-type" => "text/html"}, body}` |
| `json(value, status \\ 200)` | `{status, %{"content-type" => "application/json"}, JSON.encode!(value)}` |
| `text(body, status \\ 200)` | `{status, %{"content-type" => "text/plain"}, body}` |
| `respond(status, body, headers \\ %{})` | `{status, headers, body}` |
| `halt(status, body \\ "", headers \\ %{})` | raises `HaltError` (via `throw`) |
| `redirect(location, status \\ 302)` | raises `HaltError` with `Location: …` |

JSON encoding uses Elixir 1.18+'s built-in `JSON` module (no `jason`
dependency, matching the repo's zero-deps philosophy).

---

## Request struct

```elixir
defmodule CodingAdventures.Conduit.Request do
  defstruct [
    :env,            # raw env map from Rust
    :method,         # "GET", "POST", ...
    :path,           # "/hello/world"
    :query_string,   # "foo=bar"
    :params,         # %{"name" => "world"}
    :query_params,   # %{"foo" => "bar"}
    :headers,        # %{"content-type" => "application/json"} (lowercase keys)
    :body,           # raw body string
    :content_type,
    :content_length
  ]

  def from_env(env), do: %__MODULE__{...}
  def json_body!(%__MODULE__{body: body}), do: JSON.decode!(body)
end
```

---

## HaltError protocol

```elixir
defmodule CodingAdventures.Conduit.HaltError do
  defexception [:status, :body, :headers]

  @impl true
  def message(%{status: s, body: b}), do: "halt(#{s}, #{inspect(b)})"
end

def halt(status, body \\ "", headers \\ %{}) do
  throw {:conduit_halt, status, body, headers}
end
```

Why `throw` and not `raise`? On BEAM, `throw` is **cheap** (no stacktrace
collection) and is the idiomatic mechanism for non-local return / control
flow. Sinatra-style halts are exactly this — flow control, not errors.
The dispatcher uses `try ... catch :throw, {:conduit_halt, _, _, _} -> ...`
to convert thrown halts into responses. `raise` is reserved for genuine
unhandled errors that route to the user-supplied `error_handler`.

---

## Env map keys (mirrors all other ports)

```
"REQUEST_METHOD"          => "GET"
"PATH_INFO"               => "/hello/world"
"QUERY_STRING"            => "foo=bar"
"REMOTE_ADDR"             => "127.0.0.1"
"REMOTE_PORT"             => "54321"
"conduit.route_params"    => %{"name" => "world"}     # Elixir map, not JSON string
"conduit.query_params"    => %{"foo" => "bar"}
"conduit.headers"         => %{"content-type" => "application/json"}
"conduit.body"            => "{\"ping\":\"pong\"}"
"conduit.content_type"    => "application/json"
"conduit.content_length"  => "14"
"conduit.error"           => "RuntimeError: something bad"   # only for error handler
```

Unlike the TypeScript port (where maps are JSON-encoded strings because
N-API is awkward with object enumeration), the Elixir port passes Elixir
maps directly using `enif_make_map_*` calls. This is faster and natural.

---

## Rust NIF surface

The cdylib registers the following NIFs (all called via the
`Conduit.Native` Elixir module):

| NIF | Arity | Returns | Dirty? |
|-----|-------|---------|--------|
| `new_app/0` | 0 | `app_resource` | no |
| `app_add_route/4` | 4 (`app, method, pattern, handler_id`) | `:ok` | no |
| `app_add_before/2` | 2 (`app, handler_id`) | `:ok` | no |
| `app_add_after/2` | 2 | `:ok` | no |
| `app_set_not_found/2` | 2 | `:ok` | no |
| `app_set_error_handler/2` | 2 | `:ok` | no |
| `app_set_setting/3` | 3 (`app, key, value`) | `:ok` | no |
| `app_get_setting/2` | 2 | `value | nil` | no |
| `new_server/4` | 4 (`app, host, port, max_conn`) | `server_resource` | no |
| `server_serve/2` | 2 (`server, dispatcher_pid`) | `:ok` | **dirty I/O** |
| `server_serve_background/2` | 2 | `:ok` | no |
| `server_stop/1` | 1 | `:ok` | no |
| `server_local_port/1` | 1 | `integer` | no |
| `server_running/1` | 1 | `boolean` | no |
| `respond/2` | 2 (`slot_id, response_tuple`) | `:ok` | no |

### Handler IDs (not function refs)

BEAM atoms and refs are tied to a specific BEAM process; you cannot
"keep" an Elixir function reference in Rust across calls because the BEAM
GC may reclaim it. Instead, the `Conduit.Application` struct holds
handlers in an Elixir map keyed by integer IDs. The Rust side stores only
the ID; when a request arrives it includes the ID in the message, and the
dispatcher process looks up the function by ID.

### Resources

`new_app/0` returns a BEAM resource wrapping `Box<Mutex<NativeApp>>`.
`new_server/4` consumes the app resource and returns a server resource
wrapping `Box<Mutex<NativeServer>>`. The destructors (registered with
`enif_open_resource_type`) call `stop_handle.stop()` and join the
background thread before dropping the inner value — same finalize ordering
as WEB05's `finalize_server`.

---

## Required additions to `erl-nif-bridge`

Existing erl-nif-bridge covers atoms, integers, binaries, lists, tuples,
and resources. WEB06 needs three new symbol groups:

### 1. Maps

```rust
extern "C" {
    pub fn enif_make_new_map(env: ErlNifEnv) -> ERL_NIF_TERM;
    pub fn enif_make_map_put(env: ErlNifEnv, map_in: ERL_NIF_TERM,
                              key: ERL_NIF_TERM, value: ERL_NIF_TERM,
                              map_out: *mut ERL_NIF_TERM) -> c_int;
    pub fn enif_get_map_value(env: ErlNifEnv, map: ERL_NIF_TERM,
                               key: ERL_NIF_TERM, value: *mut ERL_NIF_TERM) -> c_int;
    pub fn enif_get_map_size(env: ErlNifEnv, map: ERL_NIF_TERM,
                              size: *mut usize) -> c_int;
    pub fn enif_map_iterator_create(env: ErlNifEnv, map: ERL_NIF_TERM,
                                     iter: *mut ErlNifMapIterator,
                                     entry: c_int) -> c_int;
    pub fn enif_map_iterator_destroy(env: ErlNifEnv, iter: *mut ErlNifMapIterator);
    pub fn enif_map_iterator_get_pair(env: ErlNifEnv, iter: *mut ErlNifMapIterator,
                                       key: *mut ERL_NIF_TERM,
                                       value: *mut ERL_NIF_TERM) -> c_int;
    pub fn enif_map_iterator_next(env: ErlNifEnv, iter: *mut ErlNifMapIterator) -> c_int;
}
```

Plus safe wrappers `make_map`, `map_get`, `map_iter_next`.

### 2. Pids and send

```rust
#[repr(C)]
pub struct ErlNifPid { pub pid: ERL_NIF_TERM }

extern "C" {
    pub fn enif_self(env: ErlNifEnv, pid: *mut ErlNifPid) -> *mut ErlNifPid;
    pub fn enif_get_local_pid(env: ErlNifEnv, term: ERL_NIF_TERM,
                               pid: *mut ErlNifPid) -> c_int;
    pub fn enif_make_pid(env: ErlNifEnv, pid: *const ErlNifPid) -> ERL_NIF_TERM;
    pub fn enif_send(caller_env: ErlNifEnv, to_pid: *const ErlNifPid,
                      msg_env: ErlNifEnv, msg: ERL_NIF_TERM) -> c_int;
}
```

`enif_send` with `caller_env = NULL` is the **off-scheduler-thread**
variant — explicitly the thing we need from a Rust I/O thread.

### 3. Long-running env (already declared, just used)

`enif_alloc_env` and `enif_free_env` are already in the bridge.
Each Rust I/O thread allocates one msg env per request, builds the env
map term inside it, sends it, and frees it after the dispatcher has
copied the term into its own scheduler env (which `enif_send` does
implicitly).

---

## BUILD

```shell
# Build the Rust NIF shared library.
cd native/conduit_native && cargo build --release

# Copy the platform-appropriate library into priv/ as conduit_native.so
mkdir -p priv && { \
  cp native/conduit_native/target/release/libconduit_native.dylib priv/conduit_native.so 2>/dev/null || \
  cp native/conduit_native/target/release/libconduit_native.so    priv/conduit_native.so; }

# Compile Elixir code and run tests.
mix deps.get --quiet && mix compile && mix test --cover
```

The same chicken-and-egg lesson (2026-04-04 lessons.md) applies: do not
add `:make` to `mix.exs` `compilers` — Mix tries to load
`Mix.Tasks.Compile.Make` before `elixir_make` is compiled. The BUILD
file does the cargo build; mix.exs leaves `compilers` at its default.

---

## Tests (target: 40+)

### test/halt_error_test.exs (~6 tests)
- `HaltError` exception has correct fields
- `halt/1` throws `{:conduit_halt, status, "", %{}}`
- `halt/2` throws with custom body
- `halt/3` throws with headers
- `redirect/1` throws with default 302 + Location
- `redirect/2` throws with explicit status

### test/handler_context_test.exs (~10 tests)
- `html/1`, `html/2` shapes
- `json/1`, `json/2` encodes value, sets content-type
- `text/1`, `text/2` shapes
- `respond/3` shapes
- `halt/2` matches HaltError tests

### test/request_test.exs (~10 tests)
- `Request.from_env/1` populates all fields from env map
- `params`, `query_params`, `headers` default to empty maps
- `json_body!/1` decodes JSON, raises on invalid
- `content_length` is integer

### test/application_test.exs (~10 tests)
- `Application.new/0` returns empty struct
- `Application.get/3`, `post/3`, `put/3`, `delete/3`, `patch/3` register routes
- `before_filter/2` and `after_filter/2` register filter lists
- `not_found_handler/2`, `error_handler/2` set single handlers
- `put_setting/3` and `get_setting/2` round-trip
- All chainable (return updated struct)

### test/server_test.exs (~12 tests, E2E)
For each: start server with `Server.start_link` + `Server.serve_background`,
fire HTTP via `:httpc.request/4`, then `Server.stop`.
- GET / returns 200 HTML
- GET /ping returns plain text "pong"
- GET /health returns JSON
- GET /hello/:name captures route param
- POST /echo echoes JSON body
- DELETE /items/:id captures route param
- before filter halt(503) short-circuits
- redirect returns 302 with Location
- not_found returns 404 with custom body
- error handler catches RuntimeError and returns JSON
- query params surfaced via `req.query_params`
- `Server.local_port/1` reports the bound port
- `Server.running?/1` toggles correctly

---

## Demo program

`code/programs/elixir/conduit-hello/` — same 8-route demo:
`/`, `/hello/:name`, `/echo`, `/redirect`, `/halt`, `/down`, `/error`,
custom `not_found` and `error_handler`. Plus 15+ ExUnit integration tests
mirroring WEB05's `conduit-hello` test suite.

---

## Future work tracked outside this spec

- **WEB07**: Conduit OTP — pure-OTP reimplementation (no Rust)
- **WEB09**: Conduit Perl — straightforward port matching Lua/TypeScript pattern
- **Long-term**: A small BEAM-like VM in Rust (lightweight processes,
  mailboxes, supervisors) backing all language ports — would let us run
  the OTP supervision-tree model in Python, Ruby, Lua, etc. Deferred
  indefinitely; not a pre-requisite for any of WEB06–WEB08.
