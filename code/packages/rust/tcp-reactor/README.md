# tcp-reactor (Rust)

Small nonblocking TCP reactor built on top of `native-event-core`.

## What is this?

This crate is a proof layer above the generic native event substrate. It accepts
many concurrent TCP clients, reads bytes, runs a handler, and flushes replies
without blocking the process per connection.

On macOS and BSD, it can be exercised locally through the `kqueue` backend.

The reactor now includes two safety rails that matter for real servers:

- a configurable cap on active connections
- a configurable cap on queued outbound bytes per connection

Those guards keep a slow reader or connection flood from turning the example
reactor into an unbounded memory sink.

## Development

```bash
cargo test -p tcp-reactor
```
