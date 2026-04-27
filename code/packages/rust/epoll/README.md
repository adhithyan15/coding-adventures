# epoll (Rust)

Thin Rust wrapper over Linux `epoll`.

## What is this?

This crate exposes Linux's readiness API directly. It is intentionally small and
close to the underlying syscall surface so higher abstractions can build on it
without losing sight of epoll's real semantics.

## Development

```bash
cargo test -p epoll
```
