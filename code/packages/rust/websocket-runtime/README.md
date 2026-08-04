# websocket-runtime (Rust)

Concrete TCP adapters for the transport-independent `websocket-core` RFC 6455
state machine.

The package provides:

- a many-connection server above `tcp-runtime`;
- a blocking client above `tcp-client`;
- OS CSPRNG nonces and fresh client mask keys;
- automatic pong and close replies;
- bounded handshakes, frames, and assembled messages; and
- real loopback TCP interoperability tests.

TLS (`wss`), extensions, subprotocols, proxies, redirects, reconnects, and
keepalive deadlines are intentionally outside version 0.1.

## Validation

```sh
cargo test -p websocket-runtime
cargo clippy -p websocket-runtime --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p websocket-runtime --no-deps
```
