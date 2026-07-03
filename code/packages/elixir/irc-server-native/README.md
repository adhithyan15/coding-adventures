# coding_adventures_irc_server_native (Elixir)

A **high-performance IRC server for the BEAM** — every line of IRC and TCP logic
runs in Rust (the [`irc-net-reactor`](../../rust/irc-net-reactor) engine on the
home-grown `kqueue`/`epoll` reactor). Elixir only launches and controls the
server, through an Erlang NIF built on the zero-dependency `erl-nif-bridge`.

## Why

This is the BEAM member of a family: one Rust IRC engine, embedded natively in
every language. Because all logic lives in Rust, the binding is a pure lifecycle
control surface — **create, serve, stop** — with no callback into Elixir.

## Usage

```elixir
alias CodingAdventures.IrcServerNative.Server

{:ok, server} = Server.new(port: 6667)
:ok = Server.serve_background(server)   # runs the loop on a Rust thread
# ... connect IRC clients to Server.local_host(server):Server.local_port(server) ...
:ok = Server.stop(server)
```

`serve/1` runs the loop in the calling process (a dirty I/O NIF, so it doesn't
starve the BEAM schedulers) and blocks until `stop/1`; `serve_background/1` runs
it on a Rust OS thread and returns immediately.

## API

`CodingAdventures.IrcServerNative.Server`:

| function           | description                                              |
|--------------------|----------------------------------------------------------|
| `new(opts)`        | build + bind (`:host`, `:port`, `:server_name`, `:motd`, `:oper_password`, `:max_connections`) |
| `serve/1`          | run the loop in the calling process (dirty I/O), blocking |
| `serve_background/1` | run the loop on a Rust thread, returns immediately      |
| `stop/1`           | signal the loop to stop and join the thread              |
| `running?/1`       | whether the loop is running                              |
| `local_host/1` / `local_port/1` / `local_addr/1` | the bound address          |

The server resource is reference-counted by the BEAM; when the last reference is
garbage-collected, the Rust destructor stops and joins the server.

## How it's built

`BUILD` runs `cargo build --release` in `native/irc_server_native`, copies the
cdylib into `priv/irc_server_native.so`, then `mix compile` + `mix test`. The NIF
is loaded with `@on_load` via `:erlang.load_nif/2`.

## Resilience

`serve`/`serve_background` run the blocking loop on an **owned clone** of the
engine, so the resource destructor (which stops + joins) never races a dangling
engine. The underlying reactor closes a single connection (never the whole loop)
when a client resets.

## Layer position

```
CodingAdventures.IrcServerNative.Server   (this package — Elixir facade)
        ↓ irc_server_native NIF (erl-nif-bridge; dirty-I/O or background-thread serve)
irc-net-reactor   (Rust IRC engine, in-process broadcast)
        ↓
tcp-runtime → transport-platform → kqueue / epoll / IOCP
```
