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

The Windows provider uses IOCP with overlapped stream operations and AcceptEx.
Timers remain managed in user space. Cross-thread wake_handle is not available
on Windows; reactors retain their configured finite polling fallback.

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


## Bounded spacing and transport reconciliation

Windows IOCP teardown closes stream/listener, pending AcceptEx and completed-but-unadopted sockets, then retains operation buffers and the completion port until matching terminal packets arrive. A fatal wait error deliberately retains outstanding storage for process lifetime; it is not successful completion. AcceptEx uses its BOOL return convention and records only successful or pending submissions.

Validation is source-attributed: Windows applicable packages and standalone
checks are recorded separately from Unix-only/hosted and whole-PR gates.
Final-source Miri is **DEFERRED/PENDING**, never passed.
