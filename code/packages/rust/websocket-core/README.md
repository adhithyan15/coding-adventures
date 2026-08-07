# websocket-core

`websocket-core` is a transport-independent RFC 6455 protocol state machine.
It validates and serializes HTTP upgrades, incrementally decodes frames,
enforces masking and canonical lengths, reassembles fragmented messages, and
validates ping, pong, and close controls.

The package opens no sockets and generates no randomness. A runtime adapter
supplies stream I/O, deadlines, and a fresh unpredictable mask key for every
client frame.

## Portable conformance

The package is the first consumer of the versioned language-neutral fixtures
in `code/specs/fixtures/websocket-core-v1/`. Those records pin exact handshake,
wire-frame, incremental-decoding, fragmentation, control-reply, error-code,
and redacted-diagnostic behavior for future implementation lanes.

## Validation

```sh
cargo test -p websocket-core
cargo test -p websocket-core --test portable_conformance
cargo clippy -p websocket-core --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p websocket-core --no-deps
```
