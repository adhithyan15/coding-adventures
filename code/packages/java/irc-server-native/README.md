# irc-server-native (Java / JVM)

A **high-performance IRC server for the JVM** — every line of IRC and TCP logic
runs in Rust (the [`irc-net-reactor`](../../rust/irc-net-reactor) engine on the
home-grown `kqueue`/`epoll` reactor). Java only launches and controls the server,
through a thin JNI layer over the zero-dependency `jni-bridge` (the Rust crate
[`irc-server-native-jni`](../../rust/irc-server-native-jni)).

## Why

This is the JVM member of a family: one Rust IRC engine, embedded natively in
every language. Because all logic lives in Rust, the binding is a pure lifecycle
control surface — **create, serve, stop** — with no callback into Java.

## Usage

```java
import com.codingadventures.ircserver.IrcServer;

try (IrcServer server = IrcServer.builder().port(6667).build()) {
    server.serveBackground();                 // returns immediately
    System.out.println("listening on " + server.localAddr());
    // ... connect IRC clients to server.localHost():server.localPort() ...
    server.stop();
}
```

`serve()` blocks the calling thread; `serveBackground()` runs the loop on a Rust
background thread and returns immediately. `IrcServer` is `AutoCloseable` —
`close()` stops, joins, and frees the native peer.

## API

`IrcServer.builder()` — `host`, `port`, `serverName`, `motd`, `operPassword`,
`maxConnections`, then `build()`. Instance methods: `serve()`,
`serveBackground()`, `stop()`, `running()`, `localHost()`, `localPort()`,
`localAddr()`, `close()`.

## How it's built

`BUILD` runs `cargo build -p irc-server-native-jni --release` (producing
`libirc_server_native_jni.{so,dylib}`) then `gradle test`. The test JVM is given
`-Djava.library.path=…/rust/target/release` so `System.loadLibrary` finds the
addon.

## Lifecycle note

The native peer is a raw pointer behind a `long` handle. Like the repo's
`conduit` JVM binding, `IrcServer` is single-owner: don't race `close()` against
other methods on the same instance from another thread.

## Layer position

```
com.codingadventures.ircserver.IrcServer   (this package — Java facade)
        ↓ irc_server_native_jni cdylib (jni-bridge; background-thread serve)
irc-net-reactor   (Rust IRC engine, in-process broadcast)
        ↓
tcp-runtime → transport-platform → kqueue / epoll / IOCP
```
