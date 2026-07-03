# irc-net-reactor

An **all-Rust IRC server engine** hosted on this repository's home-grown TCP
runtime (`tcp-runtime` → `transport-platform` → raw `kqueue`/`epoll`/IOCP), with
in-process broadcast via the runtime's `TcpMailbox`.

This is the engine that the per-language native bindings
(`python-bridge`/`ruby-bridge`/`node-bridge` wrappers) embed to give every
language a high-performance IRC server without re-implementing any IRC logic.

## Where it sits in the stack

```
irc-server (state machine) + irc-proto + irc-framing      ← reused unchanged
        ↓ wired by
irc-net-reactor   (THIS CRATE: IRCServer on tcp-runtime, broadcast via TcpMailbox)
        ↓ runs on
tcp-runtime → transport-platform → kqueue / epoll / IOCP
        ↑ embedded via per-language native bridge (create / serve / stop)
   python-bridge   node-bridge   ruby-bridge   …
```

See [`code/specs/irc-net-reactor.md`](../../../specs/irc-net-reactor.md) for the
full design, including the broadcast mechanism and concurrency model.

## Why a reactor instead of threads

`irc-net-stdlib` spends one OS thread (and its multi-MB stack) per connection.
`irc-net-reactor` uses event-loop threads that the kernel wakes only when a socket
is readable — the reactor pattern behind nginx, Redis, and Node.js. The IRC
*logic* is unchanged; only the transport differs.

## Multi-core

By default the server runs **one reactor shard per CPU** (a `ShardedTcpRuntime`):
N independent reactors on N threads, each with its own kqueue/epoll/IOCP instance,
with the kernel load-balancing accepted connections across them via
`SO_REUSEPORT`. TCP accept, reads, CRLF framing, and parsing all run in parallel
across cores; only the `IRCServer` state transition is serialized (by a single
shared mutex), and that critical section is small relative to the per-message
I/O. Because every `ConnectionId` encodes its owning shard, a response destined
for a client on another reactor is routed straight there by the shard-aware
`TcpMailbox` — so cross-shard broadcast just works. Use
`IrcReactorServer::bind_with_worker_count(config, n)` to pin the shard count
(`n = 1` reproduces the original single-reactor engine).

## The broadcast mechanism

IRC must write one client's message to *other* clients' sockets. The reactor's
`TcpMailbox.send(connection_id, bytes)` targets **any** connection by id, from
any thread. `IRCServer` returns a `Vec<Response>` where each response names its
target connection; the engine serializes each message and hands it to the
mailbox. Replies to the sender and fan-out to channel members share one path.

## Usage

```rust
use irc_net_reactor::{IrcConfig, IrcReactorServer};

let server = IrcReactorServer::bind(IrcConfig {
    host: "127.0.0.1".to_string(),
    port: 6667,
    server_name: "irc.local".to_string(),
    motd: vec!["Welcome.".to_string()],
    oper_password: String::new(),
    max_connections: 1024,
})?;

println!("listening on {}", server.local_addr());
server.serve()?; // blocks until server.stop() is called from another thread
# Ok::<(), std::io::Error>(())
```

`port = 0` binds an OS-assigned ephemeral port, readable afterwards via
`local_addr()` — convenient for tests.

## Testing

```
cargo test -p irc-net-reactor
```

The integration tests drive two real `TcpStream` clients through registration,
channel `PRIVMSG` **broadcast** (one client sees another's message — the fan-out
proof), `QUIT` broadcast on disconnect, and `PING`/`PONG`.

## Platform support

| OS                | Event backend |
|-------------------|---------------|
| macOS / *BSD      | `kqueue`      |
| Linux             | `epoll`       |
| Windows           | IOCP / WSAPoll|

## Limitations

- Plaintext only (no TLS), matching the rest of the `irc-net-*` family.
- No IRC-level per-client backpressure beyond the runtime's
  `max_pending_write_bytes` cap. See the spec for details.
