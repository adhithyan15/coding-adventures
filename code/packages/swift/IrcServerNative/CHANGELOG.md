# Changelog

## Unreleased

### Fixed

`BUILD_windows` now carries the `# build-tool: deps=rust/irc-server-capi` declaration.
The build tool reads that directive out of whichever BUILD file it selects for the
current platform, so a directive present only in `BUILD` left the `irc-server-capi`
edge missing from the dependency graph on Windows.

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-06-14

### Added

- `IrcServerNative` — a Swift facade for a high-performance IRC server whose IRC
  and TCP logic run entirely in Rust (`irc-net-reactor` on the home-grown
  kqueue/epoll reactor), bound through the reusable `irc-server-capi` C ABI via
  Swift's native C interop (no third-party FFI library).
- `IrcServer(host:port:serverName:motd:operPassword:maxConnections:)` (throwing
  `IrcServerError` on bind failure) with `serve()` (blocking), `serveBackground()`,
  `stop()`, `running`, and `localHost` / `localPort` / `localAddr`. The native
  engine is freed on `deinit` (stopping and joining first).
- SPM layout mirroring `swift/conduit`: a `CIrcServer` `systemLibrary` target
  (header + `module.modulemap`) and an `IrcServerNative` target that links
  `libirc_server_capi.a` with `-L Sources/CIrcServer -l irc_server_capi`.
- `Tests/IrcServerNativeTests/ServerE2ETests.swift` real-socket test: starts the
  engine on an ephemeral port, brings up two POSIX-socket IRC clients, registers
  both (001 welcome), checks PING/PONG, and asserts a channel `PRIVMSG` from one
  client is broadcast to the other — proving the Rust in-process mailbox fan-out.
  A watchdog thread and per-socket receive timeouts keep a wedged run from hanging.
- `BUILD` (build `irc-server-capi`, stage the static lib, run `swift test` where
  Swift is present), `BUILD_windows`, `.gitignore`, and `required_capabilities.json`
  (`rust`, `swift`, `cargo`).

### Safety / robustness

- The trust boundary is enforced once in the Rust `irc-server-capi` crate (UTF-8
  validation of all C strings, `max_connections` clamping, `catch_unwind` panic
  isolation, owned-clone serve). The Swift wrapper honours the ABI ownership
  contract: it frees the `localHost` C string and frees the handle exactly once
  (on `deinit`). `IrcServer` is `@unchecked Sendable` so it can be stopped from a
  different thread than the one serving; the C ABI makes that data-race-free
  (shared-ref-only entry points, `Mutex`-guarded join handle, atomic `running`),
  and ARC upholds the ABI's "don't free while a call is in flight" invariant
  because a thread in `serve()` keeps a strong reference for the call's duration.
