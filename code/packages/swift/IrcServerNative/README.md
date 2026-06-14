# IrcServerNative (Swift)

A **high-performance IRC server for Swift** — every line of IRC and TCP logic
runs in Rust (the [`irc-net-reactor`](../../rust/irc-net-reactor) engine on the
home-grown `kqueue`/`epoll` reactor). Swift only launches and controls the
server, binding the engine through the reusable
[`irc-server-capi`](../../rust/irc-server-capi) **C ABI** via Swift's native C
interop — no third-party FFI library.

## Why

This is the Swift member of a family: one Rust IRC engine, embedded natively in
every language. The other languages (Python, Ruby, Node, Java, Elixir, Perl)
each speak their host VM's own native protocol through a dedicated bridge crate.
Swift has no such bridge in this repo — but it speaks the **plain C ABI**
fluently, so it binds the engine through `irc-server-capi` instead. Because all
logic lives in Rust, the binding is a pure lifecycle control surface —
**create, serve, stop** — with no callback into Swift.

## Usage

```swift
import IrcServerNative

let server = try IrcServer(port: 6667)
server.serveBackground()      // runs the loop on a Rust thread
// ... connect IRC clients to server.localHost : server.localPort ...
server.stop()
```

`serve()` runs the loop on the calling thread and blocks until `stop()`;
`serveBackground()` runs it on a dedicated Rust OS thread and returns at once.
Because the IRC server has no per-request callback into Swift, the spawned
thread runs pure Rust and never re-enters the Swift runtime.

## API

`IrcServer(host:port:serverName:motd:operPassword:maxConnections:)` throws
`IrcServerError` if the socket cannot be bound. Methods/properties:

| member            | description                                              |
|-------------------|----------------------------------------------------------|
| `serve()`         | run the loop on this thread, blocking (`-> Bool`)        |
| `serveBackground()`| run the loop on a Rust thread, returns immediately (`-> Bool`) |
| `stop()`          | signal the loop to stop and join the thread              |
| `running`         | whether the loop is running                              |
| `localHost` / `localPort` / `localAddr` | the bound address          |

The instance frees the native engine on `deinit` (stopping and joining first).

## How it's built

`BUILD` runs `cargo build --release` on `irc-server-capi`, stages
`libirc_server_capi.a` into `Sources/CIrcServer/` (where the `CIrcServer`
`systemLibrary` target links it), then runs `swift test`. The C ABI is built on
every runner; `swift test` runs only where the Swift toolchain is present.

## Safety

The trust boundary lives in the Rust `irc-server-capi` crate: every untrusted C
string is UTF-8 validated, `max_connections` is clamped, every `extern "C"`
function isolates panics with `catch_unwind`, and `serve`/`serveBackground` run
an **owned clone** of the engine so the handle can be stopped/freed from another
thread without dangling. The Swift `IrcServer` honours the ABI's ownership
contract: it frees `localHost` strings and frees the handle exactly once.

## Layer position

```
IrcServerNative   (this package — Swift facade)
        ↓ irc_server_capi.h  (irc-server-capi — flat C ABI)
irc-net-reactor   (Rust IRC engine, in-process broadcast)
        ↓
tcp-runtime → transport-platform → kqueue / epoll / IOCP
```
