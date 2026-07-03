# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-06-14

### Added

- `IrcServer` — a TypeScript facade for a high-performance IRC server whose IRC
  and TCP logic run entirely in Rust (`irc-net-reactor` on the home-grown
  kqueue/epoll reactor). Control surface: `serve` (non-blocking; the loop runs on
  a background thread), `stop`, `dispose`, `running`, `localHost` / `localPort` /
  `localAddr`.
- `irc_native_node` N-API addon (Rust cdylib via the zero-dependency
  `node-bridge`): a `newServer(...)` factory returning an object with `serve` /
  `stop` / `running` / `localHost` / `localPort` / `dispose`. No per-message
  callback into JavaScript, so no threadsafe-function machinery — the blocking
  `serve()` runs on a `std::thread::spawn`ed background thread.
- `build.rs` (macOS `-undefined dynamic_lookup`), `BUILD` (cargo build → copy
  `.node` → `npm ci` → `tsc` → `vitest`), `BUILD_windows` skip (Node import lib),
  `tsconfig` / `tsconfig.test` / `vitest.config`, and an ESM loader via
  `createRequire`.
- End-to-end tests (vitest, real `net` sockets): ephemeral-address reporting,
  `running` flips after `serve`, registration + PING/PONG, channel `PRIVMSG`
  broadcast between two clients, and the dispose-while-running guard.

### Security / robustness

- `serve()` runs on an **owned clone** of the engine, so a concurrent `dispose()`
  cannot cause a use-after-free (mirrors the Python and Ruby bindings). The
  `NativeServer` `Drop` stops and joins the background thread on GC so a server
  is never leaked.
