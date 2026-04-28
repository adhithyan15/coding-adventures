# WEB07 — Conduit OTP (pure-OTP reimplementation)

## Overview

A second Elixir port of the Conduit web framework — this time built **purely
on OTP primitives**, with **no Rust, no NIFs, no `web-core`** dependency at
all. The whole HTTP server, from `:gen_tcp.listen/2` to wire-format parsing
to per-request worker processes, lives in Elixir.

Same DSL surface as WEB06 (Elixir Conduit NIF port) and the other ports —
identical user-facing API, swap one `import` line for a direct comparison.
But underneath, every "letter of OTP" is on display: `Application`,
`Supervisor`, `DynamicSupervisor`, `gen_server`, links, monitors, "let it
crash" semantics, and supervision trees.

This spec doubles as a **teaching document**. Every OTP concept has a
named section with diagrams, tables, and inline rationale. Reading the
implementation top-to-bottom should be enough to learn OTP from scratch.

---

## Why this exists

WEB06 is the *practical* Elixir port — fast (Rust I/O), production-shaped,
delegating to `web-core`. WEB07 is the *educational* one — slower (pure
BEAM I/O), simpler in scope but richer in OTP idioms. The two sit in the
repo side-by-side so:

1. You can compare a request lifecycle in Rust-vs-OTP and see exactly
   where the abstractions differ.
2. Teaching material (this spec, the inline literate comments, BENCH.md)
   is a long-form OTP tutorial grounded in real working code, not toy
   examples.
3. The "tiny BEAM-like VM" idea (eventually backing all language ports)
   has a concrete Elixir reference: every primitive we'd want to port to
   Python/Ruby/Lua is right here, with the simplest possible
   implementation.

---

## What is OTP, really?

OTP = Open Telecom Platform. Originally built at Ericsson in the 1990s to
run AXD 301 telephone switches that needed nine-nines (99.9999999%)
uptime. The same library now ships with every Erlang/Elixir install. It
sits **on top of** the BEAM virtual machine and gives you three things:

### 1. The actor model on BEAM (this is what OTP builds on)

Erlang processes are not OS threads, not green threads, but BEAM
processes — tiny user-space concurrency units. Each starts at ~2 KiB of
memory; you can spin up a million on a laptop. Properties:

- **Isolated heaps.** Each process has its own GC'd heap. There is **no
  shared mutable state** between processes — by construction, not by
  convention.
- **Message-passing only.** Processes communicate by sending immutable
  copies of terms via `send/2` to a target's mailbox; the receiver pulls
  via `receive` with pattern matching.
- **Preemptive scheduling.** BEAM's scheduler interrupts processes after
  ~2000 reductions (roughly: function calls). One runaway process cannot
  starve the others. This is the secret to BEAM's soft-realtime
  guarantees.
- **Links and monitors.** Process A can `link` to B; if either dies
  abnormally, the other receives a death signal (and, by default, dies
  too). `monitor` is one-way (A watches B without linking back). These
  are the primitive that supervisors are built from.

### 2. The "Let it crash" philosophy

Most languages: `try { ... } catch (e) { ... handle every possible error ... }`.

OTP: **don't** handle errors in the worker. Let it crash. Have a
*supervisor* notice and restart it from a known-good state. The
reasoning: if you got into a bad state, the cleanest reset is a fresh
process.

| Defensive code | OTP code |
|----------------|----------|
| Try every operation, log every error, hope you covered every branch | Trust the happy path; let unexpected states crash; supervisor restarts |
| Hidden retry logic in dozens of catch blocks | Centralized restart strategy in one supervisor spec |
| Bugs survive in stale state | Bugs cause a quick crash → fresh state → user retries → success |

### 3. The OTP behaviours (the "design patterns")

A *behaviour* is essentially an interface plus a battle-tested
implementation. You write the callbacks; OTP runs the loop. You'll see
these constantly:

