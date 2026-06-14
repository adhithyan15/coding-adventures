# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-06-14

### Added

- `CodingAdventures::IrcServerNative::IrcServer` — a Ruby facade for a
  high-performance IRC server whose IRC and TCP logic run entirely in Rust
  (`irc-net-reactor` on the home-grown kqueue/epoll reactor). Control surface:
  `serve` (GVL released around the blocking loop), `start` (background thread),
  `stop` (callable from another thread), `close`, `local_host` / `local_port` /
  `local_addr`, `running?`.
- `irc_server_native` C extension (Rust cdylib via the zero-dependency
  `ruby-bridge`): `NativeServer` with `initialize` / `serve` / `stop` /
  `dispose` / `running?` / `local_host` / `local_port`, plus an `Error` class.
  No per-message callback into Ruby — the binding is lifecycle-only.
- `extconf.rb` + `build_config.rb` + `Rakefile` that compile the cdylib via
  cargo and install it into `lib/` under the platform `DLEXT`; `build.rs`
  carries the macOS `dynamic_lookup` and Windows libruby link configuration.
- `BUILD` / `BUILD_windows`: `bundle install && rake compile && rake test`.
- End-to-end tests (minitest, real TCP sockets): registration, PING/PONG,
  channel `PRIVMSG` broadcast between two clients, `QUIT` broadcast, and the
  native dispose-while-running guard.

### Security / robustness

- `serve` runs on an **owned clone** of the engine and sets the `running` flag
  before releasing the GVL, so a concurrent `dispose` cannot cause a
  use-after-free (mirrors the Python binding's hardening).
- Hardened the shared `stream-reactor` event loop (used by every binding): a
  per-connection read or write error (e.g. `ECONNRESET` from a client that
  vanished) now closes only that connection and runs its close callback, instead
  of propagating out of `serve` and tearing down the whole event loop. This bug
  was surfaced by the Ruby broadcast test (a client closing with unread data
  sends a TCP RST). Added an `irc-net-reactor` regression test
  (`server_survives_a_client_connection_reset`).
