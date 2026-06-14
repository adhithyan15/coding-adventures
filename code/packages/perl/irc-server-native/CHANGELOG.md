# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-06-14

### Added

- `CodingAdventures::IrcServerNative` — a Perl facade for a high-performance IRC
  server whose IRC and TCP logic run entirely in Rust (`irc-net-reactor` on the
  home-grown kqueue/epoll reactor). `new(%opts)` (`host`, `port`, `server_name`,
  `motd` arrayref, `oper_password`, `max_connections`) returns a server object
  with `serve` (blocking), `serve_background`, `stop`, `running`, `local_host` /
  `local_port` / `local_addr`, and `close` (also on `DESTROY`).
- `IrcServerNative` XS extension (Rust cdylib via the zero-dependency
  `perl-bridge`): `new_server` / `server_serve` / `server_serve_background` /
  `server_stop` / `server_running` / `server_local_host` / `server_local_port` /
  `dispose_server` over an opaque peer `IV`. No callback into Perl.
- `DynaLoader` bootstrap; `build.rs` linker config; `BUILD` that compiles the
  cdylib, installs it under `lib/auto/...`, and runs `t/` with `prove`.
- `t/01-server.t` real-socket test (`IO::Socket::INET`): ephemeral address,
  `running` flips after `serve_background`, registration + PING/PONG, channel
  `PRIVMSG` broadcast between two clients.

### Safety / robustness

- Every XSUB is wrapped in `catch_unwind` so a Rust panic can't unwind into the
  Perl interpreter; all Rust→Perl strings use `newSVpvn` (explicit length, never
  `strlen` on a possibly-empty pointer — per lessons.md).
- `serve`/`serve_background` run the blocking loop on an **owned clone** of the
  engine, so the peer is never dereferenced by the background thread and can't
  dangle. Because the IRC server has no Perl callback, `serve_background`'s
  spawned thread is pure Rust and safe even on single-interpreter Perl (where
  the `conduit` Perl binding had to refuse to spawn).