| Behaviour | What it is | Web framework analogue |
|-----------|-----------|------------------------|
| **`gen_server`** | A long-lived process holding state, serving sync `call`s and async `cast`s | A long-lived service object |
| **`gen_statem`** | A `gen_server` whose state IS the state of a finite-state machine | Workflow engine, parser |
| **`supervisor`** | A process whose only job is to start, monitor, and restart child processes per a strategy | systemd, but in-process |
| **`application`** | A bundle: code + config + a top-level supervisor. Start it, a whole tree comes up | A Rails / Phoenix app |

Plus `Task` (fire-and-forget async work), `Registry` (named-process
lookup), `Agent` (simplified state-only gen_server), and a few others.

### Why OTP for a web server?

Because "an HTTP server" is *exactly* what supervision trees were
designed for:

- A single `Acceptor` socket can crash (port closes, file-descriptor
  exhaustion, OS-level fault). Restart it.
- A worker process handling a malformed request can crash. Don't bring
  down the rest of the server. Restart? No — workers are transient;
  let them die and the next request gets a new one.
- A bug in your route handler (uncaught exception, divide-by-zero) is
  isolated to a single worker process. The TCP socket survives. The
  next request works.

This is **failure isolation by construction**, encoded in the tree
shape:

```
                        ConduitOtp.Application
                                 │
                        ConduitOtp.Supervisor          ← :one_for_one,
                  ┌──────────────┼────────────────┐      max 5 restarts in 10s
                  │              │                │
            RouteTable      Acceptor         WorkerSupervisor
            (Agent)         (gen_server)     (DynamicSupervisor)
                                                    │
                            ┌───────────────────────┴─┬──────────┐
                          Worker1                  Worker2     Worker3
                       (one per active HTTP connection — transient)
```

A `Worker2` crash → DynamicSupervisor reaps it; nothing else affected.

A `RouteTable` crash → its supervisor restarts it; brief downtime; users
see 5xx for ~milliseconds.

A persistent crash that exceeds the supervisor's restart budget (5 in
10 s by default) → escalates: the `Supervisor` itself dies, taking the
`Application` down. The runtime then either restarts the application or
exits cleanly (depending on `restart` config).

---

## Architecture

```
                                       :gen_tcp listen socket
                                                ↑
ConduitOtp.Application                 (raw bytes, OS kernel buffers)
  └─ ConduitOtp.Supervisor             :one_for_one, intensity 5, period 10s
       ├─ ConduitOtp.RouteTable        Agent: holds compiled routes from Application struct
       ├─ ConduitOtp.Acceptor          gen_server: owns listen socket; accepts in a loop
       └─ ConduitOtp.WorkerSupervisor  DynamicSupervisor: restart=:temporary
            └─ Worker × N              gen_server, one per accepted connection
```

### Per-request lifecycle

1. **Acceptor** owns the listen socket from `:gen_tcp.listen/2`. It
   spawns a long-lived child `Worker` *waiting* on the socket via
   `:gen_tcp.accept/2`. As soon as a connection lands, the Worker
   becomes "ours" for that connection.
2. The Acceptor immediately spawns a *new* waiting Worker — the loop is
   "always have ≥ 1 acceptor-task ready" (this is the standard
   Cowboy/Bandit pattern; ours is simpler — one waiting at a time).
3. The Worker calls `:gen_tcp.recv/2,3` (or uses `{:active, :once}`) to
   read the HTTP/1.1 request bytes.
4. Parses request line + headers + body via `:erlang.decode_packet/3`
   (Erlang's built-in HTTP/1.1 packet decoder — runtime-provided, no
   dependency).
5. Looks up a route in the RouteTable (an `Agent` holding `%{routes: …,
   …}` — same `Application` struct shape as WEB06 / WEB05).
6. Calls the user handler, catching `{:conduit_halt, …}` throws.
7. Encodes the response status line + headers + body as bytes,
   `:gen_tcp.send/2`s them.
8. Honours `Connection: close` vs `Connection: keep-alive`. Default for
   HTTP/1.1: keep-alive; the Worker loops back to step 3 for the next
   request on the same connection.
