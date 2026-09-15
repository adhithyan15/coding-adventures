---
category: Cross-platform & Windows BUILD_windows
---

# Unix-only syscalls (`syscall.Stat_t`, `libc::getuid`, `libc::statvfs`) won't compile on Windows CI

Go: split with `//go:build !windows` / `windows` and provide stubs. Rust: `#[cfg(unix)]` / `#[cfg(not(unix))]`.
