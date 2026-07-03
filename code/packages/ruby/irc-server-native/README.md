# coding_adventures_irc_server_native

A **high-performance IRC server for Ruby** — every line of IRC and TCP logic
runs in Rust (the [`irc-net-reactor`](../../rust/irc-net-reactor) engine on the
home-grown `kqueue`/`epoll` reactor). Ruby only launches and controls the
server, through a native extension built on the zero-dependency `ruby-bridge`
(no Magnus / rb-sys abstractions).

## Why

This is the Ruby member of a family: one Rust IRC engine, embedded natively in
every language. Because all logic lives in Rust, the binding is a pure lifecycle
control surface — **create, serve, stop** — with no per-message callback into
Ruby.

## Usage

```ruby
require "coding_adventures_irc_server_native"

server = CodingAdventures::IrcServerNative::IrcServer.new(port: 6667)
server.serve   # blocks until another thread calls server.stop
```

Bind an ephemeral port and serve on a background thread (handy for tests):

```ruby
server = CodingAdventures::IrcServerNative::IrcServer.new(port: 0)
server.start                       # serves on a background thread
puts "listening on #{server.local_addr}"
# ... connect IRC clients to server.local_host:server.local_port ...
server.close                       # stop + join + release the listener
```

Point an IRC client (irssi, WeeChat, `nc`) at the host/port, register with
`NICK`/`USER`, `JOIN #channel`, and chat — the broadcast fan-out to other
channel members happens entirely in Rust.

## API

`CodingAdventures::IrcServerNative::IrcServer`:

| method                        | description                                        |
|-------------------------------|----------------------------------------------------|
| `new(host:, port:, server_name:, motd:, oper_password:, max_connections:)` | build + bind |
| `serve`                       | run the event loop, blocking (the GVL is released) |
| `start`                       | serve on a background thread, returns once running |
| `stop`                        | signal the loop to stop (safe from another thread) |
| `close`                       | stop, join the thread, release the listener        |
| `local_host` / `local_port` / `local_addr` | the bound address                    |
| `running?`                    | whether the loop is currently running              |

The lower-level `CodingAdventures::IrcServerNative::NativeServer` is the raw C
extension class; most users want the `IrcServer` facade above.

## How it's built

`rake compile` runs `cargo build --release` in `ext/irc_server_native` (via the
generated Makefile) and copies the cdylib into `lib/` under Ruby's `DLEXT`
(`.so`/`.bundle`/`.dll`). The `BUILD` script is `bundle install && rake compile
&& rake test`.

## Resilience

The native `serve` releases the GVL around the blocking loop and runs on an
**owned clone** of the engine, so a concurrent `dispose` cannot free the engine
out from under it. The underlying reactor closes a single connection (and never
the whole loop) when a client resets — so one crashed or hostile client can't
take the server down.

## Layer position

```
CodingAdventures::IrcServerNative::IrcServer        (this gem — Ruby facade)
        ↓ irc_server_native C extension (ruby-bridge, GVL released on serve)
irc-net-reactor   (Rust IRC engine, in-process broadcast)
        ↓
tcp-runtime → transport-platform → kqueue / epoll / IOCP
```
