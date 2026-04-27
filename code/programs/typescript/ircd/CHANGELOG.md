# Changelog — ircd (TypeScript)

All notable changes to this program will be documented here.

## [0.1.0] — 2026-04-12

### Added

- Initial TypeScript port of the Python `ircd` program
- `DriverHandler` class implementing the `Handler` interface from `irc-net-stdlib`
  - Per-connection `Framer` map for byte-stream reassembly
  - Feeds complete lines to `irc-proto.parse()`, dispatches to `IRCServer.onMessage()`
  - Serializes responses via `irc-proto.serialize()` and delivers via `EventLoop.sendTo()`
- `Config` interface and `parseArgs()` function for CLI flag parsing
- `main()` async function: wires all layers, installs SIGINT/SIGTERM handlers
- ESM main-module detection for `import.meta.url` vs `process.argv[1]`
- TypeScript `tsc --noEmit` type-checking as the BUILD step
