# tcp-reactor (Rust)

Small nonblocking TCP reactor built on top of `native-event-core`.

## What is this?

This crate is a proof layer above the generic native event substrate. It accepts
many concurrent TCP clients, reads bytes, runs a handler, and flushes replies
without blocking the process per connection.

This crate requires Unix-style file descriptors. On macOS and BSD, it can be
exercised locally through the `kqueue` backend. Windows sockets are unsupported;
the Windows build plan reports an explicit SKIP. That selection is not a passing
reactor test. Unix plans retain the full original `cargo test` command and the
non-Unix compile-time boundary remains intact.

The reactor now includes two safety rails that matter for real servers:

- a configurable cap on active connections
- a configurable cap on queued outbound bytes per connection

Those guards keep a slow reader or connection flood from turning the example
reactor into an unbounded memory sink.

## Development

```bash
cargo test -p tcp-reactor
```
