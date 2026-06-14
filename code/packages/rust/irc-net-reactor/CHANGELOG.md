# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-06-13

### Added

- `IrcReactorServer` — an all-Rust IRC server engine that hosts the pure
  `irc_server::IRCServer` state machine on the home-grown `tcp-runtime` reactor
  (`kqueue` on macOS/BSD, `epoll` on Linux, IOCP/WSAPoll on Windows).
- `IrcConfig` — runtime configuration (`host`, `port`, `server_name`, `motd`,
  `oper_password`, `max_connections`); `port = 0` binds an ephemeral port.
- In-process **broadcast** via the runtime's `TcpMailbox`: each
  `irc_server::Response` is serialized and delivered to its target connection by
  id, so one client's `PRIVMSG` reaches other channel members' sockets. This is
  the first server in the repo to use the mailbox for genuine fan-out.
- Per-connection `Framer` state for CRLF line reassembly; lossy UTF-8 decoding;
  silent skipping of unparseable lines (RFC-traditional behaviour).
- `bind` / `local_addr` / `serve` / `stop` / `is_running` control surface,
  mirroring `mini-redis`'s `MiniRedisServer` shape so language bindings can follow
  the proven `conduit` embedding pattern.
- Real-socket integration tests covering registration (001 welcome), PING/PONG,
  channel `PRIVMSG` broadcast to a different connection, `QUIT` broadcast on
  disconnect, config defaults, and double-`serve` rejection.

### Notes

- Reuses `irc-server`, `irc-proto`, and `irc-framing` unchanged — only the
  transport layer is new.
- See `code/specs/irc-net-reactor.md` for the full design and the native-binding
  roadmap (Python/Ruby/Node first; JVM/Swift/Elixir/Perl later).
