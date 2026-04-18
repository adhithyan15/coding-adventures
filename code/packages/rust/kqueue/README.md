# kqueue (Rust)

Thin Rust wrapper over BSD/macOS `kqueue` and `kevent`.

## What is this?

This crate exposes the TCP-first parts of `kqueue` directly: read and write
filters, token-carrying user data, and blocking waits over ready events.

Because the user is on macOS, this crate is intended to be exercised locally.

## Development

```bash
cargo test -p kqueue
```
