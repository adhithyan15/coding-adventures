# iocp (Rust)

Thin Rust wrapper over Windows I/O Completion Ports.

## What is this?

This crate exposes the completion-port primitives needed for TCP-first Windows
backends: port creation, handle association, completion waiting, and posted
user-space wakeups.

## Development

```bash
cargo test -p iocp
```
