# transport-platform (Rust)

Runtime-facing seam between higher transport runtimes and the native provider
that offers listeners, streams, timers, wakeups, and event delivery.

## What is this?

This crate sits below `stream-reactor` and `tcp-runtime`, and above raw
platform mechanics like `kqueue`, `epoll`, `WSAPoll`, sockets, and timer
registration.

The current providers are:

- `bsd::KqueueTransportPlatform` for macOS and BSD
- `linux::EpollTransportPlatform` for Linux
- `windows::WindowsTransportPlatform` for Windows

The Windows provider is intentionally phase one. It uses nonblocking sockets,
`WSAPoll`, loopback wakeup sockets, and user-space timers today so the seam is
usable immediately. A fuller IOCP-backed provider can replace that internals
later without changing the public contract.

All three providers expose one repository-owned contract for:

- binding listeners
- accepting streams
- reading and writing bytes
- arming timers
- creating wakeups
- polling normalized platform events

That keeps Redis, IRC, and future protocol runtimes from depending directly on
OS-specific event structures.

## Why it matters

This is the boundary that should let higher transport runtimes survive a future
move from host-kernel sockets to a library-OS or unikernel-style substrate.
Upper layers see listener IDs, stream IDs, timers, wakeups, and normalized
events instead of raw file descriptors or Win32 socket handles.

## Development

```bash
cargo test -p transport-platform
cargo check -p transport-platform --tests --target x86_64-unknown-linux-gnu
cargo check -p transport-platform --tests --target x86_64-pc-windows-msvc
```
