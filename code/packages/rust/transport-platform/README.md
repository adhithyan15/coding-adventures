# transport-platform (Rust)

Runtime-facing seam between higher transport runtimes and the native provider
that offers listeners, streams, timers, wakeups, and event delivery.

## What is this?

This crate sits below `stream-reactor` and `tcp-runtime`, and above raw
platform mechanics like `kqueue`, sockets, and timer registration.

The first implementation target is a `kqueue`-backed provider for macOS/BSD.
It gives upper layers one repository-owned contract for:

- binding listeners
- accepting streams
- reading and writing bytes
- arming timers
- creating wakeups
- polling normalized platform events

That keeps Redis, IRC, and future protocol runtimes from depending directly on
OS-specific event structures.

## Development

```bash
cargo test -p transport-platform
```
