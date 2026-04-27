# Changelog

All notable changes to `ircd` (Go) will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial Go implementation of the IRC server program.
- `DriverHandler` struct — bridges `irc-net-stdlib` events to `irc-server`:
  - `OnConnect`: registers a new `irc_framing.Framer` for the connection.
  - `OnData`: feeds bytes into the framer; dispatches complete frames to
    `irc_server.OnMessage`; serialises and sends each `Response`.
  - `OnDisconnect`: calls `irc_server.OnDisconnect`; removes the framer.
- `Config` struct with `host`, `port`, `serverName`, `motd`, `operPassword`.
- `parseArgs(args []string) (*Config, error)` — flag-based CLI parser.
- `runLoop(cfg *Config, stopCh <-chan struct{}) error` — testable core loop.
- `run(args []string) error` — signal-handling wrapper (SIGINT/SIGTERM).
- `main()` — entry point.
- Integration tests covering registration, JOIN/PRIVMSG, PING/PONG, nick
  collision, malformed lines, and QUIT.
- 70%+ statement coverage.
