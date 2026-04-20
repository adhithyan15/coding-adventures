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

## Capacity Experiments

The server defaults to the TCP runtime's conservative connection cap. For C10K
hold benchmarks, raise the cap explicitly so the benchmark tests the transport
stack instead of the safety limit:

```bash
cargo run --release -p mini-redis -- --host 127.0.0.1 --port 6379 --max-connections 10000
```