9. On socket close or error, the Worker exits normally
   (`{:stop, :normal, …}`); DynamicSupervisor reaps it.

### Why `DynamicSupervisor` and not just `spawn`?

Three reasons:

1. **Restart budget bookkeeping.** Even though Workers are
   `restart: :temporary` (never restarted on crash — a dead HTTP
   connection has nothing to restart), the supervisor still tracks how
   many crashed in the last interval. A flood of malformed requests
   triggering crashes will be visible.
2. **Graceful shutdown.** When the application stops, the supervisor
   sends `:shutdown` to each child and waits up to `shutdown` ms for
   them to exit. Bare `spawn`'d processes get killed without warning.
3. **Idiomatic OTP.** Per OTP design rules, no process should be
   "outside" the supervision tree. Workers under a DynamicSupervisor are
   in the tree, contributing to runtime introspection
   (`:observer.start/0` shows them).

---

## DSL — identical to WEB06

```elixir
alias CodingAdventures.ConduitOtp
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
Server.serve(server)
```

To switch from the NIF version to the OTP version, change the alias.
That's it. The DSL is *exactly* the same.

---

## Package layout

```
code/packages/elixir/conduit_otp/
├── BUILD                                     # mix deps.get + mix test --cover
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── mix.exs
├── required_capabilities.json                # ["elixir"] only — no rust/cargo
├── lib/coding_adventures/conduit_otp/
│   ├── application.ex                        # immutable Application struct (DSL)
│   ├── halt_error.ex                         # throw {:conduit_halt, …}
│   ├── handler_context.ex                    # html/json/text/respond/halt/redirect
│   ├── http_parser.ex                        # :erlang.decode_packet/3 wrapper
│   ├── otp_application.ex                    # the OTP `Application` behaviour
│   ├── otp_supervisor.ex                     # supervision root
│   ├── acceptor.ex                           # gen_server: owns listen socket
│   ├── worker.ex                             # gen_server: handles one connection
│   ├── worker_supervisor.ex                  # DynamicSupervisor of Workers
│   ├── route_table.ex                        # Agent: holds compiled Application
│   ├── router.ex                             # path-pattern matcher (pure)
│   ├── request.ex                            # Request struct from parsed env
│   └── server.ex                             # public Server API (start_link/serve/stop)
├── lib/coding_adventures/conduit_otp.ex      # umbrella module
└── test/
    ├── application_test.exs
    ├── halt_error_test.exs
    ├── handler_context_test.exs
    ├── http_parser_test.exs
    ├── request_test.exs
    ├── router_test.exs
    ├── route_table_test.exs
    └── server_test.exs                       # E2E via :httpc
```

---

## Implementation: each module is a teaching unit

### `OtpApplication` — the OTP `Application` behaviour

Implements:

```elixir
defmodule CodingAdventures.ConduitOtp.OtpApplication do
  use Application

  @impl true
  def start(_type, _args) do
    # Empty top-level supervisor by default. The framework is started
    # *per server instance* via Server.start_link/2 because a library
    # shouldn't bind to a port at app-load time.
    children = []
    Supervisor.start_link(children, strategy: :one_for_one,
      name: __MODULE__.RootSupervisor)
  end
end
```

**Teaching points:**
- The `Application` behaviour is the unit of "thing the BEAM can
  start/stop together" — declared in `mix.exs`'s `application/0`.
- `start/2` returns `{:ok, pid}` of the top supervisor.
- The application gets started automatically when the BEAM loads the
  `.app` file (e.g. when a depending library does `Application.ensure_all_started/1`).

### `OtpSupervisor` — supervision root for a Server instance

Spawned by `Server.start_link/2`:

```elixir
def start_link(opts) do
  Supervisor.start_link(__MODULE__, opts, name: name_for(opts))
end

@impl true
def init(opts) do
  children = [
    {RouteTable, opts},
    {WorkerSupervisor, []},
    {Acceptor, opts}
  ]
  Supervisor.init(children, strategy: :one_for_one,
    max_restarts: 5, max_seconds: 10)
end
```

