# tcp-reactor (Rust)

Small nonblocking TCP reactor built on top of `native-event-core`.

## What is this?

This crate is a proof layer above the native event substrates. It accepts many
concurrent TCP clients, reads bytes, runs a handler, and flushes replies without
blocking the process per connection.

There are two engines behind one public API, because Windows and Unix disagree
about what asynchronous I/O is:

| Platform | Engine | How it works |
| --- | --- | --- |
| Linux | epoll, through `native-event-core` | readiness: "this socket is ready, read it" |
| macOS / BSD | kqueue, through `native-event-core` | readiness |
| Windows | I/O completion port, through the `iocp` crate and Winsock | completion: `AcceptEx` / `WSARecv` / `WSASend` are started and the port reports them finished |

The Windows engine talks to the native networking stack directly. It is not an
`EventBackend`, because overlapped I/O is not a readiness API and is not dressed
up as one.

```rust
use tcp_reactor::bind_native;

let mut reactor = bind_native(("127.0.0.1", 0), |_conn, bytes| bytes.to_vec())?;
let stop = reactor.stop_handle();
std::thread::spawn(move || reactor.serve());
// ... later
stop.stop();
```

`bind_native` picks the platform's engine. The platform constructors
(`bind_epoll`, `bind_kqueue`, `bind_iocp`) are there when you want a specific one.

The reactor includes two safety rails that matter for real servers:

- a configurable cap on active connections
- a configurable cap on queued outbound bytes per connection

Those guards keep a slow reader or connection flood from turning the example
reactor into an unbounded memory sink. Both engines enforce them identically, and
`tests/reactor.rs` checks that on every OS.

## Development

```bash
cargo test -p tcp-reactor
```
