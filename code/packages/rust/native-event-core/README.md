# native-event-core (Rust)

Generic native event substrate above `epoll`, `kqueue`, and `iocp`.

## What is this?

This crate is the generic abstraction boundary between raw native backends and
higher layers such as TCP reactors, WebSocket runtimes, or native UI systems.

It knows about:

- tokens
- interests
- normalized events
- backend traits

It does not know about:

- TCP framing
- WebSocket messages
- widget trees

## Development

```bash
cargo test -p native-event-core
```
