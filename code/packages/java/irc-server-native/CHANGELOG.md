# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-06-14

### Added

- `com.codingadventures.ircserver.IrcServer` — a Java facade for a
  high-performance IRC server whose IRC and TCP logic run entirely in Rust
  (`irc-net-reactor` on the home-grown kqueue/epoll reactor). Builder config
  (`host`, `port`, `serverName`, `motd`, `operPassword`, `maxConnections`) plus
  `serve` (blocking), `serveBackground`, `stop`, `running`, `localHost` /
  `localPort` / `localAddr`, and `AutoCloseable` `close`.
- `irc-server-native-jni` Rust cdylib (via the zero-dependency `jni-bridge`):
  `nativeNewServer` / `nativeServe` / `nativeServeBackground` / `nativeStop` /
  `nativeRunning` / `nativeLocalHost` / `nativeLocalPort` / `nativeDisposeServer`
  over an opaque `long` peer pointer. No callback into the JVM.
- JUnit 5 real-socket tests: ephemeral address, registration + PING/PONG,
  channel `PRIVMSG` broadcast between two clients, `running` flips after serve.
- Gradle build pointing `java.library.path` at the Rust release dir; `BUILD`
  builds the cdylib then runs `gradle test`.

### Security / robustness

- `serve`/`serveBackground` run the blocking loop on an **owned clone** of the
  engine (`Clone` over `Arc`s), so the background thread never dangles even
  though `nativeDisposeServer` frees the peer (mirrors the Python/Ruby/Node
  bindings).
- Fixed a **stop-before-serve race** in the shared `stream-reactor` event loop
  that this binding surfaced: `serve()` reset the stop flag on entry, so a
  `stop()` arriving before the background serve thread started was silently
  swallowed and `serve()`/`join()` hung forever. The JVM tests flip
  `running`→`stop` synchronously, hitting it every run; the fix (don't reset the
  flag in `serve()`) makes `stop()` reliable for every binding. Added an
  `irc-net-reactor` regression test (`stop_called_before_serve_starts_is_not_lost`).
