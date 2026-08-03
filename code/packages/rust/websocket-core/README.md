# websocket-core

`websocket-core` is a transport-independent RFC 6455 protocol state machine.
It validates and serializes HTTP upgrades, incrementally decodes frames,
enforces masking and canonical lengths, reassembles fragmented messages, and
validates ping, pong, and close controls.

The package opens no sockets and generates no randomness. A runtime adapter
supplies stream I/O, deadlines, and a fresh unpredictable mask key for every
client frame.

## Validation

```sh
cargo test -p websocket-core
cargo clippy -p websocket-core --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p websocket-core --no-deps
```
