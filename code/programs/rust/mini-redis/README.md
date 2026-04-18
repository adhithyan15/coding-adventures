# mini-redis

Mini Redis TCP server.

## What is this?

This program composes:

- `tcp-server` for the current TCP transport loop
- `resp-protocol` for Redis-compatible framing
- `in-memory-data-store` for command execution and background expiry

It speaks enough Redis-compatible RESP over TCP for local development and
end-to-end testing with normal socket clients.

## Development

```bash
cargo test -p mini-redis -- --nocapture
```
