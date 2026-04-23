# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- `DriverHandler` adapter bridging `irc-net-stdlib` and `irc-server`:
  maintains a per-connection `Framer`, parses lines with `irc-proto`, calls
  `IRCServer.on_message`, and delivers responses via `EventLoop.send_to`.
- `Config` dataclass with `host`, `port`, `server_name`, `motd`, and
  `oper_password` fields.
- `parse_args()` function — argparse-based CLI argument parser that populates
  a `Config`; validates port range (0–65535).
- `main()` entry point: wires `create_listener`, `StdlibEventLoop`,
  `IRCServer`, and `DriverHandler`; installs `SIGINT`/`SIGTERM` handlers for
  graceful shutdown.
- `ircd` console script entry point via `[project.scripts]` in `pyproject.toml`.
- `src/ircd/__main__.py` enabling `python -m ircd`.
- Integration tests with real TCP sockets covering registration, MOTD,
  channel PRIVMSG, direct PRIVMSG, and PING/PONG.
- Unit tests for `DriverHandler` with mock `EventLoop`, covering partial-line
  buffering, garbage-line tolerance, framer lifecycle, and multi-client
  message dispatch.
- Unit tests for `parse_args` covering defaults, custom flags, and invalid
  port rejection.
