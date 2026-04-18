# mini-redis

Mini Redis TCP server.

## What is this?

This program composes:

- `tcp-runtime` for the current TCP transport runtime
- `resp-protocol` for Redis-compatible framing
- `in-memory-data-store` for command execution and background expiry

It speaks enough Redis-compatible RESP over TCP for local development and
end-to-end testing with normal socket clients.

Its Redis session state now lives inside `tcp-runtime` connection state, which
makes this program the first real application consumer of the newer transport
stack.

For safety, incomplete RESP input is bounded to a fixed per-connection buffer
cap. Clients that exceed that cap receive an error and are disconnected instead
of being allowed to grow memory usage without limit.

## Development

```bash
cargo test -p mini-redis -- --nocapture
```
