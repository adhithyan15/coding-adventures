# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-06-14

### Added

- `CodingAdventures.IrcServerNative.Server` — an Elixir facade for a
  high-performance IRC server whose IRC and TCP logic run entirely in Rust
  (`irc-net-reactor` on the home-grown kqueue/epoll reactor). API: `new/1`
  (`:host`, `:port`, `:server_name`, `:motd`, `:oper_password`,
  `:max_connections`), `serve/1` (dirty I/O, blocking), `serve_background/1`,
  `stop/1`, `running?/1`, `local_host/1` / `local_port/1` / `local_addr/1`.
- `irc_server_native` Erlang NIF (Rust cdylib via the zero-dependency
  `erl-nif-bridge`): a `NativeServer` resource with `new_server` /
  `server_serve` (dirty I/O) / `server_serve_background` / `server_stop` /
  `server_running` / `server_local_host` / `server_local_port`. No callback into
  Elixir — the binding is lifecycle-only. The resource destructor stops and
  joins the server on GC.
- `@on_load` NIF loader; `build.rs` linker config; `BUILD` that builds the
  cdylib, copies it into `priv/`, and runs `mix test --cover` (the pure-stub
  `Native` module is excluded from coverage).
- ExUnit real-socket tests (`:gen_tcp`): ephemeral address, `running?` flips
  after `serve_background`, registration + PING/PONG, channel `PRIVMSG`
  broadcast between two clients.

### Security / robustness

- `serve`/`serve_background` run the blocking loop on an **owned clone** of the
  engine (`Clone` over `Arc`s), so the GC destructor's stop+join never races a
  dangling engine (mirrors the Python/Ruby/Node/Java bindings).
