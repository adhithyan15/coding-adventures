# kqueue (Rust)

Thin Rust wrapper over BSD/macOS `kqueue` and `kevent`.

## What is this?

This crate exposes the TCP-first parts of `kqueue` directly and now includes
enough native surface for higher transport layers:

- read and write readiness filters
- timer filters
- user-triggered events
- token-carrying user data
- blocking waits over ready events

Because the user is on macOS, this crate is intended to be exercised locally.

## Development

```bash
cargo test -p kqueue
```
