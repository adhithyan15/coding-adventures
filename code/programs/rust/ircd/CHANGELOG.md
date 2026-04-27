# Changelog — ircd (Rust)

All notable changes to this program will be documented here.

## [0.1.0] — 2026-04-12

### Added

- `Config` struct with host, port, server_name, motd, oper_password fields
- `parse_args(&[&str]) -> Config` — command-line argument parser (no external deps)
- `DriverHandler` — implements `irc-net-stdlib::Handler`:
  - `on_connect`: creates per-connection `Framer`, notifies `IRCServer`
  - `on_data`: feeds bytes into `Framer`, parses complete lines, dispatches to `IRCServer`, sends responses
  - `on_disconnect`: notifies `IRCServer` (broadcasts QUIT), removes `Framer`
- `main()` entry point:
  - Parses args, creates server + event loop + handler
  - Starts `EventLoop::run()` in a background thread
  - Unix signal handling (SIGINT/SIGTERM) calls `stop()`
- Integration tests:
  - `test_registration_welcome_sequence` — full NICK/USER/welcome over TCP
  - `test_nick_in_use` — 433 ERR_NICKNAMEINUSE
  - `test_join_and_privmsg` — channel messaging between two clients
  - `test_parse_args_defaults` / `test_parse_args_custom`
  - `test_ping_pong` — keepalive round-trip
