# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Fixed

- Clamp Windows to the supported single-reactor mode instead of requesting
  `SO_REUSEPORT`, which the Windows TCP provider rejects. Unix sharding remains
  unchanged; Windows multi-shard support requires a future accept fan-out.

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
- **Hostile-client resilience**: each reactor callback is wrapped in
  `catch_unwind` so a panic inside `IRCServer` on a crafted message closes only
  the offending connection instead of crashing the single event-loop thread
  (which would drop every connected client); the shared `IRCServer` mutex is
  locked poison-tolerantly (`into_inner` recovery) so one contained failure does
  not permanently brick state for everyone else.
- Real-socket integration tests covering registration (001 welcome), PING/PONG,
  channel `PRIVMSG` broadcast to a different connection, graceful `QUIT`-command
  broadcast, abrupt-disconnect `QUIT` broadcast, poisoned-mutex recovery, config
  defaults, and double-`serve` rejection.

### Notes

- Reuses `irc-server`, `irc-proto`, and `irc-framing` unchanged — only the
  transport layer is new.
- See `code/specs/irc-net-reactor.md` for the full design and the native-binding
  roadmap (Python/Ruby/Node first; JVM/Swift/Elixir/Perl later).

## [0.1.1] - 2026-06-14

### Changed

- **The IRC server is now multi-core by default.** It runs on a
  `ShardedTcpRuntime` with one reactor shard per CPU — N independent reactors on N
  threads, each its own kqueue/epoll/IOCP instance, with the kernel load-balancing
  connections across them via `SO_REUSEPORT`. TCP accept, reads, CRLF framing, and
  parsing run in parallel across cores; only the `IRCServer` state transition is
  serialized by the single shared mutex (and serialization + mailbox dispatch
  already happen outside that lock). A single shared brain keeps nick/channel
  namespaces server-global; cross-shard broadcast is automatic because each
  `ConnectionId` encodes its owning shard and the `TcpMailbox` routes to it.
- `IrcReactorServer::bind` now picks the shard count from
  `std::thread::available_parallelism()`; the new
  `IrcReactorServer::bind_with_worker_count(config, n)` pins it (`n = 1`
  reproduces the original single-reactor engine, `n` clamped to ≥ 1), and
  `worker_count()` reports the chosen count. `IrcConfig` is unchanged, so all
  language bindings keep working without modification.

### Added

- Tests: `broadcast_works_across_multiple_shards` (forces 4 shards so cross-shard
  fan-out is exercised even on a single-core runner) and
  `worker_count_is_configured_and_clamped`. All existing broadcast / QUIT /
  RST-survival / poison-recovery tests pass unchanged under default sharding.
