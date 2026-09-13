---
category: Cross-platform & Windows BUILD_windows
---

# Swift POSIX `bind` collides with `Sequence.bind`

inside closures. Wrap POSIX calls at module scope: `posixBind` → `Darwin.bind`/`Glibc.bind` via `#if canImport`. `SOCK_STREAM` is `Int32` on Darwin but a `__socket_type` enum on Glibc — use `Int32(SOCK_STREAM.rawValue)` under `#elseif canImport(Glibc)`.
