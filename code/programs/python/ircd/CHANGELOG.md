# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-23

### Changed

- **Swap network backend**: replaced `irc-net-stdlib` (one OS thread per connection)
  with `irc-net-selectors` (single-threaded event loop via `selectors.DefaultSelector`).
  This is the first step down the Russian Nesting Doll — the IRC logic is completely
  unchanged.  Only imports and `pyproject.toml` dependencies were updated.
- `main()` now instantiates `SelectorsEventLoop` instead of `StdlibEventLoop`.
- `pyproject.toml` dependency changed from `coding-adventures-irc-net-stdlib` to
  `coding-adventures-irc-net-selectors`.
- `BUILD` updated to install `irc-net-selectors` instead of `irc-net-stdlib`.

### Why

The thread-per-connection model allocates 8 MB of stack per client.  The reactor
model (`selectors`) uses ~400 bytes per idle connection.  At 10,000 concurrent clients:
threads use ~80 GB virtual memory; selectors use ~4 MB.

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
