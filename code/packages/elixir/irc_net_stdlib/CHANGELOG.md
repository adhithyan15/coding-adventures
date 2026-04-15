# Changelog — irc_net_stdlib (Elixir)

## [0.1.0] — 2026-04-12

### Added

- `CodingAdventures.IrcNetStdlib.EventLoop` GenServer managing:
  - ETS table for conn_id → socket mappings (public, lock-free reads)
  - Mutex lock queue for serialising Handler callbacks
  - Accept loop lifecycle (`run/3`, `stop/1`)
- `EventLoop.start_link/1` — start the GenServer
- `EventLoop.run/3` — spawn a non-linked accept loop Task
- `EventLoop.stop/1` — close the listener socket and clear state
- `EventLoop.send_to/3` — send bytes to a connection via ETS lookup
  (safe to call from within a Handler callback — no GenServer re-entry)
- `EventLoop.dispatch/2` — serialise a callback via mutex (acquire/run/release)
- `EventLoop.register_conn/2` and `EventLoop.deregister_conn/2` — manage
  the conn_id ↔ socket mapping in ETS
- `CodingAdventures.IrcNetStdlib.Listener` module:
  - `listen/2` — create a TCP listening socket
  - `port!/1` — query the bound port
  - `close/1` — close the listening socket
- `CodingAdventures.IrcNetStdlib.Handler` behaviour:
  - `on_connect/2`, `on_data/2`, `on_disconnect/1` callbacks
- `CodingAdventures.IrcNetStdlib` facade with `defdelegate` for all public APIs
- Handles `:einval` from `:gen_tcp.accept/1` as a normal socket-closed signal
  (required on Windows)
- Safe GenServer calls (`safe_call/2`) using `try/catch :exit, _` for clean
  shutdown when GenServer dies before workers
- Comprehensive ExUnit test suite — 24 tests, 81.44% coverage
- Port of Python reference implementation to idiomatic Elixir OTP