**Teaching points:**

| Strategy | What it does | When to use |
|----------|-------------|-------------|
| `:one_for_one` | If a child dies, restart only that one. Siblings unaffected. | Independent services (our case). |
| `:one_for_all` | If any child dies, restart all of them. | Tightly coupled siblings; e.g. a connection pool + cache that must agree on state. |
| `:rest_for_one` | If a child dies, restart it AND all children defined after it. | Sibling N+1 depends on N's state being fresh. |

The `max_restarts: 5, max_seconds: 10` is the **restart budget**: if
the supervisor restarts children more than 5 times in any 10-second
window, it gives up and exits itself, propagating the failure to its
own parent. Stops infinite-restart loops on a permanently-broken
child.

### `Acceptor` — owns the listen socket

```elixir
defmodule CodingAdventures.ConduitOtp.Acceptor do
  use GenServer

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)

  @impl true
  def init(opts) do
    host = Keyword.fetch!(opts, :host) |> String.to_charlist()
    port = Keyword.fetch!(opts, :port)

    case :gen_tcp.listen(port, [
           :binary,
           {:packet, :http_bin},   # use BEAM's built-in HTTP/1.1 framing
           {:active, false},        # passive mode — we recv explicitly
           {:reuseaddr, true},
           {:ip, parse_ip(host)}
         ]) do
      {:ok, lsock} ->
        send(self(), :accept)       # kick off accept loop
        {:ok, %{lsock: lsock, opts: opts}}

      {:error, reason} ->
        {:stop, {:listen_failed, reason}}
    end
  end

  # Async accept loop: every time we accept a connection, hand it to a
  # Worker (started under WorkerSupervisor) and re-arm.
  @impl true
  def handle_info(:accept, %{lsock: lsock} = state) do
    {:ok, sock} = :gen_tcp.accept(lsock)        # blocks
    {:ok, _pid} = WorkerSupervisor.start_worker(sock, RouteTable.snapshot())
    send(self(), :accept)
    {:noreply, state}
  end

  @impl true
  def terminate(_reason, %{lsock: lsock}), do: :gen_tcp.close(lsock)
end
```

**Teaching points:**
- `init/1` returns `{:ok, state}` — state goes into every subsequent
  callback's last argument.
- `handle_info/2` handles non-`call`/`cast` messages — including `send/2`
  to self, OS process exits, etc.
- `terminate/2` is the gen_server's "destructor" — runs when the
  process is shutting down cleanly (NOT on a crash by default; you'd
  need `Process.flag(:trap_exit, true)`).
- The `send(self(), :accept)` pattern turns a blocking call (`accept/1`)
  into a non-blocking-loop — each iteration is a separate scheduler
  reduction, so other processes get fair time.

### `WorkerSupervisor` — DynamicSupervisor of per-request workers

```elixir
defmodule CodingAdventures.ConduitOtp.WorkerSupervisor do
  use DynamicSupervisor

  def start_link(_), do: DynamicSupervisor.start_link(__MODULE__, [], name: __MODULE__)

  @impl true
  def init(_), do: DynamicSupervisor.init(strategy: :one_for_one)

  def start_worker(socket, route_snapshot) do
    spec = %{
      id:       Worker,
      start:    {Worker, :start_link, [socket, route_snapshot]},
      restart:  :temporary,    # ← key insight: never restart a dead connection
      shutdown: 5_000,
      type:     :worker
    }
    DynamicSupervisor.start_child(__MODULE__, spec)
  end
end
```

**Teaching points:**
- `:transient` would restart on abnormal exit; `:permanent` always
  restarts; `:temporary` never restarts. We pick `:temporary` because a
  socket-bound Worker has no value after its socket is gone.
- `DynamicSupervisor` is the right choice when child specs are decided
  *at runtime* (one new Worker per accepted connection). A static
  `Supervisor` with a hard-coded child list won't work.

### `Worker` — handles one HTTP/1.1 connection

