# tcp-reactor (Rust)

Small nonblocking TCP reactor built on top of `native-event-core`.

## What is this?

This crate is a proof layer above the generic native event substrate. It accepts
many concurrent TCP clients, reads bytes, runs a handler, and flushes replies
without blocking the process per connection.

On macOS and BSD, it can be exercised locally through the `kqueue` backend.

## Development

```bash
cargo test -p tcp-reactor
```
