# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-06-13

### Added

- `IrcServer` — a Python facade for a high-performance IRC server whose IRC and
  TCP logic run entirely in Rust (`irc-net-reactor` on the home-grown
  kqueue/epoll reactor). Control surface: `serve` (GIL released around the
  blocking loop), `stop` (callable from another thread), `local_host` /
  `local_port`, `running`, `dispose`.
- `irc_server_native` C extension (Rust cdylib via the zero-dependency
  `python-bridge`): `server_new` / `server_serve` / `server_stop` /
  `server_local_host` / `server_local_port` / `server_running` /
  `server_dispose` over an opaque `PyCapsule` handle. No per-message callback
  into Python — the binding is lifecycle-only.
- `build.rs` linker configuration for the Python C extension (macOS
  `-undefined dynamic_lookup`, Linux `--allow-shlib-undefined`, Windows
  `python3` import lib).
- `BUILD` that compiles the cdylib, copies it under the platform `EXT_SUFFIX`,
  and runs the test suite.
- Tests: facade unit tests against a fake native module (argument coercion,
  MOTD default, lifecycle delegation, introspection) plus real end-to-end tests
  that start the Rust engine on an ephemeral port and drive two live IRC clients
  through registration, PING/PONG, and channel `PRIVMSG` broadcast.