```elixir
defmodule CodingAdventures.ConduitOtp.Worker do
  use GenServer

  alias CodingAdventures.ConduitOtp.{HttpParser, Router, Request, HaltError, HandlerContext}

  def start_link(socket, route_snapshot) do
    GenServer.start_link(__MODULE__, {socket, route_snapshot})
  end

  @impl true
  def init({socket, route_snapshot}) do
    # Hand off socket ownership from acceptor to us, then process.
    :gen_tcp.controlling_process(socket, self())
    send(self(), :process)
    {:ok, %{socket: socket, app: route_snapshot}}
  end

  @impl true
  def handle_info(:process, state) do
    case HttpParser.read_request(state.socket) do
      {:ok, {method, path, headers, body}} ->
        env = build_env(method, path, headers, body, state.socket)
        response = run_handler(env, state.app)
        :ok = send_response(state.socket, response)

        if keep_alive?(headers) do
          send(self(), :process)             # loop for next request
          {:noreply, state}
        else
          :gen_tcp.close(state.socket)
          {:stop, :normal, state}
        end

      {:error, _reason} ->
        :gen_tcp.close(state.socket)
        {:stop, :normal, state}
    end
  end
end
```

**Teaching points:**
- `:gen_tcp.controlling_process/2` is critical — in BEAM, only one
  process can `recv` from a socket at a time. The Acceptor `accept/1`s
  the connection (transferring "ownership" to it briefly), then
  immediately hands ownership to the Worker.
- The `send(self(), :process)` loop pattern again — each iteration
  is one mailbox round-trip, allowing the BEAM to interleave with
  other Workers. This is *cooperative concurrency at the OTP level*.
- A crash in `run_handler` will not destroy the socket (handlers run
  in a `try/catch/rescue` triad just like WEB06's Dispatcher) — but if
  the Worker process itself crashes (e.g. parser bug), the
  `DynamicSupervisor` reaps it and the connection just dies. The
  client gets a TCP RST and retries. The framework keeps running.

### `HttpParser` — `:erlang.decode_packet/3` wrapper

We use BEAM's runtime-provided HTTP/1.1 packet decoder. Setting
`packet: :http_bin` on the socket means each `recv` returns a
high-level structured term:

| Decoded form | Meaning |
|--------------|---------|
| `{:http_request, method, {:abs_path, path}, {1,1}}` | Request line |
| `{:http_header, _, name, _, value}` | One header |
| `:http_eoh`                        | End of headers |
| Raw binary                         | Body |

The parser module wraps `recv` calls in a small state machine:
read request line → loop reading headers → switch to `packet: :raw`
→ read body of `Content-Length` bytes.

**Teaching point:** `:erlang.decode_packet/3` (and the equivalent
socket option) is part of the BEAM runtime. Erlang specifically
shipped HTTP-aware packet decoding so tools like its built-in HTTP
client could be implemented in pure Erlang. We get to leverage it
for free.

### `RouteTable` — `Agent` for the compiled Application

```elixir
defmodule CodingAdventures.ConduitOtp.RouteTable do
  use Agent

  def start_link(opts) do
    app = Keyword.fetch!(opts, :app)
    Agent.start_link(fn -> app end, name: __MODULE__)
  end

  def snapshot, do: Agent.get(__MODULE__, & &1)

  def hot_reload(new_app), do: Agent.update(__MODULE__, fn _ -> new_app end)
end
```

**Teaching points:**
- `Agent` is the simplest OTP behaviour: a process holding a single
  state cell. Just `get` and `update`.
- `hot_reload/1` is interesting: at any moment you can swap the
  Application struct (new routes, new filters) and **the next request
  uses them** — without restarting the server. This is what Ericsson
  built OTP for. (The Worker gets a snapshot at start, so in-flight
  requests use the old version.)

### Router — pure path matcher

```elixir
defmodule CodingAdventures.ConduitOtp.Router do
  def match(routes, method, path) do
    Enum.find_value(routes, fn %{method: m, pattern: p, handler_id: id} ->
      if m == method, do: match_pattern(p, path, id)
    end)
  end

  defp match_pattern(pattern, path, id), do: ...
end
```

Standard `:foo` named-capture matching mirroring WEB06's web-core
behaviour.

### `Server` — the public API

```elixir
defmodule CodingAdventures.ConduitOtp.Server do
  def start_link(%Application{} = app, opts \\ []) do
    OtpSupervisor.start_link(Keyword.put(opts, :app, app))
  end

  def serve(_server), do: receive do _ -> :ok end  # block forever

  def stop(server), do: Supervisor.stop(server, :normal)
  def local_port(_server), do: ...    # peek at acceptor's listen socket
  def running?(_server), do: ...
end
```

---

## What gets taught (chapter list)

| Chapter | File | OTP concept |
|---------|------|-------------|
| 1 | `otp_application.ex` | The `Application` behaviour; mix.exs hook |
| 2 | `otp_supervisor.ex` | Supervisor strategies; restart budget |
| 3 | `acceptor.ex` | gen_server callbacks; passive sockets; controlling_process |
| 4 | `worker_supervisor.ex` | DynamicSupervisor; `:temporary` restart |
| 5 | `worker.ex` | gen_server lifecycle; cooperative looping; send-to-self |
| 6 | `http_parser.ex` | `:erlang.decode_packet/3`; `{:packet, :http_bin}` |
| 7 | `route_table.ex` | `Agent`; named processes; hot reloading |
| 8 | `router.ex` | Pure functions, no process, easily tested |
| 9 | `handler_context.ex` | Cheap `throw` for non-local control flow |
| 10 | `request.ex` | Plain struct; immutable view |
| 11 | `application.ex` | Functional combinators (no compile-time macros) |
| 12 | `server.ex` | The public façade; ties it all together |

Each module's moduledoc opens with "Teaching topic: …" and includes a
worked example.

---

## BENCH.md — comparing OTP vs. NIF

After implementation, run `wrk -t4 -c100 -d10s http://localhost:3000/`
against both servers and document the results in `code/packages/elixir/conduit_otp/BENCH.md`:

- Throughput (req/s)
- p50 / p99 latency
- Memory under sustained load
- Per-connection overhead

Expected: WEB06 (NIF) wins on raw throughput; WEB07 (OTP) wins on
clarity and graceful behaviour under partial failure (e.g. a buggy
handler that crashes 1% of the time).

---

## Tests (target: 60+)

- `application_test.exs` — same DSL chainability as WEB06 (~10)
- `halt_error_test.exs`  — same throw semantics (~6)
- `handler_context_test.exs` — same response helpers (~10)
- `http_parser_test.exs` — request line, headers, body, malformed input (~10)
- `request_test.exs` — env map projection (~6)
- `router_test.exs` — named captures, method matching, no match (~8)
- `route_table_test.exs` — Agent get/update; hot reload (~4)
- `server_test.exs` — E2E via `:httpc` mirroring WEB06 (~12)

---

## Demo program

`code/programs/elixir/conduit-otp-hello/` — same 8-route demo, but with
the OTP DSL. 15+ integration tests via `:httpc`.

---

## Out of scope (intentionally)

- **HTTP/2 and HTTP/3** — HTTP/1.1 only.
- **TLS** — TCP only. Document as a follow-up: `:ssl.listen/2` is a
  drop-in replacement.
- **WebSockets** — out of scope.
- **Performance** — we have a NIF version (WEB06) for that. WEB07
  optimises for *clarity*.

---

## Future work tracked outside this spec

- **WEB08 — Conduit Perl** (next).
- **The "tiny BEAM-like VM" project** — re-implement the supervision-tree
  + actor model as a small Rust library, then port WEB07's design to
  Python/Ruby/Lua/etc. Each language gets its own DSL on top of one
  shared substrate.
- **Cowboy/Bandit comparison BENCH** — once WEB07 is stable, run the
  same wrk benchmark against Cowboy and Bandit for a reality-check
  on overhead.
